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
                            "organization/report" => json!({"report": empty_report()}),
                            "organization/schedule/list" | "organization/workflow/list" => json!({"data":[],"nextCursor":null}),
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
    prefs.panes[Preset::Operations.index()].sidebar_width = 38;
    prefs.panes[Preset::Workspace.index()].sidebar = false;
    prefs.panes[Preset::Projects.index()].actions = false;
    prefs.save(&path).unwrap();
    let restored = Preferences::load(&path).unwrap();
    assert_eq!(restored.preset, Preset::Review);
    assert_eq!(restored.panes[3].width, 65);
    assert!(!restored.panes[1].inspector);
    assert!(restored.panes[2].inspector_focus);
    assert!(!restored.panes[4].vim_keys);
    assert_eq!(restored.panes[3].sidebar_width, 38);
    assert!(!restored.panes[1].sidebar);
    assert!(!restored.panes[2].actions);
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
        checkpoints: vec![],
        handoffs: vec![],
    }
}

fn dashboard() -> Dashboard {
    let (commands, _) = mpsc::channel(4);
    let (_, updates) = mpsc::channel(4);
    Dashboard {
        preferences: Preferences::default(),
        path: PathBuf::from("unused"),
        projection: Projection::Default,
        snapshot: Snapshot {
            report: Some(report_for_tasks(&["one", "two"])),
            schedules: vec![],
            workflows: vec![],
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
        report_text: None,
        report_request: None,
        review_diff: None,
        review_file: 0,
        search: String::new(),
        input: None,
        memory_request: None,
        memory_result: None,
        changes: false,
        change_target: None,
        confirmation: None,
        confirmation_text: String::new(),
        selection_confirmation: None,
        message: String::new(),
        message_is_error: false,
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
        palette: None,
        prices: None,
        frame: FrameRequester::test_dummy(),
        is_done: false,
    }
}

#[tokio::test]
async fn presets_are_discoverable_and_forms_pin_revisions_without_sending_authority() {
    let mut d = dashboard();
    let (tx, mut rx) = mpsc::channel(8);
    d.commands = tx;
    d.preferences.preset = Preset::Operations;
    d.selected = Some("owner".into());
    d.key(KeyCode::Char('D').into());
    assert!(matches!(d.form.as_ref().unwrap().command(),
        Ok(Command::Definition(OrganizationMutation::SavePreset { root_id, expected_root_revision: 1, .. })) if root_id == "owner"));
    assert!(rx.try_recv().is_err());
    d.key(KeyCode::Esc.into());
    let preset = OrganizationPreset {
        id: "preset".into(),
        label: "Arbitrary organization".into(),
        revision: 7,
        nodes: vec![node("owner", None)],
        roles: vec![],
        memberships: vec![],
        last_clone: None,
    };
    d.snapshot
        .records
        .push(OrganizationRecord::Preset(Box::new(preset)));
    d.key(KeyCode::F(9).into());
    assert_eq!(d.projection, Projection::Presets);
    assert_eq!(d.rows.len(), 1);
    d.key(KeyCode::Char('V').into());
    d.key(KeyCode::F(6).into());
    assert!(matches!(d.confirmation,
        Some(Command::Definition(OrganizationMutation::ClonePreset { ref id, expected_revision: 7, ref label, parent_id: None })) if id=="preset" && label=="Arbitrary organization copy"));
    assert!(rx.try_recv().is_err());
    for (width, height) in [(160, 48), (50, 20)] {
        let area = Rect::new(0, 0, width, height);
        d.render(area, &mut Buffer::empty(area));
    }
    d.key(KeyCode::Esc.into());
    d.key(KeyCode::Esc.into());
    for (code, expected) in [
        (KeyCode::Char('g'), Projection::Goals),
        (KeyCode::Char('S'), Projection::Schedules),
        (KeyCode::Char('w'), Projection::Workflows),
        (KeyCode::F(9), Projection::Presets),
        (KeyCode::F(9), Projection::Default),
    ] {
        d.key(code.into());
        assert_eq!(d.projection, expected);
    }
}

#[tokio::test]
async fn palette_search_and_cancel_never_dispatch_and_selection_uses_existing_form() {
    let mut d = dashboard();
    let (commands, mut receiver) = mpsc::channel(4);
    d.commands = commands;
    let mut proposed = task("proposal");
    proposed.state = "proposed".into();
    d.snapshot.records.push(OrganizationRecord::Task(proposed));
    d.selected = Some("proposal".into());
    d.key(KeyEvent::new(
        KeyCode::Char('p'),
        crossterm::event::KeyModifiers::CONTROL,
    ));
    for c in "authorize task".chars() {
        d.key(KeyCode::Char(c).into());
    }
    assert!(receiver.try_recv().is_err());
    d.key(KeyCode::Enter.into());
    assert!(d.palette.is_none() && d.form.is_some() && d.confirmation.is_none());
    assert!(receiver.try_recv().is_err());
    d.key(KeyCode::Esc.into());
    d.open_palette();
    d.key(KeyCode::Esc.into());
    assert!(d.palette.is_none() && d.form.is_none());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn goal_tree_rolls_up_accepted_tasks_without_claiming_goal_completion() {
    let root = OrganizationGoal {
        id: "mission".into(),
        parent_id: None,
        objective: "Mission".into(),
        criteria: vec!["Owner evidence".into()],
        revision: 1,
    };
    let child = OrganizationGoal {
        id: "milestone".into(),
        parent_id: Some(root.id.clone()),
        objective: "Milestone".into(),
        criteria: vec!["Check".into()],
        revision: 1,
    };
    let mut accepted = task("accepted");
    accepted.goal_id = Some(child.id.clone());
    accepted.state = "accepted".into();
    let mut review = task("review");
    review.goal_id = Some(child.id.clone());
    review.state = "needsReview".into();
    let records = vec![
        OrganizationRecord::Goal(root),
        OrganizationRecord::Goal(child),
        OrganizationRecord::Task(accepted),
        OrganizationRecord::Task(review),
    ];
    let rows = goal_tree(&records, &BTreeSet::new(), "");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| {
        row.label
            .contains("1/2 tasks accepted; criteria need owner review")
    }));
    assert_eq!(
        goal_tree(&records, &BTreeSet::from(["mission".into()]), "").len(),
        1
    );
    assert_eq!(
        goal_tree(&records, &BTreeSet::from(["mission".into()]), "milestone")[0].id,
        "milestone"
    );
}

