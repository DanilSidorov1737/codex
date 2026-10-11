use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_repeating_assistant;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::OrganizationExecutionMutateParams;
use codex_app_server_protocol::OrganizationExecutionMutateResponse;
use codex_app_server_protocol::OrganizationExecutionReadParams;
use codex_app_server_protocol::OrganizationExecutionReadResponse;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationListResponse;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationMutateResponse;
use codex_app_server_protocol::ProjectCreateParams;
use codex_app_server_protocol::ProjectCreateResponse;
use codex_app_server_protocol::ProjectRoot;
use codex_app_server_protocol::RequestId;
use codex_features::Feature;
use codex_protocol::organization::*;
use codex_protocol::organization_execution::*;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::responses;
use std::time::Duration;
use tempfile::TempDir;
use test_case::test_case;

#[path = "organization_delegation.rs"]
mod delegation;

#[path = "organization_schedules.rs"]
mod schedules;

async fn definition(
    server: &mut TestAppServer,
    mutation: OrganizationMutation,
) -> Result<OrganizationRecord> {
    let response: OrganizationMutateResponse = server
        .request(|request_id| ClientRequest::OrganizationMutate {
            request_id,
            params: OrganizationMutateParams { mutation },
        })
        .await?;
    Ok(response
        .record
        .expect("definition mutation returns its record"))
}

async fn execution(server: &mut TestAppServer, task_id: &str) -> Result<OrganizationExecutionView> {
    let response: OrganizationExecutionReadResponse = server
        .request(|request_id| ClientRequest::OrganizationExecutionRead {
            request_id,
            params: OrganizationExecutionReadParams {
                task_id: task_id.to_owned(),
            },
        })
        .await?;
    Ok(response.view)
}

async fn execute(
    server: &mut TestAppServer,
    mutation: OrganizationExecutionMutation,
) -> Result<OrganizationExecutionView> {
    let response: OrganizationExecutionMutateResponse = server
        .request(|request_id| ClientRequest::OrganizationExecutionMutate {
            request_id,
            params: OrganizationExecutionMutateParams { mutation },
        })
        .await?;
    Ok(response.view)
}

async fn denied_execution(
    server: &mut TestAppServer,
    mutation: OrganizationExecutionMutation,
) -> Result<String> {
    let id = server
        .send_request(
            "organization/execution/mutate",
            Some(serde_json::to_value(OrganizationExecutionMutateParams {
                mutation,
            })?),
        )
        .await?;
    let rejected = server
        .read_stream_until_error_message(RequestId::Integer(id))
        .await?;
    assert_eq!(rejected.error.code, -32602);
    Ok(rejected.error.message)
}

