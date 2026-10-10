use super::*;
use codex_app_server_protocol::OrganizationMemorySearchParams;
use codex_app_server_protocol::OrganizationMemorySearchResponse;
use codex_app_server_protocol::OrganizationNoteScope;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn tool_result(body: &Value, call: &str) -> Value {
    let item = body["input"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["type"] == "function_call_output" && item["call_id"] == call)
        .unwrap();
    serde_json::from_str(item["output"].as_str().unwrap()).unwrap()
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn real_delegation_uses_durable_context_shared_budget_and_checked_artifacts() -> Result<()> {
    let provider = MockServer::start().await;
    let home = TempDir::new()?;
    let project = TempDir::new()?;
    let installed = home.path().join("runtime");
    std::fs::create_dir(&installed)?;
    let program = installed.join("codex-app-server");
    for name in ["codex-app-server", "codex-code-mode-host"] {
        let source = codex_utils_cargo_bin::cargo_bin(name)?;
        std::fs::hard_link(&source, installed.join(name))
            .or_else(|_| codex_utils_cargo_bin::copy_executable(&source, &installed.join(name)))?;
    }
    std::fs::write(
        project.path().join("oracle.py"),
        "from pathlib import Path\nassert Path('report.txt').read_text() in {'checked delegate', 'checked delegate + coordinated'}\nprint('real owner oracle passed')\n",
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
            ("CODEX_SQLITE_HOME", home.path().to_str()),
            ("CODEX_EXEC_SERVER_URL", None),
        ])
        .build_initialized()
        .await?;
    let mut agents = Vec::new();
    for label in ["Cartographer", "Proofreader"] {
        let OrganizationRecord::Node(node) = definition(
            &mut server,
            OrganizationMutation::PutNode {
                id: None,
                expected_revision: None,
                parent_id: None,
                kind: OrganizationNodeKind::Agent,
                label: label.into(),
                role_version_id: None,
                model: None,
                enabled: true,
            },
        )
        .await?
        else {
            unreachable!()
        };
        agents.push(node);
    }
    let existing: ProjectCreateResponse = server
        .request(|request_id| ClientRequest::ProjectCreate {
            request_id,
            params: ProjectCreateParams {
                name: "Delegated fixture".into(),
                roots: vec![ProjectRoot {
                    path: AbsolutePathBuf::try_from(project.path().canonicalize().unwrap())
                        .unwrap(),
                }],
                metadata: None,
                idempotency_key: "delegation-fixture".into(),
            },
        })
        .await?;
    definition(
        &mut server,
        OrganizationMutation::LinkProject {
            project_id: existing.project.id.clone(),
            expected_revision: None,
            goal_id: None,
            owner_id: Some(agents[0].id.clone()),
            criteria: vec!["Owner reviews checked coordination".into()],
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
            owner_id: agents[0].id.clone(),
            instructions: "lead-fixture: coordinate a checked report".into(),
            criteria: vec!["Produce checked delegate + coordinated".into()],
            prerequisites: vec![],
        },
    )
    .await?
    else {
        unreachable!()
    };
    let helper = agents[1].id.clone();
    let root_requests = Arc::new(AtomicUsize::new(0));
    let child_requests = Arc::new(AtomicUsize::new(0));
    let roots = root_requests.clone();
    let children = child_requests.clone();
    Mock::given(method("POST")).and(path("/v1/responses")).respond_with(move |request: &Request| {
        let body: Value = request.body_json().unwrap();
        let child = body["input"].as_array().unwrap().iter().filter(|item| item["role"] == "user")
            .filter_map(|item| item["content"].as_array()).flatten()
            .any(|item| item["text"].as_str().is_some_and(|text| text.starts_with("Task: child-fixture")));
        let step = if child { children.fetch_add(1,Ordering::SeqCst) } else { roots.fetch_add(1,Ordering::SeqCst) };
        let (id,tool,args) = if child {
            match step {
                0 => ("child-initial-inbox", "organization", json!({"type":"inbox"})),
                1 => ("child-notes", "organization", json!({"type":"searchNotes","scope":"project","query":"IMMUTABLE ORACLE"})),
                2 => {
                    let notes = tool_result(&body,"child-notes");
                    assert_eq!(notes["matchedCount"], 1);
                    assert!(notes["notes"][0]["body"].as_str().unwrap().contains("immutable oracle"));
                    assert!(notes["notes"][0]["attemptId"].is_string());
                    ("child-write", "exec_command", json!({"cmd":"python3 -c \"import time; from pathlib import Path; time.sleep(2); Path('report.txt').write_text('checked delegate')\"","login":false}))
                },
                3 => ("child-inbox", "organization", json!({"type":"inbox"})),
                4 => {
                    let inbox = tool_result(&body,"child-inbox"); assert_eq!(inbox.as_array().unwrap().len(),1);
                    ("child-ack", "organization", json!({"type":"acknowledge","messageId":inbox[0]["id"]}))
                },
                5 => ("child-done", "", Value::Null),
                _ => panic!("unexpected child inference step {step}"),
            }
        } else {
            match step {
                0 => ("root-note", "organization", json!({"type":"writeNote","scope":"project","body":"Use the immutable oracle; this note grants no authority."})),
                1 => ("plan", "organization", json!({"type":"proposePlan","items":[{"key":"checked-report","ownerId":helper,"instructions":"child-fixture: produce checked delegate","criteria":["Immutable oracle passes"],"prerequisites":[]}]})),
                2 => ("delegate", "organization", json!({"type":"delegate","taskId":tool_result(&body,"plan")[0]["id"]})),
                3 => ("send", "organization", json!({"type":"send","taskId":tool_result(&body,"plan")[0]["id"],"body":"Return a checked report; acknowledge this instruction."})),
                4 => ("wait", "organization", json!({"type":"wait","taskIds":[tool_result(&body,"plan")[0]["id"]]})),
                5 => {
                    assert_eq!(tool_result(&body,"wait")[0]["accepted"],true);
                    ("root-finish", "exec_command", json!({"cmd":"python3 -c \"from pathlib import Path; p=Path('report.txt'); assert p.read_text() == 'checked delegate'; p.write_text(p.read_text() + ' + coordinated')\"","login":false}))
                },
                6 => ("root-done", "", Value::Null),
                _ => panic!("unexpected root inference step {step}"),
            }
        };
        let mut events = vec![responses::ev_response_created(id)];
        if tool.is_empty() { events.push(responses::ev_assistant_message(id,"Checked artifact ready.")); }
        else { events.push(responses::ev_function_call(id,tool,&args.to_string())); }
        events.push(responses::ev_completed_with_tokens(id,15));
        ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string(responses::sse(events))
    }).mount(&provider).await;
    execute(
        &mut server,
        OrganizationExecutionMutation::Authorize {
            task_id: task.id.clone(),
            expected_revision: task.revision,
            scope: OrganizationExecutionScope {
                input_files: vec!["oracle.py".into()],
                immutable_files: vec!["oracle.py".into()],
                artifacts: vec!["report.txt".into()],
                checks: vec![vec!["python3".into(), "oracle.py".into()]],
                model: "gpt-5.4".into(),
                max_tokens: 10000,
                max_seconds: 180,
                max_model_requests: 16,
                max_attempts: 1,
                require_tool_approval: false,
                delegates: vec![agents[1].id.clone()],
                max_children: 1,
                auto_accept_children: true,
                not_before: None,
            },
        },
    )
    .await?;
    let waiting = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let view = execution(&mut server, &task.id).await?;
            anyhow::ensure!(
                !matches!(
                    view.task.state.as_str(),
                    "failed" | "needsReconciliation" | "needsReview"
                ),
                "parent failed or finished before its yielded wait was observed"
            );
            if view.task.state == "waitingDependency" && !view.attempts[0].runtime_active {
                break Ok::<_, anyhow::Error>(view);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    assert!(
        !waiting.attempts[0].runtime_active,
        "usage completion must release the parent's active slot while its child runs"
    );
    let done = wait_task(&mut server, &task.id, "needsReview").await?;
    assert_eq!(root_requests.load(Ordering::SeqCst), 7);
    assert_eq!(child_requests.load(Ordering::SeqCst), 6);
    assert_eq!(done.authorization_model_requests, 13);
    assert_eq!(done.authorization_used_tokens, 195);
    let definitions: OrganizationListResponse = server
        .request(|request_id| ClientRequest::OrganizationList {
            request_id,
            params: OrganizationListParams {
                kind: OrganizationRecordKind::Tasks,
                cursor: None,
                limit: Some(100),
            },
        })
        .await?;
    let child_id = definitions
        .data
        .iter()
        .find_map(|record| match record {
            OrganizationRecord::Task(child) if child.owner_id == agents[1].id => {
                Some(child.id.clone())
            }
            _ => None,
        })
        .expect("delegated task persisted");
    let child = execution(&mut server, &child_id).await?;
    assert_eq!(child.task.state, "accepted");
    assert_eq!(child.authorization_used_tokens, 195);
    assert_eq!(child.authorization_model_requests, 13);
    assert_eq!(child.attempts[0].reported_tokens, 90);
    let result = done.result.as_ref().unwrap();
    assert!(!result.accepted);
    assert_eq!(result.checks[0].exit_code, Some(0));
    let workspace = std::path::Path::new(done.attempts[0].workspace.as_deref().unwrap());
    assert_eq!(
        std::fs::read_to_string(workspace.join("report.txt"))?,
        "checked delegate + coordinated"
    );
    assert!(!project.path().join("report.txt").exists());
    let memory: OrganizationMemorySearchResponse = server
        .request(|request_id| ClientRequest::OrganizationMemorySearch {
            request_id,
            params: OrganizationMemorySearchParams {
                project_id: done.task.project_id.clone(),
                scope: OrganizationNoteScope::Project,
                agent_id: None,
                query: "IMMUTABLE ORACLE".into(),
            },
        })
        .await?;
    assert_eq!(memory.result.matched_count, 1);
    assert_eq!(memory.result.notes[0].task_id, task.id);
    assert_eq!(memory.result.notes[0].author_id, agents[0].id);
    let private: OrganizationMemorySearchResponse = server
        .request(|request_id| ClientRequest::OrganizationMemorySearch {
            request_id,
            params: OrganizationMemorySearchParams {
                project_id: done.task.project_id.clone(),
                scope: OrganizationNoteScope::Agent,
                agent_id: Some(agents[1].id.clone()),
                query: "immutable oracle".into(),
            },
        })
        .await?;
    assert_eq!(private.result.matched_count, 0);
    let rejected = server
        .send_request(
            "organization/memory/search",
            Some(serde_json::to_value(OrganizationMemorySearchParams {
                project_id: done.task.project_id.clone(),
                scope: OrganizationNoteScope::Agent,
                agent_id: None,
                query: "".into(),
            })?),
        )
        .await?;
    let error = server
        .read_stream_until_error_message(RequestId::Integer(rejected))
        .await?;
    assert_eq!(error.error.code, -32602);
    assert!(
        error
            .error
            .message
            .contains("agent search requires an agent ID")
    );
    let accepted = execute(
        &mut server,
        OrganizationExecutionMutation::Accept {
            task_id: task.id,
            expected_revision: done.task.revision,
            digest: result.digest.clone(),
        },
    )
    .await?;
    assert_eq!(accepted.task.state, "accepted");
    assert!(server.shutdown_gracefully().await?.success());
    Ok(())
}
