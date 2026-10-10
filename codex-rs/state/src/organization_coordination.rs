//! Transactional, fenced agent coordination. No agent-provided grant fields.

use super::OrganizationStore;
use super::definitions::{text, validate_criteria};
use super::execution::*;
use anyhow::{Context, Result, ensure};
use codex_protocol::organization::*;
use codex_protocol::organization_coordination::*;
use codex_protocol::organization_execution::*;
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

impl OrganizationStore {
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
            "SELECT count(*) FROM organization_coordination_calls WHERE attempt_id=?",
        )
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
        let result = match call {
            OrganizationCoordinationCall::Wait { .. } => {
                anyhow::bail!("wait is handled by the host outside a database transaction")
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
                sqlx::query("UPDATE organization_tasks SET authorization_id=? WHERE id=?")
                    .bind(&grant.id)
                    .bind(&child.id)
                    .execute(&mut *tx)
                    .await?;
                set_task_state(&mut tx, &mut child, OrganizationTaskState::Queued).await?;
                event(&mut tx, "task/delegated", &child.id).await?;
                serde_json::to_value(child)?
            }
            OrganizationCoordinationCall::WriteNote { scope, body } => {
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
                let data: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_notes WHERE project_id=? AND scope=? AND (?='project' OR author_id=?) ORDER BY id DESC LIMIT 16")
                    .bind(&task.project_id).bind(note_scope(scope)).bind(note_scope(scope)).bind(&task.owner_id).fetch_all(&mut *tx).await?;
                let notes = data
                    .into_iter()
                    .map(|data| serde_json::from_str::<OrganizationNote>(&data))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                serde_json::to_value(notes)?
            }
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
        sqlx::query("INSERT INTO organization_coordination_calls(attempt_id,call_id,arguments,result) VALUES (?,?,?,?)")
            .bind(attempt_id).bind(call_id).bind(arguments).bind(serde_json::to_string(&result)?).execute(&mut *tx).await?;
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
        let (mut attempt, _) = check_live_fence(&mut tx, attempt_id, worker, now).await?;
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
        if !attempt.runtime_active && active >= i64::from(max_active) {
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

fn note_scope(scope: OrganizationNoteScope) -> &'static str {
    match scope {
        OrganizationNoteScope::Project => "project",
        OrganizationNoteScope::Agent => "agent",
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
    let mut remaining = BTreeMap::new();
    for item in items {
        text(&item.key, 128)?;
        text(&item.instructions, 64 * 1024)?;
        validate_criteria(&item.criteria)?;
        ensure!(
            item.prerequisites.len() <= 32
                && item.prerequisites.iter().collect::<HashSet<_>>().len()
                    == item.prerequisites.len(),
            "invalid plan prerequisites"
        );
        ensure!(
            authorization.scope.delegates.contains(&item.owner_id),
            "plan owner outside authorized roster"
        );
        let node = read_node(tx, &item.owner_id).await?;
        ensure!(
            node.kind == OrganizationNodeKind::Agent,
            "plan owner is not an agent"
        );
        ensure!(!remaining.contains_key(&item.key), "duplicate plan key");
        remaining.insert(item.key.clone(), item);
    }
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    let mut tasks = Vec::new();
    while !remaining.is_empty() {
        let key = remaining
            .iter()
            .find(|(_, item)| item.prerequisites.iter().all(|id| ids.contains_key(id)))
            .map(|(key, _)| key.clone())
            .context("cyclic plan or unknown prerequisite key")?;
        let item = remaining.remove(&key).context("missing plan item")?;
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
            grant.policy_version == ORGANIZATION_POLICY_VERSION,
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
