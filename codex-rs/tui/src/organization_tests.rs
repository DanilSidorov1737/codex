use super::*;

#[tokio::test]
async fn background_requests_survive_startup_overlap_and_cancelled_dashboard() {
    use futures::{SinkExt, StreamExt};
    use serde_json::json;

    tokio::time::timeout(Duration::from_secs(10), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            crate::resolve_remote_addr(&format!("ws://{}", listener.local_addr().unwrap()))
                .unwrap();
        let (account_seen, mut account_ready) = mpsc::channel(1);
        let (list_seen, mut list_ready) = mpsc::channel(1);
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut held_account = None;
            let mut held_list = None;
            let mut first_list = true;
            let mut seen_ids = BTreeSet::new();
            while let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) =
                socket.next().await
            {
                let JSONRPCMessage::Request(request) = serde_json::from_str(&text).unwrap() else {
                    continue;
                };
                let result = match request.method.as_str() {
                    "initialize" => json!({"userAgent": "organization-overlap-test"}),
                    "account/read" => {
                        assert_eq!(request.id, RequestId::Integer(0));
                        held_account = Some(request.id);
                        account_seen.send(()).await.unwrap();
                        continue;
                    }
                    "configRequirements/read" => {
                        // Release both old replies only after the replacement worker succeeded.
                        for (id, result) in [
                            (
                                held_account.take().unwrap(),
                                json!({"account": null, "requiresOpenaiAuth": false}),
                            ),
                            (
                                held_list.take().unwrap(),
                                json!({"data": [], "nextCursor": null}),
                            ),
                        ] {
                            socket
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    json!({"id": id, "result": result}).to_string().into(),
                                ))
                                .await
                                .unwrap();
                        }
                        json!({"requirements": null})
                    }
                    method => {
                        assert!(
                            seen_ids.insert(serde_json::to_string(&request.id).unwrap()),
                            "cancelled and replacement dashboard requests must not share IDs"
                        );
                        match method {
                            "organization/list" => {
                                if first_list {
                                    first_list = false;
                                    held_list = Some(request.id);
                                    list_seen.send(()).await.unwrap();
                                    continue;
                                }
                                let params = request.params.as_ref().unwrap();
                                let next = (params["kind"] == "nodes"
                                    && params["cursor"].is_null())
                                .then_some("second-page");
                                json!({"data": [], "nextCursor": next})
                            }
                            "organization/execution/read" | "organization/execution/mutate" => {
                                json!({"view": view("one")})
                            }
                            "project/create" => {
                                let params = request.params.as_ref().unwrap();
                                assert_eq!(params["name"], "Native project");
                                assert_eq!(params["idempotencyKey"], "stable-project-key");
                                json!({"project": {"id":"server-project-id","name":"Native project","roots":[{"path":"/tmp"}],"metadata":{},"position":0,"createdAt":0,"updatedAt":0,"recencyAt":null}})
                            }
                            "organization/mutate" => {
                                let mutation = &request.params.as_ref().unwrap()["mutation"];
                                if mutation["type"] == "linkProject" {
                                    assert_eq!(mutation["projectId"], "server-project-id");
                                    assert_eq!(mutation["goalId"], "goal");
                                    assert_eq!(mutation["ownerId"], "owner");
                                    json!({"record":{"type":"project","data":{"id":"server-project-id","name":"Native project","roots":["/tmp"],"goalId":"goal","ownerId":"owner","criteria":["Checked"],"revision":1}}})
                                } else { json!({"record": null}) }
                            },
                            _ => panic!("unexpected request: {method}"),
                        }
                    }
                };
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!({"id": request.id, "result": result})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
            }
        });
        let client = crate::connect_remote_app_server(endpoint).await.unwrap();
        let handle = client.request_handle();
        let account_handle = handle.clone();
        let startup = tokio::spawn(async move {
            account_handle
                .request_typed::<GetAccountResponse>(ClientRequest::GetAccount {
                    request_id: RequestId::Integer(0),
                    params: GetAccountParams {
                        refresh_token: false,
                    },
                })
                .await
        });
        account_ready.recv().await.unwrap();
        let old_handle = handle.clone();
        let mut old_dashboard = tokio::spawn(async move { load(&old_handle).await });
        tokio::select! {
            seen = list_ready.recv() => { seen.unwrap(); }
            result = &mut old_dashboard => panic!("dashboard collided with startup: {:?}",
                result.map(|r| r.map(|s| s.records.len()))),
        }
        old_dashboard.abort();
        assert!(matches!(old_dashboard.await, Err(e) if e.is_cancelled()));
        let (commands, command_rx) = mpsc::channel(4);
        let (updates, mut update_rx) = mpsc::channel(4);
        let background = tokio::spawn(worker(
            handle.clone(),
            FrameRequester::test_dummy(),
            command_rx,
            updates,
        ));
        assert!(matches!(update_rx.recv().await, Some(Update::Snapshot(_))));
        commands.send(Command::Inspect("one".into())).await.unwrap();
        assert!(matches!(update_rx.recv().await, Some(Update::Detail(v)) if v.task.id == "one"));
        commands
            .send(Command::Definition(OrganizationMutation::SetMembership {
                node_id: "owner".into(),
                group_id: "group".into(),
                present: true,
            }))
            .await
            .unwrap();
        assert!(matches!(update_rx.recv().await, Some(Update::Changed)));
        commands.send(Command::Project {
            params: serde_json::from_value(json!({"name":"Native project","roots":[{"path":"/tmp"}],"idempotencyKey":"stable-project-key"})).unwrap(),
            goal_id: Some("goal".into()), owner_id: Some("owner".into()), criteria: vec!["Checked".into()],
        }).await.unwrap();
        assert!(matches!(update_rx.recv().await, Some(Update::Saved(OrganizationRecord::Project(p))) if p.id == "server-project-id"));
        assert!(matches!(update_rx.recv().await, Some(Update::Changed)));
        commands
            .send(Command::Execution(OrganizationExecutionMutation::Stop {
                task_id: "one".into(),
                expected_revision: 4,
                cancel: false,
            }))
            .await
            .unwrap();
        assert!(matches!(update_rx.recv().await, Some(Update::Detail(v)) if v.task.id == "one"));
        assert!(matches!(update_rx.recv().await, Some(Update::Changed)));
        commands.send(Command::Refresh).await.unwrap();
        assert!(matches!(update_rx.recv().await, Some(Update::Snapshot(_))));
        assert!(matches!(update_rx.recv().await, Some(Update::Detail(v)) if v.task.id == "one"));
        background.abort();
        assert!(background.await.unwrap_err().is_cancelled());
        let _: ConfigRequirementsReadResponse = handle
            .request_typed(ClientRequest::ConfigRequirementsRead {
                request_id: RequestId::String("release-startup".into()),
                params: None,
            })
            .await
            .unwrap();
        let account = startup.await.unwrap().unwrap();
        assert!(account.account.is_none());
        assert!(!account.requires_openai_auth);
        client.shutdown().await.unwrap();
        server.await.unwrap();
    })
    .await
    .expect("overlapping requests must complete without waiting for old replies");
}

