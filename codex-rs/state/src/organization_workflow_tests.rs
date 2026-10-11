use super::*;
use codex_protocol::organization_coordination::OrganizationPlanItem;
use codex_protocol::organization_workflow::*;

async fn fixture(store: &OrganizationStore) -> Result<OrganizationWorkflowTemplate> {
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Workflow agent", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(store, &node.id).await?;
    let items = vec![
        OrganizationPlanItem {
            key: "write".into(),
            owner_id: node.id.clone(),
            instructions: "Write evidence".into(),
            criteria: vec!["Cited".into()],
            prerequisites: vec![],
        },
        OrganizationPlanItem {
            key: "check".into(),
            owner_id: node.id,
            instructions: "Check evidence".into(),
            criteria: vec!["Verified".into()],
            prerequisites: vec!["write".into()],
        },
    ];
    let OrganizationWorkflowView::Template { template } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: Uuid::now_v7().to_string(),
                expected_revision: None,
                project_id: project,
                label: "Evidence café".into(),
                items,
            },
            1000,
        )
        .await?
    else {
        panic!("expected template")
    };
    Ok(template)
}

fn request(template: &OrganizationWorkflowTemplate) -> OrganizationWorkflowPreviewRequest {
    OrganizationWorkflowPreviewRequest {
        id: Uuid::now_v7().to_string(),
        template_id: template.id.clone(),
        expected_revision: template.revision,
        shared_limits: None,
        scopes: template
            .items
            .iter()
            .map(|item| OrganizationWorkflowScope {
                key: item.key.clone(),
                scope: execution_scope(),
            })
            .collect(),
    }
}

fn series_config(runs: u32) -> codex_protocol::organization_schedule::OrganizationScheduleConfig {
    codex_protocol::organization_schedule::OrganizationScheduleConfig {
        label: "Finite workflow".into(),
        interval_seconds: 10,
        first_run_at: 1001,
        ends_at: 1100,
        max_runs: runs,
        max_tokens: 200,
        max_model_requests: 3,
    }
}

#[tokio::test]
async fn workflow_series_skip_overlap_cancel_preserve_usage_and_reject_stale_controls() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let preview = store.preview_workflow(request(&template)).await?;
    let id = Uuid::now_v7().to_string();
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::CreateSeries {
                id: id.clone(),
                preview: Box::new(preview),
                config: series_config(3),
                trigger: OrganizationWorkflowTrigger::Interval,
            },
            1000,
        )
        .await?;
    assert_eq!(store.materialize_workflow_series(1001).await?, 1);
    assert_eq!(store.materialize_workflow_series(1001).await?, 0);
    assert_eq!(
        store.materialize_workflow_series(1011).await?,
        0,
        "unfinished invocation prevents overlap"
    );
    let (series, _) = store
        .list_workflows(OrganizationWorkflowKind::Series, None, None, 10)
        .await?;
    let OrganizationWorkflowView::Series { series, .. } = &series[0] else {
        panic!("series")
    };
    assert_eq!(series.issued_runs, 1);
    assert_eq!(series.skipped_runs, 1);
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1012, 2)
        .await?
        .unwrap();
    let (sequence, _) = store
        .admit_model_request(&attempt.id, &worker, 1012)
        .await?;
    store
        .checkpoint_usage(&attempt.id, &worker, sequence, 15)
        .await?;
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::ControlSeries {
                    id: id.clone(),
                    expected_revision: 1,
                    action: OrganizationWorkflowControl::Cancel
                },
                1012
            )
            .await
            .is_err()
    );
    let original = store
        .execution_view(&attempt.task_id)
        .await?
        .authorization
        .unwrap();
    let OrganizationWorkflowView::Series {
        series: paused,
        usage,
    } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::ControlSeries {
                id: id.clone(),
                expected_revision: series.revision,
                action: OrganizationWorkflowControl::Pause,
            },
            1012,
        )
        .await?
    else {
        panic!("series")
    };
    assert_eq!((usage.model_requests, usage.reported_tokens), (1, 15));
    assert_eq!(
        store
            .execution_view(&attempt.task_id)
            .await?
            .authorization
            .unwrap(),
        original,
        "pausing future invocations keeps current work"
    );
    assert_eq!(store.materialize_workflow_series(1030).await?, 0);
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::ControlSeries {
                id,
                expected_revision: paused.revision,
                action: OrganizationWorkflowControl::Cancel,
            },
            1030,
        )
        .await?;
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.task.state,
        "cancelled"
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    let (series, _) = store
        .list_workflows(OrganizationWorkflowKind::Series, None, None, 10)
        .await?;
    assert!(
        matches!(&series[0],OrganizationWorkflowView::Series {series,usage} if series.state==codex_protocol::organization_schedule::OrganizationScheduleState::Cancelled && usage.model_requests==1 && usage.reported_tokens==15)
    );
    Ok(())
}

