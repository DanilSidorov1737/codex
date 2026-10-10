use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_repeating_assistant;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationListResponse;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationMutateResponse;
use codex_app_server_protocol::ProjectCreateParams;
use codex_app_server_protocol::ProjectCreateResponse;
use codex_features::Feature;
use codex_protocol::organization::*;
use tempfile::TempDir;

#[tokio::test]
async fn owner_definitions_use_existing_projects_and_survive_server_restart() -> Result<()> {
    let responses = create_mock_responses_server_repeating_assistant("No inference required").await;
    let home = TempDir::new()?;
    MockResponsesConfig::new(&responses.uri())
        .enable_feature(Feature::Sqlite)
        .write(home.path())?;
    let mut server = TestAppServer::builder()
        .with_codex_home(home.path())
        .build_initialized()
        .await?;
    let created: OrganizationMutateResponse = server
        .request(|request_id| ClientRequest::OrganizationMutate {
            request_id,
            params: OrganizationMutateParams {
                mutation: OrganizationMutation::PutNode {
                    id: None,
                    expected_revision: None,
                    parent_id: None,
                    kind: OrganizationNodeKind::Agent,
                    label: "My standalone agent".into(),
                    role_version_id: None,
                    model: None,
                    enabled: true,
                },
            },
        })
        .await?;
    let Some(OrganizationRecord::Node(node)) = created.record else {
        panic!("expected agent");
    };
    let existing: ProjectCreateResponse = server
        .request(|request_id| ClientRequest::ProjectCreate {
            request_id,
            params: ProjectCreateParams {
                name: "Document project".into(),
                roots: Vec::new(),
                metadata: None,
                idempotency_key: "organization-test-project".into(),
            },
        })
        .await?;
    let linked: OrganizationMutateResponse = server
        .request(|request_id| ClientRequest::OrganizationMutate {
            request_id,
            params: OrganizationMutateParams {
                mutation: OrganizationMutation::LinkProject {
                    project_id: existing.project.id.clone(),
                    expected_revision: None,
                    goal_id: None,
                    owner_id: Some(node.id.clone()),
                    criteria: vec!["Report accepted by owner".into()],
                },
            },
        })
        .await?;
    let Some(OrganizationRecord::Project(plan)) = linked.record else {
        panic!("expected linked project");
    };
    assert_eq!(plan.id, existing.project.id);
    assert_eq!(plan.name, existing.project.name);
    assert!(server.shutdown_gracefully().await?.success());
    let mut restarted = TestAppServer::builder()
        .with_codex_home(home.path())
        .build_initialized()
        .await?;
    let listed: OrganizationListResponse = restarted
        .request(|request_id| ClientRequest::OrganizationList {
            request_id,
            params: OrganizationListParams {
                kind: OrganizationRecordKind::Nodes,
                cursor: None,
                limit: Some(10),
            },
        })
        .await?;
    assert_eq!(listed.data, vec![OrganizationRecord::Node(node)]);
    assert!(listed.next_cursor.is_none());
    let projects: OrganizationListResponse = restarted
        .request(|request_id| ClientRequest::OrganizationList {
            request_id,
            params: OrganizationListParams {
                kind: OrganizationRecordKind::Projects,
                cursor: None,
                limit: Some(10),
            },
        })
        .await?;
    assert_eq!(projects.data, vec![OrganizationRecord::Project(plan)]);
    assert!(restarted.shutdown_gracefully().await?.success());
    Ok(())
}