fn node(id: &str, parent: Option<&str>) -> OrganizationNode {
    OrganizationNode {
        id: id.into(),
        parent_id: parent.map(str::to_string),
        kind: OrganizationNodeKind::Agent,
        label: format!("Custom {id}"),
        role_version_id: None,
        model: None,
        enabled: true,
        revision: 1,
    }
}

#[test]
fn deep_tree_search_crosses_collapsed_branches_and_keeps_stable_ids() {
    let nodes: Vec<_> = (0..1000)
        .map(|i| {
            node(
                &format!("n{i:04}"),
                if i == 0 { None } else { Some("unused") },
            )
        })
        .enumerate()
        .map(|(i, mut n)| {
            n.parent_id = (i > 0).then(|| format!("n{:04}", i - 1));
            n
        })
        .collect();
    let collapsed = BTreeSet::from(["n0000".to_string()]);
    assert_eq!(tree(&nodes, &collapsed, "").len(), 1);
    let found = tree(&nodes, &collapsed, "n0999");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "n0999");
    assert!(found[0].label.len() < 100);
    assert_eq!(tree(&nodes, &BTreeSet::new(), "").len(), 1000);
}

#[test]
fn preferences_roundtrip_all_presets_and_reject_aliases() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("layouts.json");
    let mut prefs = Preferences::default();
    prefs.preset = Preset::Review;
    prefs.panes[Preset::Operations.index()].width = 65;
    prefs.panes[Preset::Workspace.index()].inspector = false;
    prefs.panes[Preset::Projects.index()].inspector_focus = true;
    prefs.panes[Preset::Review.index()].vim_keys = false;
    prefs.save(&path).unwrap();
    let restored = Preferences::load(&path).unwrap();
    assert_eq!(restored.preset, Preset::Review);
    assert_eq!(restored.panes[3].width, 65);
    assert!(!restored.panes[1].inspector);
    assert!(restored.panes[2].inspector_focus);
    assert!(!restored.panes[4].vim_keys);
    #[cfg(unix)]
    {
        let alias = home.path().join("alias");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        assert!(Preferences::load(&alias).is_err());
        std::fs::remove_file(&alias).unwrap();
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(Preferences::load(&path).is_err());
    }
}