#[tokio::test]
async fn accepted_trigger_waits_for_exact_acceptance_deduplicates_and_imports_only_scoped_artifacts()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let source = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    let grant = store
        .authorize_task(&source.id, source.revision, execution_scope(), 1000)
        .await?;
    let mut q = request(&template);
    q.scopes[0].scope.input_files.push("report.txt".into());
    let preview = store.preview_workflow(q).await?;
    let id = Uuid::now_v7().to_string();
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::CreateSeries {
                id: id.clone(),
                preview: Box::new(preview),
                config: series_config(1),
                trigger: OrganizationWorkflowTrigger::Accepted {
                    task_id: source.id.clone(),
                    authorization_id: grant.id.clone(),
                },
            },
            1000,
        )
        .await?;
    assert_eq!(store.materialize_workflow_series(1001).await?, 0);
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 2)
        .await?
        .unwrap();
    let result = store
        .record_result(
            &attempt.id,
            &worker,
            vec![OrganizationArtifactEvidence {
                path: "report.txt".into(),
                sha256: "a".repeat(64),
                bytes: 3,
            }],
            vec![OrganizationCheckEvidence {
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "state fixture".into(),
                truncated: false,
            }],
            1002,
        )
        .await?;
    assert_eq!(
        store.materialize_workflow_series(1006).await?,
        0,
        "needsReview is not acceptance"
    );
    let v = store.execution_view(&source.id).await?;
    store
        .accept_result(&source.id, v.task.revision, &result.digest)
        .await?;
    let (one, two) = tokio::join!(
        store.materialize_workflow_series(1011),
        store.materialize_workflow_series(1011)
    );
    assert_eq!(one? + two?, 1, "concurrent polls issue only one follow-up");
    assert_eq!(store.materialize_workflow_series(1012).await?, 0);
    let (runs, _) = store
        .list_workflows(OrganizationWorkflowKind::Runs, None, None, 10)
        .await?;
    let OrganizationWorkflowView::Run { run, .. } = &runs[0] else {
        panic!("run")
    };
    let inputs = store.workflow_inputs(&run.task_ids["write"]).await?;
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].0.digest, result.digest);
    assert_eq!(inputs[0].0.artifacts, result.artifacts);
    let (series, _) = store
        .list_workflows(OrganizationWorkflowKind::Series, None, None, 10)
        .await?;
    assert!(
        matches!(&series[0],OrganizationWorkflowView::Series {series,usage} if series.issued_runs==1 && series.source_result_digest.as_deref()==Some(result.digest.as_str()) && usage.model_requests==0)
    );
    Ok(())
}

#[tokio::test]
async fn series_scope_drift_and_unknown_usage_block_restart_without_replay() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let preview = store.preview_workflow(request(&template)).await?;
    let id = Uuid::now_v7().to_string();
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::CreateSeries {
                id: id.clone(),
                preview: Box::new(preview),
                config: series_config(2),
                trigger: OrganizationWorkflowTrigger::Interval,
            },
            1000,
        )
        .await?;
    assert_eq!(store.materialize_workflow_series(1001).await?, 1);
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 2)
        .await?
        .unwrap();
    store
        .admit_model_request(&attempt.id, &worker, 1002)
        .await?;
    store.reconcile_interrupted_attempts().await?;
    store.reconcile_workflow_series(1020).await?;
    let (series, _) = store
        .list_workflows(OrganizationWorkflowKind::Series, None, None, 10)
        .await?;
    let OrganizationWorkflowView::Series { series, usage } = &series[0] else {
        panic!("series")
    };
    assert_eq!(
        series.state,
        codex_protocol::organization_schedule::OrganizationScheduleState::Blocked
    );
    assert_eq!(usage.model_requests, 1);
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::ControlSeries {
                    id,
                    expected_revision: series.revision,
                    action: OrganizationWorkflowControl::Resume
                },
                1020
            )
            .await
            .is_err()
    );
    assert_eq!(store.materialize_workflow_series(1020).await?, 0);
    let template2 = fixture(&store).await?;
    let preview = store.preview_workflow(request(&template2)).await?;
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::CreateSeries {
                id: Uuid::now_v7().to_string(),
                preview: Box::new(preview),
                config: series_config(2),
                trigger: OrganizationWorkflowTrigger::Interval,
            },
            1000,
        )
        .await?;
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: template2.id.clone(),
                expected_revision: Some(template2.revision),
                project_id: template2.project_id,
                label: "Changed pinned template".into(),
                items: template2.items,
            },
            1000,
        )
        .await?;
    assert_eq!(
        store.materialize_workflow_series(1001).await?,
        0,
        "changed definitions cannot silently widen future grants"
    );
    Ok(())
}

