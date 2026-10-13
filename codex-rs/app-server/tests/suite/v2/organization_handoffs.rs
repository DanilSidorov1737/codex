use super::*;

#[cfg(target_os = "linux")]
#[tokio::test]
async fn blocked_checkpoint_handoff_reuses_grant_verified_files_and_remaining_budget() -> Result<()>
{
    let sequence = vec![
        responses::sse(vec![
            responses::ev_response_created("h1"),
            responses::ev_function_call(
                "partial",
                "exec_command",
                r#"{"cmd":"printf 'partial report' > report.txt","login":false}"#,
            ),
            responses::ev_completed_with_tokens("h1", 15),
        ]),
        responses::sse(vec![
            responses::ev_response_created("h2"),
            responses::ev_function_call(
                "checkpoint",
                "organization",
                r#"{"type":"checkpoint","content":{"completedSteps":["Prepared partial report"],"remainingSteps":["Finish and run oracle"],"blocker":"Owner must choose continuation"}}"#,
            ),
            responses::ev_completed_with_tokens("h2", 15),
        ]),
        responses::sse(vec![
            responses::ev_response_created("h3"),
            responses::ev_function_call(
                "continue",
                "exec_command",
                r#"{"cmd":"python3 -c \"from pathlib import Path; assert Path('input.txt').read_text() == 'pinned input'; assert Path('report.txt').read_text() == 'partial report'; Path('report.txt').write_text('checked continuation')\"","login":false}"#,
            ),
            responses::ev_completed_with_tokens("h3", 15),
        ]),
        responses::sse(vec![
            responses::ev_response_created("h4"),
            responses::ev_assistant_message("done", "Continuation ready for review."),
            responses::ev_completed_with_tokens("h4", 15),
        ]),
    ];
    let provider = app_test_support::create_mock_responses_server_sequence(sequence).await;
    let home = TempDir::new()?;
    let project = TempDir::new()?;
    let installed = home.path().join("runtime");
    std::fs::create_dir(&installed)?;
    for name in ["codex-app-server", "codex-code-mode-host"] {
        let source = codex_utils_cargo_bin::cargo_bin(name)?;
        std::fs::hard_link(&source, installed.join(name))
            .or_else(|_| codex_utils_cargo_bin::copy_executable(&source, &installed.join(name)))?;
    }
    std::fs::write(project.path().join("input.txt"), "pinned input")?;
    std::fs::write(
        project.path().join("oracle.py"),
        "from pathlib import Path\nassert Path('report.txt').read_text() == 'checked continuation'\nprint('HANDOFF_ORACLE_PASSED')\n",
    )?;
    MockResponsesConfig::new(&provider.uri())
        .with_model("gpt-5.4")
        .with_provider_config("supports_websockets = false")
        .enable_feature(Feature::Sqlite)
        .write(home.path())?;
    let mut server = TestAppServer::builder()
        .with_codex_home(home.path())
        .with_program(&installed.join("codex-app-server"))
        .without_auto_env()
        .with_env_overrides(&[("CODEX_EXEC_SERVER_URL", None)])
        .build_initialized()
        .await?;
    let mut nodes = Vec::new();
    for label in ["Custom first owner", "Custom receiver"] {
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
        nodes.push(node);
    }
    let project_root = AbsolutePathBuf::try_from(project.path())?;
    let linked: ProjectCreateResponse = server
        .request(|request_id| ClientRequest::ProjectCreate {
            request_id,
            params: ProjectCreateParams {
                name: "Handoff fixture".into(),
                roots: vec![ProjectRoot { path: project_root }],
                metadata: None,
                idempotency_key: "handoff-project".into(),
            },
        })
        .await?;
    definition(
        &mut server,
        OrganizationMutation::LinkProject {
            project_id: linked.project.id.clone(),
            expected_revision: None,
            goal_id: None,
            owner_id: Some(nodes[0].id.clone()),
            criteria: vec!["Verified continuation".into()],
        },
    )
    .await?;
    let OrganizationRecord::Task(task) = definition(
        &mut server,
        OrganizationMutation::PutTask {
            id: None,
            expected_revision: None,
            project_id: linked.project.id,
            owner_id: nodes[0].id.clone(),
            goal_id: None,
            instructions: "Prepare, block, then continue the report from a verified checkpoint"
                .into(),
            criteria: vec!["Immutable owner oracle passes".into()],
            prerequisites: vec![],
        },
    )
    .await?
    else {
        unreachable!()
    };
    let scope = OrganizationExecutionScope {
        cost_budget: None,
        model: "gpt-5.4".into(),
        input_files: vec!["input.txt".into(), "oracle.py".into()],
        immutable_files: vec!["oracle.py".into()],
        artifacts: vec!["report.txt".into()],
        checks: vec![vec!["python3".into(), "oracle.py".into()]],
        max_attempts: 3,
        max_model_requests: 4,
        max_tokens: 1000,
        max_seconds: 120,
        require_tool_approval: true,
        delegates: vec![nodes[1].id.clone()],
        max_children: 0,
        auto_accept_children: false,
        not_before: None,
    };
    let authorized = execute(
        &mut server,
        OrganizationExecutionMutation::Authorize {
            task_id: task.id.clone(),
            expected_revision: task.revision,
            scope: scope.clone(),
        },
    )
    .await?;
    let grant = authorized.authorization.unwrap();
    let waiting = wait_task(&mut server, &task.id, "waitingApproval").await?;
    let approval = &waiting.approvals[0];
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
    let blocked = tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            let view = execution(&mut server, &task.id).await?;
            if view.task.state == "blocked" && !view.attempts[0].runtime_active {
                return Ok::<_, anyhow::Error>(view);
            }
            anyhow::ensure!(
                !matches!(view.task.state.as_str(), "failed" | "needsReconciliation"),
                "checkpoint failed: {view:?}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;
    assert_eq!(
        (
            blocked.authorization_used_tokens,
            blocked.authorization_model_requests
        ),
        (30, 2)
    );
    let checkpoint = &blocked.checkpoints[0];
    assert_eq!(
        checkpoint.completed_action_ids,
        vec![blocked.actions[0].id.clone()]
    );
    assert_eq!(checkpoint.files.len(), 3);
    assert_eq!(checkpoint.authorization_id, grant.id);
    let source = std::path::Path::new(blocked.attempts[0].workspace.as_deref().unwrap());
    let mutation = OrganizationExecutionMutation::Handoff {
        task_id: task.id.clone(),
        expected_revision: blocked.task.revision,
        checkpoint_id: checkpoint.id.clone(),
        owner_id: nodes[1].id.clone(),
        expected_owner_revision: nodes[1].revision,
        instructions: "Owner resolved the blocker; finish only the remaining report".into(),
    };
    std::fs::write(source.join("report.txt"), "tampered checkpoint")?;
    assert!(
        denied_execution(&mut server, mutation.clone())
            .await?
            .contains("checkpoint file changed")
    );
    let unchanged = execution(&mut server, &task.id).await?;
    assert_eq!(unchanged.task, blocked.task);
    assert!(unchanged.handoffs.is_empty());
    // Restore only this test-owned file; canonical state and counters never reset.
    std::fs::write(source.join("report.txt"), "partial report")?;
    std::fs::write(project.path().join("input.txt"), "owner changed input")?;
    execute(&mut server, mutation.clone()).await?;
    assert!(
        denied_execution(&mut server, mutation)
            .await?
            .contains("exact blocked task revision")
    );
    let receiver = wait_task(&mut server, &task.id, "waitingApproval").await?;
    assert_eq!(receiver.task.owner_id, nodes[1].id);
    assert_eq!(receiver.authorization.as_ref().unwrap().id, grant.id);
    assert_eq!(receiver.authorization.as_ref().unwrap().scope, scope);
    assert_eq!(receiver.attempts[0].generation, 2);
    assert_eq!(receiver.handoffs.len(), 1);
    let destination = std::path::Path::new(receiver.attempts[0].workspace.as_deref().unwrap());
    assert_ne!(source, destination);
    assert_eq!(
        std::fs::read_to_string(destination.join("input.txt"))?,
        "pinned input"
    );
    assert_eq!(
        std::fs::read_to_string(destination.join("report.txt"))?,
        "partial report"
    );
    let approval = receiver
        .approvals
        .iter()
        .find(|approval| approval.state == OrganizationApprovalState::Pending)
        .unwrap();
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
    let reviewed = wait_task(&mut server, &task.id, "needsReview").await?;
    assert_eq!(
        (
            reviewed.authorization_used_tokens,
            reviewed.authorization_model_requests
        ),
        (60, 4)
    );
    assert_eq!(reviewed.attempts.len(), 2);
    let result = reviewed.result.unwrap();
    assert_eq!(result.checks[0].exit_code, Some(0));
    assert!(result.checks[0].output.contains("HANDOFF_ORACLE_PASSED"));
    assert_eq!(result.artifact_bases, checkpoint.artifact_bases);
    assert_eq!(
        std::fs::read_to_string(destination.join("report.txt"))?,
        "checked continuation"
    );
    assert!(!project.path().join("report.txt").exists());
    assert_eq!(provider.received_requests().await.unwrap().len(), 4);
    assert!(server.shutdown_gracefully().await?.success());
    Ok(())
}
