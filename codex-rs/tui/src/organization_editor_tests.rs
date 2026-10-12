use super::*;

#[tokio::test]
async fn actual_neovim_project_explorer_opens_owner_files_and_preserves_dirty_buffers() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("owner.txt");
    let other = root.path().join("outside-grant.txt");
    std::fs::write(&original, "saved owner buffer\n").unwrap();
    std::fs::write(
        &other,
        "owner project file beyond the selected task grant\n",
    )
    .unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&original, false).unwrap();
    editor.paste("unsaved owner content ").unwrap();
    let buffer = editor.rpc("nvim_get_current_buf", vec![]).unwrap();
    let lines = editor
        .rpc(
            "nvim_buf_get_lines",
            vec![buffer.clone(), 0.into(), (-1).into(), true.into()],
        )
        .unwrap();
    assert!(editor.dirty().unwrap());
    editor.project_explorer(root.path()).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["&filetype".into()])
            .unwrap()
            .as_str(),
        Some("netrw")
    );
    assert_eq!(
        editor
            .rpc(
                "nvim_buf_get_lines",
                vec![buffer, 0.into(), (-1).into(), true.into()]
            )
            .unwrap(),
        lines
    );
    editor.guarded_lua("for i,line in ipairs(vim.api.nvim_buf_get_lines(0,0,-1,false)) do if line:find('outside-grant.txt',1,true) then vim.api.nvim_win_set_cursor(0,{i,0}); return true end end; error('owner file absent from native explorer')",vec![]).unwrap();
    editor
        .key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    let mut opened = false;
    for _ in 0..100 {
        if editor
            .rpc("nvim_eval", vec!["expand('%:p')".into()])
            .unwrap()
            .as_str()
            == other.to_str()
        {
            opened = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(opened, "native Enter opens the owner-selected file");
    assert!(editor.dirty().unwrap());
    assert_eq!(
        std::fs::read_to_string(&original).unwrap(),
        "saved owner buffer\n"
    );
    assert_eq!(
        std::fs::read_to_string(&other).unwrap(),
        "owner project file beyond the selected task grant\n"
    );
}

#[tokio::test]
async fn real_three_way_review_preserves_common_base_owner_and_agent_and_refuses_dirty_buffers() {
    let project = tempfile::tempdir().unwrap();
    let base = project.path().join("base.txt");
    let owner = project.path().join("owner.txt");
    let agent = project.path().join("agent.txt");
    std::fs::write(&base, "common base\n").unwrap();
    std::fs::write(&owner, "owner change\n").unwrap();
    std::fs::write(&agent, "agent change\n").unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&owner, false).unwrap();
    editor.paste("unsaved owner change").unwrap();
    assert!(editor.three_way(&base, &owner, &agent).is_err());
    editor.rpc("nvim_command", vec!["edit!".into()]).unwrap();
    editor
        .rpc("nvim_command", vec!["setlocal winbar=OwnerConfig".into()])
        .unwrap();
    editor.three_way(&base, &owner, &agent).unwrap();
    for width in [150, 102] {
        editor.resize(width, 28).unwrap();
        let widths = editor
            .rpc(
                "nvim_eval",
                vec!["[winwidth(1),winwidth(2),winwidth(3)]".into()],
            )
            .unwrap();
        let widths: Vec<_> = widths
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        assert!(
            widths.iter().max().unwrap() - widths.iter().min().unwrap() <= 1,
            "native review columns stay balanced: {widths:?}"
        );
    }
    for (index, (path, label)) in [
        (base.clone(), "Base · captured original"),
        (owner.clone(), "Owner · at review capture"),
        (agent.clone(), "Agent · verified artifact"),
    ]
    .iter()
    .enumerate()
    {
        editor
            .command("wincmd", vec![format!("{}w", index + 1).into()])
            .unwrap();
        assert_eq!(
            editor
                .rpc("nvim_eval", vec!["expand('%:p')".into()])
                .unwrap()
                .as_str(),
            path.to_str()
        );
        assert_eq!(
            editor
                .rpc("nvim_eval", vec!["&winbar".into()])
                .unwrap()
                .as_str(),
            Some(*label)
        );
    }
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["winnr('$')".into()])
            .unwrap()
            .as_i64(),
        Some(3)
    );
    for _ in 0..3 {
        assert_eq!(
            editor
                .rpc(
                    "nvim_eval",
                    vec!["&diff && &readonly && !&modifiable".into()]
                )
                .unwrap()
                .as_i64(),
            Some(1)
        );
        editor.command("wincmd", vec!["w".into()]).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&owner).unwrap(), "owner change\n");
    editor.open(&owner, false).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["&winbar".into()])
            .unwrap()
            .as_str(),
        Some("OwnerConfig")
    );
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                (-1).into(),
                true.into(),
                Value::Array(vec!["owner resolves both changes".into()]),
            ],
        )
        .unwrap();
    editor.command("write", vec![]).unwrap();
    assert_eq!(
        std::fs::read_to_string(&owner).unwrap(),
        "owner resolves both changes\n"
    );
    assert_eq!(std::fs::read_to_string(&base).unwrap(), "common base\n");
    assert_eq!(std::fs::read_to_string(&agent).unwrap(), "agent change\n");
    editor.diff(&base, &agent).unwrap();
    editor.resize(120, 24).unwrap();
    for (index, path) in [&base, &agent].iter().enumerate() {
        editor
            .command("wincmd", vec![format!("{}w", index + 1).into()])
            .unwrap();
        assert_eq!(
            editor
                .rpc("nvim_eval", vec!["expand('%:p')".into()])
                .unwrap()
                .as_str(),
            path.to_str()
        );
    }
    let widths = editor
        .rpc("nvim_eval", vec!["[winwidth(1),winwidth(2)]".into()])
        .unwrap();
    assert!(
        (widths.as_array().unwrap()[0].as_i64().unwrap()
            - widths.as_array().unwrap()[1].as_i64().unwrap())
        .abs()
            <= 1
    );
}

