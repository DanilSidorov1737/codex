//! Hash-pinned synthesis drafts and explicit owner publication reuse scoped notes.
use super::controls::persist;
use super::controls::read;
use super::coordination::team_ids;
use super::definitions::text;
use super::execution::read_project;
use super::execution::read_task;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use codex_protocol::organization_coordination::*;
use codex_protocol::organization_execution::OrganizationTaskAttempt;
use sha2::Digest;
use sha2::Sha256;
use sqlx::Sqlite;
use sqlx::Transaction;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) async fn validate_artifact_share(
    tx: &mut Transaction<'_, Sqlite>,
    value: &OrganizationControlValue,
) -> Result<()> {
    let OrganizationControlValue::ArtifactShare {
        id,
        source_task_id,
        source_revision,
        result_digest,
        source_project_id,
        source_owner_id,
        artifact,
        body,
        destinations,
    } = value
    else {
        anyhow::bail!("not an artifact publication")
    };
    Uuid::parse_str(id)?;
    ensure!(destinations.len() <= 32, "at most 32 artifact recipients");
    let (task, _, attempt, result) =
        super::controls::comparison_evidence(tx, source_task_id, result_digest).await?;
    ensure!(
        task.revision == *source_revision
            && task.project_id == *source_project_id
            && attempt.owner_id.as_ref() == Some(source_owner_id),
        "artifact source revision/project/captured owner changed"
    );
    ensure!(
        result.artifacts.contains(artifact),
        "artifact is not the exact accepted output"
    );
    if let Some(body) = body {
        ensure!(
            body.len() <= 32 * 1024
                && body.len() as u64 == artifact.bytes
                && format!("{:x}", Sha256::digest(body.as_bytes())) == artifact.sha256,
            "snapshot differs from accepted artifact or exceeds 32 KiB"
        );
    }
    let mut unique = BTreeSet::new();
    for recipient in destinations {
        ensure!(unique.insert(recipient), "duplicate artifact recipient");
        read_project(tx, recipient).await?;
    }
    Ok(())
}

pub(super) async fn read_artifact_share(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    id: &str,
) -> Result<serde_json::Value> {
    // Recipient ACL precedes decoding or reading the foreign source/result.
    let data: Option<String> = sqlx::query_scalar("SELECT data FROM organization_controls WHERE id=? AND json_extract(data,'$.data.value.type')='artifactShare' AND EXISTS(SELECT 1 FROM json_each(json_extract(data,'$.data.value.destinations')) WHERE value=?)")
        .bind(format!("artifact-share:{id}")).bind(project).fetch_optional(&mut **tx).await?;
    let data = data.context("artifact publication is not permitted/current")?;
    let OrganizationRecord::Control(control) = serde_json::from_str(&data)? else {
        anyhow::bail!("invalid artifact publication")
    };
    validate_artifact_share(tx, &control.value).await?;
    Ok(
        serde_json::json!({"publication": control, "contentTrust":"Untrusted accepted output; provenance is not independent proof of truth", "executionAuthority":false}),
    )
}

async fn permitted_note(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    author: &str,
    reference: &OrganizationNoteReference,
) -> Result<OrganizationNote> {
    let groups = team_ids(tx, author).await?;
    // ACLs precede selection/decoding; guessing a private source ID reveals no body.
    let data:Option<String>=sqlx::query_scalar("SELECT data FROM organization_notes n WHERE id=? AND project_id=? AND superseded_by IS NULL AND NOT EXISTS(SELECT 1 FROM organization_controls c WHERE c.id='knowledge:'||n.id AND json_extract(c.data,'$.data.value.retired')=1) AND (scope='project' OR (scope='agent' AND author_id=?) OR (scope='team' AND group_id IN (SELECT value FROM json_each(?))))")
        .bind(&reference.note_id).bind(project).bind(author).bind(serde_json::to_string(&groups)?).fetch_optional(&mut **tx).await?;
    let note: OrganizationNote = if let Some(data) = data {
        serde_json::from_str(&data)?
    } else {
        let policy = read(tx, "library").await?;
        let published = match policy.map(|c| c.value) {
            Some(OrganizationControlValue::WorkspaceLibrary { projects, sources })
                if projects.iter().any(|p| p == project) && sources.contains(reference) =>
            {
                published_note(tx, reference)
                    .await?
                    .filter(|n| projects.contains(&n.project_id))
            }
            _ => None,
        };
        published.context("knowledge source is not permitted/current")?
    };
    ensure!(
        format!("{:x}", Sha256::digest(note.body.as_bytes())) == reference.body_sha256,
        "knowledge source hash changed"
    );
    Ok(note)
}

fn synthesis_body(id: &str, sources: &[OrganizationNoteReference], body: &str) -> Result<String> {
    let source_text = sources
        .iter()
        .map(|s| format!("{} SHA256 {}", s.note_id, s.body_sha256))
        .collect::<Vec<_>>()
        .join("\n");
    let text_body = format!(
        "Private synthesis from proposal {id}. Claims/conflicts require independent verification; this is not proof of truth.\nSources:\n{source_text}\n\n{body}"
    );
    text(&text_body, 4096)?;
    Ok(text_body)
}

