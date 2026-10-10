use super::*;

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
        frame: FrameRequester::test_dummy(),
        is_done: false,
    }
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
    d.preferences.preset = Preset::Review;
    d.selected = Some("two".into());
    d.rebuild();
    assert_eq!(d.selected.as_deref(), Some("two"));
    d.snapshot
        .records
        .retain(|r| !matches!(r, OrganizationRecord::Task(t) if t.id == "two"));
    d.rebuild();
    assert_eq!(d.selected.as_deref(), Some("one"));
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
        for (width, height) in [(1, 1), (36, 12), (120, 30)] {
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
        artifacts: vec![],
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
    d.decision('A');
    d.key(KeyEvent::new(
        KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(rx.try_recv().is_err());
    assert!(!d.applying);
    assert!(d.editor.as_mut().unwrap().dirty().unwrap());
    assert!(!home.path().join("human.txt").exists());
}
