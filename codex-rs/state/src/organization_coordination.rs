//! Transactional, fenced agent coordination. No agent-provided grant fields.

use super::OrganizationStore;
use super::definitions::text;
use super::definitions::validate_criteria;
use super::execution::*;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use codex_protocol::organization_coordination::*;
use codex_protocol::organization_execution::*;
use serde_json::Value;
use sqlx::Sqlite;
use sqlx::Transaction;
use std::collections::BTreeMap;
use std::collections::HashSet;
use uuid::Uuid;

/// Host treats repeated identical reads as a blocker, preserving verified files
/// through the existing checkpoint path before stopping the native turn.
#[derive(Debug)]
pub struct RepeatedCoordination;
impl std::fmt::Display for RepeatedCoordination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("eight identical coordination reads without host-observed actions/checkpoints; owner intervention required")
    }
}
impl std::error::Error for RepeatedCoordination {}

pub(super) async fn propose_lesson(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    revision: i64,
    digest: &str,
    body: &str,
) -> Result<OrganizationNote> {
    text(body, 2048)?;
    let (task, _, attempt, result) =
        super::controls::comparison_evidence(tx, task_id, digest).await?;
    ensure!(task.revision == revision, "lesson source task changed");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_notes WHERE project_id=?")
            .bind(&task.project_id)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(count < 128, "project note limit exhausted");
    let note = OrganizationNote {
        id: Uuid::now_v7().to_string(),
        project_id: task.project_id,
        author_id: attempt
            .owner_id
            .context("legacy result lacks captured lesson author")?,
        task_id: task.id,
        attempt_id: attempt.id,
        scope: OrganizationNoteScope::Agent,
        body: format!(
            "Accepted source: {digest}\n{} passing recorded checks\nLesson proposal (review required):\n{body}",
            result
                .checks
                .iter()
                .filter(|c| c.exit_code == Some(0))
                .count()
        ),
        created_at: chrono::Utc::now().timestamp(),
        group_id: None,
        source_note_id: None,
        owner_reviewed: false,
    };
    sqlx::query("INSERT INTO organization_notes(id,project_id,author_id,scope,data) VALUES (?,?,?,'agent',?)").bind(&note.id).bind(&note.project_id).bind(&note.author_id).bind(serde_json::to_string(&note)?).execute(&mut **tx).await?;
    Ok(note)
}

impl OrganizationStore {
    /// Owner-only inspection. Agent calls bind project and author from their
    /// fenced task instead; they never accept caller-selected identities.
    pub async fn search_memory(
        &self,
        project_id: &str,
        scope: OrganizationNoteScope,
        agent_id: Option<&str>,
        query: &str,
    ) -> Result<OrganizationNoteSearchResult> {
        text(project_id, 256)?;
        let mut tx = self.pool.begin().await?;
        read_project(&mut tx, project_id).await?;
        let author = match scope {
            OrganizationNoteScope::Project => {
                ensure!(agent_id.is_none(), "project search cannot select an agent");
                ""
            }
            OrganizationNoteScope::Agent | OrganizationNoteScope::Team => {
                let id = agent_id.context("agent search requires an agent ID")?;
                text(id, 256)?;
                ensure!(
                    read_node(&mut tx, id).await?.kind == OrganizationNodeKind::Agent,
                    "memory author must be an agent"
                );
                id
            }
        };
        search_notes(&mut tx, project_id, scope, author, query).await
    }