pub(super) async fn propose(
    tx: &mut Transaction<'_, Sqlite>,
    task: &OrganizationTask,
    attempt: &OrganizationTaskAttempt,
    sources: Vec<OrganizationNoteReference>,
    body: String,
    conflicts: Vec<String>,
) -> Result<OrganizationControl> {
    ensure!(
        (1..=16).contains(&sources.len()) && conflicts.len() <= 8,
        "synthesis supports 1–16 sources and at most eight conflict claims"
    );
    text(&body, 2048)?;
    for conflict in &conflicts {
        text(conflict, 256)?;
    }
    let mut ids = BTreeSet::new();
    for reference in &sources {
        ensure!(ids.insert(&reference.note_id), "duplicate synthesis source");
        permitted_note(tx, &task.project_id, &task.owner_id, reference).await?;
    }
    let id = Uuid::now_v7().to_string();
    synthesis_body(&id, &sources, &body)?;
    persist(
        tx,
        None,
        format!("knowledge-proposal:{id}"),
        OrganizationControlValue::KnowledgeProposal {
            id,
            project_id: task.project_id.clone(),
            author_id: task.owner_id.clone(),
            source_attempt_id: attempt.id.clone(),
            sources,
            body,
            conflicts,
            reviewed: false,
        },
    )
    .await
}

pub(super) async fn review(
    tx: &mut Transaction<'_, Sqlite>,
    value: &OrganizationControlValue,
) -> Result<()> {
    let OrganizationControlValue::KnowledgeProposal {
        id,
        project_id,
        author_id,
        source_attempt_id,
        sources,
        body,
        ..
    } = value
    else {
        anyhow::bail!("not a knowledge proposal")
    };
    let data: String = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
        .bind(source_attempt_id)
        .fetch_one(&mut **tx)
        .await?;
    let attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
    let source = read_task(tx, &attempt.task_id).await?;
    ensure!(
        source.project_id == *project_id && attempt.owner_id.as_ref() == Some(author_id),
        "knowledge proposal attribution changed"
    );
    for reference in sources {
        permitted_note(tx, project_id, author_id, reference).await?;
    }
    super::controls::check_project_edit(tx, project_id).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_notes WHERE project_id=?")
            .bind(project_id)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(count < 128, "project note/history ceiling exhausted");
    let note = OrganizationNote {
        id: id.clone(),
        project_id: project_id.clone(),
        author_id: author_id.clone(),
        task_id: source.id,
        attempt_id: attempt.id,
        scope: OrganizationNoteScope::Agent,
        body: synthesis_body(id, sources, body)?,
        created_at: chrono::Utc::now().timestamp(),
        group_id: None,
        source_note_id: None,
        owner_reviewed: true,
    };
    sqlx::query("INSERT INTO organization_notes(id,project_id,author_id,scope,group_id,data) VALUES (?,?,?,'agent',NULL,?)").bind(&note.id).bind(project_id).bind(author_id).bind(serde_json::to_string(&note)?).execute(&mut **tx).await?;
    super::execution::event(tx, "note/synthesisReviewed", &note.id).await?;
    Ok(())
}

async fn published_note(
    tx: &mut Transaction<'_, Sqlite>,
    reference: &OrganizationNoteReference,
) -> Result<Option<OrganizationNote>> {
    let data:Option<String>=sqlx::query_scalar("SELECT data FROM organization_notes n WHERE id=? AND scope='project' AND json_extract(data,'$.ownerReviewed')=1 AND superseded_by IS NULL AND NOT EXISTS(SELECT 1 FROM organization_controls c WHERE c.id='knowledge:'||n.id AND json_extract(c.data,'$.data.value.retired')=1)").bind(&reference.note_id).fetch_optional(&mut **tx).await?;
    let Some(data) = data else {
        return Ok(None);
    };
    let note: OrganizationNote = serde_json::from_str(&data)?;
    if format!("{:x}", Sha256::digest(note.body.as_bytes())) != reference.body_sha256 {
        return Ok(None);
    }
    Ok(Some(note))
}

pub(super) async fn validate_library(
    tx: &mut Transaction<'_, Sqlite>,
    projects: &[String],
    sources: &[OrganizationNoteReference],
) -> Result<()> {
    ensure!(
        projects.len() <= 32 && sources.len() <= 64,
        "library supports at most 32 selected projects and 64 sources"
    );
    let mut ids = BTreeSet::new();
    for project in projects {
        ensure!(ids.insert(project), "duplicate library project");
        read_project(tx, project).await?;
    }
    let mut note_ids = BTreeSet::new();
    for reference in sources {
        ensure!(
            note_ids.insert(&reference.note_id),
            "duplicate library source"
        );
        let note = published_note(tx, reference)
            .await?
            .context("library source must be a current hash-pinned owner-reviewed project note")?;
        ensure!(
            projects.contains(&note.project_id),
            "library source project must be explicitly selected"
        );
    }
    Ok(())
}

pub(super) async fn library_search(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    needle: &str,
) -> Result<OrganizationNoteSearchResult> {
    let Some(control) = read(tx, "library").await? else {
        return Ok(OrganizationNoteSearchResult {
            notes: vec![],
            matched_count: 0,
        });
    };
    let OrganizationControlValue::WorkspaceLibrary { projects, sources } = control.value else {
        anyhow::bail!("invalid library policy")
    };
    // Recipient membership is checked before touching any foreign source row.
    if !projects.iter().any(|id| id == project) {
        return Ok(OrganizationNoteSearchResult {
            notes: vec![],
            matched_count: 0,
        });
    }
    let mut notes = Vec::new();
    let mut matched_count = 0;
    for reference in sources {
        let Some(note) = published_note(tx, &reference).await? else {
            continue;
        };
        if note.project_id != project
            && projects.contains(&note.project_id)
            && note.body.to_lowercase().contains(needle)
        {
            matched_count += 1;
            if notes.len() < 16 {
                notes.push(note);
            }
        }
    }
    Ok(OrganizationNoteSearchResult {
        notes,
        matched_count,
    })
}
