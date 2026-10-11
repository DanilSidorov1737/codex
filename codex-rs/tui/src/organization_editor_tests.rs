use super::*;

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