#[tokio::test]
async fn finite_series_preview_refuses_changed_scope_and_late_closed_draft() {
    let mut d = dashboard();
    let (updates, receiver) = mpsc::channel(4);
    d.updates = receiver;
    let template = OrganizationWorkflowTemplate { discussion:None,
        id: "template".into(),
        project_id: "project".into(),
        label: "Finite work".into(),
        revision: 4,
        items: vec![OrganizationPlanItem {
            key: "step".into(),
            owner_id: "owner".into(),
            instructions: "Write evidence".into(),
            criteria: vec!["Check".into()],
            prerequisites: vec![],
        }],
    };
    let mut form = form::Form::new(
        form::Kind::WorkflowSeries {
            template: template.clone(),
            goal: None,
        },
        &d.snapshot.records,
        None,
        "model",
    );
    form.prefill_scope(&OrganizationExecutionScope {
        input_files: vec!["evidence.txt".into()],
        immutable_files: vec![],
        artifacts: vec!["evidence.txt".into()],
        checks: vec![],
        model: "model".into(),
        max_tokens: 100,
        max_seconds: 60,
        max_model_requests: 2,
        max_attempts: 1,
        require_tool_approval: true,
        delegates: vec![],
        max_children: 0,
        auto_accept_children: false,
        not_before: None,
    });
    let Command::SeriesPreview {
        request,
        config,
        source_task,
        supervision,
    } = form.command().unwrap()
    else {
        panic!("expected series preview")
    };
    d.form = Some(form);
    let mut preview = OrganizationWorkflowPreview {
        id: request.id.clone(),
        template,
        project_revision: 1,
        node_revisions: BTreeMap::from([("owner".into(), 1)]),
        policy_version: 2,
        scopes: request.scopes.clone(),
        max_tokens: 100,
        max_model_requests: 2,
        shared_limits: None,
    };
    preview.scopes[0].scope.max_tokens += 1;
    updates
        .send(Update::SeriesPreview {
            request: request.clone(),
            config: config.clone(),
            source_task: source_task.clone(),
            supervision: supervision.clone(),
            preview: Box::new(preview.clone()),
            trigger: OrganizationWorkflowTrigger::Interval,
        })
        .await
        .unwrap();
    d.update();
    assert!(
        d.confirmation.is_none(),
        "a changed response cannot create authority"
    );
    preview.scopes = request.scopes.clone();
    updates.send(Update::SeriesPreview {
        request: request.clone(), config:config.clone(),source_task:source_task.clone(),
        supervision:Some((Box::new(OrganizationGoal { id:"other-goal".into(),parent_id:None,objective:"Changed policy".into(),criteria:vec![],revision:1 }),"Other work".into())),
        preview:Box::new(preview.clone()),trigger:OrganizationWorkflowTrigger::Interval,
    }).await.unwrap();
    d.update(); assert!(d.confirmation.is_none(),"late policy changes cannot expand the owner's draft");
    updates
        .send(Update::SeriesPreview {
            request: request.clone(),
            config: config.clone(),
            source_task: source_task.clone(),
            supervision: supervision.clone(),
            preview: Box::new(preview.clone()),
            trigger: OrganizationWorkflowTrigger::Interval,
        })
        .await
        .unwrap();
    d.update();
    assert!(matches!(
        d.confirmation,
        Some(Command::Workflow(
            OrganizationWorkflowMutation::CreateSeries { .. }
        ))
    ));
    d.confirmation = None;
    d.form = None;
    updates
        .send(Update::SeriesPreview {
            request,
            config,
            source_task,
            supervision,
            preview: Box::new(preview),
            trigger: OrganizationWorkflowTrigger::Interval,
        })
        .await
        .unwrap();
    d.update();
    assert!(
        d.confirmation.is_none(),
        "closing the draft fences a late response"
    );
}

