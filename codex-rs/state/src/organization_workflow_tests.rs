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