#[tokio::test]
async fn actual_neovim_terminal_runs_owner_commands_beside_unsaved_code_and_captured_tests() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let original = project.path().join("code.txt");
    std::fs::write(&original, "saved code\n").unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&original, false).unwrap();
    let buffer = editor.rpc("nvim_get_current_buf", vec![]).unwrap();
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                buffer.clone(),
                0.into(),
                (-1).into(),
                true.into(),
                Value::Array(vec!["unsaved owner code".into()]),
            ],
        )
        .unwrap();
    editor.terminal(project.path()).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["&buftype".into()])
            .unwrap()
            .as_str(),
        Some("terminal")
    );
    editor
        .paste("printf native-terminal-ok > terminal-proof.txt\n")
        .unwrap();
    let proof = project.path().join("terminal-proof.txt");
    for _ in 0..100 {
        if proof.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        editor.poll();
    }
    assert_eq!(
        std::fs::read_to_string(&proof).unwrap(),
        "native-terminal-ok"
    );
    assert_eq!(std::fs::read_to_string(&original).unwrap(), "saved code\n");
    assert_eq!(
        editor
            .rpc(
                "nvim_buf_get_lines",
                vec![buffer, 0.into(), (-1).into(), true.into()]
            )
            .unwrap(),
        Value::Array(vec!["unsaved owner code".into()])
    );
    assert!(
        editor
            .save_session(home.path(), &[project.path().into()])
            .is_err(),
        "terminal jobs must not be serialized as project files"
    );
    let panel = crate::organization_preview::Preview::captured_text(
        home.path(),
        "Captured oracle\nexit 0\nPASS\n",
    )
    .unwrap();
    editor.captured_panel(&panel.path()).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("Captured oracle")
    );
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["&modifiable".into()])
            .unwrap()
            .as_i64(),
        Some(0)
    );
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["winnr('$')".into()])
            .unwrap()
            .as_i64(),
        Some(3)
    );
}