async fn wait_task(
    server: &mut TestAppServer,
    task_id: &str,
    state: &str,
) -> Result<OrganizationExecutionView> {
    tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            let view = execution(server, task_id).await?;
            anyhow::ensure!(
                !matches!(view.task.state.as_str(), "failed" | "needsReconciliation"),
                "task failed before expected {state}: {:?}",
                view.task
            );
            if view.task.state == state {
                return Ok(view);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await?
}

#[cfg(target_os = "linux")]
#[test_case(false, false; "exact_approval_and_real_artifact")]
#[test_case(true, false; "pending_approval_survives_crash_without_replay")]
#[test_case(false, true; "selective_apply_preserves_unselected_file")]
#[tokio::test]
async fn background_task_requires_exact_approval_and_collects_real_host_evidence(
    crash: bool,
    selective: bool,
) -> Result<()> {
    let mut sequence = vec![responses::sse(vec![
        responses::ev_response_created("r1"),
        responses::ev_function_call(
            "write-artifact",
            "exec_command",
            r#"{"cmd":"python3 probe.py","login":false}"#,
        ),
        responses::ev_completed_with_tokens("r1", 15),
    ])];
    if !crash {
        sequence.push(responses::sse(vec![
            responses::ev_response_created("r2"),
            responses::ev_assistant_message("done", "Artifact prepared for owner review."),
            responses::ev_completed_with_tokens("r2", 15),
        ]));
    }
    let provider = app_test_support::create_mock_responses_server_sequence(sequence).await;
    let home = TempDir::new()?;
    // Match the daemon's real installation: native helpers inside its private
    // owner home. Hard links avoid duplicating large test binaries and are
    // trusted executable fixtures, not accepted task input/artifact links.
    let installed = home.path().join("runtime");
    std::fs::create_dir(&installed)?;
    let program = installed.join("codex-app-server");
    for name in ["codex-app-server", "codex-code-mode-host"] {
        let source = codex_utils_cargo_bin::cargo_bin(name)?;
        std::fs::hard_link(&source, installed.join(name))
            .or_else(|_| codex_utils_cargo_bin::copy_executable(&source, &installed.join(name)))?;
    }
    let separate_state = TempDir::new()?;
    let state = if crash {
        separate_state.path()
    } else {
        home.path()
    };
    let project = TempDir::new()?;
    std::fs::write(
        home.path().join("owner-credentials.json"),
        "fixture credential; no real secret",
    )?;
    let control = home.path().join("owner-control.sock");
    let _control_listener = std::os::unix::net::UnixListener::bind(&control)?;
    let _control_connection = std::os::unix::net::UnixStream::connect(&control)?;
    let paths = serde_json::json!({
        "state": state.join("organization/organization.sqlite"),
        "credentials": home.path().join("owner-credentials.json"),
        "source": project.path().join("oracle.py"),
        "control": control,
        "port": provider.address().port(),
    });
    let probe = format!(
        r#"import json, os, socket
from pathlib import Path
paths = json.loads({:?})
def denied(path, mode):
    try:
        descriptor = os.open(path, mode)
        os.close(descriptor)
        return False
    except OSError:
        return True
def connect_denied(family, address):
    client = None
    try:
        client = socket.socket(family, socket.SOCK_STREAM)
        client.settimeout(0.5)
        client.connect(address)
        return False
    except OSError:
        return True
    finally:
        if client is not None:
            client.close()
proof = {{
    'state_read': denied(paths['state'], os.O_RDONLY),
    'state_write': denied(paths['state'], os.O_WRONLY),
    'credentials_read': denied(paths['credentials'], os.O_RDONLY),
    'original_read': denied(paths['source'], os.O_RDONLY),
    'original_write': denied(paths['source'], os.O_WRONLY),
    'immutable_check_write': denied('oracle.py', os.O_WRONLY),
    'owner_socket': connect_denied(socket.AF_UNIX, paths['control']),
    'provider_network': connect_denied(socket.AF_INET, ('127.0.0.1', paths['port'])),
    'inherited_credentials': 'CODEX_TEST_PROTECTED_CREDENTIAL' not in os.environ,
}}
Path('security.json').write_text(json.dumps(proof))
Path('report.txt').write_text('checked background artifact')
"#,
        paths.to_string()
    );
    std::fs::write(project.path().join("probe.py"), probe)?;
    std::fs::write(
        project.path().join("oracle.py"),
        "from pathlib import Path\nimport json\nassert Path('report.txt').read_text() == 'checked background artifact'\nproof = json.loads(Path('security.json').read_text())\nassert len(proof) == 9 and all(value is True for value in proof.values()), proof\nprint('owner oracle and native isolation passed')\n",
    )?;
    MockResponsesConfig::new(&provider.uri())
        .with_model("gpt-5.4")
        .with_provider_config("supports_websockets = false")
        .enable_feature(Feature::Sqlite)
        .write(home.path())?;
    let mut server = TestAppServer::builder()
        .with_codex_home(home.path())
        .with_program(&program)
        .without_auto_env()
        .with_env_overrides(&[
            ("CODEX_SQLITE_HOME", state.to_str()),
            ("CODEX_TEST_PROTECTED_CREDENTIAL", Some("fixture only")),
            ("CODEX_EXEC_SERVER_URL", None),
        ])
        .build_initialized()
        .await?;
    let OrganizationRecord::Node(node) = definition(
        &mut server,
        OrganizationMutation::PutNode {
            id: None,
            expected_revision: None,
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Standalone document author".into(),
            role_version_id: None,
            model: None,
            enabled: true,
        },
    )
    .await?
    else {
        unreachable!()
    };
    let project_root = AbsolutePathBuf::try_from(project.path().canonicalize()?)?;
    let existing: ProjectCreateResponse = server
        .request(|request_id| ClientRequest::ProjectCreate {
            request_id,
            params: ProjectCreateParams {
                name: "Checked document".into(),
                roots: vec![ProjectRoot { path: project_root }],
                metadata: None,
                idempotency_key: "background-document-project".into(),
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
            criteria: vec!["Owner accepts checked artifact".into()],
        },
    )
    .await?;
    let OrganizationRecord::Task(task) = definition(
        &mut server,
        OrganizationMutation::PutTask {
            id: None,
            expected_revision: None,
            project_id: existing.project.id,
            goal_id: None,
            owner_id: node.id,
            instructions: "Write report.txt containing checked background artifact".into(),
            criteria: vec!["Run the immutable owner oracle".into()],
            prerequisites: vec![],
        },
    )
    .await?
    else {
        unreachable!()
    };
    execute(
        &mut server,
        OrganizationExecutionMutation::Authorize {
            task_id: task.id.clone(),
            expected_revision: task.revision,
            scope: OrganizationExecutionScope {
                input_files: vec!["oracle.py".into(), "probe.py".into()],
                immutable_files: vec!["oracle.py".into(), "probe.py".into()],
                artifacts: vec!["report.txt".into(), "security.json".into()],
                checks: vec![vec!["python3".into(), "oracle.py".into()]],
                model: "gpt-5.4".into(),
                max_tokens: 20000,
                max_seconds: 120,
                max_model_requests: 2,
                max_attempts: 1,
                require_tool_approval: true,
                delegates: vec![],
                max_children: 0,
                auto_accept_children: false,
                not_before: None,
            },
        },
    )
    .await?;
    let waiting = wait_task(&mut server, &task.id, "waitingApproval").await?;
    let attempt = &waiting.attempts[0];
    let workspace = std::path::Path::new(
        attempt
            .workspace
            .as_ref()
            .expect("runtime workspace persisted before start"),
    );
    assert!(
        !workspace.join("report.txt").exists(),
        "native write must be blocked before owner approval"
    );
    assert!(!project.path().join("report.txt").exists());
    let approval = waiting
        .approvals
        .first()
        .expect("approval durably recorded");
    assert!(approval.arguments.contains("python3 probe.py"));
    assert!(state.join("organization/organization.sqlite").is_file());
    assert!(home.path().join("owner-credentials.json").is_file());
    assert!(project.path().join("oracle.py").is_file());
    if crash {
        let approval = approval.clone();
        server.crash_for_test().await?;
        drop(server);
        let mut server = TestAppServer::builder()
            .with_codex_home(home.path())
            .with_program(&program)
            .without_auto_env()
            .with_env_overrides(&[
                ("CODEX_SQLITE_HOME", state.to_str()),
                ("CODEX_EXEC_SERVER_URL", None),
            ])
            .build_initialized()
            .await?;
        let recovered = execution(&mut server, &task.id).await?;
        assert_eq!(recovered.task.state, "needsReconciliation");
        assert_eq!(
            recovered.approvals[0].state,
            OrganizationApprovalState::Pending
        );
        assert_eq!(recovered.approvals[0].action_hash, approval.action_hash);
        let request_id = server
            .send_request(
                "organization/execution/mutate",
                Some(serde_json::to_value(OrganizationExecutionMutateParams {
                    mutation: OrganizationExecutionMutation::DecideApproval {
                        task_id: task.id.clone(),
                        approval_id: approval.id,
                        expected_hash: approval.action_hash,
                        approve: true,
                    },
                })?),
            )
            .await?;
        let denied = server
            .read_stream_until_error_message(RequestId::Integer(request_id))
            .await?;
        assert_eq!(denied.error.code, -32602);
        assert!(
            denied.error.message.contains("stale"),
            "an unexpired old lease cannot authorize a crashed attempt"
        );
        assert!(!workspace.join("report.txt").exists());
        assert!(!project.path().join("report.txt").exists());
        assert!(server.shutdown_gracefully().await?.success());
        return Ok(());
    }
    execute(
        &mut server,
        OrganizationExecutionMutation::DecideApproval {
            task_id: task.id.clone(),
            approval_id: approval.id.clone(),
            expected_hash: approval.action_hash.clone(),
            approve: true,
        },
    )
    .await?;
    let result = wait_task(&mut server, &task.id, "needsReview").await?;
    assert_eq!(
        std::fs::read_to_string(workspace.join("report.txt"))?,
        "checked background artifact"
    );
    assert!(
        !project.path().join("report.txt").exists(),
        "source project stays separate from isolated work"
    );
    assert_eq!(result.authorization_used_tokens, 30);
    assert_eq!(result.authorization_model_requests, 2);
    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0].state, OrganizationActionState::Completed);
    let evidence = result.result.expect("host evidence persisted");
    assert_eq!(evidence.checks[0].exit_code, Some(0));
    assert!(
        evidence.checks[0]
            .output
            .contains("owner oracle and native isolation passed")
    );
    assert_eq!(
        evidence.artifact_bases,
        vec![
            OrganizationArtifactBase {
                path: "report.txt".into(),
                sha256: None
            },
            OrganizationArtifactBase {
                path: "security.json".into(),
                sha256: None
            }
        ]
    );
    let proof: serde_json::Value =
        serde_json::from_slice(&std::fs::read(workspace.join("security.json"))?)?;
    assert_eq!(proof.as_object().unwrap().len(), 9);
    assert!(
        proof
            .as_object()
            .unwrap()
            .values()
            .all(|value| value == true)
    );
    assert_eq!(evidence.artifacts[0].bytes, 27);
    let accepted = execute(
        &mut server,
        OrganizationExecutionMutation::Accept {
            task_id: task.id.clone(),
            expected_revision: result.task.revision,
            digest: evidence.digest,
        },
    )
    .await?;
    assert_eq!(accepted.task.state, "accepted");
    let result = accepted.result.unwrap();
    assert!(result.accepted);
    for invalid in [
        vec![],
        vec!["report.txt".into(); 2],
        vec!["../report.txt".into()],
        vec!["oracle.py".into()],
    ] {
        assert!(
            denied_execution(
                &mut server,
                OrganizationExecutionMutation::Apply {
                    task_id: task.id.clone(),
                    expected_revision: accepted.task.revision,
                    digest: result.digest.clone(),
                    paths: Some(invalid),
                }
            )
            .await?
            .contains("execution mutation rejected")
        );
    }
    let pending: OrganizationExecutionReadResponse = server
        .request(|request_id| ClientRequest::OrganizationExecutionRead {
            request_id,
            params: OrganizationExecutionReadParams {
                task_id: task.id.clone(),
            },
        })
        .await?;
    assert!(pending.view.applications.is_empty());
    if selective {
        std::fs::write(
            project.path().join("security.json"),
            "human file left unchanged",
        )?;
    }
    // Changed originals and changed accepted artifacts are rejected before any
    // application intent/write. The exact reviewed bytes are then applied once.
    std::fs::write(project.path().join("report.txt"), "human change")?;
    assert!(
        denied_execution(
            &mut server,
            OrganizationExecutionMutation::Apply {
                task_id: task.id.clone(),
                expected_revision: accepted.task.revision,
                digest: result.digest.clone(),
                paths: selective.then(|| vec!["report.txt".into()]),
            }
        )
        .await?
        .contains("original file changed")
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join("report.txt"))?,
        "human change"
    );
    std::fs::remove_file(project.path().join("report.txt"))?;
    let original = std::fs::read(workspace.join("report.txt"))?;
    std::fs::write(workspace.join("report.txt"), "unreviewed artifact")?;
    assert!(
        denied_execution(
            &mut server,
            OrganizationExecutionMutation::Apply {
                task_id: task.id.clone(),
                expected_revision: accepted.task.revision,
                digest: result.digest.clone(),
                paths: selective.then(|| vec!["report.txt".into()]),
            }
        )
        .await?
        .contains("artifact changed after acceptance")
    );
    assert!(!project.path().join("report.txt").exists());
    std::fs::write(workspace.join("report.txt"), &original)?;
    let applied = execute(
        &mut server,
        OrganizationExecutionMutation::Apply {
            task_id: task.id.clone(),
            expected_revision: accepted.task.revision,
            digest: result.digest.clone(),
            paths: selective.then(|| vec!["report.txt".into()]),
        },
    )
    .await?;
    assert_eq!(
        applied.applications[0].state,
        OrganizationApplicationState::Completed
    );
    assert_eq!(std::fs::read(project.path().join("report.txt"))?, original);
    assert_eq!(
        applied.applications[0].paths,
        if selective {
            vec!["report.txt"]
        } else {
            vec!["report.txt", "security.json"]
        }
    );
    if selective {
        assert_eq!(
            std::fs::read_to_string(project.path().join("security.json"))?,
            "human file left unchanged"
        );
    } else {
        assert_eq!(
            std::fs::read(project.path().join("security.json"))?,
            std::fs::read(workspace.join("security.json"))?
        );
    }
    assert!(
        denied_execution(
            &mut server,
            OrganizationExecutionMutation::Apply {
                task_id: task.id.clone(),
                expected_revision: accepted.task.revision,
                digest: result.digest,
                paths: None,
            }
        )
        .await?
        .contains("already recorded")
    );
    assert!(server.shutdown_gracefully().await?.success());
    Ok(())
}

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