async fn control(
    store: &OrganizationStore,
    run: &OrganizationWorkflowRun,
    action: OrganizationWorkflowControl,
) -> Result<OrganizationWorkflowRun> {
    let OrganizationWorkflowView::Run { run, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Control {
                id: run.id.clone(),
                expected_revision: run.revision,
                action,
            },
            1002,
        )
        .await?
    else {
        panic!("expected run")
    };
    Ok(run)
}

#[tokio::test]
async fn whole_run_controls_persist_reject_stale_versions_and_never_reset_grants() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let preview = store.preview_workflow(request(&template)).await?;
    let OrganizationWorkflowView::Run { run, tasks } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("expected run")
    };
    let mut grants = Vec::new();
    for task in &tasks {
        grants.push(store.execution_view(&task.id).await?);
    }
    let paused = control(&store, &run, OrganizationWorkflowControl::Pause).await?;
    assert_eq!(paused.state, OrganizationWorkflowState::Paused);
    assert!(
        control(&store, &run, OrganizationWorkflowControl::Resume)
            .await
            .is_err()
    );
    let worker = Uuid::now_v7().to_string();
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 2)
            .await?
            .is_none()
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    let (views, _) = store
        .list_workflows(OrganizationWorkflowKind::Runs, None, None, 100)
        .await?;
    let OrganizationWorkflowView::Run { run: persisted, .. } = &views[0] else {
        panic!("expected run")
    };
    assert_eq!(persisted, &paused);
    let resumed = control(&store, persisted, OrganizationWorkflowControl::Resume).await?;
    for (task, before) in tasks.iter().zip(&grants) {
        let after = store.execution_view(&task.id).await?;
        assert_eq!(after.authorization, before.authorization);
        assert_eq!(after.task, before.task);
    }
    let cancelled = control(&store, &resumed, OrganizationWorkflowControl::Cancel).await?;
    assert_eq!(cancelled.state, OrganizationWorkflowState::Cancelled);
    assert!(
        control(&store, &cancelled, OrganizationWorkflowControl::Resume)
            .await
            .is_err()
    );
    for task in &tasks {
        assert_eq!(
            store.execution_view(&task.id).await?.task.state,
            "cancelled"
        );
    }
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 2)
            .await?
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn paused_running_work_drains_usage_releases_capacity_and_resumes_the_same_attempt()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let preview = store.preview_workflow(request(&template)).await?;
    let OrganizationWorkflowView::Run { run, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("expected run")
    };
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 1)
        .await?
        .unwrap();
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    let paused = control(&store, &run, OrganizationWorkflowControl::Pause).await?;
    assert!(
        !store
            .workflow_dispatch_ready(&attempt.id, &worker, 1002, 1)
            .await?
    );
    assert!(
        store.execution_view(&attempt.task_id).await?.attempts[0].runtime_active,
        "in-flight usage still reserves capacity"
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 15).await?;
    assert!(
        !store
            .workflow_dispatch_ready(&attempt.id, &worker, 1002, 1)
            .await?
    );
    assert!(!store.execution_view(&attempt.task_id).await?.attempts[0].runtime_active);
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .unwrap_err()
            .is::<crate::organization::WorkflowPaused>()
    );
    assert!(
        store
            .prepare_action(&attempt.id, &worker, "paused", "exec_command", "{}", 1002)
            .await
            .is_err()
    );
    store.heartbeat_attempt(&attempt.id, &worker, 1002).await?;
    let other = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    store
        .authorize_task(&other.id, other.revision, execution_scope(), 1002)
        .await?;
    let unrelated = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 1)
        .await?
        .unwrap();
    assert_eq!(unrelated.task_id, other.id);
    let resumed = control(&store, &paused, OrganizationWorkflowControl::Resume).await?;
    assert!(
        !store
            .workflow_dispatch_ready(&attempt.id, &worker, 1002, 1)
            .await?,
        "resume must reacquire capacity"
    );
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    let current = store.execution_view(&other.id).await?;
    store
        .stop_task(&other.id, current.task.revision, true)
        .await?;
    store
        .release_attempt_capacity(&unrelated.id, &worker)
        .await?;
    assert!(
        store
            .workflow_dispatch_ready(&attempt.id, &worker, 1002, 1)
            .await?
    );
    assert_eq!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await?,
        (2, 15)
    );
    store.checkpoint_usage(&attempt.id, &worker, 2, 20).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(
        view.authorization.as_ref().unwrap().id,
        attempt.authorization_id
    );
    assert_eq!(view.authorization_used_tokens, 20);
    let paused = control(&store, &resumed, OrganizationWorkflowControl::Pause).await?;
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.reconcile_interrupted_attempts().await?, 1);
    assert!(
        control(&store, &paused, OrganizationWorkflowControl::Resume)
            .await
            .is_err(),
        "restart must not replay the interrupted attempt"
    );
    Ok(())
}