fn task(id: &str) -> OrganizationTask {
    OrganizationTask {
        id: id.into(),
        project_id: "project".into(),
        goal_id: None,
        owner_id: "owner".into(),
        instructions: "Check a real artifact".into(),
        criteria: vec!["Check passes".into()],
        prerequisites: vec![],
        state: "needsReview".into(),
        revision: 4,
    }
}

fn view(id: &str) -> OrganizationExecutionView {
    OrganizationExecutionView {
        task: task(id),
        authorization: None,
        authorization_used_tokens: 10,
        authorization_model_requests: 1,
        attempts: vec![],
        actions: vec![],
        approvals: vec![OrganizationActionApproval {
            id: "approval".into(),
            attempt_id: "attempt".into(),
            action_id: "action".into(),
            action_hash: "frozen-hash".into(),
            tool: "exec_command".into(),
            arguments: "{\"cmd\":\"python3 oracle.py\"}".into(),
            state: OrganizationApprovalState::Pending,
            expires_at: 100,
        }],
        result: None,
        applications: vec![],
    }
}

fn dashboard() -> Dashboard {
    let (commands, _) = mpsc::channel(4);
    let (_, updates) = mpsc::channel(4);
    Dashboard {
        preferences: Preferences::default(),
        path: PathBuf::from("unused"),
        snapshot: Snapshot {
            records: vec![
                OrganizationRecord::Node(node("owner", None)),
                OrganizationRecord::Task(task("one")),
                OrganizationRecord::Task(task("two")),
            ],
        },
        rows: vec![],
        selected: None,
        collapsed: BTreeSet::new(),
        detail: None,
        scroll: 0,
        detail_text: String::new(),
        review_diff: None,
        review_file: 0,
        search: String::new(),
        input: None,
        changes: false,
        change_target: None,
        confirmation: None,
        confirmation_text: String::new(),
        message: String::new(),
        commands,
        updates,
        worker: tokio::spawn(std::future::pending()),
        editor: None,
        preview: None,
        editor_area: Rect::default(),
        editor_file: 0,
        following: false,
        applying: false,
        provider: "test".into(),
        model: "model".into(),
        form: None,
        prices: None,
        frame: FrameRequester::test_dummy(),
        is_done: false,
    }
}

