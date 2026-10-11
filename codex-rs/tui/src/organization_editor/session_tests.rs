use super::session::hash;
use super::*;

#[tokio::test]
async fn official_neovim_session_restores_splits_cursors_and_explicit_unsaved_text_without_writing()
{
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let first = project.path().join("first.txt");
    let second = project.path().join("second.txt");
    std::fs::write(&first, "first\nsecond\nthird\n").unwrap();
    std::fs::write(&second, "other\n").unwrap();
    let roots = vec![project.path().to_path_buf()];
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&first, false).unwrap();
    editor
        .rpc(
            "nvim_win_set_cursor",
            vec![0.into(), Value::Array(vec![3.into(), 2.into()])],
        )
        .unwrap();
    editor
        .command("vsplit", vec![second.to_str().unwrap().into()])
        .unwrap();
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                (-1).into(),
                true.into(),
                Value::Array(vec!["unsaved 界".into()]),
            ],
        )
        .unwrap();
    let snapshot = editor.save_session(home.path(), &roots).unwrap();
    assert!(snapshot.summary().contains("1 unsaved"));
    drop(editor);
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    let loaded = Session::load(home.path(), &roots).unwrap();
    editor.restore_session(&loaded, &roots, false).unwrap();
    assert!(!editor.dirty().unwrap());
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "other\n");
    editor.restore_session(&loaded, &roots, true).unwrap();
    assert!(editor.dirty().unwrap());
    assert_eq!(
        editor
            .rpc("nvim_eval", vec!["winnr('$')".into()])
            .unwrap()
            .as_i64(),
        Some(2)
    );
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("unsaved 界")
    );
    editor.command("undo", vec![]).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("other")
    );
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "other\n");
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                (-1).into(),
                true.into(),
                Value::Array(vec!["saved owner revision".into()]),
            ],
        )
        .unwrap();
    editor.command("write", vec![]).unwrap();
    assert_eq!(
        std::fs::read_to_string(&second).unwrap(),
        "saved owner revision\n"
    );
    assert!(
        !editor
            .save_session(home.path(), &roots)
            .unwrap()
            .has_unsaved(),
        "native owner saves advance the verified disk base"
    );
    std::fs::write(&first, "external change\n").unwrap();
    assert!(Session::load(home.path(), &roots).is_err());
    assert!(
        home.path()
            .join("organization-editor-session.json")
            .is_file()
    );
}

#[tokio::test]
async fn official_neovim_buffer_proposals_pin_identity_tick_path_text_and_use_native_undo() {
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join("file.txt");
    std::fs::write(&path, "original\n").unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&path, false).unwrap();
    let proposal = BufferProposal {
        pin: editor.pin_buffer().unwrap(),
        text: "proposal 界\n".into(),
        result_digest: "accepted-result".into(),
        artifact_sha256: hash("proposal 界\n".as_bytes()),
    };
    editor.apply_buffer_proposal(&proposal).unwrap();
    assert!(editor.dirty().unwrap());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original\n");
    assert!(editor.apply_buffer_proposal(&proposal).is_err());
    editor.command("undo", vec![]).unwrap();
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("original")
    );
    assert!(
        editor.apply_buffer_proposal(&proposal).is_err(),
        "undo changes the revision even when text matches again"
    );
    let pin = editor.pin_buffer().unwrap();
    let mut restarted = Editor::start(FrameRequester::test_dummy()).unwrap();
    restarted.open(&path, false).unwrap();
    let mut old = proposal.clone();
    old.pin = pin;
    assert!(
        restarted.apply_buffer_proposal(&old).is_err(),
        "buffer IDs can be reused after restart, editor identity cannot"
    );
    editor
        .rpc(
            "nvim_buf_set_lines",
            vec![
                0.into(),
                0.into(),
                (-1).into(),
                true.into(),
                Value::Array(vec!["human change".into()]),
            ],
        )
        .unwrap();
    assert!(editor.pin_buffer().is_err());
    assert_eq!(
        editor
            .rpc("nvim_get_current_line", vec![])
            .unwrap()
            .as_str(),
        Some("human change")
    );
}

#[tokio::test]
async fn native_session_refuses_outside_roots_hidden_dirty_buffers_and_changed_disk_bases() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join("source");
    std::fs::write(&path, "before\n").unwrap();
    let mut editor = Editor::start(FrameRequester::test_dummy()).unwrap();
    editor.open(&path, false).unwrap();
    assert!(
        editor
            .save_session(home.path(), &[home.path().to_path_buf()])
            .is_err()
    );
    std::fs::write(&path, "external\n").unwrap();
    assert!(
        editor
            .save_session(home.path(), &[project.path().to_path_buf()])
            .is_err()
    );
    assert!(
        !home
            .path()
            .join("organization-editor-session.json")
            .exists()
    );
}