#[tokio::test]
async fn workflow_graph_validation_and_late_authorization_failure_roll_back() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    for invalid in ["cycle", "unknown", "duplicate"] {
        let mut items = template.items.clone();
        match invalid {
            "cycle" => items[0].prerequisites = vec!["check".into()],
            "unknown" => items[1].prerequisites = vec!["outside-task-id".into()],
            _ => items[1].key = items[0].key.clone(),
        }
        assert!(
            store
                .mutate_workflow(
                    OrganizationWorkflowMutation::Put {
                        id: template.id.clone(),
                        expected_revision: Some(1),
                        project_id: template.project_id.clone(),
                        label: "Invalid".into(),
                        items
                    },
                    1000
                )
                .await
                .is_err()
        );
    }
    assert_eq!(
        store
            .list_workflows(OrganizationWorkflowKind::Templates, None, None, 100)
            .await?
            .0,
        vec![OrganizationWorkflowView::Template {
            template: template.clone()
        }]
    );
    let mut input = request(&template);
    input.scopes[1].scope.not_before = Some(-1); // Fails only when the second ordinary grant is authorized.
    let preview = store.preview_workflow(input).await?;
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::Run {
                    preview: Box::new(preview)
                },
                1000
            )
            .await
            .is_err()
    );
    for query in [
        "SELECT count(*) FROM organization_tasks",
        "SELECT count(*) FROM organization_authorizations",
        "SELECT count(*) FROM organization_workflow_runs",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&store.pool)
                .await?,
            0
        );
    }
    Ok(())
}

#[tokio::test]
async fn terminated_unknown_usage_blocks_shared_work_until_a_late_receipt_reconciles_it()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::SetLimits {
                project_id: template.project_id.clone(),
                run_id: None,
                expected_revision: None,
                max_tokens: 100,
                max_model_requests: 3,
                max_active: 2,
            },
            1000,
        )
        .await?;
    let first = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    let other = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    for task in [&first, &other] {
        store
            .authorize_task(&task.id, task.revision, execution_scope(), 1000)
            .await?;
    }
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store
        .stop_task(
            &attempt.task_id,
            store.execution_view(&attempt.task_id).await?.task.revision,
            true,
        )
        .await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none(),
        "unreported terminated usage cannot be treated as free capacity"
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 15).await?;
    assert_eq!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .unwrap()
            .task_id,
        other.id
    );
    let (limits, _) = store
        .list_workflows(OrganizationWorkflowKind::Limits, None, None, 100)
        .await?;
    assert!(
        matches!(&limits[0],OrganizationWorkflowView::Limit { usage,.. } if usage.reported_tokens==15 && usage.model_requests==1)
    );
    Ok(())
}

#[tokio::test]
async fn concurrent_workflow_requests_reserve_the_last_shared_slot_once() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let mut template = fixture(&store).await?;
    template.items[1].prerequisites.clear();
    let OrganizationWorkflowView::Template { template } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: template.id,
                expected_revision: Some(template.revision),
                project_id: template.project_id,
                label: template.label,
                items: template.items,
            },
            1000,
        )
        .await?
    else {
        panic!("template")
    };
    let mut input = request(&template);
    input.shared_limits = Some(OrganizationSharedCeilings {
        max_tokens: 100,
        max_model_requests: 1,
        max_active: 2,
    });
    let preview = store.preview_workflow(input).await?;
    let OrganizationWorkflowView::Run { run, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("run")
    };
    let worker = Uuid::now_v7().to_string();
    let first = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    let second = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::SetLimits {
                    project_id: template.project_id.clone(),
                    run_id: Some(run.id),
                    expected_revision: Some(1),
                    max_tokens: 100,
                    max_model_requests: 1,
                    max_active: 1
                },
                1001
            )
            .await
            .is_err()
    );
    let (left, right) = tokio::join!(
        store.admit_model_request(&first.id, &worker, 1001),
        store.admit_model_request(&second.id, &worker, 1001)
    );
    assert_eq!(u32::from(left.is_ok()) + u32::from(right.is_ok()), 1);
    let (limits, _) = store
        .list_workflows(OrganizationWorkflowKind::Limits, None, None, 100)
        .await?;
    assert!(
        matches!(&limits[0], OrganizationWorkflowView::Limit { limit, usage } if limit.revision==1 && usage.model_requests==1 && usage.active==2)
    );
    Ok(())
}

