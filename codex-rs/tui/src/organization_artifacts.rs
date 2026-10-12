//! Bounded on-demand owner catalog and evidence-verified plain text previews.
use super::*;

pub(super) async fn catalog(
    handle: &AppServerRequestHandle,
    project: &str,
    query: &str,
) -> anyhow::Result<String> {
    anyhow::ensure!(
        query.len() <= 256 && !query.chars().any(char::is_control),
        "artifact query exceeds 256 bytes or contains controls"
    );
    let report = reporting::load(handle, Some(project)).await?;
    let candidates: Vec<_> = report
        .tasks
        .iter()
        .filter(|t| t.result_digest.is_some())
        .collect();
    let mut text = format!(
        "ARTIFACT LIBRARY · project {project} · audit {}\nLiteral query {query:?}\nExamining at most 32 current result tasks and 128 outputs. Historical result versions remain in execution-read and audit history.\n",
        report.sequence
    );
    let query = query.to_lowercase();
    let mut outputs = 0;
    for task in candidates.iter().take(32) {
        let response: OrganizationExecutionReadResponse = handle
            .request_typed(ClientRequest::OrganizationExecutionRead {
                request_id: request_id(),
                params: OrganizationExecutionReadParams {
                    task_id: task.task_id.clone(),
                },
            })
            .await?;
        let view = response.view;
        let Some(result) = &view.result else {
            continue;
        };
        for artifact in &result.artifacts {
            outputs += 1;
            if outputs > 128 {
                break;
            }
            let entry = format!(
                "\n{}\nTask {} · {} · {}\nProject {} · captured owner {}\nAttempt {} · version {} · accepted {}\nSHA256 {} · {} bytes\nChecks {} pass / {} fail\n",
                artifact.path,
                view.task.id,
                view.task.instructions,
                view.task.state,
                view.task.project_id,
                view.attempts
                    .iter()
                    .find(|a| a.id == result.attempt_id)
                    .and_then(|a| a.owner_id.as_deref())
                    .unwrap_or("unavailable (legacy)"),
                result.attempt_id,
                result.digest,
                result.accepted,
                artifact.sha256,
                artifact.bytes,
                result
                    .checks
                    .iter()
                    .filter(|c| c.exit_code == Some(0))
                    .count(),
                result
                    .checks
                    .iter()
                    .filter(|c| c.exit_code != Some(0))
                    .count()
            );
            if query.is_empty() || entry.to_lowercase().contains(&query) {
                text.push_str(&entry);
            }
        }
        if outputs > 128 {
            break;
        }
    }
    let current: OrganizationReportResponse = handle
        .request_typed(ClientRequest::OrganizationReport {
            request_id: request_id(),
            params: OrganizationReportParams {
                project_id: Some(project.into()),
                cursor: None,
                limit: Some(1),
            },
        })
        .await?;
    anyhow::ensure!(
        report.sequence == current.report.sequence,
        "artifact state changed while loading; refresh this read-only catalog"
    );
    text.push_str(&format!("\n{} result tasks in loaded report; {} output records examined. This is a bounded current-result catalog, not a complete historical scan.\nSelect the task in Projects/Review; n cycles outputs; Commands opens text/data preview; F2 opens actual Neovim. Captured metadata is not a fresh file check; previews recheck bytes.\n",candidates.len(),outputs.min(128)));
    Ok(clean(&text))
}

pub(super) fn text(
    view: &OrganizationExecutionView,
    home: &Path,
    index: usize,
) -> anyhow::Result<String> {
    let result = view
        .result
        .as_ref()
        .context("select a task with captured artifacts")?;
    let artifact = result
        .artifacts
        .get(index % result.artifacts.len().max(1))
        .context("no captured artifact")?;
    anyhow::ensure!(
        artifact.bytes <= 65536,
        "inline text/data preview is limited to 64 KiB; use actual Neovim"
    );
    let attempt = view
        .attempts
        .iter()
        .find(|a| a.id == result.attempt_id)
        .context("captured attempt unavailable")?;
    let workspace = attempt
        .workspace
        .as_deref()
        .context("captured workspace unavailable")?;
    let preview = Preview::new(home, Path::new(workspace), &artifact.path, Some(artifact))?;
    let body = std::fs::read_to_string(preview.path()).context("artifact is not UTF-8 text")?;
    anyhow::ensure!(!body.contains('\0'), "binary artifact has no text preview");
    Ok(clean(&format!(
        "TEXT / MARKDOWN / CSV PREVIEW · literal text, no code execution\n{} · {} bytes · SHA256 {}\nTask {} / attempt {} / result {} / accepted {}\nCaptured owner {} · checks {} pass / {} fail\n\n{}",
        artifact.path,
        artifact.bytes,
        artifact.sha256,
        view.task.id,
        result.attempt_id,
        result.digest,
        result.accepted,
        attempt
            .owner_id
            .as_deref()
            .unwrap_or("unavailable (legacy)"),
        result
            .checks
            .iter()
            .filter(|c| c.exit_code == Some(0))
            .count(),
        result
            .checks
            .iter()
            .filter(|c| c.exit_code != Some(0))
            .count(),
        body
    )))
}
