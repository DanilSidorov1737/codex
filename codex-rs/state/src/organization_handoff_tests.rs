use super::*;
use codex_protocol::organization_coordination::{
    OrganizationCheckpointContent, OrganizationCoordinationCall, OrganizationNoteScope,
};

async fn fixture(
    store: &OrganizationStore,
    attempts: u32,
    requests: u32,
) -> Result<(String, OrganizationTaskAttempt, OrganizationNode)> {
    let source = node_from(
        store
            .mutate_owner(
                node_mutation("Custom source", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let receiver = node_from(
        store
            .mutate_owner(
                node_mutation("Custom receiver", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(store, &source.id).await?;
    let task = execution_task(store, &source.id, &project).await?;
    let mut scope = execution_scope();
    scope.max_attempts = attempts;
    scope.max_model_requests = requests;
    scope.require_tool_approval = false;
    scope.delegates = vec![receiver.id.clone()];
    store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    store
        .bind_attempt_runtime(
            &attempt.id,
            &worker,
            &OrganizationRuntimeBinding {
                workspace: "/tmp/state-only-checkpoint-fixture".into(),
                turn_id: None,
                artifact_bases: vec![OrganizationArtifactBase {
                    path: "report.txt".into(),
                    sha256: Some("b".repeat(64)),
                }],
            },
            1000,
        )
        .await?;
    Ok((worker, attempt, receiver))
}

fn files() -> Vec<OrganizationArtifactEvidence> {
    ["input.txt", "oracle.py", "report.txt"]
        .into_iter()
        .map(|path| OrganizationArtifactEvidence {
            path: path.into(),
            sha256: "a".repeat(64),
            bytes: 1,
        })
        .collect()
}

fn content(blocked: bool) -> OrganizationCheckpointContent {
    OrganizationCheckpointContent {
        completed_steps: vec!["Prepared partial report".into()],
        remaining_steps: vec!["Finish the declared report".into()],
        blocker: blocked.then(|| "Owner decision required".into()),
    }
}

async fn request(
    store: &OrganizationStore,
    task: &str,
    receiver: &OrganizationNode,
) -> Result<OrganizationExecutionMutation> {
    let view = store.execution_view(task).await?;
    Ok(OrganizationExecutionMutation::Handoff {
        task_id: task.into(),
        expected_revision: view.task.revision,
        checkpoint_id: view.checkpoints[0].id.clone(),
        owner_id: receiver.id.clone(),
        expected_owner_revision: receiver.revision,
        instructions: "Continue only the recorded remaining work".into(),
    })
}

#[tokio::test]
async fn checkpoint_handoff_persists_same_grant_usage_bases_and_fences_previous_worker()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, source, receiver) = fixture(&store, 3, 4).await?;
    store.admit_model_request(&source.id, &worker, 1001).await?;
    store.checkpoint_usage(&source.id, &worker, 1, 10).await?;
    let action = store
        .prepare_action(
            &source.id,
            &worker,
            "known-effect",
            "exec_command",
            "{}",
            1001,
        )
        .await?;
    assert!(store.dispatch_action(&action.id, &worker, 1001, 2).await?);
    store
        .finish_action(
            &source.id,
            &worker,
            "known-effect",
            true,
            "native completed effect",
        )
        .await?;
    let checkpoint = store
        .record_checkpoint(
            &source.id,
            &worker,
            "checkpoint",
            content(true),
            files(),
            1001,
        )
        .await?;
    assert_eq!(checkpoint.completed_action_ids, [action.id]);
    let before = store.execution_view(&source.task_id).await?;
    assert_eq!(before.task.state, "blocked");
    let grant = before.authorization.clone().unwrap();
    let handoff = request(&store, &source.task_id, &receiver).await?;
    assert!(
        store.handoff_task(&handoff, 1002).await.is_err(),
        "native capacity must actually be released"
    );
    assert!(
        store
            .admit_model_request(&source.id, &worker, 1002)
            .await
            .is_err()
    );
    store.release_attempt_capacity(&source.id, &worker).await?;
    let recorded = store.handoff_task(&handoff, 1002).await?;
    assert_eq!(recorded.authorization_id, grant.id);
    assert!(
        store.handoff_task(&handoff, 1002).await.is_err(),
        "exact transition cannot be repeated"
    );
    store.close().await;

    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.reconcile_interrupted_attempts().await?, 0);
    let continued = store.execution_view(&source.task_id).await?;
    assert_eq!(continued.task.state, "queued");
    assert_eq!(continued.task.owner_id, receiver.id);
    assert_eq!(continued.checkpoints, [checkpoint]);
    assert_eq!(continued.handoffs, [recorded]);
    let current = continued.authorization.as_ref().unwrap();
    assert_eq!(current.id, grant.id);
    assert_eq!(current.scope, grant.scope);
    assert_eq!(current.expires_at, grant.expires_at);
    assert_eq!(current.node_revision, receiver.revision);
    assert_eq!(
        (
            continued.authorization_used_tokens,
            continued.authorization_model_requests
        ),
        (10, 1)
    );
    let next = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 2)
        .await?
        .unwrap();
    assert_eq!(next.generation, 2);
    assert_eq!(next.authorization_id, grant.id);
    assert!(
        store
            .record_result(&source.id, &worker, vec![], vec![], 1002)
            .await
            .is_err()
    );
    assert!(
        store
            .coordinate(
                &source.id,
                &worker,
                "stale",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Project,
                    body: "stale callback".into()
                },
                1002
            )
            .await
            .is_err()
    );
    store.admit_model_request(&next.id, &worker, 1002).await?;
    store.checkpoint_usage(&next.id, &worker, 1, 20).await?;
    let final_view = store.execution_view(&source.task_id).await?;
    assert_eq!(
        (
            final_view.authorization_used_tokens,
            final_view.authorization_model_requests
        ),
        (30, 2)
    );
    assert_eq!(final_view.task.owner_id, receiver.id);
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn checkpoint_handoff_rejects_stale_scope_and_cannot_reset_limits() -> Result<()> {
    for boundary in [
        "attempts",
        "requests",
        "tokens",
        "usage",
        "effect",
        "receiver",
        "checkpoint",
        "revision",
        "roster",
    ] {
        let home = Fixture::new();
        let store = OrganizationStore::open(&home.0).await?;
        let (worker, source, receiver) = fixture(
            &store,
            if boundary == "attempts" { 1 } else { 3 },
            if boundary == "requests" { 1 } else { 4 },
        )
        .await?;
        store.admit_model_request(&source.id, &worker, 1001).await?;
        if boundary != "usage" {
            store
                .checkpoint_usage(
                    &source.id,
                    &worker,
                    1,
                    if boundary == "tokens" { 100 } else { 10 },
                )
                .await?;
        }
        if boundary == "effect" {
            let action = store
                .prepare_action(&source.id, &worker, "uncertain", "exec_command", "{}", 1001)
                .await?;
            assert!(store.dispatch_action(&action.id, &worker, 1001, 2).await?);
        }
        store
            .record_checkpoint(
                &source.id,
                &worker,
                "checkpoint",
                content(true),
                files(),
                1001,
            )
            .await?;
        store.release_attempt_capacity(&source.id, &worker).await?;
        let before = store.execution_view(&source.task_id).await?;
        let mut mutation = request(&store, &source.task_id, &receiver).await?;
        let OrganizationExecutionMutation::Handoff {
            expected_revision,
            expected_owner_revision,
            checkpoint_id,
            owner_id,
            ..
        } = &mut mutation
        else {
            unreachable!()
        };
        match boundary {
            "receiver" => *expected_owner_revision += 1,
            "checkpoint" => *checkpoint_id = Uuid::now_v7().to_string(),
            "revision" => *expected_revision -= 1,
            "roster" => *owner_id = Uuid::now_v7().to_string(),
            _ => {}
        }
        assert!(
            store.handoff_task(&mutation, 1002).await.is_err(),
            "{boundary}"
        );
        assert_eq!(
            store.execution_view(&source.task_id).await?,
            before,
            "rejected {boundary} must be atomic"
        );
        assert!(
            store
                .authorize_task(
                    &source.task_id,
                    before.task.revision,
                    before.authorization.as_ref().unwrap().scope.clone(),
                    1002
                )
                .await
                .is_err()
        );
        let grants: i64 =
            sqlx::query_scalar("SELECT count(*) FROM organization_authorizations WHERE task_id=?")
                .bind(&source.task_id)
                .fetch_one(&store.pool)
                .await?;
        assert_eq!(grants, 1);
        store.close().await;
    }
    Ok(())
}

#[tokio::test]
async fn handoff_preserves_recurring_accounting_and_workflow_dependency_grants() -> Result<()> {
    use codex_protocol::organization_coordination::OrganizationPlanItem;
    use codex_protocol::organization_schedule::*;
    use codex_protocol::organization_workflow::*;
    for recurring in [true, false] {
        let home = Fixture::new();
        let store = OrganizationStore::open(&home.0).await?;
        let source = node_from(
            store
                .mutate_owner(
                    node_mutation("Source", None, OrganizationNodeKind::Agent),
                    None,
                )
                .await?,
        );
        let receiver = node_from(
            store
                .mutate_owner(
                    node_mutation("Receiver", None, OrganizationNodeKind::Agent),
                    None,
                )
                .await?,
        );
        let project = execution_project(&store, &source.id).await?;
        let mut scope = execution_scope();
        scope.max_attempts = 3;
        scope.max_model_requests = 4;
        scope.require_tool_approval = false;
        scope.delegates = vec![receiver.id.clone()];
        let dependent = if recurring {
            let template = execution_task(&store, &source.id, &project).await?;
            store
                .mutate_schedule(
                    OrganizationScheduleMutation::Create {
                        id: Uuid::now_v7().to_string(),
                        task_id: template.id,
                        expected_revision: template.revision,
                        scope: Box::new(scope),
                        config: OrganizationScheduleConfig {
                            label: "Continuity series".into(),
                            interval_seconds: 10,
                            first_run_at: 1010,
                            ends_at: 2000,
                            max_runs: 2,
                            max_tokens: 1000,
                            max_model_requests: 10,
                        },
                    },
                    1000,
                )
                .await?;
            assert_eq!(store.materialize_schedules(1010).await?, 1);
            None
        } else {
            let items = vec![
                OrganizationPlanItem {
                    key: "write".into(),
                    owner_id: source.id.clone(),
                    instructions: "Write".into(),
                    criteria: vec!["Checked".into()],
                    prerequisites: vec![],
                },
                OrganizationPlanItem {
                    key: "check".into(),
                    owner_id: source.id,
                    instructions: "Check accepted producer".into(),
                    criteria: vec!["Checked".into()],
                    prerequisites: vec!["write".into()],
                },
            ];
            let OrganizationWorkflowView::Template { template } = store
                .mutate_workflow(
                    OrganizationWorkflowMutation::Put {
                        id: Uuid::now_v7().to_string(),
                        expected_revision: None,
                        project_id: project,
                        label: "Continuity workflow".into(),
                        items,
                    },
                    1000,
                )
                .await?
            else {
                panic!("missing template")
            };
            let preview = store
                .preview_workflow(OrganizationWorkflowPreviewRequest {
                    id: Uuid::now_v7().to_string(),
                    template_id: template.id,
                    expected_revision: 1,
                    scopes: template
                        .items
                        .iter()
                        .map(|item| OrganizationWorkflowScope {
                            key: item.key.clone(),
                            scope: scope.clone(),
                        })
                        .collect(),
                })
                .await?;
            let OrganizationWorkflowView::Run { run, .. } = store
                .mutate_workflow(
                    OrganizationWorkflowMutation::Run {
                        preview: Box::new(preview),
                    },
                    1000,
                )
                .await?
            else {
                panic!("missing run")
            };
            Some(run.task_ids["check"].clone())
        };
        let worker = Uuid::now_v7().to_string();
        let attempt = store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1011, 2)
            .await?
            .unwrap();
        store
            .bind_attempt_runtime(
                &attempt.id,
                &worker,
                &OrganizationRuntimeBinding {
                    workspace: "/tmp/state-only-continuity".into(),
                    turn_id: None,
                    artifact_bases: vec![OrganizationArtifactBase {
                        path: "report.txt".into(),
                        sha256: None,
                    }],
                },
                1011,
            )
            .await?;
        store
            .admit_model_request(&attempt.id, &worker, 1011)
            .await?;
        store.checkpoint_usage(&attempt.id, &worker, 1, 10).await?;
        store
            .record_checkpoint(&attempt.id, &worker, "block", content(true), files(), 1011)
            .await?;
        store.release_attempt_capacity(&attempt.id, &worker).await?;
        let before = store.execution_view(&attempt.task_id).await?;
        store
            .handoff_task(&request(&store, &attempt.task_id, &receiver).await?, 1012)
            .await?;
        let replacement = store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1012, 2)
            .await?
            .unwrap();
        assert_eq!(replacement.task_id, attempt.task_id);
        assert_eq!(replacement.authorization_id, attempt.authorization_id);
        assert_eq!(replacement.generation, 2);
        let continued = store.execution_view(&attempt.task_id).await?;
        assert_eq!(continued.authorization_used_tokens, 10);
        assert_eq!(continued.authorization_model_requests, 1);
        assert_eq!(
            continued.authorization.as_ref().unwrap().scope,
            before.authorization.as_ref().unwrap().scope
        );
        store
            .admit_model_request(&replacement.id, &worker, 1012)
            .await?;
        store
            .checkpoint_usage(&replacement.id, &worker, 1, 20)
            .await?;
        if recurring {
            let series = store.list_schedules(None, None, 10).await?.0.remove(0);
            assert_eq!(
                (
                    series.used_tokens,
                    series.model_requests,
                    series.schedule.issued_runs
                ),
                (30, 2, 1)
            );
            assert_eq!(series.occurrences.len(), 1);
            assert_eq!(series.occurrences[0].task_id, attempt.task_id);
            assert_eq!(store.materialize_schedules(1020).await?, 0);
            store
                .mutate_schedule(
                    OrganizationScheduleMutation::Control {
                        id: series.schedule.id,
                        expected_revision: store.list_schedules(None, None, 10).await?.0[0]
                            .schedule
                            .revision,
                        action: OrganizationScheduleControl::Cancel,
                    },
                    1020,
                )
                .await?;
            assert!(
                store
                    .admit_model_request(&replacement.id, &worker, 1020)
                    .await
                    .is_err()
            );
            let retained = store.list_schedules(None, None, 10).await?.0.remove(0);
            assert_eq!((retained.used_tokens, retained.model_requests), (30, 2));
        } else {
            let dependent = dependent.unwrap();
            let blocked = store.execution_view(&dependent).await?;
            assert_eq!(blocked.task.state, "queued");
            assert!(blocked.attempts.is_empty());
            assert_eq!(
                (
                    blocked.authorization_used_tokens,
                    blocked.authorization_model_requests
                ),
                (0, 0)
            );
            assert_eq!(
                store
                    .claim_task(&worker, &Uuid::now_v7().to_string(), 1013, 2)
                    .await?,
                None
            );
            let run = store
                .list_workflows(OrganizationWorkflowKind::Runs, None, None, 10)
                .await?
                .0
                .remove(0);
            let OrganizationWorkflowView::Run { run, tasks } = run else {
                panic!("missing run")
            };
            assert_eq!(
                (run.preview.max_tokens, run.preview.max_model_requests),
                (200, 8)
            );
            assert_eq!(tasks.len(), 2);
            assert_eq!(
                tasks
                    .iter()
                    .find(|task| task.id == attempt.task_id)
                    .unwrap()
                    .owner_id,
                receiver.id
            );
            assert_eq!(
                tasks
                    .iter()
                    .find(|task| task.id == dependent)
                    .unwrap()
                    .state,
                "queued"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn checkpoint_validation_dedupe_identity_and_blocker_do_not_stop_unrelated_work() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, source, _) = fixture(&store, 3, 4).await?;
    let first = store
        .record_checkpoint(&source.id, &worker, "same", content(false), files(), 1001)
        .await?;
    assert_eq!(
        store
            .record_checkpoint(&source.id, &worker, "same", content(false), files(), 1001)
            .await?,
        first
    );
    assert!(
        store
            .record_checkpoint(&source.id, &worker, "same", content(true), files(), 1001)
            .await
            .is_err()
    );
    assert!(
        store
            .coordinate(
                &source.id,
                &worker,
                "same",
                OrganizationCoordinationCall::Inbox {},
                1001
            )
            .await
            .is_err()
    );
    store
        .coordinate(
            &source.id,
            &worker,
            "note",
            OrganizationCoordinationCall::WriteNote {
                scope: OrganizationNoteScope::Project,
                body: "note".into(),
            },
            1001,
        )
        .await?;
    assert!(
        store
            .record_checkpoint(&source.id, &worker, "note", content(false), files(), 1001)
            .await
            .is_err()
    );
    for invalid in ["missing", "path", "hash", "bytes", "duplicate"] {
        let mut evidence = files();
        match invalid {
            "missing" => {
                evidence.remove(0);
            }
            "path" => evidence[0].path = "../owner-state".into(),
            "hash" => evidence[0].sha256 = "untrusted".into(),
            "bytes" => evidence[0].bytes = 64 * 1024 * 1024 + 1,
            _ => evidence.push(evidence[0].clone()),
        }
        assert!(
            store
                .record_checkpoint(&source.id, &worker, invalid, content(false), evidence, 1001)
                .await
                .is_err(),
            "{invalid}"
        );
    }
    let before = store.execution_view(&source.task_id).await?;
    assert_eq!(before.checkpoints.len(), 1);
    assert_eq!(before.task.state, "running");
    store
        .record_checkpoint(&source.id, &worker, "blocked", content(true), files(), 1001)
        .await?;
    store.release_attempt_capacity(&source.id, &worker).await?;
    let (_, unrelated) = authorized_attempt(&store, false).await?;
    assert_ne!(unrelated.task_id, source.task_id);
    assert_eq!(
        store.execution_view(&source.task_id).await?.task.state,
        "blocked"
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn continuity_migration_from_eight_preserves_existing_authorizations() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt, _) = fixture(&store, 3, 4).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 10).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    sqlx::query("DROP TABLE organization_handoffs")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_checkpoints")
        .execute(&store.pool)
        .await?;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version=9")
        .execute(&store.pool)
        .await?;
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.execution_view(&attempt.task_id).await?, before);
    assert_eq!(store.reconcile_interrupted_attempts().await?, 1);
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}