#[tokio::test]
async fn paused_and_exhausted_queues_cannot_hide_unrelated_work_behind_candidate_limit()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let owner = template.items[0].owner_id.clone();
    let items = (0..32)
        .map(|index| OrganizationPlanItem {
            key: format!("step-{index}"),
            owner_id: owner.clone(),
            instructions: "Bounded work".into(),
            criteria: vec!["Checked".into()],
            prerequisites: vec![],
        })
        .collect();
    let OrganizationWorkflowView::Template { template } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: template.id,
                expected_revision: Some(1),
                project_id: template.project_id,
                label: template.label,
                items,
            },
            1000,
        )
        .await?
    else {
        panic!("template")
    };
    let preview = store.preview_workflow(request(&template)).await?;
    let OrganizationWorkflowView::Run { run, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("run")
    };
    control(&store, &run, OrganizationWorkflowControl::Pause).await?;
    let task = execution_task(&store, &owner, &template.project_id).await?;
    store
        .authorize_task(&task.id, task.revision, execution_scope(), 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 4)
        .await?
        .unwrap();
    assert_eq!(attempt.task_id, task.id);
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 10).await?;
    store
        .stop_task(
            &task.id,
            store.execution_view(&task.id).await?.task.revision,
            true,
        )
        .await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    for _ in 0..32 {
        let task = execution_task(&store, &owner, &template.project_id).await?;
        store
            .authorize_task(&task.id, task.revision, execution_scope(), 1001)
            .await?;
    }
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::SetLimits {
                project_id: template.project_id,
                run_id: None,
                expected_revision: None,
                max_tokens: 100,
                max_model_requests: 1,
                max_active: 4,
            },
            1001,
        )
        .await?;
    let other_project = execution_project(&store, &owner).await?;
    let other = execution_task(&store, &owner, &other_project).await?;
    store
        .authorize_task(&other.id, other.revision, execution_scope(), 1001)
        .await?;
    assert_eq!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .unwrap()
            .task_id,
        other.id
    );
    Ok(())
}

#[tokio::test]
async fn atomic_workflow_shared_limits_bound_concurrency_and_inference_without_reset() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let mut template = fixture(&store).await?;
    template.items[1].prerequisites.clear();
    let OrganizationWorkflowView::Template { template } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: template.id,
                expected_revision: Some(template.revision),
                project_id: template.project_id,
                label: template.label,
                items: template.items,
            },
            1000,
        )
        .await?
    else {
        panic!("template")
    };
    let mut input = request(&template);
    input.shared_limits = Some(OrganizationSharedCeilings {
        max_tokens: 100,
        max_model_requests: 1,
        max_active: 1,
    });
    let preview = store.preview_workflow(input).await?;
    let OrganizationWorkflowView::Run { run, tasks } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("run")
    };
    let worker = Uuid::now_v7().to_string();
    let first = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
            .await?
            .is_none(),
        "the run cap applies before either step starts"
    );
    store.admit_model_request(&first.id, &worker, 1001).await?;
    store.checkpoint_usage(&first.id, &worker, 1, 10).await?;
    store
        .stop_task(
            &first.task_id,
            store.execution_view(&first.task_id).await?.task.revision,
            true,
        )
        .await?;
    store.release_attempt_capacity(&first.id, &worker).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none(),
        "another root cannot bypass the shared request cap"
    );
    let OrganizationWorkflowView::Limit { limit, usage } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::SetLimits {
                project_id: template.project_id.clone(),
                run_id: Some(run.id.clone()),
                expected_revision: Some(1),
                max_tokens: 100,
                max_model_requests: 2,
                max_active: 1,
            },
            1002,
        )
        .await?
    else {
        panic!("limits")
    };
    assert_eq!((usage.model_requests, usage.reported_tokens), (1, 10));
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::SetLimits {
                    project_id: template.project_id.clone(),
                    run_id: Some(run.id),
                    expected_revision: Some(1),
                    max_tokens: 100,
                    max_model_requests: 3,
                    max_active: 1,
                },
                1002
            )
            .await
            .is_err(),
        "a stale edit cannot expand the cap"
    );
    let second = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    assert!(tasks.iter().any(|task| task.id == second.task_id));
    store.admit_model_request(&second.id, &worker, 1002).await?;
    store.checkpoint_usage(&second.id, &worker, 1, 15).await?;
    assert!(
        store
            .admit_model_request(&second.id, &worker, 1002)
            .await
            .is_err()
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    let (views, _) = store
        .list_workflows(OrganizationWorkflowKind::Limits, None, None, 100)
        .await?;
    assert!(
        matches!(&views[0],OrganizationWorkflowView::Limit { limit:stored,usage } if stored==&limit && usage.reported_tokens==25 && usage.model_requests==2)
    );
    Ok(())
}

