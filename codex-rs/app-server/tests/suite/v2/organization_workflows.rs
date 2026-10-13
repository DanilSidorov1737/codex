use super::*;
use codex_app_server_protocol::OrganizationWorkflowListParams;
use codex_app_server_protocol::OrganizationWorkflowListResponse;
use codex_app_server_protocol::OrganizationWorkflowMutateParams;
use codex_app_server_protocol::OrganizationWorkflowMutateResponse;
use codex_app_server_protocol::OrganizationWorkflowPreviewParams;
use codex_app_server_protocol::OrganizationWorkflowPreviewResponse;
use codex_protocol::organization_coordination::OrganizationPlanItem;
use codex_protocol::organization_workflow::*;

async fn mutate(
    server: &mut TestAppServer,
    mutation: OrganizationWorkflowMutation,
) -> Result<OrganizationWorkflowView> {
    let response: OrganizationWorkflowMutateResponse = server
        .request(|request_id| ClientRequest::OrganizationWorkflowMutate {
            request_id,
            params: OrganizationWorkflowMutateParams { mutation },
        })
        .await?;
    Ok(response.view)
}

async fn denied(server: &mut TestAppServer, mutation: OrganizationWorkflowMutation) -> Result<()> {
    let request = server
        .send_request(
            "organization/workflow/mutate",
            Some(serde_json::to_value(OrganizationWorkflowMutateParams {
                mutation,
            })?),
        )
        .await?;
    let error = server
        .read_stream_until_error_message(RequestId::Integer(request))
        .await?;
    assert_eq!(error.error.code, -32602);
    Ok(())
}