#[tokio::test]
async fn creation_requires_review_and_confirm_and_cancel_preserves_the_draft() {
    let mut d = dashboard();
    let (tx, mut rx) = mpsc::channel(4);
    d.commands = tx;
    d.open_form('G');
    d.form.as_mut().unwrap().paste("Prepare a verified report");
    let key = |code| KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    d.key(key(KeyCode::Tab));
    d.form.as_mut().unwrap().paste("Facts match the source");
    d.key(key(KeyCode::F(6)));
    assert!(rx.try_recv().is_err());
    assert!(d.confirmation_text.contains("Prepare a verified report"));
    d.key(key(KeyCode::Esc));
    assert!(d.form.is_some());
    assert!(rx.try_recv().is_err());
    d.key(key(KeyCode::F(6)));
    d.key(key(KeyCode::Char('y')));
    assert!(d.form.is_none());
    assert!(
        matches!(rx.try_recv(), Ok(Command::Definition(OrganizationMutation::PutGoal { objective, .. })) if objective == "Prepare a verified report")
    );
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn authorization_draft_is_bound_to_captured_task_and_revision() {
    let mut d = dashboard();
    d.snapshot.records = vec![OrganizationRecord::Task(OrganizationTask {
        state: "proposed".into(),
        ..task("one")
    })];
    d.preferences.preset = Preset::Review;
    d.rebuild();
    d.open_form('s');
    d.selected = Some("another-task".into());
    let form = d.form.as_mut().unwrap();
    let key = |code| KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    form.key(key(KeyCode::Tab));
    form.key(key(KeyCode::Tab));
    form.paste("report.txt");
    d.key(key(KeyCode::F(6)));
    assert!(
        matches!(&d.confirmation, Some(Command::Execution(OrganizationExecutionMutation::Authorize { task_id, expected_revision: 4, .. })) if task_id == "one")
    );
    assert!(d.confirmation_text.contains("no agent network"));
    assert!(d.confirmation_text.contains("Cost estimate unavailable"));
}

#[tokio::test]
async fn selection_never_approves_another_tasks_detail_and_confirmation_is_frozen() {
    let mut d = dashboard();
    d.preferences.preset = Preset::Review;
    d.rebuild();
    d.selected = Some("two".into());
    d.detail = Some(Box::new(view("one")));
    d.decision('a');
    assert!(d.confirmation.is_none());
    d.detail = Some(Box::new(view("two")));
    d.decision('a');
    d.detail.as_mut().unwrap().approvals[0].action_hash = "changed".into();
    let Some(Command::Execution(OrganizationExecutionMutation::DecideApproval {
        task_id,
        expected_hash,
        ..
    })) = &d.confirmation
    else {
        panic!("missing owner decision")
    };
    assert_eq!(task_id, "two");
    assert_eq!(expected_hash, "frozen-hash");
    assert!(d.details().contains("frozen-hash"));
    assert!(d.confirmation_text.contains("frozen-hash"));
    assert!(!d.confirmation_text.contains("changed"));
    d.key(KeyEvent::new(
        KeyCode::PageDown,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert_eq!(d.scroll, 10);
    assert!(d.confirmation.is_some());
}

#[tokio::test]
async fn request_changes_draft_remains_bound_to_the_original_task_revision() {
    let mut d = dashboard();
    d.preferences.preset = Preset::Review;
    d.rebuild();
    d.selected = Some("one".into());
    d.detail = Some(Box::new(view("one")));
    d.decision('c');
    d.selected = Some("two".into());
    d.detail = Some(Box::new(view("two")));
    d.input = Some("Clarify the conclusion".into());
    d.key(KeyEvent::new(
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ));
    let Some(Command::Execution(OrganizationExecutionMutation::RequestChanges {
        task_id,
        expected_revision,
        instructions,
    })) = &d.confirmation
    else {
        panic!("missing captured change request")
    };
    assert_eq!(task_id, "one");
    assert_eq!(*expected_revision, 4);
    assert_eq!(instructions, "Clarify the conclusion");
}

#[tokio::test]
async fn refreshed_snapshot_preserves_selection_but_removes_deleted_selection() {
    let mut d = dashboard();
    let (commands, mut command_rx) = mpsc::channel(4);
    let (updates, update_rx) = mpsc::channel(4);
    d.commands = commands;
    d.updates = update_rx;
    d.preferences.preset = Preset::Review;
    d.selected = Some("two".into());
    d.rebuild();
    assert_eq!(d.selected.as_deref(), Some("two"));
    updates
        .send(Update::Snapshot(Snapshot {
            records: vec![OrganizationRecord::Task(task("one"))],
        }))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.selected.as_deref(), Some("one"));
    assert!(matches!(command_rx.try_recv(), Ok(Command::Inspect(id)) if id == "one"));
    updates
        .send(Update::Snapshot(Snapshot {
            records: vec![OrganizationRecord::Task(task("one"))],
        }))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.selected.as_deref(), Some("one"));
    assert!(matches!(
        command_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    d.search = "new".into();
    d.rebuild();
    assert!(d.selected.is_none());
    updates
        .send(Update::Snapshot(Snapshot {
            records: vec![OrganizationRecord::Task(task("new"))],
        }))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.selected.as_deref(), Some("new"));
    assert!(matches!(command_rx.try_recv(), Ok(Command::Inspect(id)) if id == "new"));
}

#[tokio::test]
async fn all_dashboard_presets_render_narrow_empty_and_untrusted_content_safely() {
    let mut d = dashboard();
    d.snapshot
        .records
        .push(OrganizationRecord::Node(OrganizationNode {
            label: "\x1b[2Jinjected\u{7}".into(),
            ..node("unsafe", None)
        }));
    for preset in [
        Preset::Workspace,
        Preset::Projects,
        Preset::Operations,
        Preset::Review,
    ] {
        d.preferences.preset = preset;
        d.rebuild();
        for (width, height) in [(1, 1), (36, 12), (50, 20), (120, 30), (160, 48)] {
            let area = Rect::new(0, 0, width, height);
            let mut buffer = Buffer::empty(area);
            d.render(area, &mut buffer);
            assert!(
                buffer
                    .content
                    .iter()
                    .all(|cell| !cell.symbol().chars().any(|c| c == '\x1b' || c == '\u{7}'))
            );
        }
    }
}

#[tokio::test]
async fn inline_review_diff_uses_verified_artifact_and_cannot_enable_acceptance() {
    use sha2::{Digest, Sha256};
    let home = tempfile::tempdir().unwrap();
    let original = tempfile::tempdir().unwrap();
    let attempt = tempfile::tempdir().unwrap();
    std::fs::write(original.path().join("report.txt"), "Original\n").unwrap();
    std::fs::write(attempt.path().join("report.txt"), "Verified output\n").unwrap();
    std::fs::write(attempt.path().join("notes.md"), "Second artifact\n").unwrap();
    let mut d = dashboard();
    d.path = home.path().join("layouts.json");
    d.snapshot
        .records
        .push(OrganizationRecord::Project(OrganizationProjectPlan {
            id: "project".into(),
            name: "Reviewed project".into(),
            roots: vec![original.path().to_str().unwrap().into()],
            goal_id: None,
            owner_id: Some("owner".into()),
            criteria: vec!["Check passes".into()],
            revision: 1,
        }));
    d.preferences.preset = Preset::Review;
    d.selected = Some("one".into());
    let mut detail = view("one");
    detail.attempts = vec![serde_json::from_value(serde_json::json!({"id":"attempt","taskId":"one","authorizationId":"grant","generation":1,"threadId":"thread","turnId":null,"workspace":attempt.path(),"state":"needsReview","startedAt":0,"leaseUntil":0,"reportedTokens":0,"modelRequests":0,"reportedModelRequests":0})).unwrap()];
    detail.result = Some(OrganizationTaskResult {
        attempt_id: "attempt".into(),
        artifacts: vec![
            OrganizationArtifactEvidence {
                path: "report.txt".into(),
                bytes: 16,
                sha256: format!("{:x}", Sha256::digest(b"Verified output\n")),
            },
            OrganizationArtifactEvidence {
                path: "notes.md".into(),
                bytes: 16,
                sha256: format!("{:x}", Sha256::digest(b"Second artifact\n")),
            },
        ],
        artifact_bases: vec![],
        checks: vec![],
        digest: "evidence-digest".into(),
        accepted: false,
    });
    d.detail = Some(Box::new(detail));
    d.refresh_diff();
    let diff = &d.review_diff.as_ref().unwrap().2;
    assert!(diff.contains("-Original"));
    assert!(diff.contains("+Verified output"));
    assert!(d.confirmation.is_none());
    assert!(!d.detail.as_ref().unwrap().result.as_ref().unwrap().accepted);
    d.key(KeyEvent::new(
        KeyCode::Char('n'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(
        d.review_diff
            .as_ref()
            .unwrap()
            .2
            .contains("+Second artifact")
    );
    assert!(
        !d.review_diff
            .as_ref()
            .unwrap()
            .2
            .contains("+Verified output")
    );
    assert_eq!(d.editor_file, 1);
    d.key(KeyEvent::new(
        KeyCode::Char('n'),
        crossterm::event::KeyModifiers::NONE,
    ));
    std::fs::write(attempt.path().join("report.txt"), "Unreviewed bytes\n").unwrap();
    d.review_diff = None;
    d.refresh_diff();
    let diff = &d.review_diff.as_ref().unwrap().2;
    assert!(diff.contains("artifact changed"));
    assert!(!diff.contains("+Unreviewed"));
}

#[tokio::test]
async fn actual_dirty_editor_blocks_host_exit_and_owner_apply_dispatch() {
    let home = tempfile::tempdir().unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&home.path().join("human.txt"), false).unwrap();
    editor
        .key(KeyEvent::new(
            KeyCode::Char('i'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
    editor.paste("unsaved human change").unwrap();
    editor
        .key(KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
    editor.focused = false;
    let mut d = dashboard();
    d.editor = Some(editor);
    d.preferences.preset = Preset::Review;
    d.rebuild();
    d.selected = Some("one".into());
    let mut v = view("one");
    v.task.state = "accepted".into();
    v.result = Some(OrganizationTaskResult {
        attempt_id: "attempt".into(),
        artifacts: vec![OrganizationArtifactEvidence {
            path: "human.txt".into(),
            sha256: "a".repeat(64),
            bytes: 7,
        }],
        artifact_bases: vec![],
        checks: vec![],
        digest: "exact-result".into(),
        accepted: true,
    });
    d.detail = Some(Box::new(v));
    let (tx, mut rx) = mpsc::channel(4);
    d.commands = tx;
    d.key(KeyEvent::new(
        KeyCode::Char('q'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(!d.is_done);
    assert!(d.message.contains("modified Neovim"));
    d.open_form('A');
    assert!(d.form.is_some());
    d.key(KeyEvent::new(
        KeyCode::F(6),
        crossterm::event::KeyModifiers::NONE,
    ));
    d.key(KeyEvent::new(
        KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(rx.try_recv().is_err());
    assert!(!d.applying);
    assert!(d.message.contains("Cannot freeze editor revision"));
    assert!(d.editor.as_mut().unwrap().dirty().unwrap());
    assert!(!home.path().join("human.txt").exists());
    let (updates, incoming) = mpsc::channel(4);
    d.updates = incoming;
    updates
        .send(Update::Snapshot(Snapshot {
            records: d.snapshot.records.clone(),
        }))
        .await
        .unwrap();
    d.update();
    assert!(d.message.contains("Cannot freeze editor revision"));
    assert!(d.confirmation.is_some());
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn selective_apply_confirmation_keeps_captured_paths_task_and_revision() {
    let mut d = dashboard();
    d.preferences.preset = Preset::Review;
    d.selected = Some("one".into());
    let mut accepted = view("one");
    accepted.task.state = "accepted".into();
    accepted.result = Some(OrganizationTaskResult {
        attempt_id: "attempt".into(),
        artifacts: ["report.txt", "notes.md"]
            .into_iter()
            .map(|p| OrganizationArtifactEvidence {
                path: p.into(),
                sha256: "a".repeat(64),
                bytes: 1,
            })
            .collect(),
        artifact_bases: vec![],
        checks: vec![],
        digest: "captured-result".into(),
        accepted: true,
    });
    d.detail = Some(Box::new(accepted));
    let (tx, mut rx) = mpsc::channel(4);
    d.commands = tx;
    let key = |code| KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    d.key(key(KeyCode::Char('A')));
    d.key(key(KeyCode::Tab));
    d.key(key(KeyCode::Char(' ')));
    assert!(rx.try_recv().is_err());
    d.selected = Some("two".into());
    d.detail = Some(Box::new(view("two")));
    d.key(key(KeyCode::F(6)));
    assert!(d.confirmation_text.contains("installed subset"));
    d.key(key(KeyCode::Esc));
    assert!(d.form.is_some());
    assert!(rx.try_recv().is_err());
    d.key(key(KeyCode::F(6)));
    d.key(key(KeyCode::Char('y')));
    let Command::Execution(OrganizationExecutionMutation::Apply {
        task_id,
        expected_revision,
        digest,
        paths,
    }) = rx.try_recv().unwrap()
    else {
        panic!("missing apply")
    };
    assert_eq!(task_id, "one");
    assert_eq!(expected_revision, 4);
    assert_eq!(digest, "captured-result");
    assert_eq!(paths, Some(vec!["report.txt".into()]));
    assert!(d.applying && d.form.is_none());
}