#[tokio::test]
async fn project_shared_limits_retain_old_grants_and_allow_only_reported_inflight_overrun()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::SetLimits {
                project_id: template.project_id.clone(),
                run_id: None,
                expected_revision: None,
                max_tokens: 5,
                max_model_requests: 2,
                max_active: 1,
            },
            1000,
        )
        .await?;
    let first = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    let other = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    for task in [&first, &other] {
        store
            .authorize_task(&task.id, task.revision, execution_scope(), 1000)
            .await?;
    }
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
            .await?
            .is_none()
    );
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 9).await?;
    assert!(
        store
            .prepare_action(&attempt.id, &worker, "over-cap", "exec_command", "{}", 1001)
            .await
            .is_err()
    );
    store
        .stop_task(
            &attempt.task_id,
            store.execution_view(&attempt.task_id).await?.task.revision,
            false,
        )
        .await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    store
        .authorize_task(
            &attempt.task_id,
            store.execution_view(&attempt.task_id).await?.task.revision,
            execution_scope(),
            1002,
        )
        .await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none(),
        "new root grants do not reset project consumption"
    );
    let OrganizationWorkflowView::Limit { usage, .. } = store
        .list_workflows(
            OrganizationWorkflowKind::Limits,
            Some(&template.project_id),
            None,
            100,
        )
        .await?
        .0
        .remove(0)
    else {
        panic!("limit")
    };
    assert_eq!(
        (usage.reported_tokens, usage.model_requests, usage.active),
        (9, 1, 0)
    );
    for (tokens, requests, active) in [(0, 1, 1), (1, 0, 1), (1, 1, 0), (1, 1, 5)] {
        assert!(
            store
                .mutate_workflow(
                    OrganizationWorkflowMutation::SetLimits {
                        project_id: template.project_id.clone(),
                        run_id: None,
                        expected_revision: Some(1),
                        max_tokens: tokens,
                        max_model_requests: requests,
                        max_active: active
                    },
                    1002
                )
                .await
                .is_err()
        );
    }
    Ok(())
}