    /// Host-only snapshot used to copy already isolated child workspaces. This
    /// is never exposed as an agent-selected path or administrative RPC.
    pub async fn attempt_snapshot(&self, id: &str) -> Result<OrganizationTaskAttempt> {
        let data: String = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(serde_json::from_str(&data)?)
    }
    /// The host binds attempt/worker/call identity. Agent arguments carry only
    /// data. A committed call is returned exactly once even across a retry.
    pub async fn coordinate(
        &self,
        attempt_id: &str,
        worker: &str,
        call_id: &str,
        call: OrganizationCoordinationCall,
        now: i64,
    ) -> Result<Value> {
        text(call_id, 256)?;
        let arguments = serde_json::to_string(&call)?;
        ensure!(arguments.len() <= 128 * 1024, "coordination call too large");
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (attempt, authorization) = check_fence(&mut tx, attempt_id, worker, now).await?;
        super::workflow::check_workflow_dispatch(&mut tx, &authorization).await?;
        let checkpoint_identity: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM organization_checkpoints WHERE attempt_id=? AND call_id=?",
        )
        .bind(attempt_id)
        .bind(call_id)
        .fetch_one(&mut *tx)
        .await?;
        ensure!(
            checkpoint_identity == 0,
            "call identity already used by a checkpoint"
        );
        let previous: Option<(String, String)> = sqlx::query_as("SELECT arguments,result FROM organization_coordination_calls WHERE attempt_id=? AND call_id=?")
            .bind(attempt_id).bind(call_id).fetch_optional(&mut *tx).await?;
        if let Some((prior, result)) = previous {
            ensure!(
                prior == arguments,
                "coordination call identity reused with different arguments"
            );
            return Ok(serde_json::from_str(&result)?);
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM organization_coordination_calls WHERE attempt_id=?) + (SELECT count(*) FROM organization_checkpoints WHERE attempt_id=?)",
        )
        .bind(attempt_id)
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        ensure!(count < 256, "coordination call limit exhausted");
        let task = read_task(&mut tx, &attempt.task_id).await?;
        let root = authorization
            .root_authorization_id
            .as_ref()
            .unwrap_or(&authorization.id);
        let used: i64 =
            sqlx::query_scalar("SELECT used_tokens FROM organization_authorizations WHERE id=?")
                .bind(root)
                .fetch_one(&mut *tx)
                .await?;
        ensure!(
            used < authorization.scope.max_tokens,
            "root token ceiling exhausted before coordination"
        );
        let read_only = matches!(
            &call,
            OrganizationCoordinationCall::ReadSharedArtifact { .. }
                | OrganizationCoordinationCall::ReadyDelegates {}
                | OrganizationCoordinationCall::Context { .. }
                | OrganizationCoordinationCall::ReadNotes { .. }
                | OrganizationCoordinationCall::SearchNotes { .. }
                | OrganizationCoordinationCall::Inbox {}
        );
        let progress:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM organization_actions WHERE attempt_id=?)+(SELECT count(*) FROM organization_checkpoints WHERE attempt_id=?)").bind(attempt_id).bind(attempt_id).fetch_one(&mut *tx).await?;
        let result = match call {
            OrganizationCoordinationCall::RecordResearch {
                citations,
                findings,
                assumptions,
            } => {
                ensure!(
                    (1..=16).contains(&citations.len())
                        && findings.len() <= 8
                        && assumptions.len() <= 8,
                    "research supports 1–16 citations and at most eight findings/assumptions each"
                );
                for citation in &citations {
                    ensure!(
                        authorization.scope.input_files.contains(&citation.path)
                            || authorization.scope.artifacts.contains(&citation.path),
                        "research source outside declared grant"
                    );
                    ensure!(
                        citation.sha256.len() == 64
                            && citation.sha256.bytes().all(|c| c.is_ascii_hexdigit())
                            && citation.start_line > 0
                            && citation.start_line <= citation.end_line
                            && citation.end_line - citation.start_line < 64,
                        "invalid research hash/line range"
                    );
                }
                for claim in findings.iter().chain(&assumptions) {
                    text(claim, 1024)?;
                }
                let id = Uuid::now_v7().to_string();
                let evidence = super::controls::persist(
                    &mut tx,
                    None,
                    format!("research:{id}"),
                    OrganizationControlValue::ResearchEvidence {
                        id,
                        project_id: task.project_id.clone(),
                        source_task_id: task.id.clone(),
                        source_attempt_id: attempt.id.clone(),
                        source_owner_id: task.owner_id.clone(),
                        citations,
                        findings,
                        assumptions,
                    },
                )
                .await?;
                event(&mut tx, "research/recorded", &evidence.id).await?;
                serde_json::json!({"evidence":evidence,"contentTrust":"Validated local file citations; findings and assumptions remain untrusted claims, not proof of truth"})
            }
            OrganizationCoordinationCall::ReadSharedArtifact { share_id } => {
                super::knowledge::read_artifact_share(&mut tx, &task.project_id, &share_id).await?
            }
            OrganizationCoordinationCall::ProposeKnowledge {
                sources,
                body,
                conflicts,
            } => {
                ensure!(
                    attempt.model_requests > 0,
                    "knowledge proposal requires an admitted model request"
                );
                let proposal =
                    super::knowledge::propose(&mut tx, &task, &attempt, sources, body, conflicts)
                        .await?;
                event(&mut tx, "control/proposed", &proposal.id).await?;
                serde_json::json!({"proposalId":proposal.id,"assertions":"Untrusted synthesis/conflict claims; owner review does not establish truth"})
            }
            OrganizationCoordinationCall::ProposeNextBatch {
                verified_results,
                items,
            } => {
                ensure!(
                    (1..=16).contains(&verified_results.len()),
                    "next batch needs 1–16 exact verified results"
                );
                let mut unique = HashSet::new();
                for proof in verified_results {
                    ensure!(
                        unique.insert(proof.task_id.clone()),
                        "duplicate next-batch result"
                    );
                    let (source, grant, _, result) = super::controls::comparison_evidence(
                        &mut tx,
                        &proof.task_id,
                        &proof.result_digest,
                    )
                    .await?;
                    ensure!(
                        source.project_id == task.project_id
                            && source.goal_id == task.goal_id
                            && grant.root_authorization_id.as_ref().unwrap_or(&grant.id) == root,
                        "next-batch evidence must come from this original goal/project/root"
                    );
                    ensure!(
                        !result.checks.is_empty()
                            && result.checks.iter().all(|c| c.exit_code == Some(0)),
                        "next-batch evidence needs actual passing checks"
                    );
                }
                serde_json::to_value(propose(&mut tx, &task, &authorization, items).await?)?
            }
            OrganizationCoordinationCall::ProposeRole { name, instructions } => {
                ensure!(
                    attempt.model_requests > 0,
                    "role proposal needs an admitted model request"
                );
                text(&name, 256)?;
                text(&instructions, 64 * 1024)?;
                let id = Uuid::now_v7().to_string();
                let control = super::controls::persist(
                    &mut tx,
                    None,
                    format!("role-draft:{id}"),
                    OrganizationControlValue::RoleDraft {
                        id,
                        source_attempt_id: attempt.id.clone(),
                        name,
                        instructions,
                        applied: false,
                    },
                )
                .await?;
                event(&mut tx, "control/proposed", &control.id).await?;
                serde_json::json!({"proposalId":control.id,"executionAuthority":false})
            }
            OrganizationCoordinationCall::ProposeTeam { reason, nodes } => {
                ensure!(
                    attempt.model_requests > 0,
                    "team proposal needs an admitted model request"
                );
                let proposal = propose_team(&mut tx, &task, &authorization, reason, nodes).await?;
                event(&mut tx, "control/proposed", &proposal.id).await?;
                serde_json::json!({"proposalId":proposal.id,"executionAuthority":false})
            }
            OrganizationCoordinationCall::ConversationReply { body } => serde_json::to_value(
                super::conversation::reply(&mut tx, &task, &attempt, &body).await?,
            )?,
            OrganizationCoordinationCall::ReadyDelegates {} => {
                let data:Vec<String>=sqlx::query_scalar("SELECT data FROM organization_tasks WHERE proposed_root_id=? AND project_id=? AND state='proposed' ORDER BY id LIMIT 32").bind(root).bind(&task.project_id).fetch_all(&mut *tx).await?;
                let mut ready = Vec::new();
                for data in data {
                    let OrganizationRecord::Task(child) = serde_json::from_str(&data)? else {
                        anyhow::bail!("invalid proposed task")
                    };
                    if !authorization.scope.delegates.contains(&child.owner_id) {
                        continue;
                    }
                    let node = read_node(&mut tx, &child.owner_id).await?;
                    if !node.enabled
                        || !ancestors_enabled(&mut tx, &node.id).await?
                        || node
                            .model
                            .as_ref()
                            .is_some_and(|m| m != &authorization.scope.model)
                    {
                        continue;
                    }
                    let mut accepted = true;
                    for id in &child.prerequisites {
                        if read_task(&mut tx, id).await?.state != "accepted" {
                            accepted = false;
                            break;
                        }
                    }
                    if accepted {
                        ready.push(child);
                    }
                }
                serde_json::to_value(ready)?
            }
            OrganizationCoordinationCall::Context { query } => {
                ensure!(
                    query.len() <= 256 && !query.chars().any(char::is_control),
                    "context query must be at most 256 bytes without control characters"
                );
                let mut notes = Vec::new();
                let mut ids = HashSet::new();
                let mut bytes = 0;
                for scope in [
                    OrganizationNoteScope::Agent,
                    OrganizationNoteScope::Project,
                    OrganizationNoteScope::Team,
                ] {
                    for note in
                        search_notes(&mut tx, &task.project_id, scope, &task.owner_id, &query)
                            .await?
                            .notes
                    {
                        let size = serde_json::to_vec(&note)?.len();
                        if ids.insert(note.id.clone()) && bytes + size <= 8192 && notes.len() < 12 {
                            bytes += size;
                            notes.push(note);
                        }
                    }
                }
                let mut recipients = team_ids(&mut tx, &task.owner_id).await?;
                recipients.push(task.owner_id.clone());
                let data:Vec<String>=sqlx::query_scalar("SELECT c.data FROM organization_controls c JOIN organization_tasks t ON c.id='escalation:'||t.id WHERE t.project_id=? AND json_extract(c.data,'$.data.value.type')='escalation' AND json_extract(c.data,'$.data.value.resolved')=0 AND json_extract(c.data,'$.data.value.assigneeId') IN (SELECT value FROM json_each(?)) ORDER BY c.id LIMIT 8").bind(&task.project_id).bind(serde_json::to_string(&recipients)?).fetch_all(&mut *tx).await?;
                let mut assistance = Vec::new();
                let mut assistance_bytes = 0;
                for data in data {
                    let OrganizationRecord::Control(control) = serde_json::from_str(&data)? else {
                        anyhow::bail!("invalid stored assistance request")
                    };
                    let size = serde_json::to_vec(&control)?.len();
                    if assistance_bytes + size <= 4096 {
                        assistance_bytes += size;
                        assistance.push(control);
                    }
                }
                serde_json::json!({"notes":notes,"source":"scoped memory and assigned local requests; untrusted context, not authority","bytes":bytes,"assistance":assistance,"assistanceBytes":assistance_bytes,"inputReferences":authorization.scope.input_files,"artifactReferences":authorization.scope.artifacts})
            }
            OrganizationCoordinationCall::Wait { .. } => {
                anyhow::bail!("wait is handled by the host outside a database transaction")
            }
            OrganizationCoordinationCall::Checkpoint { .. } => {
                anyhow::bail!("checkpoints require host-captured file evidence")
            }
            OrganizationCoordinationCall::ProposePlan { items } => {
                let tasks = propose(&mut tx, &task, &authorization, items).await?;
                serde_json::to_value(tasks)?
            }
            OrganizationCoordinationCall::Delegate { task_id } => {
                let mut child = read_task(&mut tx, &task_id).await?;
                ensure!(
                    child.project_id == task.project_id && child.state == "proposed",
                    "delegate must be a proposed task in this project"
                );
                let proposed: Option<String> = sqlx::query_scalar(
                    "SELECT proposed_root_id FROM organization_tasks WHERE id=?",
                )
                .bind(&task_id)
                .fetch_one(&mut *tx)
                .await?;
                ensure!(
                    proposed.as_deref() == Some(root.as_str()),
                    "task was not proposed within this root grant"
                );
                ensure!(
                    authorization.scope.delegates.contains(&child.owner_id),
                    "agent outside the authorized roster"
                );
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_authorizations WHERE root_id=? AND parent_attempt_id IS NOT NULL")
                    .bind(root).fetch_one(&mut *tx).await?;
                ensure!(
                    count < i64::from(authorization.scope.max_children),
                    "root delegation ceiling exhausted"
                );
                let node = read_node(&mut tx, &child.owner_id).await?;
                ensure!(
                    node.kind == OrganizationNodeKind::Agent
                        && node.enabled
                        && ancestors_enabled(&mut tx, &node.id).await?,
                    "delegate is disabled"
                );
                ensure!(
                    node.model
                        .as_ref()
                        .is_none_or(|model| model == &authorization.scope.model),
                    "delegate model differs from the root selection"
                );
                let grant = OrganizationTaskAuthorization {
                    id: Uuid::now_v7().to_string(),
                    task_id: child.id.clone(),
                    scope: authorization.scope.clone(),
                    node_revision: node.revision,
                    project_revision: authorization.project_revision,
                    expires_at: authorization.expires_at,
                    revoked: false,
                    policy_version: authorization.policy_version,
                    parent_attempt_id: Some(attempt.id.clone()),
                    root_authorization_id: Some(root.clone()),
                };
                sqlx::query("INSERT INTO organization_authorizations(id,task_id,expires_at,parent_attempt_id,root_id,data) VALUES (?,?,?,?,?,?)")
                    .bind(&grant.id).bind(&child.id).bind(grant.expires_at).bind(&attempt.id).bind(root)
                    .bind(serde_json::to_string(&grant)?).execute(&mut *tx).await?;
                sqlx::query("UPDATE organization_tasks SET authorization_id=?,queue_since=COALESCE(queue_since,?) WHERE id=?")
                    .bind(&grant.id)
                    .bind(now)
                    .bind(&child.id)
                    .execute(&mut *tx)
                    .await?;
                set_task_state(&mut tx, &mut child, OrganizationTaskState::Queued).await?;
                event(&mut tx, "task/delegated", &child.id).await?;
                serde_json::to_value(child)?
            }
            OrganizationCoordinationCall::WriteNote { scope, body } => {
                ensure!(
                    scope != OrganizationNoteScope::Team,
                    "only the owner can promote team knowledge"
                );
                text(&body, 4096)?;
                let count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM organization_notes WHERE project_id=?",
                )
                .bind(&task.project_id)
                .fetch_one(&mut *tx)
                .await?;
                ensure!(count < 128, "project note limit exhausted");
                let note = OrganizationNote {
                    id: Uuid::now_v7().to_string(),
                    project_id: task.project_id.clone(),
                    author_id: task.owner_id.clone(),
                    task_id: task.id.clone(),
                    attempt_id: attempt.id.clone(),
                    scope,
                    body,
                    created_at: now,
                    group_id: None,
                    source_note_id: None,
                    owner_reviewed: false,
                };
                sqlx::query("INSERT INTO organization_notes(id,project_id,author_id,scope,data) VALUES (?,?,?,?,?)")
                    .bind(&note.id).bind(&note.project_id).bind(&note.author_id).bind(note_scope(scope))
                    .bind(serde_json::to_string(&note)?).execute(&mut *tx).await?;
                event(&mut tx, "note/written", &note.id).await?;
                serde_json::to_value(note)?
            }
            OrganizationCoordinationCall::ReadNotes { scope } => {
                // Filter before deserialization or model context assembly. Agent
                // notes are private to the agent within this authorized project.
                serde_json::to_value(
                    search_notes(&mut tx, &task.project_id, scope, &task.owner_id, "")
                        .await?
                        .notes,
                )?
            }
            OrganizationCoordinationCall::SearchNotes { scope, query } => serde_json::to_value(
                search_notes(&mut tx, &task.project_id, scope, &task.owner_id, &query).await?,
            )?,
            OrganizationCoordinationCall::Send { task_id, body } => {
                text(&body, 4096)?;
                let recipient = read_task(&mut tx, &task_id).await?;
                let recipient_root: Option<String> = sqlx::query_scalar("SELECT a.root_id FROM organization_tasks t JOIN organization_authorizations a ON a.id=t.authorization_id WHERE t.id=?")
                    .bind(&task_id).fetch_optional(&mut *tx).await?;
                ensure!(
                    recipient.project_id == task.project_id
                        && recipient_root.as_deref() == Some(root),
                    "message recipient outside authorized work"
                );
                let count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM organization_messages WHERE root_id=?",
                )
                .bind(root)
                .fetch_one(&mut *tx)
                .await?;
                ensure!(count < 256, "root message limit exhausted");
                let message = OrganizationMessage {
                    id: Uuid::now_v7().to_string(),
                    root_authorization_id: root.clone(),
                    project_id: task.project_id.clone(),
                    sender_id: task.owner_id.clone(),
                    sender_task_id: task.id.clone(),
                    recipient_task_id: task_id,
                    body,
                    created_at: now,
                    acknowledged: false,
                };
                sqlx::query("INSERT INTO organization_messages(id,root_id,recipient_task_id,data) VALUES (?,?,?,?)")
                    .bind(&message.id).bind(root).bind(&message.recipient_task_id).bind(serde_json::to_string(&message)?).execute(&mut *tx).await?;
                event(&mut tx, "message/sent", &message.id).await?;
                serde_json::to_value(message)?
            }
            OrganizationCoordinationCall::Inbox {} => {
                // A fresh owner grant after reconciliation retains the same task
                // inbox. Old messages remain untrusted content with their source
                // grant recorded; they never revive that grant's authority.
                let data: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_messages WHERE recipient_task_id=? AND json_extract(data,'$.projectId')=? AND acknowledged=0 ORDER BY id LIMIT 32")
                    .bind(&task.id).bind(&task.project_id).fetch_all(&mut *tx).await?;
                let messages = data
                    .into_iter()
                    .map(|data| serde_json::from_str::<OrganizationMessage>(&data))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                serde_json::to_value(messages)?
            }
            OrganizationCoordinationCall::Acknowledge { message_id } => {
                let data: String = sqlx::query_scalar("SELECT data FROM organization_messages WHERE id=? AND recipient_task_id=? AND json_extract(data,'$.projectId')=?")
                    .bind(&message_id).bind(&task.id).bind(&task.project_id).fetch_one(&mut *tx).await?;
                let mut message: OrganizationMessage = serde_json::from_str(&data)?;
                message.acknowledged = true;
                sqlx::query("UPDATE organization_messages SET acknowledged=1,data=? WHERE id=?")
                    .bind(serde_json::to_string(&message)?)
                    .bind(&message_id)
                    .execute(&mut *tx)
                    .await?;
                event(&mut tx, "message/acknowledged", &message_id).await?;
                serde_json::to_value(message)?
            }
        };
        let result_json = serde_json::to_string(&result)?;
        if read_only {
            let previous:Vec<(String,String,i64)>=sqlx::query_as("SELECT arguments,result,progress_generation FROM organization_coordination_calls WHERE attempt_id=? ORDER BY rowid DESC LIMIT 8").bind(attempt_id).fetch_all(&mut *tx).await?;
            ensure!(
                previous.len() < 8
                    || !previous
                        .iter()
                        .all(|(a, r, p)| a == &arguments && r == &result_json && *p == progress),
                RepeatedCoordination
            );
        }
        sqlx::query("INSERT INTO organization_coordination_calls(attempt_id,call_id,arguments,result,progress_generation) VALUES (?,?,?,?,?)")
            .bind(attempt_id).bind(call_id).bind(arguments).bind(result_json).bind(progress).execute(&mut *tx).await?;
        event(&mut tx, "coordination/completed", attempt_id).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Only direct children can be waited on. This produces an acyclic wait
    /// tree even when the organization itself has arbitrary roles and levels.
    pub async fn wait_for_children(
        &self,
        attempt_id: &str,
        worker: &str,
        task_ids: &[String],
        now: i64,
    ) -> Result<()> {
        ensure!(
            (1..=16).contains(&task_ids.len())
                && task_ids.iter().collect::<HashSet<_>>().len() == task_ids.len(),
            "invalid child wait list"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, _) = check_fence(&mut tx, attempt_id, worker, now).await?;
        for id in task_ids {
            let parent: Option<String> = sqlx::query_scalar("SELECT a.parent_attempt_id FROM organization_tasks t JOIN organization_authorizations a ON a.id=t.authorization_id WHERE t.id=?")
                .bind(id).fetch_one(&mut *tx).await?;
            ensure!(
                parent.as_deref() == Some(attempt_id),
                "wait target must be a direct delegated child"
            );
            sqlx::query("INSERT INTO organization_waits(attempt_id,task_id) VALUES (?,?)")
                .bind(attempt_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        attempt.state = OrganizationTaskState::WaitingDependency;
        // A streamed function can arrive before its provider usage receipt.
        // Retain capacity until that receipt is durably recorded.
        attempt.runtime_active = attempt.model_requests != attempt.reported_model_requests;
        write_attempt(&mut tx, &attempt).await?;
        let mut task = read_task(&mut tx, &attempt.task_id).await?;
        set_task_state(&mut tx, &mut task, OrganizationTaskState::WaitingDependency).await?;
        event(&mut tx, "attempt/waitingChildren", attempt_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Reacquire capacity transactionally before returning a tool response that
    /// permits further inference. No model session runs while the parent waits.
    pub async fn resume_after_children(
        &self,
        attempt_id: &str,
        worker: &str,
        now: i64,
        max_active: u32,
    ) -> Result<Option<Vec<OrganizationTaskResult>>> {
        ensure!(
            (1..=4).contains(&max_active),
            "active execution limit must be 1–4"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, grant) = check_live_fence(&mut tx, attempt_id, worker, now).await?;
        if let Err(error) = super::workflow::check_workflow_dispatch(&mut tx, &grant).await {
            if error.is::<super::WorkflowPaused>() {
                return Ok(None);
            }
            return Err(error);
        }
        ensure!(
            attempt.state == OrganizationTaskState::WaitingDependency,
            "attempt is not waiting on children"
        );
        let children: Vec<(String,String)> = sqlx::query_as("SELECT t.id,t.state FROM organization_waits w JOIN organization_tasks t ON t.id=w.task_id WHERE w.attempt_id=? ORDER BY t.id")
            .bind(attempt_id).fetch_all(&mut *tx).await?;
        ensure!(!children.is_empty(), "missing durable child wait");
        ensure!(
            children.iter().all(|(_, state)| !matches!(
                state.as_str(),
                "failed" | "cancelled" | "paused" | "needsReconciliation"
            )),
            "child cannot complete without owner intervention"
        );
        if children.iter().any(|(_, state)| state != "accepted")
            || attempt.model_requests != attempt.reported_model_requests
        {
            return Ok(None);
        }
        let active: i64 =
            sqlx::query_scalar("SELECT count(*) FROM organization_attempts WHERE occupied=1")
                .fetch_one(&mut *tx)
                .await?;
        if !attempt.runtime_active
            && (active >= i64::from(max_active)
                || !super::resource::shared_capacity_available(&mut tx, &grant).await?)
        {
            return Ok(None);
        }
        let mut results = Vec::new();
        for (id, _) in children {
            let data: String = sqlx::query_scalar("SELECT r.data FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id JOIN organization_tasks t ON t.id=p.task_id WHERE t.id=? AND p.authorization_id=t.authorization_id AND p.state='accepted' ORDER BY p.generation DESC LIMIT 1")
                .bind(id).fetch_one(&mut *tx).await?;
            let result: OrganizationTaskResult = serde_json::from_str(&data)?;
            ensure!(result.accepted, "child evidence is not accepted");
            results.push(result);
        }
        sqlx::query("DELETE FROM organization_waits WHERE attempt_id=?")
            .bind(attempt_id)
            .execute(&mut *tx)
            .await?;
        attempt.state = OrganizationTaskState::Running;
        attempt.runtime_active = true;
        write_attempt(&mut tx, &attempt).await?;
        let mut task = read_task(&mut tx, &attempt.task_id).await?;
        set_task_state(&mut tx, &mut task, OrganizationTaskState::Running).await?;
        event(&mut tx, "attempt/childrenCompleted", attempt_id).await?;
        tx.commit().await?;
        Ok(Some(results))
    }
}

pub(super) async fn promote_note(
    tx: &mut Transaction<'_, Sqlite>,
    source_id: &str,
    group_id: Option<String>,
    body: String,
) -> Result<()> {
    text(&body, 4096)?;
    ensure!(
        !matches!(
            super::controls::read(tx, &format!("knowledge:{source_id}"))
                .await?
                .map(|c| c.value),
            Some(
                codex_protocol::organization::OrganizationControlValue::Knowledge {
                    retired: true,
                    ..
                }
            )
        ),
        "source note retired; inspect its history before restoring or correcting it"
    );
    let (data, superseded): (String, Option<String>) =
        sqlx::query_as("SELECT data,superseded_by FROM organization_notes WHERE id=?")
            .bind(source_id)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(
        superseded.is_none(),
        "source note was corrected; inspect its latest version"
    );
    let source: OrganizationNote = serde_json::from_str(&data)?;
    if let Some(group_id) = &group_id {
        let node = read_node(tx, group_id).await?;
        ensure!(
            node.kind == OrganizationNodeKind::Group
                && node.enabled
                && ancestors_enabled(tx, group_id).await?,
            "knowledge team must be an enabled group"
        );
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_notes WHERE project_id=?")
            .bind(&source.project_id)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(count < 128, "project note/history limit exhausted");
    let scope = if group_id.is_some() {
        OrganizationNoteScope::Team
    } else {
        OrganizationNoteScope::Project
    };
    let correction = source.owner_reviewed && source.scope == scope && source.group_id == group_id;
    let note = OrganizationNote {
        id: Uuid::now_v7().to_string(),
        group_id,
        source_note_id: Some(source.id.clone()),
        owner_reviewed: true,
        scope,
        body,
        created_at: chrono::Utc::now().timestamp(),
        ..source
    };
    sqlx::query("INSERT INTO organization_notes(id,project_id,author_id,scope,group_id,data) VALUES (?,?,?,?,?,?)").bind(&note.id).bind(&note.project_id).bind(&note.author_id).bind(note_scope(scope)).bind(&note.group_id).bind(serde_json::to_string(&note)?).execute(&mut **tx).await?;
    if correction {
        sqlx::query(
            "UPDATE organization_notes SET superseded_by=? WHERE id=? AND superseded_by IS NULL",
        )
        .bind(&note.id)
        .bind(source_id)
        .execute(&mut **tx)
        .await?;
    }
    owner_event(tx, "knowledge/promoted", &note.id).await?;
    Ok(())
}

pub(super) async fn share_note(
    tx: &mut Transaction<'_, Sqlite>,
    source_id: &str,
    source_sha256: &str,
    target_project_id: &str,
    expected_project_revision: i64,
    body: String,
) -> Result<()> {
    use sha2::Digest;
    text(&body, 4096)?;
    let (data, superseded): (String, Option<String>) =
        sqlx::query_as("SELECT data,superseded_by FROM organization_notes WHERE id=?")
            .bind(source_id)
            .fetch_one(&mut **tx)
            .await?;
    let source: OrganizationNote = serde_json::from_str(&data)?;
    ensure!(
        source.owner_reviewed
            && source.scope == OrganizationNoteScope::Project
            && superseded.is_none(),
        "only current owner-reviewed project knowledge may cross projects; private/team notes require explicit promotion first"
    );
    ensure!(
        source.project_id != target_project_id
            && source_sha256 == format!("{:x}", sha2::Sha256::digest(source.body.as_bytes())),
        "source changed or target is the same project"
    );
    ensure!(
        !matches!(
            super::controls::read(tx, &format!("knowledge:{source_id}"))
                .await?
                .map(|c| c.value),
            Some(
                codex_protocol::organization::OrganizationControlValue::Knowledge {
                    retired: true,
                    ..
                }
            )
        ),
        "retired source cannot be shared"
    );
    let target = read_project(tx, target_project_id).await?;
    ensure!(
        target.revision == expected_project_revision,
        "target project changed"
    );
    super::controls::check_project_edit(tx, target_project_id).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_notes WHERE project_id=?")
            .bind(target_project_id)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(count < 128, "target project note/history limit exhausted");
    let note = OrganizationNote {
        id: Uuid::now_v7().to_string(),
        project_id: target_project_id.into(),
        body,
        source_note_id: Some(source.id.clone()),
        created_at: chrono::Utc::now().timestamp(),
        ..source
    };
    sqlx::query("INSERT INTO organization_notes(id,project_id,author_id,scope,group_id,data) VALUES (?,?,?,'project',NULL,?)").bind(&note.id).bind(&note.project_id).bind(&note.author_id).bind(serde_json::to_string(&note)?).execute(&mut **tx).await?;
    // This is an independently reviewed copy. Never supersede or expose the original private context.
    owner_event(tx, "knowledge/shared", &note.id).await?;
    Ok(())
}

pub(super) async fn team_ids(
    tx: &mut Transaction<'_, Sqlite>,
    author_id: &str,
) -> Result<Vec<String>> {
    let node = read_node(tx, author_id).await?;
    if !node.enabled || !ancestors_enabled(tx, author_id).await? {
        return Ok(vec![]);
    }
    let ids:Vec<String>=sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT ? UNION SELECT n.parent_id FROM organization_nodes n JOIN ancestors a ON n.id=a.id WHERE n.parent_id IS NOT NULL), affiliated(id) AS (SELECT id FROM ancestors UNION SELECT m.group_id FROM organization_memberships m JOIN ancestors a ON a.id=m.node_id) SELECT n.id FROM organization_nodes n JOIN affiliated a ON a.id=n.id WHERE n.kind='group' AND json_extract(n.data,'$.data.enabled')=1").bind(author_id).fetch_all(&mut **tx).await?;
    let mut enabled = vec![];
    for id in ids {
        if ancestors_enabled(tx, &id).await? {
            enabled.push(id);
        }
    }
    Ok(enabled)
}

async fn search_notes(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
    scope: OrganizationNoteScope,
    author_id: &str,
    query: &str,
) -> Result<OrganizationNoteSearchResult> {
    ensure!(
        query.len() <= 256 && !query.chars().any(char::is_control),
        "memory query must be at most 256 bytes without control characters"
    );
    let needle = query.trim().to_lowercase();
    // Scope before decoding/matching. ponytail: at most 128 notes/project;
    // use indexed full-text retrieval if that measured storage ceiling grows.
    let groups = if scope == OrganizationNoteScope::Team {
        team_ids(tx, author_id).await?
    } else {
        vec![]
    };
    let data: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_notes WHERE project_id=? AND scope=? AND superseded_by IS NULL AND NOT EXISTS(SELECT 1 FROM organization_controls c WHERE c.id='knowledge:'||organization_notes.id AND json_extract(c.data,'$.data.value.retired')=1) AND (?='project' OR (?='agent' AND author_id=?) OR (?='team' AND group_id IN (SELECT value FROM json_each(?)))) ORDER BY id DESC LIMIT 129")
        .bind(project_id).bind(note_scope(scope)).bind(note_scope(scope)).bind(note_scope(scope)).bind(author_id).bind(note_scope(scope)).bind(serde_json::to_string(&groups)?)
        .fetch_all(&mut **tx).await?;
    ensure!(data.len() <= 128, "memory storage ceiling exceeded");
    let mut notes = Vec::new();
    let mut matched_count = 0;
    for data in data {
        let note: OrganizationNote = serde_json::from_str(&data)?;
        if note.body.to_lowercase().contains(&needle) {
            matched_count += 1;
            if notes.len() < 16 {
                notes.push(note);
            }
        }
    }
    if scope == OrganizationNoteScope::Project {
        let shared = super::knowledge::library_search(tx, project_id, &needle).await?;
        matched_count += shared.matched_count;
        notes.extend(shared.notes);
        notes.truncate(16);
    }
    Ok(OrganizationNoteSearchResult {
        notes,
        matched_count,
    })
}

fn note_scope(scope: OrganizationNoteScope) -> &'static str {
    match scope {
        OrganizationNoteScope::Project => "project",
        OrganizationNoteScope::Agent => "agent",
        OrganizationNoteScope::Team => "team",
    }
}

async fn propose(
    tx: &mut Transaction<'_, Sqlite>,
    parent: &OrganizationTask,
    authorization: &OrganizationTaskAuthorization,
    items: Vec<OrganizationPlanItem>,
) -> Result<Vec<OrganizationTask>> {
    ensure!(
        (1..=32).contains(&items.len()),
        "plan must contain 1–32 tasks"
    );
    let root = authorization
        .root_authorization_id
        .as_ref()
        .unwrap_or(&authorization.id);
    let proposed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_tasks WHERE proposed_root_id=?")
            .bind(root)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(
        proposed + i64::try_from(items.len())? <= i64::from(authorization.scope.max_children),
        "root proposed-task ceiling exhausted"
    );
    let items = validate_plan_items(items)?;
    for item in &items {
        ensure!(
            authorization.scope.delegates.contains(&item.owner_id),
            "plan owner outside authorized roster"
        );
        ensure!(
            read_node(tx, &item.owner_id).await?.kind == OrganizationNodeKind::Agent,
            "plan owner is not an agent"
        );
    }
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    let mut tasks = Vec::new();
    for item in items {
        let key = item.key.clone();
        let task = OrganizationTask {
            id: Uuid::now_v7().to_string(),
            project_id: parent.project_id.clone(),
            goal_id: parent.goal_id.clone(),
            owner_id: item.owner_id,
            instructions: item.instructions,
            criteria: item.criteria,
            prerequisites: item
                .prerequisites
                .iter()
                .map(|key| ids[key].clone())
                .collect(),
            state: "proposed".into(),
            revision: 1,
        };
        sqlx::query("INSERT INTO organization_tasks(id,project_id,goal_id,owner_id,revision,state,proposed_root_id,data) VALUES (?,?,?,?,1,'proposed',?,?)")
            .bind(&task.id).bind(&task.project_id).bind(&task.goal_id).bind(&task.owner_id).bind(root)
            .bind(serde_json::to_string(&OrganizationRecord::Task(task.clone()))?).execute(&mut **tx).await?;
        for prerequisite in &task.prerequisites {
            sqlx::query(
                "INSERT INTO organization_task_dependencies(task_id,prerequisite_id) VALUES (?,?)",
            )
            .bind(&task.id)
            .bind(prerequisite)
            .execute(&mut **tx)
            .await?;
        }
        event(tx, "task/proposed", &task.id).await?;
        ids.insert(key, task.id.clone());
        tasks.push(task);
    }
    Ok(tasks)
}

async fn propose_team(
    tx: &mut Transaction<'_, Sqlite>,
    task: &OrganizationTask,
    authorization: &OrganizationTaskAuthorization,
    reason: String,
    drafts: Vec<OrganizationTeamNodeDraft>,
) -> Result<OrganizationControl> {
    ensure!(
        (1..=32).contains(&drafts.len()),
        "team draft supports 1–32 changes"
    );
    let mut bases = BTreeMap::new();
    let mut nodes = Vec::new();
    for draft in drafts {
        text(&draft.label, 256)?;
        if let Some(model) = &draft.model {
            text(model, 256)?;
        }
        ensure!(
            draft.kind == OrganizationNodeKind::Agent
                || (draft.role_version_id.is_none() && draft.model.is_none()),
            "groups have no execution role/model"
        );
        for id in draft.id.iter().chain(draft.parent_id.iter()) {
            if id != &task.owner_id && !authorization.scope.delegates.contains(id) {
                let ancestor:i64=sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT ? UNION SELECT n.parent_id FROM organization_nodes n JOIN ancestors a ON a.id=n.id WHERE n.parent_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=?)").bind(&task.owner_id).bind(id).fetch_one(&mut **tx).await?;
                ensure!(
                    ancestor == 1,
                    "team proposals may reference only caller, its ancestors or the original approved roster"
                );
            }
            bases.insert(id.clone(), read_node(tx, id).await?);
        }
        let (id, revision) = if let Some(id) = draft.id {
            let base = bases.get(&id).context("team base missing")?;
            ensure!(
                base.kind == draft.kind,
                "existing team node kind cannot change"
            );
            (id, base.revision)
        } else {
            (Uuid::now_v7().to_string(), 0)
        };
        nodes.push(OrganizationNode {
            id,
            revision,
            parent_id: draft.parent_id,
            kind: draft.kind,
            label: draft.label,
            role_version_id: draft.role_version_id,
            model: draft.model,
            enabled: draft.enabled,
        });
    }
    super::controls::set(
        tx,
        None,
        OrganizationControlValue::Team {
            id: Uuid::now_v7().to_string(),
            reason,
            expected_nodes: bases.into_values().collect(),
            nodes,
            applied: false,
        },
    )
    .await
}

/// Shared owner-template/agent-plan validation. Keys never refer to existing tasks.
pub(super) fn validate_plan_items(
    items: Vec<OrganizationPlanItem>,
) -> Result<Vec<OrganizationPlanItem>> {
    ensure!(
        (1..=32).contains(&items.len()),
        "plan must contain 1–32 tasks"
    );
    let mut remaining = BTreeMap::new();
    for item in items {
        text(&item.key, 128)?;
        text(&item.owner_id, 256)?;
        text(&item.instructions, 64 * 1024)?;
        validate_criteria(&item.criteria)?;
        ensure!(
            item.prerequisites.len() <= 32
                && item.prerequisites.iter().collect::<HashSet<_>>().len()
                    == item.prerequisites.len(),
            "invalid plan prerequisites"
        );
        ensure!(!remaining.contains_key(&item.key), "duplicate plan key");
        remaining.insert(item.key.clone(), item);
    }
    let mut ordered = Vec::new();
    let mut keys = HashSet::new();
    // ponytail: quadratic topological scan is bounded to 32 steps; use a queue only if that ceiling changes.
    while !remaining.is_empty() {
        let key = remaining
            .iter()
            .find(|(_, item)| item.prerequisites.iter().all(|key| keys.contains(key)))
            .map(|(key, _)| key.clone())
            .context("cyclic plan or unknown prerequisite key")?;
        ordered.push(remaining.remove(&key).context("missing plan item")?);
        keys.insert(key);
    }
    Ok(ordered)
}

/// Follow host-created parent grants, never arbitrary model-supplied IDs. The
/// root's elapsed time, lease, current role/policy and revoke fence every child.
pub(super) async fn check_parent_grants(
    tx: &mut Transaction<'_, Sqlite>,
    authorization: &OrganizationTaskAuthorization,
    now: i64,
) -> Result<()> {
    let mut parent = authorization.parent_attempt_id.clone();
    for _ in 0..=32 {
        let Some(id) = parent else { return Ok(()) };
        let (attempt_data, grant_data): (String,String) = sqlx::query_as("SELECT p.data,a.data FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id JOIN organization_tasks t ON t.id=p.task_id WHERE p.id=? AND p.lease_until>? AND p.state IN ('running','waitingApproval','waitingDependency') AND a.revoked=0 AND a.expires_at>? AND t.authorization_id=a.id AND t.state=p.state")
            .bind(id).bind(now).bind(now).fetch_one(&mut **tx).await.context("parent grant stopped, expired or changed")?;
        let attempt: OrganizationTaskAttempt = serde_json::from_str(&attempt_data)?;
        let grant: OrganizationTaskAuthorization = serde_json::from_str(&grant_data)?;
        ensure!(
            grant.policy_version == grant.scope.policy_version(),
            "parent host policy changed"
        );
        ensure!(
            now >= attempt.started_at
                && now - attempt.started_at < i64::from(grant.scope.max_seconds),
            "parent elapsed budget exhausted"
        );
        let task = read_task(tx, &attempt.task_id).await?;
        let node = read_node(tx, &task.owner_id).await?;
        let project = read_project(tx, &task.project_id).await?;
        ensure!(
            node.revision == grant.node_revision
                && project.revision == grant.project_revision
                && ancestors_enabled(tx, &node.id).await?,
            "parent role or scope changed"
        );
        parent = grant.parent_attempt_id;
    }
    anyhow::bail!("delegation ancestry exceeds root ceiling")
}
