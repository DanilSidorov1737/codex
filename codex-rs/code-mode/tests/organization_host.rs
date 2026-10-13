//! Qualification uses the actual installed host, never an interpreter substitute.
use codex_code_mode::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn actual_host_has_no_ambient_io_and_fences_yielded_cells_by_session() {
    let program = std::env::var_os("CARGO_BIN_EXE_codex_code_mode_host")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            codex_install_context::InstallContext::current().code_mode_host_program()
        });
    let provider = ProcessOwnedCodeModeSessionProvider::with_host_program(program);
    provider
        .availability()
        .expect("build/install the actual codex-code-mode-host before this qualification");
    let limits = CodeModeSessionCellExecutionLimits {
        max_yield_time_ms: Some(10),
        max_heap_size_bytes: Some(64 * 1024 * 1024),
    };
    let session = provider
        .create_session_with_limits(limits.clone())
        .await
        .unwrap();
    let other = provider.create_session_with_limits(limits).await.unwrap();
    let execute = |source: &str| ExecuteRequest {
        tool_call_id: "qualified-cell".into(),
        enabled_tools: vec![],
        source: source.into(),
        yield_time_ms: Some(5000),
        max_output_tokens: Some(100),
    };
    let started = session
        .execute(
            execute("text([typeof process, typeof require, typeof fetch, typeof Deno]);"),
            Arc::new(NoopCodeModeSessionDelegate),
            None,
        )
        .await
        .unwrap();
    let response = started.initial_response().await.unwrap();
    let RuntimeResponse::Result {
        content_items,
        error_text,
        ..
    } = response
    else {
        panic!("pure native cell should complete")
    };
    assert_eq!(error_text, None);
    let text = serde_json::to_string(&content_items).unwrap();
    assert_eq!(
        text.matches("undefined").count(),
        4,
        "no ambient process/modules/network runtime: {text}"
    );
    let started = session
        .execute(
            execute("await new Promise(() => {});"),
            Arc::new(NoopCodeModeSessionDelegate),
            None,
        )
        .await
        .unwrap();
    let cell = started.cell_id.clone();
    let response = tokio::time::timeout(Duration::from_secs(3), started.initial_response())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(response, RuntimeResponse::Yielded { .. }));
    let foreign = other
        .wait(
            WaitRequest {
                cell_id: cell.clone(),
                yield_time_ms: 1,
            },
            None,
        )
        .await
        .unwrap();
    assert!(
        matches!(foreign, WaitOutcome::MissingCell(_)),
        "another session cannot follow this cell"
    );
    let own = session.wait(
        WaitRequest {
            cell_id: cell.clone(),
            yield_time_ms: 60_000,
        },
        None,
    );
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), own)
            .await
            .unwrap()
            .unwrap(),
        WaitOutcome::LiveCell(RuntimeResponse::Yielded { .. })
    ));
    assert!(matches!(
        session.terminate(cell).await.unwrap(),
        WaitOutcome::LiveCell(RuntimeResponse::Terminated { .. })
    ));

    let busy = session
        .execute(
            execute("while (true) {}"),
            Arc::new(NoopCodeModeSessionDelegate),
            None,
        )
        .await
        .unwrap();
    let busy_cell = busy.cell_id.clone();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), busy.initial_response())
            .await
            .unwrap()
            .unwrap(),
        RuntimeResponse::Yielded { .. }
    ));
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), session.terminate(busy_cell))
            .await
            .unwrap()
            .unwrap(),
        WaitOutcome::LiveCell(RuntimeResponse::Terminated { .. })
    ));
    session.shutdown().await.unwrap();
    other.shutdown().await.unwrap();
}