#[tokio::test]
async fn workflow_preview_pins_scope_and_revisions_without_creating_authority() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let input = request(&template);
    for invalid in ["missing", "extra", "duplicate", "immutable", "path"] {
        let mut input = input.clone();
        match invalid {
            "missing" => {
                input.scopes.pop();
            }
            "extra" => input.scopes.push(OrganizationWorkflowScope {
                key: "third".into(),
                scope: execution_scope(),
            }),
            "duplicate" => input.scopes[1].key = input.scopes[0].key.clone(),
            "immutable" => {
                input.scopes[0].scope.artifacts = vec!["oracle.py".into()];
            }
            _ => input.scopes[0].scope.artifacts = vec!["../secret".into()],
        }
        assert!(store.preview_workflow(input).await.is_err(), "{invalid}");
    }
    let mut items = template.items.clone();
    let mut parallel = items[0].clone();
    parallel.key = "parallel".into();
    items[1].prerequisites.push(parallel.key.clone());
    items.push(parallel);
    let OrganizationWorkflowView::Template {
        template: ambiguous,
    } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: Uuid::now_v7().to_string(),
                expected_revision: None,
                project_id: template.project_id.clone(),
                label: "Ambiguous producers".into(),
                items,
            },
            1000,
        )
        .await?
    else {
        panic!("expected template")
    };
    let mut ambiguous_input = request(&ambiguous);
    ambiguous_input
        .scopes
        .iter_mut()
        .find(|step| step.key == "check")
        .unwrap()
        .scope
        .input_files
        .push("report.txt".into());
    assert!(
        store.preview_workflow(ambiguous_input).await.is_err(),
        "multiple producers cannot overwrite the same imported input"
    );
    let preview = store.preview_workflow(input).await?;
    assert_eq!((preview.max_tokens, preview.max_model_requests), (200, 4));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM organization_authorizations")
            .fetch_one(&store.pool)
            .await?,
        0
    );
    let mut tampered = preview.clone();
    tampered.max_tokens += 1;
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::Run {
                    preview: Box::new(tampered)
                },
                1000
            )
            .await
            .is_err()
    );
    let node = preview.template.items[0].owner_id.clone();
    store
        .mutate_owner(
            OrganizationMutation::PutNode {
                id: Some(node),
                expected_revision: Some(1),
                parent_id: None,
                kind: OrganizationNodeKind::Agent,
                label: "Changed role".into(),
                role_version_id: None,
                model: None,
                enabled: true,
            },
            None,
        )
        .await?;
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::Run {
                    preview: Box::new(preview)
                },
                1000
            )
            .await
            .is_err()
    );
    let input = request(&template);
    let preview = store.preview_workflow(input).await?;
    store
        .mutate_workflow(
            OrganizationWorkflowMutation::Put {
                id: template.id,
                expected_revision: Some(1),
                project_id: template.project_id,
                label: "Updated".into(),
                items: template.items,
            },
            1000,
        )
        .await?;
    assert!(
        store
            .mutate_workflow(
                OrganizationWorkflowMutation::Run {
                    preview: Box::new(preview)
                },
                1000
            )
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn workflow_runs_are_distinct_restart_durable_and_wait_for_exact_acceptance() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let mut input = request(&template);
    input.scopes[1].scope.input_files.push("report.txt".into());
    input.scopes[1].scope.artifacts = vec!["verified.txt".into()];
    let preview = store.preview_workflow(input).await?;
    let mutation = OrganizationWorkflowMutation::Run {
        preview: Box::new(preview),
    };
    let OrganizationWorkflowView::Run { run, tasks } =
        store.mutate_workflow(mutation.clone(), 1000).await?
    else {
        panic!("expected run")
    };
    assert!(store.mutate_workflow(mutation, 1000).await.is_err());
    assert_eq!(tasks[1].prerequisites, vec![tasks[0].id.clone()]);
    assert!(store.workflow_inputs(&tasks[1].id).await.is_err());
    let worker = Uuid::now_v7().to_string();
    let first = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    assert_eq!(first.task_id, tasks[0].id);
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 4)
            .await?
            .is_none()
    );
    store
        .bind_attempt_runtime(
            &first.id,
            &worker,
            &OrganizationRuntimeBinding {
                workspace: "/tmp/workflow-fixture".into(),
                turn_id: None,
                artifact_bases: vec![OrganizationArtifactBase {
                    path: "report.txt".into(),
                    sha256: None,
                }],
            },
            1001,
        )
        .await?;
    for _ in 0..2 {
        let (sequence, _) = store.admit_model_request(&first.id, &worker, 1001).await?;
        store
            .checkpoint_usage(&first.id, &worker, sequence, 1)
            .await?;
    }
    assert!(
        store
            .admit_model_request(&first.id, &worker, 1001)
            .await
            .is_err(),
        "workflow grants retain request ceilings"
    );
    let result = store
        .record_result(
            &first.id,
            &worker,
            vec![OrganizationArtifactEvidence {
                path: "report.txt".into(),
                sha256: "a".repeat(64),
                bytes: 3,
            }],
            vec![OrganizationCheckEvidence {
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "state fixture".into(),
                truncated: false,
            }],
            1002,
        )
        .await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 4)
            .await?
            .is_none()
    );
    let current = store.execution_view(&first.task_id).await?;
    assert!(
        store
            .accept_result(&first.task_id, current.task.revision, "wrong")
            .await
            .is_err()
    );
    store
        .accept_result(&first.task_id, current.task.revision, &result.digest)
        .await?;
    let inherited = store.workflow_inputs(&tasks[1].id).await?;
    assert_eq!(inherited.len(), 1);
    assert_eq!(inherited[0].0.artifacts, result.artifacts);
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    let second = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1004, 4)
        .await?
        .unwrap();
    assert_eq!(second.task_id, tasks[1].id);
    let mut input = request(&template);
    input.scopes[0].scope.not_before = Some(2000);
    input.scopes[1].scope.not_before = Some(2000);
    let preview = store.preview_workflow(input).await?;
    let OrganizationWorkflowView::Run { run: other, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1004,
        )
        .await?
    else {
        panic!("expected run")
    };
    assert!(
        other
            .task_ids
            .values()
            .all(|id| !run.task_ids.values().any(|prior| prior == id))
    );
    store
        .stop_task(
            &tasks[1].id,
            store.execution_view(&tasks[1].id).await?.task.revision,
            true,
        )
        .await?;
    let current = store.execution_view(&tasks[1].id).await?;
    assert!(
        store
            .authorize_task(&tasks[1].id, current.task.revision, execution_scope(), 1005)
            .await
            .is_err()
    );
    let (page, next) = store
        .list_workflows(
            OrganizationWorkflowKind::Runs,
            Some(&template.project_id),
            None,
            1,
        )
        .await?;
    assert_eq!(page.len(), 1);
    assert!(next.is_some());
    assert_eq!(
        store
            .list_workflows(OrganizationWorkflowKind::Runs, None, next.as_deref(), 1)
            .await?
            .0
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn workflow_migration_from_seven_preserves_existing_tasks_and_crash_blocks_dependents()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let old = execution_task(&store, &template.items[0].owner_id, &template.project_id).await?;
    sqlx::query("DROP VIEW organization_shared_usage")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP VIEW organization_project_run_usage")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_occurrences")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_shared_limits")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_series")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_handoffs")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_checkpoints")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_tasks")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_runs")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_templates")
        .execute(&store.pool)
        .await?;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version>=8")
        .execute(&store.pool)
        .await?;
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.execution_view(&old.id).await?.task, old);
    let template = fixture(&store).await?;
    let preview = store.preview_workflow(request(&template)).await?;
    let OrganizationWorkflowView::Run { tasks, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("expected run")
    };
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    assert_eq!(attempt.task_id, tasks[0].id);
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.reconcile_interrupted_attempts().await?, 1);
    assert_eq!(
        store.execution_view(&tasks[0].id).await?.task.state,
        "needsReconciliation"
    );
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none()
    );
    assert!(store.workflow_inputs(&tasks[1].id).await.is_err());
    assert!(
        store
            .authorize_task(
                &tasks[0].id,
                store.execution_view(&tasks[0].id).await?.task.revision,
                execution_scope(),
                1002
            )
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn reports_link_delegated_attempts_to_the_original_workflow_run() -> Result<()> {
    use codex_protocol::organization_coordination::OrganizationCoordinationCall;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let template = fixture(&store).await?;
    let helper = node_from(
        store
            .mutate_owner(
                node_mutation("Custom delegate", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let mut input = request(&template);
    input.scopes[0].scope.delegates = vec![helper.id.clone()];
    input.scopes[0].scope.max_children = 1;
    input.scopes[0].scope.max_model_requests = 8;
    input.scopes[0].scope.require_tool_approval = false;
    input.shared_limits = Some(OrganizationSharedCeilings {
        max_tokens: 100,
        max_model_requests: 1,
        max_active: 2,
    });
    input.scopes[1].scope.artifacts = vec!["verified.txt".into()];
    let preview = store.preview_workflow(input).await?;
    let OrganizationWorkflowView::Run { run, .. } = store
        .mutate_workflow(
            OrganizationWorkflowMutation::Run {
                preview: Box::new(preview),
            },
            1000,
        )
        .await?
    else {
        panic!("expected run")
    };
    let worker = Uuid::now_v7().to_string();
    let root = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    let tasks: Vec<OrganizationTask> = serde_json::from_value(
        store
            .coordinate(
                &root.id,
                &worker,
                "plan",
                OrganizationCoordinationCall::ProposePlan {
                    items: vec![OrganizationPlanItem {
                        key: "helper".into(),
                        owner_id: helper.id.clone(),
                        instructions: "Prepare the granted report".into(),
                        criteria: vec!["Verified".into()],
                        prerequisites: vec![],
                    }],
                },
                1001,
            )
            .await?,
    )?;
    store
        .coordinate(
            &root.id,
            &worker,
            "delegate",
            OrganizationCoordinationCall::Delegate {
                task_id: tasks[0].id.clone(),
            },
            1001,
        )
        .await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    store.admit_model_request(&child.id, &worker, 1002).await?;
    store.checkpoint_usage(&child.id, &worker, 1, 15).await?;
    assert!(
        store
            .admit_model_request(&root.id, &worker, 1002)
            .await
            .is_err(),
        "delegated requests consume the same run pool"
    );
    let (limits, _) = store
        .list_workflows(OrganizationWorkflowKind::Limits, None, None, 100)
        .await?;
    assert!(
        matches!(&limits[0], OrganizationWorkflowView::Limit { usage, .. } if usage.model_requests==1 && usage.reported_tokens==15),
        "root plus descendant ledgers count once"
    );
    let report = store
        .operational_report(Some(&template.project_id), None, 200, 1002)
        .await?;
    let delegated = report
        .tasks
        .iter()
        .find(|task| task.task_id == child.task_id)
        .unwrap();
    assert_eq!(delegated.workflow_run_id.as_deref(), Some(run.id.as_str()));
    assert_eq!(
        report
            .usage
            .iter()
            .find(|usage| usage.task_id == child.task_id)
            .unwrap()
            .workflow_run_id
            .as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(report.totals.reported_tokens, 15);
    assert_eq!(
        report
            .usage
            .iter()
            .map(|usage| usage.reported_tokens)
            .sum::<i64>(),
        15
    );
    assert_eq!(
        delegated.budget.as_ref().unwrap().root_id,
        root.authorization_id
    );
    store.close().await;
    Ok(())
}
