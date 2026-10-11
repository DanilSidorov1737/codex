use super::*;
use codex_app_server_protocol::{
    OrganizationScheduleListParams, OrganizationScheduleListResponse,
    OrganizationScheduleMutateParams, OrganizationScheduleMutateResponse,
};
use codex_protocol::organization_schedule::*;

async fn mutate(
    server: &mut TestAppServer,
    mutation: OrganizationScheduleMutation,
) -> Result<OrganizationScheduleView> {
    let response: OrganizationScheduleMutateResponse = server
        .request(|request_id| ClientRequest::OrganizationScheduleMutate {
            request_id,
            params: OrganizationScheduleMutateParams { mutation },
        })
        .await?;
    Ok(response.view)
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn owner_schedule_api_persists_controls_and_rejects_stale_or_duplicate_authority()
-> Result<()> {
    let responses =
        create_mock_responses_server_repeating_assistant("No scheduled inference required").await;
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
            label: "Periodic owner".into(),
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
                name: "Periodic project".into(),
                roots: vec![ProjectRoot {
                    path: AbsolutePathBuf::try_from(project.path()).unwrap(),
                }],
                metadata: None,
                idempotency_key: "schedule-api-project".into(),
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
    let OrganizationRecord::Task(task) = definition(
        &mut server,
        OrganizationMutation::PutTask {
            id: None,
            expected_revision: None,
            project_id: existing.project.id.clone(),
            goal_id: None,
            owner_id: node.id,
            instructions: "Periodic report".into(),
            criteria: vec!["Checked".into()],
            prerequisites: vec![],
        },
    )
    .await?
    else {
        unreachable!()
    };
    let now = chrono::Utc::now().timestamp();
    let id = uuid::Uuid::now_v7().to_string();
    let create = OrganizationScheduleMutation::Create {
        id: id.clone(),
        task_id: task.id,
        expected_revision: task.revision,
        scope: Box::new(OrganizationExecutionScope {
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
            not_before: None,
        }),
        config: OrganizationScheduleConfig {
            label: "Hourly report".into(),
            interval_seconds: 3600,
            first_run_at: now + 3600,
            ends_at: now + 86400,
            max_runs: 3,
            max_tokens: 300,
            max_model_requests: 6,
        },
    };
    let initial = mutate(&mut server, create.clone()).await?;
    assert_eq!(initial.schedule.next_run_at, Some(now + 3600));
    assert!(initial.occurrences.is_empty());
    for mutation in [
        create,
        OrganizationScheduleMutation::Control {
            id: id.clone(),
            expected_revision: 0,
            action: OrganizationScheduleControl::Cancel,
        },
    ] {
        let request = server
            .send_request(
                "organization/schedule/mutate",
                Some(serde_json::to_value(OrganizationScheduleMutateParams {
                    mutation,
                })?),
            )
            .await?;
        let denied = server
            .read_stream_until_error_message(RequestId::Integer(request))
            .await?;
        assert_eq!(denied.error.code, -32602);
    }
    let paused = mutate(
        &mut server,
        OrganizationScheduleMutation::Control {
            id: id.clone(),
            expected_revision: initial.schedule.revision,
            action: OrganizationScheduleControl::Pause,
        },
    )
    .await?;
    assert_eq!(paused.schedule.state, OrganizationScheduleState::Paused);
    assert!(server.shutdown_gracefully().await?.success());
    let mut restarted = TestAppServer::builder()
        .with_codex_home(home.path())
        .build_initialized()
        .await?;
    let listed: OrganizationScheduleListResponse = restarted
        .request(|request_id| ClientRequest::OrganizationScheduleList {
            request_id,
            params: OrganizationScheduleListParams {
                project_id: Some(existing.project.id),
                cursor: None,
                limit: Some(1),
            },
        })
        .await?;
    assert_eq!(listed.data, vec![paused.clone()]);
    assert!(listed.next_cursor.is_none());
    let resumed = mutate(
        &mut restarted,
        OrganizationScheduleMutation::Control {
            id: id.clone(),
            expected_revision: paused.schedule.revision,
            action: OrganizationScheduleControl::Resume,
        },
    )
    .await?;
    assert_eq!(resumed.schedule.state, OrganizationScheduleState::Active);
    let cancelled = mutate(
        &mut restarted,
        OrganizationScheduleMutation::Control {
            id,
            expected_revision: resumed.schedule.revision,
            action: OrganizationScheduleControl::Cancel,
        },
    )
    .await?;
    assert_eq!(
        cancelled.schedule.state,
        OrganizationScheduleState::Cancelled
    );
    assert_eq!(cancelled.schedule.next_run_at, None);
    assert!(cancelled.occurrences.is_empty());
    assert!(restarted.shutdown_gracefully().await?.success());
    Ok(())
}
