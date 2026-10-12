use super::*;

#[tokio::test]
async fn background_requests_survive_startup_overlap_and_cancelled_dashboard() {
    use futures::SinkExt;
    use futures::StreamExt;
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
        let mut old_dashboard = tokio::spawn(async move { load(&old_handle, &mut None, 0, false).await });
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

fn branching_records() -> Vec<OrganizationRecord> {
    let mut root = node("root", None);
    root.kind = OrganizationNodeKind::Group;
    let mut work = task("active");
    work.owner_id = "left".into();
    work.state = "running".into();
    let mut blocked = task("blocked");
    blocked.owner_id = "right".into();
    blocked.state = "needsReconciliation".into();
    vec![
        root,
        node("left", Some("root")),
        node("right", Some("root")),
    ]
    .into_iter()
    .map(OrganizationRecord::Node)
    .chain([
        OrganizationRecord::Task(work),
        OrganizationRecord::Task(blocked),
    ])
    .collect()
}

#[test]
fn branching_graph_preserves_relationships_and_rolls_up_actual_loaded_work() {
    let records = branching_records();
    let chart = graph::Graph::build(&records, &BTreeSet::new(), "", &graph::Settings::default());
    assert_eq!(chart.cards.len(), 3);
    let root = &chart.cards[0];
    let left = &chart.cards[1];
    let right = &chart.cards[2];
    assert_eq!(root.children, [1, 2]);
    assert!(left.position.0 < right.position.0);
    assert_eq!(left.position.1, right.position.1);
    assert!(root.position.1 < left.position.1);
    assert!(left.position.0 <= root.position.0 && root.position.0 <= right.position.0);
    assert_eq!(
        (
            root.counts.agents,
            root.counts.tasks,
            root.counts.running,
            root.counts.issues
        ),
        (2, 2, 1, 1)
    );
    assert_eq!(left.state, "running");
    assert_eq!(right.state, "blocked");
    assert_eq!(
        chart.navigate(Some("root"), KeyCode::Down).as_deref(),
        Some("left")
    );
    assert_eq!(
        chart.navigate(Some("left"), KeyCode::Right).as_deref(),
        Some("right")
    );
    assert_eq!(
        chart.navigate(Some("right"), KeyCode::Up).as_deref(),
        Some("root")
    );
}

#[test]
fn graph_search_and_state_filters_reveal_matches_with_ancestry_without_changing_definitions() {
    let records = branching_records();
    let captured = serde_json::to_value(&records).unwrap();
    let collapsed = BTreeSet::from(["root".into()]);
    let settings = graph::Settings::default();
    assert_eq!(
        graph::Graph::build(&records, &collapsed, "", &settings)
            .cards
            .len(),
        1
    );
    let found = graph::Graph::build(&records, &collapsed, "right", &settings);
    assert_eq!(
        found
            .cards
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["root", "right"]
    );
    let mut issue_settings = settings;
    issue_settings.filter = 2;
    let issues = graph::Graph::build(&records, &collapsed, "", &issue_settings);
    assert_eq!(
        issues
            .cards
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["root", "right"]
    );
    assert_eq!(serde_json::to_value(&records).unwrap(), captured);
}

#[test]
fn diagram_viewport_clips_cards_and_hit_targets_and_fits_an_overview() {
    let mut settings = graph::Settings::default();
    let mut chart = graph::Graph::build(&branching_records(), &BTreeSet::new(), "", &settings);
    let outer = Rect::new(0, 0, 100, 40);
    let area = Rect::new(10, 7, 66, 22);
    let mut buffer = Buffer::empty(outer);
    buffer[(0, 0)].set_symbol("!");
    chart.render(
        &mut settings,
        Some("root"),
        &BTreeSet::new(),
        area,
        &mut buffer,
    );
    assert_eq!(chart.hits.len(), 3);
    assert!(
        chart
            .hits
            .iter()
            .all(|(rect, _)| rect.intersection(area) == *rect)
    );
    assert!(buffer.content.iter().any(|cell| cell.symbol() == "│"));
    settings.pan = (u32::MAX, u32::MAX);
    chart.render(
        &mut settings,
        Some("root"),
        &BTreeSet::new(),
        area,
        &mut buffer,
    );
    assert!(settings.pan.0 <= chart.width && settings.pan.1 <= chart.height);
    settings.overview = true;
    chart.render(
        &mut settings,
        Some("right"),
        &BTreeSet::new(),
        area,
        &mut buffer,
    );
    assert!(
        chart.hits.is_empty(),
        "overview clusters cannot pretend to uniquely select a node"
    );
    assert!(buffer.content.iter().any(|cell| cell.symbol() == "◎"));
    assert_eq!(buffer[(0, 0)].symbol(), "!");
}

#[test]
fn diagram_layout_handles_ten_thousand_deep_nodes_without_recursion_or_wrapped_coordinates() {
    let records: Vec<_> = (0..10000)
        .map(|i| OrganizationRecord::Node(node(&format!("n{i:05}"), None)))
        .enumerate()
        .map(|(i, mut r)| {
            if let OrganizationRecord::Node(n) = &mut r {
                n.parent_id = (i > 0).then(|| format!("n{:05}", i - 1));
            }
            r
        })
        .collect();
    let start = std::time::Instant::now();
    let chart = graph::Graph::build(&records, &BTreeSet::new(), "", &graph::Settings::default());
    assert_eq!(chart.cards.len(), 10000);
    assert!(
        chart.height > 65535,
        "layout geometry must not wrap terminal u16 coordinates"
    );
    assert_eq!(chart.cards[0].counts.agents, 10000);
    assert!(
        chart
            .cards
            .windows(2)
            .all(|c| c[0].position.1 < c[1].position.1)
    );
    eprintln!("10000-deep diagram layout: {:?}", start.elapsed());
}

#[test]
fn goal_diagram_links_owned_tasks_and_preserves_acceptance_and_prerequisite_evidence() {
    let goal:OrganizationGoal=serde_json::from_value(serde_json::json!({"id":"mission","parentId":null,"objective":"Verified goal","criteria":["Owner verifies final outcome"],"revision":1})).unwrap();
    let mut accepted = task("producer");
    accepted.goal_id = Some(goal.id.clone());
    accepted.state = "accepted".into();
    let mut dependent = task("consumer");
    dependent.goal_id = Some(goal.id.clone());
    dependent.prerequisites = vec![accepted.id.clone()];
    dependent.state = "waitingDependency".into();
    let records = vec![
        OrganizationRecord::Goal(goal),
        OrganizationRecord::Task(accepted),
        OrganizationRecord::Task(dependent),
    ];
    let captured = serde_json::to_value(&records).unwrap();
    let graph = graph::Graph::goals(&records, &BTreeSet::new(), "", &graph::Settings::default());
    assert_eq!(graph.cards.len(), 3);
    let root = graph.cards.iter().find(|c| c.id == "mission").unwrap();
    assert_eq!((root.counts.accepted, root.counts.tasks), (1, 2));
    assert_eq!(
        root.state, "defined",
        "task acceptance does not certify a goal's criteria"
    );
    let child = graph.cards.iter().find(|c| c.id == "consumer").unwrap();
    assert_eq!(child.parent.as_deref(), Some("mission"));
    assert_eq!(child.role, "1 prerequisites · 0 pending");
    assert_eq!(child.state, "waitingDependency");
    assert_eq!(serde_json::to_value(&records).unwrap(), captured);
}

#[test]
fn deep_tree_search_crosses_collapsed_branches_and_keeps_stable_ids() {
    let nodes: Vec<_> = (0..10000)
        .map(|i| {
            node(
                &format!("n{i:05}"),
                if i == 0 { None } else { Some("unused") },
            )
        })
        .enumerate()
        .map(|(i, mut n)| {
            n.parent_id = (i > 0).then(|| format!("n{:05}", i - 1));
            n
        })
        .collect();
    let measured = std::time::Instant::now();
    let collapsed = BTreeSet::from(["n00000".to_string()]);
    assert_eq!(tree(&nodes, &collapsed, "").len(), 1);
    let found = tree(&nodes, &collapsed, "n09999");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "n09999");
    assert!(found[0].label.len() < 100);
    assert_eq!(tree(&nodes, &BTreeSet::new(), "").len(), 10000);
    eprintln!(
        "registry_tree_measurement: 10000-deep nodes; collapse/search/full traversal {:?}",
        measured.elapsed()
    );
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
    prefs.panes[Preset::Operations.index()].order = [Pane::Inspector, Pane::Sidebar, Pane::Main];
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
    assert_eq!(
        restored.panes[3].order,
        [Pane::Inspector, Pane::Sidebar, Pane::Main]
    );
    let rectangles = restored.panes[3].regions(Rect::new(0, 0, 160, 40));
    assert_eq!(
        rectangles.iter().map(|(pane, _)| *pane).collect::<Vec<_>>(),
        vec![Pane::Inspector, Pane::Sidebar, Pane::Main]
    );
    assert!(
        rectangles
            .windows(2)
            .all(|pair| pair[0].1.right() <= pair[1].1.x)
    );
    assert_eq!(rectangles[1].1.width, 38);
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

#[tokio::test]
async fn named_views_preserve_settings_and_drop_stale_identities_without_execution() {
    let mut d = dashboard();
    let home = tempfile::tempdir().unwrap();
    d.path = home.path().join("layouts.json");
    d.preferences.preset = Preset::Operations;
    d.search = "Custom owner".into();
    d.selected = Some("owner".into());
    d.preferences.graph.focus = Some("owner".into());
    d.preferences.panes[3].width = 52;
    d.named_view("Team", "save").unwrap();
    d.preferences.panes[3].width = 25;
    d.search.clear();
    d.named_view("Team", "restore").unwrap();
    assert_eq!(d.preferences.panes[3].width, 52);
    assert_eq!(d.search, "Custom owner");
    assert_eq!(d.selected.as_deref(), Some("owner"));
    let prefs = Preferences::load(&d.path).unwrap();
    assert_eq!(prefs.views[0].name, "Team");
    d.snapshot.records.clear();
    d.named_view("Team", "restore").unwrap();
    assert_eq!(d.selected, None);
    assert_eq!(d.preferences.graph.focus, None);
    assert!(d.named_view("bad\nname", "save").is_err());
    d.named_view("Team", "delete").unwrap();
    assert!(Preferences::load(&d.path).unwrap().views.is_empty());
}

#[tokio::test]
async fn named_view_confirmation_stays_on_the_frontend_with_no_backend_receiver() {
    let mut d = dashboard();
    let home = tempfile::tempdir().unwrap();
    d.path = home.path().join("layouts.json");
    d.confirmation = Some(Command::NamedView {
        name: "Confirmed view".into(),
        action: "save".into(),
    });
    d.key(KeyCode::Char('y').into());
    let prefs = Preferences::load(&d.path).unwrap();
    assert_eq!(prefs.views[0].name, "Confirmed view");
    assert!(d.confirmation.is_none());
    assert!(!d.message_is_error);
}

#[test]
fn cached_definitions_require_contiguous_nonowner_events_and_the_same_serving_process() {
    let mut previous = report_for_tasks(&[]);
    previous.sequence = 1;
    let mut report = previous.clone();
    assert!(cache_valid(Some(&previous), &report));
    report.sequence = 2;
    assert!(!cache_valid(Some(&previous), &report));
    report.audit_events = Some(vec![OrganizationAuditEvent {
        sequence: 2,
        actor: "host".into(),
        operation: "attempt/progress".into(),
        reference: Some("attempt".into()),
        created_at: 0,
    }]);
    assert!(cache_valid(Some(&previous), &report));
    report.audit_events.as_mut().unwrap()[0].actor = "owner".into();
    assert!(!cache_valid(Some(&previous), &report));
    report.audit_events.as_mut().unwrap()[0].actor = "host".into();
    report.sequence = 3;
    assert!(!cache_valid(Some(&previous), &report));
    report.sequence = 1;
    report.runtime_resources = Some(OrganizationRuntimeResources {
        process_id: 123,
        measured_at_ms: 1,
        resident_bytes: None,
        cpu_time_ms: None,
    });
    assert!(!cache_valid(Some(&previous), &report));
}

#[test]
fn operations_cards_resize_and_memberships_do_not_change_the_primary_tree() {
    let mut records = branching_records();
    records.push(OrganizationRecord::Membership(OrganizationMembership {
        id: "membership".into(),
        node_id: "left".into(),
        group_id: "right".into(),
    }));
    let mut settings = graph::Settings::default();
    let mut chart = graph::Graph::build(&records, &BTreeSet::new(), "", &settings);
    let original = serde_json::to_value(&records).unwrap();
    let mut widths = vec![];
    for width in [200, 50, 120, 1] {
        let area = Rect::new(0, 0, width, 40);
        let mut buffer = Buffer::empty(area);
        chart.render(
            &mut settings,
            Some("left"),
            &BTreeSet::new(),
            area,
            &mut buffer,
        );
        widths.push(chart.width);
        assert!(
            chart
                .hits
                .iter()
                .all(|(rect, _)| rect.intersection(area) == *rect)
        );
    }
    assert!(
        widths[0] > widths[1],
        "wide cards adapt to smaller terminals"
    );
    assert_eq!(serde_json::to_value(&records).unwrap(), original);
    assert_eq!(
        chart
            .cards
            .iter()
            .find(|c| c.id == "left")
            .unwrap()
            .parent
            .as_deref(),
        Some("root")
    );
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

#[tokio::test]
async fn persistent_transcript_reports_canonical_task_state_without_granting_execution() {
    let mut d = dashboard();
    d.snapshot
        .records
        .push(OrganizationRecord::Control(OrganizationControl {
            id: "conversation:c".into(),
            revision: 2,
            value: OrganizationControlValue::Conversation {
                id: "c".into(),
                project_id: "project".into(),
                agent_id: "owner".into(),
                topic: "Planning".into(),
                entries: vec![OrganizationConversationEntry {
                    task_id: "one".into(),
                    prompt: "Draft the next project".into(),
                    reply: Some("Review the evidence first".into()),
                    reply_attempt_id: Some("attempt".into()),
                }],
            },
        }));
    let transcript = d.conversation_transcript("conversation:c").unwrap();
    assert!(
        transcript.contains("Draft the next project")
            && transcript.contains("Review the evidence first")
    );
    assert!(transcript.contains("separately authorized"));
    d.selected = Some("conversation:c".into());
    d.key(KeyCode::F(44).into());
    assert!(d.report_text.is_some());
    assert!(d.confirmation.is_none());
}

#[tokio::test]
async fn business_date_inspector_distinguishes_overdue_from_closed_work() {
    let mut d = dashboard();
    d.selected = Some("one".into());
    d.snapshot
        .records
        .push(OrganizationRecord::Control(OrganizationControl {
            id: "deadline:task:one".into(),
            revision: 1,
            value: OrganizationControlValue::Deadline {
                kind: OrganizationDeadlineKind::Task,
                target_id: "one".into(),
                target_revision: 4,
                due_at: Some(1),
                display_offset_minutes: -240,
            },
        }));
    assert!(d.details().contains("overdue"));
    let OrganizationRecord::Task(task) = &mut d.snapshot.records[1] else {
        panic!("task")
    };
    task.state = "accepted".into();
    let closed = d.details();
    assert!(closed.contains("work closed") && !closed.contains("overdue"));
    assert!(closed.contains("-04:00") && closed.contains("independently fixed"));
}

#[tokio::test]
async fn readiness_explains_actual_disabled_dependencies_and_pending_receipts_without_authority() {
    let mut d = dashboard();
    d.selected = Some("owner".into());
    let OrganizationRecord::Node(n) = &mut d.snapshot.records[0] else {
        panic!("agent")
    };
    n.enabled = false;
    let OrganizationRecord::Task(t) = &mut d.snapshot.records[1] else {
        panic!("task")
    };
    t.state = "proposed".into();
    t.prerequisites = vec!["not-loaded".into()];
    let report = d.snapshot.report.as_mut().unwrap();
    let t = &mut report.tasks[0];
    t.state = "proposed".into();
    t.model_requests = 1;
    t.reported_model_requests = 0;
    let text = d.agent_readiness_text().unwrap();
    assert!(
        text.contains("Disabled definition owner")
            && text.contains("Missing execution authorization")
            && text.contains("not-loaded not loaded")
            && text.contains("remain charged")
    );
    assert!(text.contains("grants no execution authority"));
    d.key(KeyCode::F(48).into());
    assert!(d.report_text.is_some() && d.confirmation.is_none());
}

fn view(id: &str) -> OrganizationExecutionView {
    OrganizationExecutionView {
        token_reservations: vec![],
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

#[test]
fn artifact_data_preview_rechecks_hash_and_declared_origin_and_refuses_binary_and_oversize() {
    use sha2::Digest;
    use sha2::Sha256;
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let file = workspace.path().join("data.csv");
    let body = "name,value\n\"quoted name\",7\n";
    std::fs::write(&file, body).unwrap();
    let mut v = view("one");
    v.attempts.push(serde_json::from_value(serde_json::json!({"id":"captured","taskId":"one","authorizationId":"grant","ownerId":"original-worker","generation":1,"threadId":"thread","workspace":workspace.path(),"state":"needsReview","startedAt":1,"leaseUntil":2,"reportedTokens":0,"modelRequests":0,"reportedModelRequests":0})).unwrap());
    v.result = Some(OrganizationTaskResult {
        attempt_id: "captured".into(),
        digest: "exact-version".into(),
        artifacts: vec![OrganizationArtifactEvidence {
            path: "data.csv".into(),
            sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
            bytes: body.len() as u64,
        }],
        artifact_bases: vec![],
        checks: vec![],
        accepted: false,
    });
    let text = artifacts::text(&v, home.path(), 0).unwrap();
    assert!(text.contains(body));
    assert!(text.contains("original-worker"));
    assert!(text.contains("exact-version"));
    std::fs::write(&file, "changed after recording").unwrap();
    assert!(artifacts::text(&v, home.path(), 0).is_err());
    v.result.as_mut().unwrap().artifacts[0].bytes = 65537;
    assert!(
        artifacts::text(&v, home.path(), 0)
            .unwrap_err()
            .to_string()
            .contains("64 KiB")
    );
    let binary = b"binary\0text";
    std::fs::write(&file, binary).unwrap();
    v.result.as_mut().unwrap().artifacts[0] = OrganizationArtifactEvidence {
        path: "data.csv".into(),
        sha256: format!("{:x}", Sha256::digest(binary)),
        bytes: binary.len() as u64,
    };
    assert!(
        artifacts::text(&v, home.path(), 0)
            .unwrap_err()
            .to_string()
            .contains("binary")
    );
    v.result.as_mut().unwrap().artifacts[0].path = "../data.csv".into();
    assert!(artifacts::text(&v, home.path(), 0).is_err());
}

#[test]
fn model_advice_requires_the_exact_configured_endpoint_and_captured_checked_outcomes() {
    let mut report = empty_report();
    let provider = OrganizationProviderEvidence {
        id: "selected".into(),
        origin: Some("http://localhost".into()),
        endpoint_sha256: "a".repeat(64),
    };
    let card:OrganizationScorecard=serde_json::from_value(serde_json::json!({"projectId":"project","workflowRunId":null,"ownerId":"worker","provider":provider,"model":"captured-compatible","attempts":2,"acceptedResults":1,"failedAttempts":0,"blockedAttempts":1,"uncertainAttempts":0,"checksPassed":1,"checksFailed":0,"reportedTokens":22,"modelRequests":2,"reportedModelRequests":2})).unwrap();
    report.scorecards = Some(OrganizationScorecards {
        entries: vec![card],
        truncated: false,
    });
    assert!(reporting::model_advice(&report, &provider).contains("Candidate captured-compatible"));
    let changed = OrganizationProviderEvidence {
        endpoint_sha256: "b".repeat(64),
        ..provider.clone()
    };
    assert!(!reporting::model_advice(&report, &changed).contains("Candidate captured-compatible"));
    report.scorecards.as_mut().unwrap().entries[0].uncertain_attempts = 1;
    assert!(!reporting::model_advice(&report, &provider).contains("Candidate captured-compatible"));
    assert!(reporting::model_advice(&report, &provider).contains("recommendation unavailable"));
}

#[tokio::test]
async fn resource_preview_distinguishes_demand_remaining_pools_and_unknown_quote() {
    let mut d = dashboard();
    let scope:OrganizationExecutionScope=serde_json::from_value(serde_json::json!({"inputFiles":[],"immutableFiles":[],"artifacts":["report.txt"],"checks":[],"model":"model","maxTokens":100,"maxSeconds":60,"maxModelRequests":2,"maxAttempts":1,"requireToolApproval":true,"delegates":[],"maxChildren":0})).unwrap();
    d.snapshot
        .records
        .push(OrganizationRecord::Control(OrganizationControl {
            id: "global".into(),
            revision: 1,
            value: OrganizationControlValue::Global {
                paused: false,
                limits: Some(OrganizationSharedCeilings {
                    max_tokens: 150,
                    max_model_requests: 3,
                    max_active: 2,
                }),
            },
        }));
    d.snapshot.report.as_mut().unwrap().global_usage = Some(OrganizationGlobalUsage {
        reported_tokens: 50,
        model_requests: 1,
        active: 1,
        unresolved_requests: 0,
    });
    let text = d.resource_preview("project", &[&scope, &scope]);
    assert!(
        text.contains("200 reported tokens / 4 admitted requests / 120 summed attempt seconds")
    );
    assert!(text.contains("100 tokens / 2 requests / 1 active slots"));
    assert!(text.contains("quote estimate unavailable"));
    assert!(text.contains("not a reservation"));
}

#[tokio::test]
async fn palette_opens_declared_file_from_projects_without_losing_task_or_dirty_buffer() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let file = project.path().join("review.txt");
    std::fs::write(&file, "Owner original\n").unwrap();
    let mut d = dashboard();
    d.path = home.path().join("layouts.json");
    d.preferences.preset = Preset::Projects;
    d.snapshot
        .records
        .push(OrganizationRecord::Project(OrganizationProjectPlan {
            id: "project".into(),
            name: "Owned project".into(),
            roots: vec![project.path().to_str().unwrap().into()],
            goal_id: None,
            owner_id: Some("owner".into()),
            criteria: vec![],
            revision: 1,
        }));
    d.selected = Some("one".into());
    let mut detail = view("one");
    detail.authorization=Some(serde_json::from_value(serde_json::json!({"id":"grant","taskId":"one","nodeRevision":1,"projectRevision":1,"expiresAt":99999,"revoked":false,"scope":{"inputFiles":[],"immutableFiles":[],"artifacts":["review.txt"],"checks":[],"model":"fixture","maxTokens":100,"maxSeconds":120,"maxModelRequests":2,"maxAttempts":1,"requireToolApproval":true,"delegates":[],"maxChildren":0,"autoAcceptChildren":false}})).unwrap());
    d.detail = Some(Box::new(detail.clone()));
    d.key(KeyCode::F(23).into());
    assert_eq!(d.preferences.preset, Preset::Workspace);
    assert_eq!(d.selected.as_deref(), Some("one"));
    let editor = d.editor.as_mut().unwrap();
    assert!(editor.focused);
    assert_eq!(editor.pin_buffer().unwrap().path, file.to_str().unwrap());
    editor
        .key(KeyEvent::new(
            KeyCode::Char('i'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
    editor
        .key(KeyEvent::new(
            KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
    editor.key(KeyCode::Esc.into()).unwrap();
    editor.focused = false;
    d.preferences.preset = Preset::Projects;
    d.detail = Some(Box::new(detail));
    d.key(KeyCode::F(23).into());
    assert_eq!(
        d.preferences.preset,
        Preset::Projects,
        "dirty editor refusal must retain the source layout"
    );
    assert!(d.editor.as_mut().unwrap().dirty().unwrap());
    assert_eq!(std::fs::read_to_string(file).unwrap(), "Owner original\n");
}

fn dashboard() -> Dashboard {
    let (commands, _) = mpsc::channel(4);
    let (_, updates) = mpsc::channel(4);
    Dashboard {
        preferences: Preferences::default(),
        path: PathBuf::from("unused"),
        projection: Projection::Default,
        snapshot: Snapshot {
            limited: false,
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
        graph: graph::Graph::default(),
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
        setup_offered: true,
        selection_confirmation: None,
        message: String::new(),
        message_is_error: false,
        commands,
        updates,
        worker: tokio::spawn(std::future::pending()),
        editor: None,
        preview: None,
        follow_files: None,
        review_previews: Vec::new(),
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
async fn changed_original_refusal_survives_background_snapshot() {
    use crate::organization_editor::BufferProposal;
    use sha2::Digest;
    use sha2::Sha256;

    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("owner.txt");
    std::fs::write(&path, "Original owner text\n").unwrap();
    let mut d = dashboard();
    d.snapshot
        .records
        .push(OrganizationRecord::Project(OrganizationProjectPlan {
            id: "project".into(),
            name: "Owner project".into(),
            roots: vec![root.path().to_str().unwrap().into()],
            goal_id: None,
            owner_id: None,
            criteria: vec![],
            revision: 1,
        }));
    d.confirmation = Some(Command::BufferProposal(Box::new(BufferProposal {
        pin: serde_json::from_value(serde_json::json!({
            "editorId": "captured-editor",
            "buffer": 1,
            "path": path,
            "changedtick": 1,
            "sha256": format!("{:x}", Sha256::digest(b"Original owner text\n")),
            "modified": false,
        }))
        .unwrap(),
        text: "Accepted proposal\n".into(),
        result_digest: "accepted-result".into(),
        artifact_sha256: format!("{:x}", Sha256::digest(b"Accepted proposal\n")),
    })));
    std::fs::write(&path, "Changed by the owner\n").unwrap();
    d.key(KeyCode::Char('y').into());
    assert!(d.confirmation.is_none());
    assert!(
        d.message
            .contains("original changed since proposal preview")
    );
    let refusal = d.message.clone();
    let (updates, incoming) = mpsc::channel(4);
    d.updates = incoming;
    updates
        .send(Update::Snapshot(Box::new(Snapshot {
            limited: false,
            records: d.snapshot.records.clone(),
            schedules: vec![],
            workflows: vec![],
            report: None,
        })))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.message, refusal);
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "Changed by the owner\n"
    );
    assert!(d.editor.is_none(), "refusal must precede native mutation");
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
    let template = OrganizationWorkflowTemplate {
        discussion: None,
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
    updates
        .send(Update::SeriesPreview {
            request: request.clone(),
            config: config.clone(),
            source_task: source_task.clone(),
            supervision: Some((
                Box::new(OrganizationGoal {
                    id: "other-goal".into(),
                    parent_id: None,
                    objective: "Changed policy".into(),
                    criteria: vec![],
                    revision: 1,
                }),
                "Other work".into(),
            )),
            preview: Box::new(preview.clone()),
            trigger: OrganizationWorkflowTrigger::Interval,
        })
        .await
        .unwrap();
    d.update();
    assert!(
        d.confirmation.is_none(),
        "late policy changes cannot expand the owner's draft"
    );
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
async fn recent_follow_uses_protected_native_previews_and_preserves_dirty_or_independent_editing() {
    let home = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("first.txt"), "first permitted output\n").unwrap();
    let owner = home.path().join("owner.txt");
    std::fs::write(&owner, "independent saved owner text\n").unwrap();
    let mut d = dashboard();
    d.path = home.path().join("layouts.json");
    let preview = Preview::new(home.path(), source.path(), "first.txt", None).unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&preview.path(), true).unwrap();
    editor.focused = false;
    d.editor = Some(editor);
    d.preview = Some(preview);
    d.following = true;
    d.follow_files = Some(
        FollowFiles::new(
            source.path().into(),
            ["first.txt".into(), "second.txt".into()],
            vec![],
        )
        .unwrap(),
    );
    std::fs::write(source.path().join("second.txt"), "new permitted output\n").unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    d.refresh_follow().unwrap();
    let editor = d.editor.as_mut().unwrap();
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["getline(1)".into()])
            .unwrap()
            .as_str(),
        Some("new permitted output")
    );
    assert!(
        editor
            .rpc("nvim_eval", vec!["expand('%:p')".into()])
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with(home.path().to_str().unwrap())
    );
    editor
        .rpc(
            "nvim_command",
            vec!["setlocal modifiable noreadonly".into()],
        )
        .unwrap();
    editor.paste("unsaved owner change ").unwrap();
    editor.focused = false;
    let dirty = editor
        .rpc(
            "nvim_buf_get_lines",
            vec![0.into(), 0.into(), (-1).into(), true.into()],
        )
        .unwrap();
    std::fs::write(
        source.path().join("first.txt"),
        "a more recent permitted change\n",
    )
    .unwrap();
    for preset in [
        Preset::Console,
        Preset::Workspace,
        Preset::Projects,
        Preset::Operations,
        Preset::Review,
    ] {
        d.switch(preset);
        d.key(KeyEvent::new(
            KeyCode::F(52),
            crossterm::event::KeyModifiers::NONE,
        ));
    }
    d.refresh_follow().unwrap();
    assert_eq!(
        d.editor
            .as_mut()
            .unwrap()
            .rpc(
                "nvim_buf_get_lines",
                vec![0.into(), 0.into(), (-1).into(), true.into()]
            )
            .unwrap(),
        dirty
    );
    let editor = d.editor.as_mut().unwrap();
    editor.rpc("nvim_exec_lua",vec!["local path=...; vim.api.nvim_cmd({cmd='edit',bang=true,args={path}},{}); return true".into(),rmpv::Value::Array(vec![owner.to_str().unwrap().into()])]).unwrap();
    editor.focused = false;
    d.refresh_follow().unwrap();
    assert!(!d.following);
    assert_eq!(
        d.editor
            .as_mut()
            .unwrap()
            .rpc("nvim_eval", vec!["expand('%:p')".into()])
            .unwrap()
            .as_str(),
        owner.to_str()
    );
    assert_eq!(
        std::fs::read_to_string(&owner).unwrap(),
        "independent saved owner text\n"
    );
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
    let template = OrganizationWorkflowTemplate {
        discussion: None,
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
    d.snapshot.workflows.push(OrganizationWorkflowView::Run {
        run: Box::new(run),
        tasks: vec![],
    });
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
        .send(Update::Snapshot(Box::new(Snapshot {
            limited: false,
            report: Some(report_for_tasks(&["one"])),
            schedules: vec![],
            workflows: vec![],
            records: vec![OrganizationRecord::Task(task("one"))],
        })))
        .await
        .unwrap();
    d.update();
    assert_eq!(d.selected.as_deref(), Some("one"));
    assert!(matches!(command_rx.try_recv(), Ok(Command::Inspect(id)) if id == "one"));
    updates
        .send(Update::Snapshot(Box::new(Snapshot {
            limited: false,
            report: Some(report_for_tasks(&["one"])),
            schedules: vec![],
            workflows: vec![],
            records: vec![OrganizationRecord::Task(task("one"))],
        })))
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
        .send(Update::Snapshot(Box::new(Snapshot {
            limited: false,
            report: Some(report_for_tasks(&["new"])),
            schedules: vec![],
            workflows: vec![],
            records: vec![OrganizationRecord::Task(task("new"))],
        })))
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
    use sha2::Digest;
    use sha2::Sha256;
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
        .send(Update::Snapshot(Box::new(Snapshot {
            limited: false,
            report: None,
            schedules: vec![],
            workflows: vec![],
            records: d.snapshot.records.clone(),
        })))
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
        ..
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
    tx.send(Update::Snapshot(Box::new(Snapshot::default())))
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
    tx.send(Update::Snapshot(Box::new(Snapshot::default())))
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
fn scorecard_rendering_keeps_legacy_dimensions_unavailable_and_captured_scope() {
    let mut report = empty_report();
    let text = reporting::text(&report, &reporting::Focus::All, None);
    assert!(text.contains("Scorecards unavailable"));
    let card:OrganizationScorecard=serde_json::from_value(serde_json::json!({"projectId":"project","workflowRunId":null,"ownerId":null,"provider":null,"model":"captured-model","attempts":2,"acceptedResults":1,"failedAttempts":0,"blockedAttempts":1,"uncertainAttempts":0,"checksPassed":1,"checksFailed":0,"reportedTokens":22,"modelRequests":2,"reportedModelRequests":2})).unwrap();
    report.scorecards = Some(OrganizationScorecards {
        entries: vec![card],
        truncated: true,
    });
    let text = reporting::text(&report, &reporting::Focus::Project("project".into()), None);
    assert!(text.contains("unavailable (historical)/captured-model"));
    assert!(text.contains("22 reported tokens"));
    assert!(text.contains("truncated at 2,000"));
    let other = reporting::text(
        &report,
        &reporting::Focus::Agent("current-receiver".into()),
        None,
    );
    assert!(!other.contains("captured-model"));
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

pub(super) fn empty_report() -> OrganizationReport {
    OrganizationReport {
        resource_pressure: None,
        sequence: 0,
        generated_at: 0,
        project_id: None,
        totals: OrganizationReportTotals::default(),
        global_usage: Some(OrganizationGlobalUsage::default()),
        scorecards: None,
        audit_events: None,
        runtime_resources: None,
        tasks: vec![],
        usage: vec![],
        usage_truncated: false,
        upcoming_runs: vec![],
        next_cursor: None,
    }
}

#[tokio::test]
async fn report_pagination_refuses_mixed_canonical_revisions() {
    use futures::SinkExt;
    use futures::StreamExt;
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