#[tokio::test]
async fn official_neovim_loads_opt_in_owner_config_and_bundled_plugin_without_changing_it() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("init.lua");
    let content = "vim.g.codex_owner_config=true\nvim.cmd('packadd matchit')\nvim.o.number=false\n";
    std::fs::write(&path, content).unwrap();
    let mut editor = Editor::start_configured(FrameRequester::test_dummy(), Some(&path)).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["g:codex_owner_config".into()])
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["exists('g:loaded_matchit')".into()])
            .unwrap()
            .as_i64(),
        Some(1)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    drop(editor);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
        assert!(Editor::start_configured(FrameRequester::test_dummy(), Some(&path)).is_err());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_neovim_queries_do_not_schedule_redraws_but_native_redraw_and_exit_do() {
    let (draws, mut receiver) = tokio::sync::broadcast::channel(32);
    let frame = FrameRequester::new(draws);
    let mut editor = Editor::start(frame.clone()).unwrap();
    editor.rpc("nvim_eval", vec!["1".into()]).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    while receiver.try_recv().is_ok() {}

    for _ in 0..12 {
        assert!(!editor.dirty().unwrap());
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        receiver.try_recv().is_err(),
        "ordinary RPC replies must not perpetuate idle Follow draws"
    );

    editor.rpc("nvim_command", vec!["redraw!".into()]).unwrap();
    tokio::time::timeout(Duration::from_secs(1), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    while receiver.try_recv().is_ok() {}
    editor.child.kill().unwrap();
    editor.child.wait().unwrap();
    tokio::time::timeout(Duration::from_secs(1), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    editor.poll();
    assert!(editor.error.is_some());
    drop(frame);
}

#[test]
fn native_key_encoding_preserves_unicode_and_literal_angle_brackets() {
    assert_eq!(
        key_text(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::NONE)),
        "<lt>"
    );
    assert_eq!(
        key_text(KeyEvent::new(KeyCode::Char('界'), KeyModifiers::NONE)),
        "界"
    );
    assert_eq!(
        key_text(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        "<C-r>"
    );
    assert_eq!(
        key_text(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
        "<S-Tab>"
    );
}

#[tokio::test]
async fn native_changedtick_fence_retains_a_modified_buffer_instead_of_reloading_it() {
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.freeze().unwrap();
    assert!(
        editor
            .key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
            .is_err()
    );
    assert!(editor.paste("blocked human input").is_err());
    // Simulate an additional human-authorized editor actor; ordinary host input
    // is blocked while applying. Changedtick still detects that native edit.
    editor
        .rpc(
            "nvim_exec_lua",
            vec![
                "vim.bo.modifiable=true; vim.api.nvim_buf_set_lines(0,0,-1,false,{'human change'})"
                    .into(),
                Value::Array(vec![]),
            ],
        )
        .unwrap();
    assert!(editor.thaw().is_err());
    assert!(editor.dirty().unwrap());
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("human change")
    );
}

#[test]
fn oversized_and_overdeep_transport_values_fail_bounded_decoding() {
    let bytes = vec![0xc4, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut stream = Limited {
        inner: std::io::Cursor::new(bytes),
        remaining: 5,
    };
    assert!(rmpv::decode::read_value_with_max_depth(&mut stream, 64).is_err());
    let mut nested = vec![0x91; 65];
    nested.push(0xc0);
    assert!(
        rmpv::decode::read_value_with_max_depth(&mut std::io::Cursor::new(nested), 64).is_err()
    );
}

#[tokio::test]
async fn real_neovim_edits_unicode_saves_undoes_diffs_and_blocks_dirty_mode_changes() {
    let home = tempfile::tempdir().unwrap();
    let base = home.path().join("literal | name.txt");
    let changed = home.path().join("changed.txt");
    std::fs::write(&base, "base\n").unwrap();
    std::fs::write(&changed, "agent\n").unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&base, false).unwrap();
    editor
        .key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    editor.paste("世界 café ").unwrap();
    editor
        .key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(editor.dirty().unwrap());
    assert!(editor.freeze().is_err());
    assert!(editor.open(&changed, true).is_err());
    editor.command("write", vec![]).unwrap();
    assert_eq!(std::fs::read_to_string(&base).unwrap(), "世界 café base\n");
    editor
        .key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE))
        .unwrap();
    editor.command("write", vec![]).unwrap();
    assert_eq!(std::fs::read_to_string(&base).unwrap(), "base\n");
    editor.freeze().unwrap();
    assert_eq!(
        editor
            .rpc(
                "nvim_get_option_value",
                vec![
                    "modifiable".into(),
                    Value::Map(vec![("buf".into(), 0.into())])
                ]
            )
            .unwrap()
            .as_bool(),
        Some(false)
    );
    editor.thaw().unwrap();
    assert_eq!(
        editor
            .rpc(
                "nvim_get_option_value",
                vec![
                    "modifiable".into(),
                    Value::Map(vec![("buf".into(), 0.into())])
                ]
            )
            .unwrap()
            .as_bool(),
        Some(true)
    );
    editor.diff(&base, &changed).unwrap();
    let windows = editor.rpc("nvim_list_wins", vec![]).unwrap();
    assert_eq!(array(&windows).unwrap().len(), 2);
    assert_eq!(
        editor
            .rpc(
                "nvim_get_option_value",
                vec!["diff".into(), Value::Map(vec![])]
            )
            .unwrap()
            .as_bool(),
        Some(true)
    );
    editor.resize(50, 12).unwrap();
    editor.rpc("nvim_eval", vec!["1".into()]).unwrap();
    editor.poll();
    assert!(editor.error.is_none());
    assert_eq!(editor.visible.width, 50);
    assert!(editor.visible.cells.iter().any(|(text, _)| text == "a"));
    let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 12));
    assert!(editor.render(buffer.area, &mut buffer).is_some());
    // Follow reload preserves the human window/cursor and refuses focused editing.
    editor.focused = false;
    let position = editor.rpc("nvim_win_get_cursor", vec![0.into()]).unwrap();
    std::fs::write(&changed, "new agent content\n").unwrap();
    editor.reload_follow().unwrap();
    assert_eq!(
        editor.rpc("nvim_win_get_cursor", vec![0.into()]).unwrap(),
        position
    );
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("new agent content")
    );
    editor.child.kill().unwrap();
    editor.child.wait().unwrap();
    for _ in 0..20 {
        editor.poll();
        if editor.error.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(editor.error.is_some());
    assert_eq!(std::fs::read_to_string(&base).unwrap(), "base\n");
}