#[tokio::test]
async fn pane_controls_are_persisted_per_layout_and_reset_only_the_selected_layout() {
    let home = tempfile::tempdir().unwrap();
    let mut d = dashboard();
    d.path = home.path().join("layouts.json");
    d.preferences.preset = Preset::Operations;
    for key in ['}', '}', '~'] {
        d.key(KeyEvent::new(
            KeyCode::Char(key),
            crossterm::event::KeyModifiers::NONE,
        ));
    }
    assert_eq!(d.pane().sidebar_width, 28);
    assert!(!d.pane().actions);
    d.switch(Preset::Projects);
    d.key(KeyEvent::new(
        KeyCode::Char('\\'),
        crossterm::event::KeyModifiers::NONE,
    ));
    let restored = Preferences::load(&d.path).unwrap();
    assert_eq!(restored.panes[Preset::Operations.index()].sidebar_width, 28);
    assert!(!restored.panes[Preset::Operations.index()].actions);
    assert!(!restored.panes[Preset::Projects.index()].sidebar);
    d.switch(Preset::Operations);
    d.key(KeyEvent::new(
        KeyCode::Char('0'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert_eq!(d.pane().sidebar_width, 24);
    assert!(d.pane().actions);
    assert!(!d.preferences.panes[Preset::Projects.index()].sidebar);
}

#[tokio::test]
async fn workflow_views_save_prerequisite_closure_and_freeze_preview_confirmation() {
    let mut d = dashboard();
    let preferences = tempfile::tempdir().unwrap();
    d.path = preferences.path().join("layouts.json");
    let (commands, mut rx) = mpsc::channel(16);
    d.commands = commands;
    let (updates, receiver) = mpsc::channel(16);
    d.updates = receiver;
    if let OrganizationRecord::Task(task) = &mut d.snapshot.records[2] {
        task.prerequisites = vec!["one".into()];
    }
    d.selected = Some("two".into());
    let (project_id, items) = d.workflow_items().unwrap();
    assert_eq!(project_id, "project");
    assert_eq!(items.len(), 2);
    assert_eq!(
        items
            .iter()
            .find(|item| item.key == "two")
            .unwrap()
            .prerequisites,
        vec!["one".to_string()]
    );
    let template = OrganizationWorkflowTemplate { discussion:None,
        id: "template".into(),
        project_id,
        label: "Research café\x1b[31m".into(),
        revision: 2,
        items,
    };
    d.snapshot
        .workflows
        .push(OrganizationWorkflowView::Template {
            template: template.clone(),
        });
    d.projection = Projection::Workflows;
    d.selected = Some("workflow:template".into());
    d.rebuild();
    assert!(d.rows.iter().any(|row| row.id == "workflow:template"));
    d.open_form('s');
    d.form
        .as_mut()
        .unwrap()
        .prefill_scope(&OrganizationExecutionScope {
            input_files: vec!["oracle.py".into()],
            immutable_files: vec!["oracle.py".into()],
            artifacts: vec!["report.txt".into()],
            checks: vec![],
            model: "model".into(),
            max_tokens: 100,
            max_seconds: 120,
            max_model_requests: 2,
            max_attempts: 1,
            require_tool_approval: true,
            delegates: vec![],
            max_children: 0,
            auto_accept_children: false,
            not_before: None,
        });
    let Command::WorkflowPreview(request) = d.form.as_ref().unwrap().command().unwrap() else {
        panic!("expected preview")
    };
    let preview = OrganizationWorkflowPreview {
        shared_limits: request.shared_limits.clone(),
        id: request.id.clone(),
        template,
        project_revision: 3,
        node_revisions: BTreeMap::from([("owner".into(), 1)]),
        policy_version: 2,
        scopes: request.scopes,
        max_tokens: 200,
        max_model_requests: 4,
    };
    d.key(KeyEvent::new(
        KeyCode::F(6),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        Command::WorkflowPreview(_)
    ));
    assert!(d.confirmation.is_none());
    updates
        .send(Update::WorkflowPreview(Box::new(preview.clone())))
        .await
        .unwrap();
    d.update();
    assert!(
        matches!(&d.confirmation, Some(Command::Workflow(OrganizationWorkflowMutation::Run { preview: captured })) if **captured == preview)
    );
    let text = d.details();
    assert!(text.contains("200 tokens / 4 requests"));
    assert!(text.contains("Check passes"));
    for preset in [Preset::Projects, Preset::Operations] {
        d.preferences.preset = preset;
        for (w, h) in [(1, 1), (50, 20), (160, 48)] {
            let area = Rect::new(0, 0, w, h);
            let mut buffer = Buffer::empty(area);
            d.render(area, &mut buffer);
            assert!(
                buffer
                    .content
                    .iter()
                    .all(|cell| !cell.symbol().contains('\x1b'))
            );
        }
    }
    d.confirmation = None;
    d.form = None;
    updates
        .send(Update::WorkflowPreview(Box::new(preview)))
        .await
        .unwrap();
    d.update();
    assert!(
        d.confirmation.is_none(),
        "closed drafts cannot be reopened by late previews"
    );
}

#[tokio::test]
async fn workflow_controls_capture_run_revision_and_need_explicit_confirmation() {
    let mut d = dashboard();
    let (commands, mut rx) = mpsc::channel(16);
    d.commands = commands;
    let run: OrganizationWorkflowRun = serde_json::from_value(serde_json::json!({
        "id":"run-id", "revision":7, "state":"paused", "createdAt":1000,
        "taskIds":{}, "preview":{"id":"run-id", "template":{"id":"template", "projectId":"project", "revision":1, "label":"Workflow", "items":[]},
            "projectRevision":1, "nodeRevisions":{}, "policyVersion":2, "scopes":[], "maxTokens":100, "maxModelRequests":2}
    })).unwrap();
    d.snapshot
        .workflows
        .push(OrganizationWorkflowView::Run { run, tasks: vec![] });
    d.projection = Projection::Workflows;
    d.selected = Some("run:run-id".into());
    d.decision('u');
    assert!(
        rx.try_recv().is_err(),
        "opening confirmation cannot authorize work"
    );
    assert!(
        matches!(d.confirmation.as_ref(), Some(Command::Workflow(OrganizationWorkflowMutation::Control { id, expected_revision:7, action:OrganizationWorkflowControl::Resume })) if id == "run-id")
    );
    assert!(d.confirmation_text.contains("Paused"));
    d.key(KeyEvent::new(
        KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        Command::Workflow(OrganizationWorkflowMutation::Control {
            expected_revision: 7,
            action: OrganizationWorkflowControl::Resume,
            ..
        })
    ));
}

#[tokio::test]
async fn operations_edits_freeze_node_revision_and_latest_role_version() {
    let mut d = dashboard();
    let first = OrganizationRoleVersion {
        id: "role-v1".into(),
        role_id: "custom-role".into(),
        name: "Arbitrary role".into(),
        instructions: "Captured first version".into(),
        version: 1,
    };
    let latest = OrganizationRoleVersion {
        id: "role-v2".into(),
        instructions: "Second version".into(),
        version: 2,
        ..first.clone()
    };
    let mut owner = node("owner", None);
    owner.role_version_id = Some(first.id.clone());
    owner.revision = 9;
    d.snapshot.records = vec![
        OrganizationRecord::Node(owner),
        OrganizationRecord::Role(first),
        OrganizationRecord::Role(latest),
    ];
    d.selected = Some("owner".into());
    d.open_form('L');
    assert!(matches!(
        d.form.as_ref().unwrap().command().unwrap(),
        Command::Definition(OrganizationMutation::CreateRoleVersion {
            expected_version: Some(2),
            ..
        })
    ));
    d.open_form('E');
    d.key(KeyEvent::new(
        KeyCode::F(6),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(
        matches!(&d.confirmation,Some(Command::Definition(OrganizationMutation::PutNode {id:Some(id),expected_revision:Some(9),..})) if id=="owner")
    );
    assert!(d.confirmation_text.contains("may be fenced"));
    if let OrganizationRecord::Node(node) = &mut d.snapshot.records[0] {
        node.revision = 10;
        node.label = "Changed in background".into();
    }
    d.key(KeyEvent::new(
        KeyCode::Char('B'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(
        &d.confirmation,
        Some(Command::Definition(OrganizationMutation::PutNode {
            expected_revision: Some(9),
            ..
        }))
    ));
    assert!(!d.confirmation_text.contains("Changed in background"));
}

#[tokio::test]
async fn schedule_views_capture_controls_and_keep_tree_and_selection_stable() {
    let key = |code| KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    let mut d = dashboard();
    let preferences = tempfile::tempdir().unwrap();
    d.path = preferences.path().join("layouts.json");
    let (commands, mut rx) = mpsc::channel(16);
    d.commands = commands;
    let (updates, receiver) = mpsc::channel(16);
    d.updates = receiver;
    let sample = OrganizationScheduleView {
        schedule: OrganizationSchedule {
            id: "series".into(),
            template: task("template"),
            scope: OrganizationExecutionScope {
                input_files: vec![],
                immutable_files: vec![],
                artifacts: vec!["report.txt".into()],
                checks: vec![],
                model: "fixture".into(),
                max_tokens: 1000,
                max_seconds: 120,
                max_model_requests: 2,
                max_attempts: 1,
                require_tool_approval: true,
                delegates: vec![],
                max_children: 0,
                auto_accept_children: false,
                not_before: None,
            },
            config: OrganizationScheduleConfig {
                label: "Unicode café routine".into(),
                interval_seconds: 60,
                first_run_at: 1010,
                ends_at: 2010,
                max_runs: 3,
                max_tokens: 1000,
                max_model_requests: 6,
            },
            node_revision: 1,
            project_revision: 1,
            policy_version: 2,
            state: OrganizationScheduleState::Active,
            revision: 7,
            next_run_at: Some(1070),
            issued_runs: 1,
            skipped_runs: 2,
            reason: None,
        },
        used_tokens: 10,
        model_requests: 2,
        occurrences: vec![OrganizationScheduleOccurrence {
            task_id: "source-task".into(),
            authorization_id: "source-grant".into(),
            due_at: 1010,
            task_state: "needsReview".into(),
        }],
    };
    d.snapshot.schedules = vec![sample.clone()];
    d.rebuild();
    assert!(d.rows.iter().all(|r| !r.id.starts_with("schedule:"))); // Operations defaults to tree
    d.key(key(KeyCode::Char('S')));
    assert_eq!(d.projection, Projection::Schedules);
    assert_eq!(d.selected.as_deref(), Some("schedule:series"));
    let text = d.details();
    for evidence in [
        "Unicode café routine",
        "Next start (UTC)",
        "1970-01-01",
        "Issued: 1 / 3",
        "skipped: 2",
        "Reported tokens: 10 / 1000",
        "source-task",
    ] {
        assert!(text.contains(evidence), "missing {evidence}");
    }
    d.key(key(KeyCode::Char('p')));
    d.snapshot.schedules[0].schedule.revision = 8;
    d.key(key(KeyCode::Char('y')));
    let Command::Schedule(OrganizationScheduleMutation::Control {
        id,
        expected_revision,
        action,
    }) = rx.try_recv().unwrap()
    else {
        panic!("missing pause")
    };
    assert_eq!(id, "series");
    assert_eq!(expected_revision, 7);
    assert_eq!(action, OrganizationScheduleControl::Pause);
    d.switch(Preset::Projects);
    d.selected = Some("one".into());
    updates
        .send(Update::ScheduleSaved(Box::new(sample), false))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.selected.as_deref(), Some("one"));
    d.key(key(KeyCode::Char('R')));
    assert!(d.form.is_some());
    d.key(key(KeyCode::Esc));
    d.key(key(KeyCode::Char('S')));
    for (w, h) in [(160, 48), (120, 36), (50, 20), (1, 1)] {
        let area = Rect::new(0, 0, w, h);
        d.render(area, &mut Buffer::empty(area));
    }
    d.key(key(KeyCode::Char('u')));
    assert!(matches!(
        d.confirmation,
        Some(Command::Schedule(OrganizationScheduleMutation::Control {
            action: OrganizationScheduleControl::Resume,
            ..
        }))
    ));
    d.key(key(KeyCode::Esc));
    d.key(key(KeyCode::Char('x')));
    d.key(key(KeyCode::Esc));
    assert!(d.confirmation.is_none());
}

#[tokio::test]
async fn memory_search_captures_scope_ignores_stale_results_and_renders_provenance() {
    let mut d = dashboard();
    d.preferences.preset = Preset::Projects;
    for record in &mut d.snapshot.records {
        if let OrganizationRecord::Task(task) = record
            && task.id == "two"
        {
            task.project_id = "different-project".into();
            task.owner_id = "different-agent".into();
        }
    }
    let (tx, mut rx) = mpsc::channel(4);
    let (updates, update_rx) = mpsc::channel(4);
    d.commands = tx;
    d.updates = update_rx;
    let key = |code| KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    d.open_memory(false);
    assert!(d.memory_request.is_none());
    assert!(rx.try_recv().is_err());
    d.selected = Some("one".into());
    d.key(key(KeyCode::Char('M')));
    let target = d.memory_request.clone().unwrap();
    assert_eq!(target.scope, OrganizationNoteScope::Agent);
    assert_eq!(target.agent_id, Some(task("one").owner_id));
    d.selected = Some("two".into());
    for c in "ÜBERPRÜFUNG".chars() {
        d.key(key(KeyCode::Char(c)));
    }
    d.key(key(KeyCode::Enter));
    let Command::MemorySearch(params) = rx.try_recv().unwrap() else {
        panic!("missing search")
    };
    assert_eq!(params.query, "ÜBERPRÜFUNG");
    assert_eq!(params.project_id, task("one").project_id);
    let mut stale = params.clone();
    stale.query = "different query".into();
    updates
        .send(Update::Memory(stale, Err("obsolete error".into())))
        .await
        .unwrap();
    d.update();
    assert!(d.memory_result.is_none());
    let mut wrong_project = params.clone();
    wrong_project.project_id = "different-project".into();
    updates
        .send(Update::Memory(
            wrong_project,
            Err("other project's response".into()),
        ))
        .await
        .unwrap();
    d.update();
    assert!(d.memory_result.is_none());
    let note = OrganizationNote {
        id: "source-note".into(),
        project_id: params.project_id.clone(),
        author_id: params.agent_id.clone().unwrap(),
        task_id: "source-task".into(),
        attempt_id: "source-attempt".into(),
        scope: params.scope,
        body: "Überprüfung uses SQLite.\x1b[31mUntrusted text".into(),
        created_at: 1234,
        group_id: None,
        source_note_id: None,
        owner_reviewed: false,
    };
    updates
        .send(Update::Memory(
            params.clone(),
            Ok(OrganizationNoteSearchResult {
                notes: vec![note],
                matched_count: 1,
            }),
        ))
        .await
        .unwrap();
    d.update();
    let text = d.memory_text();
    for evidence in [
        "source-note",
        "source-task",
        "source-attempt",
        "1234",
        "Überprüfung",
        "Showing 1 of 1",
    ] {
        assert!(text.contains(evidence), "missing {evidence}");
    }
    assert!(!text.contains('\x1b'));
    for (width, height) in [(160, 48), (50, 20), (20, 8)] {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        d.render(area, &mut buffer);
    }
    d.key(key(KeyCode::Char('/')));
    d.input = Some("é".repeat(128));
    d.key(key(KeyCode::Char('x')));
    assert_eq!(d.input.as_ref().unwrap().len(), 256);
    d.key(key(KeyCode::Esc));
    assert!(d.input.is_none() && d.memory_request.is_none() && d.memory_result.is_none());
    updates
        .send(Update::Memory(
            params,
            Err("late result after close".into()),
        ))
        .await
        .unwrap();
    d.update();
    assert!(d.memory_result.is_none());
    d.key(key(KeyCode::Char('m')));
    d.key(key(KeyCode::Enter));
    let Command::MemorySearch(params) = rx.try_recv().unwrap() else {
        panic!("missing browse")
    };
    assert_eq!(params.scope, OrganizationNoteScope::Project);
    assert!(params.agent_id.is_none() && params.query.is_empty());
    d.key(key(KeyCode::Char('f')));
    let Command::MemorySearch(refresh) = rx.try_recv().unwrap() else {
        panic!("missing refresh")
    };
    assert_eq!(refresh, params);
    drop(rx);
    d.key(key(KeyCode::Char('f')));
    assert!(d.memory_text().contains("Owner request queue busy"));
    assert!(!d.memory_text().contains("Loading scoped notes"));
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
    d.preferences.preset = Preset::Projects;
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
async fn blocker_handoff_requires_matching_details_and_freezes_owner_confirmation() {
    let mut d = dashboard();
    let (commands, mut receiver) = mpsc::channel(4);
    d.commands = commands;
    let mut blocked = view("one");
    blocked.task.state = "blocked".into();
    blocked.authorization=Some(serde_json::from_value(serde_json::json!({
        "id":"original-grant","taskId":"one","nodeRevision":1,"projectRevision":1,"expiresAt":1000,
        "revoked":false,"policyVersion":2,"scope":{"inputFiles":[],"immutableFiles":[],"artifacts":["report.txt"],
        "checks":[],"model":"fixture","maxTokens":100,"maxSeconds":60,"maxModelRequests":4,"maxAttempts":3,
        "requireToolApproval":true,"delegates":[],"maxChildren":0}
    })).unwrap());
    blocked.checkpoints.push(OrganizationTaskCheckpoint {
        id: "immutable-checkpoint".into(),
        task_id: "one".into(),
        project_id: "project".into(),
        owner_id: "owner".into(),
        attempt_id: "fenced-source".into(),
        authorization_id: "original-grant".into(),
        created_at: 1,
        completed_action_ids: vec!["known-effect".into()],
        content: codex_protocol::organization_coordination::OrganizationCheckpointContent {
            completed_steps: vec![],
            remaining_steps: vec!["Finish report".into()],
            blocker: Some("Owner decision needed".into()),
        },
        files: vec![],
        artifact_bases: vec![],
    });
    d.detail = Some(Box::new(blocked));
    d.selected = Some("two".into());
    d.open_form('H');
    assert!(d.form.is_none());
    d.selected = Some("one".into());
    d.open_form('H');
    let key = |code| KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    d.key(key(KeyCode::Tab));
    d.form.as_mut().unwrap().paste("Owner resolved the blocker");
    d.key(key(KeyCode::F(6)));
    assert!(receiver.try_recv().is_err());
    assert!(d.confirmation_text.contains("immutable-checkpoint"));
    assert!(
        d.confirmation_text
            .contains("original grant and remaining budgets")
    );
    d.detail.as_mut().unwrap().checkpoints[0].id = "changed-checkpoint".into();
    d.selected = Some("two".into());
    d.key(key(KeyCode::Char('y')));
    assert!(
        matches!(receiver.try_recv(),Ok(Command::Execution(OrganizationExecutionMutation::Handoff {
        task_id,expected_revision:4,checkpoint_id,owner_id,expected_owner_revision:1,..
    })) if task_id=="one" && checkpoint_id=="immutable-checkpoint" && owner_id=="owner")
    );
    assert!(receiver.try_recv().is_err());
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
            report: Some(report_for_tasks(&["one"])),
            schedules: vec![],
            workflows: vec![],
            records: vec![OrganizationRecord::Task(task("one"))],
        }))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.selected.as_deref(), Some("one"));
    assert!(matches!(command_rx.try_recv(), Ok(Command::Inspect(id)) if id == "one"));
    updates
        .send(Update::Snapshot(Snapshot {
            report: Some(report_for_tasks(&["one"])),
            schedules: vec![],
            workflows: vec![],
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
            report: Some(report_for_tasks(&["new"])),
            schedules: vec![],
            workflows: vec![],
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
    d.preferences.preset = Preset::Projects;
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
            report: None,
            schedules: vec![],
            workflows: vec![],
            records: d.snapshot.records.clone(),
        }))
        .await
        .unwrap();
    d.update();
    assert!(d.message.contains("Cannot freeze editor revision"));
    assert!(d.confirmation.is_some());
    assert!(rx.try_recv().is_err());
    // Actual Neovim stays outside focus while memory owns the host keys.
    d.confirmation = None;
    d.form = None;
    d.preferences.preset = Preset::Workspace;
    d.open_memory(false);
    d.key(KeyEvent::new(
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ));
    let _ = rx.try_recv().unwrap();
    for key in ['e', 'o', 't', 'd', 'n'] {
        d.key(KeyEvent::new(
            KeyCode::Char(key),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(!d.editor.as_ref().unwrap().focused);
        assert!(d.memory_request.is_some());
    }
    assert!(d.editor.as_mut().unwrap().dirty().unwrap());
    d.key(KeyEvent::new(
        KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(d.memory_request.is_none());
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

#[tokio::test]
async fn confirmed_apply_rejection_restores_real_editor_and_survives_refresh() {
    let mut d = dashboard();
    let (commands, _command_rx) = mpsc::channel(4);
    d.commands = commands;
    let (tx, rx) = mpsc::channel(4);
    d.updates = rx;
    d.editor = Some(Editor::start(FrameRequester::test_dummy()).unwrap());
    d.editor.as_mut().unwrap().freeze().unwrap();
    d.applying = true;
    tx.send(Update::ApplyRejected(
        "original file changed; inspect before confirming again".into(),
    ))
    .await
    .unwrap();
    tx.send(Update::Snapshot(Snapshot::default()))
        .await
        .unwrap();
    d.update();
    assert!(!d.applying && d.message_is_error);
    assert!(d.message.contains("original file changed"));
    let editor = d.editor.as_mut().unwrap();
    editor
        .key(KeyEvent::new(
            KeyCode::Char('i'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
    editor.paste("human editing restored").unwrap();
    assert!(editor.dirty().unwrap());
}

#[tokio::test]
async fn uncertain_apply_stays_frozen_after_refresh_and_unrelated_success() {
    let mut d = dashboard();
    let (commands, _command_rx) = mpsc::channel(4);
    d.commands = commands;
    let (tx, rx) = mpsc::channel(4);
    d.updates = rx;
    d.editor = Some(Editor::start(FrameRequester::test_dummy()).unwrap());
    d.editor.as_mut().unwrap().freeze().unwrap();
    d.applying = true;
    tx.send(Update::Error("Owner API timed out; unknown outcome".into()))
        .await
        .unwrap();
    tx.send(Update::Changed).await.unwrap();
    tx.send(Update::Snapshot(Snapshot::default()))
        .await
        .unwrap();
    d.update();
    assert!(d.applying && d.message_is_error);
    assert!(d.message.contains("unknown outcome"));
    assert!(
        d.editor
            .as_mut()
            .unwrap()
            .paste("must stay blocked")
            .is_err()
    );
    tx.send(Update::ApplyFinished).await.unwrap();
    d.update();
    assert!(!d.applying && !d.message_is_error);
    d.editor
        .as_mut()
        .unwrap()
        .key(KeyEvent::new(
            KeyCode::Char('i'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
    d.editor.as_mut().unwrap().paste("settled").unwrap();
}

#[tokio::test]
async fn owner_controls_discover_capture_and_confirm_without_early_authority() {
    let mut d = dashboard();
    let (tx, mut rx) = mpsc::channel(4);
    d.commands = tx;
    let limits = OrganizationSharedCeilings {
        max_tokens: 500,
        max_model_requests: 3,
        max_active: 2,
    };
    d.snapshot
        .records
        .push(OrganizationRecord::Control(OrganizationControl {
            id: "global".into(),
            revision: 7,
            value: OrganizationControlValue::Global {
                paused: false,
                limits: Some(limits.clone()),
            },
        }));
    d.key(KeyCode::F(12).into());
    assert!(d.confirmation_text.contains("EXACT OWNER DECISION"));
    assert!(
        matches!(d.confirmation,Some(Command::Definition(OrganizationMutation::SetControl {expected_revision:Some(7),value:OrganizationControlValue::Global {paused:true,ref limits}})) if limits.as_ref()==Some(&OrganizationSharedCeilings {max_tokens:500,max_model_requests:3,max_active:2}))
    );
    assert!(rx.try_recv().is_err());
    d.key(KeyCode::Esc.into());
    assert!(rx.try_recv().is_err());
    d.key(KeyCode::F(12).into());
    d.key(KeyCode::Char('y').into());
    assert!(matches!(
        rx.try_recv(),
        Ok(Command::Definition(OrganizationMutation::SetControl {
            expected_revision: Some(7),
            ..
        }))
    ));
    d.key(KeyCode::F(10).into());
    assert!(
        matches!(d.form.as_ref().unwrap().command(),Ok(Command::Definition(OrganizationMutation::SetControl {expected_revision:Some(7),value:OrganizationControlValue::Global {paused:false,ref limits}})) if limits.as_ref()==Some(&OrganizationSharedCeilings {max_tokens:500,max_model_requests:3,max_active:2}))
    );
    d.key(KeyCode::Esc.into());
    d.selected = Some("one".into());
    d.key(KeyCode::Char('!').into());
    assert!(
        matches!(d.form.as_ref().unwrap().command(),Ok(Command::Definition(OrganizationMutation::SetControl {value:OrganizationControlValue::Priority {ref task_id,priority:0,..},..})) if task_id=="one")
    );
    assert!(rx.try_recv().is_err());
    d.key(KeyCode::Esc.into());
    d.snapshot
        .records
        .push(OrganizationRecord::Project(OrganizationProjectPlan {
            id: "project".into(),
            name: "Reviewable project".into(),
            roots: vec!["/tmp".into()],
            goal_id: None,
            owner_id: Some("owner".into()),
            criteria: vec!["Verified result".into()],
            revision: 3,
        }));
    d.snapshot.report.as_mut().unwrap().tasks[0].state = "accepted".into();
    d.selected = Some("project".into());
    d.key(KeyCode::Char('F').into());
    d.key(KeyCode::Right.into());
    d.key(KeyCode::Right.into());
    d.key(KeyCode::Tab.into());
    d.key(KeyCode::Right.into());
    d.key(KeyCode::Tab.into());
    d.form
        .as_mut()
        .unwrap()
        .paste("Owner read and verified the criterion");
    d.key(KeyCode::F(6).into());
    assert!(rx.try_recv().is_err());
    assert!(d.confirmation_text.contains("fixture-result"));
    d.key(KeyCode::Char('y').into());
    assert!(
        matches!(rx.try_recv(),Ok(Command::Definition(OrganizationMutation::SetControl {value:OrganizationControlValue::Project {project_revision:3,state:OrganizationProjectState::Completed,ref evidence,..},..})) if evidence.len()==1 && evidence[0].task_id=="one" && evidence[0].result_digest=="fixture-result")
    );
    d.key(KeyCode::F(11).into());
    assert_eq!(d.projection, Projection::Controls);
    assert_eq!(d.rows.len(), 1);
    for (width, height) in [(160, 48), (50, 20)] {
        let area = Rect::new(0, 0, width, height);
        d.render(area, &mut Buffer::empty(area));
    }
    d.key(KeyCode::F(11).into());
    assert_eq!(d.projection, Projection::Default);
}

#[test]
fn absent_legacy_global_usage_is_unavailable_instead_of_invented_zero() {
    let mut wire = serde_json::to_value(empty_report()).unwrap();
    wire.as_object_mut().unwrap().remove("globalUsage");
    let report: OrganizationReport = serde_json::from_value(wire).unwrap();
    assert!(report.global_usage.is_none());
    let text = reporting::text(&report, &reporting::Focus::All, None);
    assert!(text.contains("Whole organization usage unavailable"));
    assert!(!text.contains("Whole organization: 0"));
}

fn empty_report() -> OrganizationReport {
    OrganizationReport {
        sequence: 0,
        generated_at: 0,
        project_id: None,
        totals: OrganizationReportTotals::default(),
        global_usage: Some(OrganizationGlobalUsage::default()),
            scorecards:None,
        tasks: vec![],
        usage: vec![],
        usage_truncated: false,
        upcoming_runs: vec![],
        next_cursor: None,
    }
}

#[tokio::test]
async fn report_pagination_refuses_mixed_canonical_revisions() {
    use futures::{SinkExt, StreamExt};
    use serde_json::json;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint =
        crate::resolve_remote_addr(&format!("ws://{}", listener.local_addr().unwrap())).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) =
            socket.next().await
        {
            let JSONRPCMessage::Request(request) = serde_json::from_str(&text).unwrap() else {
                continue;
            };
            let response = if request.method == "initialize" {
                json!({"userAgent":"revision-fixture"})
            } else {
                assert_eq!(request.method, "organization/report");
                let mut report = empty_report();
                if request.params.as_ref().unwrap()["cursor"].is_null() {
                    report.sequence = 1;
                    report.next_cursor = Some("next-page".into());
                } else {
                    report.sequence = 2;
                }
                json!({"report":report})
            };
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    json!({"id":request.id,"result":response})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
    });
    let client = crate::connect_remote_app_server(endpoint).await.unwrap();
    let error = tokio::time::timeout(
        Duration::from_secs(10),
        reporting::load(&client.request_handle(), None),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("State changed during report pagination")
    );
    drop(client);
    server.abort();
}

#[tokio::test]
async fn selection_confirmation_rechecks_actual_native_buffer_before_owner_submission() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join("café.py");
    std::fs::write(&source, "café\n").unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&source, false).unwrap();
    editor
        .rpc(
            "nvim_exec_lua",
            vec![
                "vim.api.nvim_feedkeys('ggV','nx',false)".into(),
                rmpv::Value::Array(vec![]),
            ],
        )
        .unwrap();
    let selection = editor.selection().unwrap();
    let mut d = dashboard();
    let (commands, mut submitted) = mpsc::channel(4);
    d.commands = commands;
    d.editor = Some(editor);
    d.form = Some(form::Form::new(
        form::Kind::Selection {
            project_id: "project".into(),
            selection: selection.clone(),
        },
        &d.snapshot.records,
        None,
        "model",
    ));
    d.editor
        .as_mut()
        .unwrap()
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                1.into(),
                false.into(),
                rmpv::Value::Array(vec!["human edit".into()]),
            ],
        )
        .unwrap();
    d.key(KeyEvent::new(
        KeyCode::F(6),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(d.confirmation.is_none());
    assert!(d.message.contains("source changed"));
    assert!(submitted.try_recv().is_err());
    d.confirmation = Some(Command::Definition(OrganizationMutation::PutTask {
        id: None,
        expected_revision: None,
        project_id: "project".into(),
        owner_id: "owner".into(),
        goal_id: None,
        instructions: "Pinned request".into(),
        criteria: vec!["Checked".into()],
        prerequisites: vec![],
    }));
    d.selection_confirmation = Some(selection);
    d.key(KeyEvent::new(
        KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(d.confirmation.is_none());
    assert!(d.message.contains("source changed"));
    assert!(submitted.try_recv().is_err());
    assert!(d.editor.as_ref().unwrap().error.is_none());
}

fn report_for_tasks(ids: &[&str]) -> OrganizationReport {
    let mut report = empty_report();
    report.tasks = ids
        .iter()
        .map(|id| OrganizationTaskReport {
            task_id: (*id).into(),
            project_id: "project".into(),
            owner_id: "owner".into(),
            workflow_run_id: None,
            title: "Check a real artifact".into(),
            state: "needsReview".into(),
            revision: 4,
            attempt_id: Some("attempt".into()),
            result_digest: Some("fixture-result".into()),
            checkpoint_id: None,
            blocker: None,
            changes_requested_sequence: None,
            pending_approvals: 0,
            checks_passed: 1,
            checks_failed: 0,
            reported_tokens: 10,
            model_requests: 1,
            reported_model_requests: 1,
            budget: None,
        })
        .collect();
    report
}

#[tokio::test]
async fn late_summary_or_inventory_success_and_failure_cannot_retarget_an_open_scope() {
    let mut d = dashboard();
    let (updates, incoming) = mpsc::channel(4);
    d.updates = incoming;
    d.report_request = Some("current-project-request".into());
    d.report_text = Some("current project loading".into());
    updates
        .send(Update::Report(
            "cancelled-agent-request".into(),
            Ok(Box::new(empty_report())),
            reporting::Focus::Agent("other-agent".into()),
        ))
        .await
        .unwrap();
    updates
        .send(Update::ReportText(
            "cancelled-inventory-request".into(),
            Err("old failure".into()),
        ))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.report_text.as_deref(), Some("current project loading"));
    updates
        .send(Update::Report(
            "current-project-request".into(),
            Err("exact current refusal".into()),
            reporting::Focus::Project("project".into()),
        ))
        .await
        .unwrap();
    d.update();
    assert_eq!(
        d.report_text.as_deref(),
        Some("Summary unavailable: exact current refusal")
    );
    d.key(KeyEvent::new(
        KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ));
    updates
        .send(Update::ReportText(
            "current-project-request".into(),
            Ok("late data".into()),
        ))
        .await
        .unwrap();
    d.update();
    assert!(d.report_text.is_none());
    assert!(d.report_request.is_none());
}