#[tokio::test]
async fn owner_workflow_api_previews_exact_grants_and_persists_fresh_runs() -> Result<()> {
    let responses =
        create_mock_responses_server_repeating_assistant("No future inference required").await;
    let home = TempDir::new()?;
    let project = TempDir::new()?;
    MockResponsesConfig::new(&responses.uri())
        .enable_feature(Feature::Sqlite)
        .write(home.path())?;
    let mut server = TestAppServer::builder()
        .with_codex_home(home.path())
        .build_initialized()
        .await?;
    let OrganizationRecord::Node(node) = definition(
        &mut server,
        OrganizationMutation::PutNode {
            id: None,
            expected_revision: None,
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Workflow author".into(),
            role_version_id: None,
            model: None,
            enabled: true,
        },
    )
    .await?
    else {
        unreachable!()
    };
    let existing: ProjectCreateResponse = server
        .request(|request_id| ClientRequest::ProjectCreate {
            request_id,
            params: ProjectCreateParams {
                name: "Workflow project".into(),
                roots: vec![ProjectRoot {
                    path: AbsolutePathBuf::try_from(project.path()).unwrap(),
                }],
                metadata: None,
                idempotency_key: "workflow-project".into(),
            },
        })
        .await?;
    definition(
        &mut server,
        OrganizationMutation::LinkProject {
            project_id: existing.project.id.clone(),
            expected_revision: None,
            goal_id: None,
            owner_id: Some(node.id.clone()),
            criteria: vec!["Checked".into()],
        },
    )
    .await?;
    let items = vec![
        OrganizationPlanItem {
            key: "inspect".into(),
            owner_id: node.id.clone(),
            instructions: "Inspect".into(),
            criteria: vec!["Cited".into()],
            prerequisites: vec![],
        },
        OrganizationPlanItem {
            key: "write".into(),
            owner_id: node.id,
            instructions: "Write".into(),
            criteria: vec!["Checked".into()],
            prerequisites: vec!["inspect".into()],
        },
    ];
    let OrganizationWorkflowView::Template { template } = mutate(
        &mut server,
        OrganizationWorkflowMutation::Put {
            id: uuid::Uuid::now_v7().to_string(),
            expected_revision: None,
            project_id: existing.project.id.clone(),
            label: "Research café".into(),
            items: items.clone(),
        },
    )
    .await?
    else {
        unreachable!()
    };
    let scope = OrganizationExecutionScope {
        cost_budget: None,
        code_mode: false,
        input_files: vec![],
        immutable_files: vec![],
        artifacts: vec!["report.txt".into()],
        checks: vec![],
        model: "gpt-5.4".into(),
        max_tokens: 100,
        max_seconds: 60,
        max_model_requests: 2,
        max_attempts: 1,
        require_tool_approval: true,
        delegates: vec![],
        max_children: 0,
        auto_accept_children: false,
        not_before: Some(chrono::Utc::now().timestamp() + 7200),
    };
    let response: OrganizationWorkflowPreviewResponse = server
        .request(|request_id| ClientRequest::OrganizationWorkflowPreview {
            request_id,
            params: OrganizationWorkflowPreviewParams {
                request: OrganizationWorkflowPreviewRequest {
                    shared_limits: None,
                    id: uuid::Uuid::now_v7().to_string(),
                    template_id: template.id.clone(),
                    expected_revision: 1,
                    scopes: items
                        .iter()
                        .map(|item| OrganizationWorkflowScope {
                            key: item.key.clone(),
                            scope: scope.clone(),
                        })
                        .collect(),
                },
            },
        })
        .await?;
    assert_eq!(
        (
            response.preview.max_tokens,
            response.preview.max_model_requests
        ),
        (200, 4)
    );
    let first = mutate(
        &mut server,
        OrganizationWorkflowMutation::Run {
            preview: Box::new(response.preview.clone()),
        },
    )
    .await?;
    denied(
        &mut server,
        OrganizationWorkflowMutation::Run {
            preview: Box::new(response.preview.clone()),
        },
    )
    .await?;
    let mut second = response.preview.clone();
    second.id = uuid::Uuid::now_v7().to_string();
    let other = mutate(
        &mut server,
        OrganizationWorkflowMutation::Run {
            preview: Box::new(second),
        },
    )
    .await?;
    let (
        OrganizationWorkflowView::Run { run, tasks },
        OrganizationWorkflowView::Run { run: other, .. },
    ) = (&first, &other)
    else {
        unreachable!()
    };
    assert!(
        other
            .task_ids
            .values()
            .all(|id| !run.task_ids.values().any(|prior| prior == id))
    );
    assert!(tasks.iter().all(|task| task.state == "queued"));
    let paused = mutate(
        &mut server,
        OrganizationWorkflowMutation::Control {
            id: other.id.clone(),
            expected_revision: other.revision,
            action: OrganizationWorkflowControl::Pause,
        },
    )
    .await?;
    let OrganizationWorkflowView::Run {
        run: paused,
        tasks: paused_tasks,
    } = paused
    else {
        unreachable!()
    };
    assert_eq!(paused.state, OrganizationWorkflowState::Paused);
    assert!(paused_tasks.iter().all(|task| task.state == "queued"));
    denied(
        &mut server,
        OrganizationWorkflowMutation::Control {
            id: paused.id.clone(),
            expected_revision: 1,
            action: OrganizationWorkflowControl::Resume,
        },
    )
    .await?;
    let resumed = mutate(
        &mut server,
        OrganizationWorkflowMutation::Control {
            id: paused.id.clone(),
            expected_revision: paused.revision,
            action: OrganizationWorkflowControl::Resume,
        },
    )
    .await?;
    let OrganizationWorkflowView::Run { run: resumed, .. } = resumed else {
        unreachable!()
    };
    let cancelled = mutate(
        &mut server,
        OrganizationWorkflowMutation::Control {
            id: resumed.id.clone(),
            expected_revision: resumed.revision,
            action: OrganizationWorkflowControl::Cancel,
        },
    )
    .await?;
    let OrganizationWorkflowView::Run {
        run: cancelled,
        tasks: cancelled_tasks,
    } = cancelled
    else {
        unreachable!()
    };
    assert_eq!(cancelled.state, OrganizationWorkflowState::Cancelled);
    assert_eq!(cancelled.task_ids, other.task_ids);
    assert!(cancelled_tasks.iter().all(|task| task.state == "cancelled"));
    let OrganizationWorkflowView::Template { template: updated } = mutate(
        &mut server,
        OrganizationWorkflowMutation::Put {
            id: template.id.clone(),
            expected_revision: Some(1),
            project_id: template.project_id.clone(),
            label: "Updated".into(),
            items,
        },
    )
    .await?
    else {
        unreachable!()
    };
    let mut stale = response.preview;
    stale.id = uuid::Uuid::now_v7().to_string();
    denied(
        &mut server,
        OrganizationWorkflowMutation::Run {
            preview: Box::new(stale),
        },
    )
    .await?;
    assert!(server.shutdown_gracefully().await?.success());
    let mut server = TestAppServer::builder()
        .with_codex_home(home.path())
        .build_initialized()
        .await?;
    let listed: OrganizationWorkflowListResponse = server
        .request(|request_id| ClientRequest::OrganizationWorkflowList {
            request_id,
            params: OrganizationWorkflowListParams {
                kind: OrganizationWorkflowKind::Runs,
                project_id: Some(template.project_id),
                cursor: None,
                limit: Some(200),
            },
        })
        .await?;
    assert_eq!(listed.data.len(), 2);
    assert!(listed.data.contains(&first));
    assert!(listed.data.iter().any(
        |view| matches!(view, OrganizationWorkflowView::Run { run, .. } if run == &cancelled)
    ));
    let templates: OrganizationWorkflowListResponse = server
        .request(|request_id| ClientRequest::OrganizationWorkflowList {
            request_id,
            params: OrganizationWorkflowListParams {
                kind: OrganizationWorkflowKind::Templates,
                project_id: None,
                cursor: None,
                limit: Some(200),
            },
        })
        .await?;
    assert_eq!(
        templates.data,
        vec![OrganizationWorkflowView::Template { template: updated }]
    );
    assert!(server.shutdown_gracefully().await?.success());
    Ok(())
}