#[tokio::test]
async fn actual_neovim_selection_preserves_unicode_exclusive_lines_and_rejects_stale_buffers() {
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                (-1).into(),
                false.into(),
                Value::Array(vec!["café 界".into(), "next line".into()]),
            ],
        )
        .unwrap();
    editor
        .rpc(
            "nvim_exec_lua",
            vec![
                "vim.api.nvim_feedkeys('gg0v3l','nx',false)".into(),
                Value::Array(vec![]),
            ],
        )
        .unwrap();
    let capture = editor.selection().unwrap();
    assert_eq!(capture.text, "café");
    assert_eq!(capture.positions[0][0][1..3], [1, 1]);
    editor.validate_selection(&capture).unwrap();
    editor.rpc("nvim_exec_lua",vec!["vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes('<Esc>',true,false,true),'nx',false); vim.o.selection='exclusive'; vim.api.nvim_feedkeys('gg0v4l','nx',false)".into(),Value::Array(vec![])]).unwrap();
    assert_eq!(editor.selection().unwrap().text, "café");
    editor.rpc("nvim_exec_lua",vec!["vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes('<Esc>',true,false,true)..'ggVj','nx',false)".into(),Value::Array(vec![])]).unwrap();
    let lines = editor.selection().unwrap();
    assert_eq!(lines.text, "café 界\nnext line");
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                1.into(),
                false.into(),
                Value::Array(vec!["human edit".into()]),
            ],
        )
        .unwrap();
    assert!(editor.validate_selection(&capture).is_err());
    assert!(editor.error.is_none());
    editor.rpc("nvim_exec_lua",vec!["vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes('<Esc><C-v>j',true,false,true),'nx',false)".into(),Value::Array(vec![])]).unwrap();
    assert!(
        editor
            .selection()
            .unwrap_err()
            .to_string()
            .contains("block selections")
    );
    assert!(
        editor.error.is_none(),
        "semantic refusal must not poison native RPC"
    );
    editor.rpc("nvim_exec_lua",vec!["vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes('<Esc>',true,false,true),'nx',false);vim.api.nvim_buf_set_lines(0,0,-1,false,{string.rep('x',12289)});vim.api.nvim_feedkeys('ggV','nx',false)".into(),Value::Array(vec![])]).unwrap();
    assert!(
        editor
            .selection()
            .unwrap_err()
            .to_string()
            .contains("12 KiB")
    );
    assert!(editor.error.is_none());
    editor
        .rpc(
            "nvim_buf_set_name",
            vec![0.into(), "/tmp/renamed-selection-source".into()],
        )
        .unwrap();
    assert!(editor.validate_selection(&lines).is_err());
}
