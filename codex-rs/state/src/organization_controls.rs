//! Owner controls retain immutable versions and project existing accounting.
use super::definitions::text;
use super::execution::*;
use super::resource::validate_shared_ceilings;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use codex_protocol::organization_execution::OrganizationTaskAuthorization;
use codex_protocol::organization_workflow::OrganizationWorkflowSeries;
use codex_protocol::organization_workflow::OrganizationWorkflowState;
use sha2::Digest;
use sqlx::Sqlite;
use sqlx::Transaction;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) async fn read(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Option<OrganizationControl>> {
    let data: Option<String> =
        sqlx::query_scalar("SELECT data FROM organization_controls WHERE id=?")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    data.map(|data| {
        let OrganizationRecord::Control(control) = serde_json::from_str(&data)? else {
            anyhow::bail!("invalid stored control");
        };
        Ok(control)
    })
    .transpose()
}

pub(super) async fn set(
    tx: &mut Transaction<'_, Sqlite>,
    expected_revision: Option<i64>,
    value: OrganizationControlValue,
) -> Result<OrganizationControl> {
    let id = match &value {
        OrganizationControlValue::RoleDraft { id, .. } => format!("role-draft:{id}"),
        OrganizationControlValue::GoalMetadata { goal, .. } => format!("goal-meta:{}", goal.id),
        OrganizationControlValue::Deadline {
            kind, target_id, ..
        } => format!("deadline:{}:{target_id}", super::goal::deadline_kind(*kind)),
        OrganizationControlValue::Conversation { .. } => {
            anyhow::bail!("conversation transcripts require the prompt or live reply API")
        }
        OrganizationControlValue::Reconciliation { id, .. } => format!("reconciliation:{id}"),
        OrganizationControlValue::ExecutionProfile { id, .. } => format!("profile:{id}"),
        OrganizationControlValue::ResourcePressure { .. } => "pressure".into(),
        OrganizationControlValue::RoleRollback { id, .. } => format!("rollback-role:{id}"),
        OrganizationControlValue::WorkflowRollback { id, .. } => format!("rollback-workflow:{id}"),
        OrganizationControlValue::Mission { .. } => "mission".into(),
        OrganizationControlValue::GoalReview { goal, .. } => format!("goal:{}", goal.id),
        OrganizationControlValue::Escalation { task_id, .. } => format!("escalation:{task_id}"),
        OrganizationControlValue::WorkflowImprovement { id, .. } => {
            format!("workflow-improvement:{id}")
        }
        OrganizationControlValue::Improvement { id, .. } => format!("improvement:{id}"),
        OrganizationControlValue::Team { id, .. } => format!("team:{id}"),
        OrganizationControlValue::SafeRecovery { task_id, .. } => format!("recovery:{task_id}"),
        OrganizationControlValue::Knowledge { note_id, .. } => format!("knowledge:{note_id}"),
        OrganizationControlValue::Replan { id, .. } => format!("replan:{id}"),
        OrganizationControlValue::Supervision { series_id, .. } => {
            format!("supervision:{series_id}")
        }
        OrganizationControlValue::Global { .. } => "global".into(),
        OrganizationControlValue::Project { project_id, .. } => format!("project:{project_id}"),
        OrganizationControlValue::Priority { task_id, .. } => format!("priority:{task_id}"),
    };
    let previous = read(tx, &id).await?;
    ensure!(
        previous.as_ref().map(|c| c.revision) == expected_revision,
        "control revision conflict"
    );
    match &value {
        OrganizationControlValue::RoleDraft {
            id,
            source_attempt_id,
            name,
            instructions,
            applied,
        } => {
            ensure!(
                *applied,
                "role drafts originate from a fenced agent call; owners create ordinary role versions separately"
            );
            Uuid::parse_str(id)?;
            text(name, 256)?;
            text(instructions, 64 * 1024)?;
            let mut pending = value.clone();
            if let OrganizationControlValue::RoleDraft { applied, .. } = &mut pending {
                *applied = false;
            }
            ensure!(
                previous.as_ref().is_some_and(|old| old.value == pending),
                "review the exact pending role draft; application is immutable"
            );
            let exists: i64 =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM organization_attempts WHERE id=?)")
                    .bind(source_attempt_id)
                    .fetch_one(&mut **tx)
                    .await?;
            ensure!(exists == 1, "role proposal source attempt missing");
            let role = OrganizationRoleVersion {
                id: id.clone(),
                role_id: id.clone(),
                name: name.clone(),
                instructions: instructions.clone(),
                version: 1,
            };
            sqlx::query(
                "INSERT INTO organization_role_versions(id,role_id,version,data) VALUES (?,?,1,?)",
            )
            .bind(id)
            .bind(id)
            .bind(serde_json::to_string(&OrganizationRecord::Role(role))?)
            .execute(&mut **tx)
            .await?;
        }
        OrganizationControlValue::GoalMetadata {
            goal,
            project_id,
            responsible_node_id,
            prerequisites,
        } => {
            super::goal::validate_metadata(
                tx,
                goal,
                project_id,
                responsible_node_id,
                prerequisites,
            )
            .await?;
        }
        OrganizationControlValue::Deadline {
            kind,
            target_id,
            target_revision,
            due_at,
            display_offset_minutes,
        } => {
            super::goal::validate_deadline(
                tx,
                *kind,
                target_id,
                *target_revision,
                *due_at,
                *display_offset_minutes,
            )
            .await?;
        }
        OrganizationControlValue::Conversation { .. } => {
            unreachable!("conversation content uses its scoped API")
        }
        OrganizationControlValue::ExecutionProfile {
            id,
            label,
            policy_version,
            scope,
        } => {
            Uuid::parse_str(id)?;
            text(label, 256)?;
            ensure!(
                *policy_version
                    == codex_protocol::organization_execution::ORGANIZATION_POLICY_VERSION,
                "profile policy version is unsupported"
            );
            super::execution::validate_scope(scope)?;
        }
        OrganizationControlValue::Reconciliation {
            id,
            attempt_id,
            request_sequence,
            fingerprint,
            evidence,
        } => {
            Uuid::parse_str(id)?;
            Uuid::parse_str(attempt_id)?;
            text(evidence, 16 * 1024)?;
            ensure!(
                previous.is_none(),
                "inspection records are immutable; capture new evidence separately"
            );
            let data: String =
                sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
                    .bind(attempt_id)
                    .fetch_one(&mut **tx)
                    .await?;
            let attempt: codex_protocol::organization_execution::OrganizationTaskAttempt =
                serde_json::from_str(&data)?;
            let data: Vec<String> = sqlx::query_scalar(
                "SELECT data FROM organization_actions WHERE attempt_id=? ORDER BY id LIMIT 100",
            )
            .bind(attempt_id)
            .fetch_all(&mut **tx)
            .await?;
            let actions: Vec<codex_protocol::organization_execution::OrganizationActionIntent> =
                data.into_iter()
                    .map(|d| serde_json::from_str(&d))
                    .collect::<std::result::Result<_, _>>()?;
            ensure!(*request_sequence==attempt.model_requests && (attempt.model_requests>attempt.reported_model_requests || actions.iter().any(|a|a.state==codex_protocol::organization_execution::OrganizationActionState::Uncertain)),"select the exact unresolved request/effects");
            ensure!(
                *fingerprint == super::actions::uncertainty_fingerprint(&attempt, &actions),
                "request/effect evidence changed; inspect the latest receipt"
            );
        }
        OrganizationControlValue::ResourcePressure { path, .. } => {
            ensure!(
                std::path::Path::new(path).is_absolute() && std::fs::canonicalize(path)?.is_dir(),
                "pressure path must be an existing absolute directory"
            );
        }
        OrganizationControlValue::RoleRollback {
            id,
            reason,
            current,
            source,
            applied,
        } => {
            check_rollback_proposal(&value, &previous, *applied)?;
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                current.role_id == source.role_id && source.version < current.version,
                "select an earlier version of the same role"
            );
            for role in [current, source] {
                let stored: String =
                    sqlx::query_scalar("SELECT data FROM organization_role_versions WHERE id=?")
                        .bind(&role.id)
                        .fetch_one(&mut **tx)
                        .await?;
                ensure!(
                    serde_json::from_str::<OrganizationRecord>(&stored)?
                        == OrganizationRecord::Role((**role).clone()),
                    "captured role changed"
                );
            }
            let latest: i64 = sqlx::query_scalar(
                "SELECT max(version) FROM organization_role_versions WHERE role_id=?",
            )
            .bind(&current.role_id)
            .fetch_one(&mut **tx)
            .await?;
            ensure!(
                latest == current.version,
                "latest role changed; recapture rollback"
            );
            if *applied {
                let replacement = OrganizationRoleVersion {
                    id: id.clone(),
                    role_id: current.role_id.clone(),
                    version: latest.checked_add(1).context("role version exhausted")?,
                    name: source.name.clone(),
                    instructions: source.instructions.clone(),
                };
                let data = serde_json::to_string(&OrganizationRecord::Role(replacement.clone()))?;
                sqlx::query("INSERT INTO organization_role_versions(id,role_id,version,data) VALUES (?,?,?,?)").bind(id).bind(&replacement.role_id).bind(replacement.version).bind(data).execute(&mut **tx).await?;
            }
        }
        OrganizationControlValue::WorkflowRollback {
            id,
            reason,
            current,
            source,
            source_run_id,
            source_run_revision,
            applied,
        } => {
            check_rollback_proposal(&value, &previous, *applied)?;
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                source.id == current.id
                    && source.project_id == current.project_id
                    && source.revision < current.revision,
                "select an earlier captured version of the same workflow"
            );
            let data: String = sqlx::query_scalar(
                "SELECT data FROM organization_workflow_templates WHERE id=? AND revision=?",
            )
            .bind(&current.id)
            .bind(current.revision)
            .fetch_one(&mut **tx)
            .await?;
            ensure!(
                serde_json::from_str::<
                    codex_protocol::organization_workflow::OrganizationWorkflowTemplate,
                >(&data)?
                    == **current,
                "current workflow changed"
            );
            let run = comparison_run(tx, source_run_id, *source_run_revision).await?;
            ensure!(
                run.preview.template == **source,
                "rollback source differs from captured run"
            );
            if *applied {
                let mut replacement = (**source).clone();
                replacement.revision = current
                    .revision
                    .checked_add(1)
                    .context("workflow revision exhausted")?;
                sqlx::query("UPDATE organization_workflow_templates SET revision=?,data=? WHERE id=? AND revision=?").bind(replacement.revision).bind(serde_json::to_string(&replacement)?).bind(&current.id).bind(current.revision).execute(&mut **tx).await?;
            }
        }
        OrganizationControlValue::Mission {
            purpose,
            priorities,
            constraints,
        } => {
            text(purpose, 4096)?;
            super::definitions::validate_criteria(priorities)?;
            super::definitions::validate_criteria(constraints)?;
            ensure!(
                purpose.len()
                    + priorities.iter().map(String::len).sum::<usize>()
                    + constraints.iter().map(String::len).sum::<usize>()
                    <= 8192,
                "mission context exceeds 8 KiB"
            );
        }
        OrganizationControlValue::GoalReview {
            goal,
            completed,
            evidence,
        } => {
            check_goal(tx, goal).await?;
            let mut covered = BTreeSet::new();
            for proof in evidence {
                ensure!(
                    (proof.criterion as usize) < goal.criteria.len()
                        && covered.insert(proof.criterion),
                    "invalid or duplicate goal criterion"
                );
                text(&proof.explanation, 4096)?;
                let (task, _, _, result) =
                    comparison_evidence(tx, &proof.task_id, &proof.result_digest).await?;
                let project = read_project(tx, &task.project_id).await?;
                ensure!(
                    super::goal::descendant(
                        tx,
                        task.goal_id.as_deref().or(project.goal_id.as_deref()),
                        &goal.id
                    )
                    .await?,
                    "goal evidence belongs to another goal tree"
                );
                ensure!(
                    !result.checks.is_empty()
                        && result.checks.iter().all(|c| c.exit_code == Some(0)),
                    "goal evidence needs actual passing checks"
                );
            }
            if *completed {
                ensure!(
                    !goal.criteria.is_empty() && covered.len() == goal.criteria.len(),
                    "every goal criterion needs exact reviewed evidence"
                );
                let unfinished:i64=sqlx::query_scalar("WITH RECURSIVE descendants(id) AS (SELECT ? UNION SELECT g.id FROM organization_goals g JOIN descendants d ON g.parent_id=d.id) SELECT count(*) FROM organization_tasks t JOIN organization_projects p ON p.id=t.project_id WHERE COALESCE(json_extract(t.data,'$.data.goalId'),json_extract(p.data,'$.data.goalId')) IN descendants AND t.state NOT IN ('accepted','cancelled')").bind(&goal.id).fetch_one(&mut **tx).await?;
                ensure!(
                    unfinished == 0,
                    "goal has unfinished work; resolve or cancel it first"
                );
            }
        }
        OrganizationControlValue::Escalation {
            task_id,
            task_revision,
            assignee_id,
            reason,
            requested_action,
            resolved,
        } => {
            text(reason, 4096)?;
            text(requested_action, 4096)?;
            if *resolved {
                let mut pending = value.clone();
                if let OrganizationControlValue::Escalation { resolved, .. } = &mut pending {
                    *resolved = false;
                }
                ensure!(
                    previous.as_ref().is_some_and(|old| old.value == pending),
                    "resolve the exact existing assistance request"
                );
            } else {
                let task = read_task(tx, task_id).await?;
                ensure!(task.revision == *task_revision, "escalation task changed");
                ensure!(
                    matches!(
                        task.state.as_str(),
                        "blocked" | "paused" | "waitingApproval" | "needsReconciliation" | "failed"
                    ),
                    "select work needing assistance"
                );
                if let Some(id) = assignee_id {
                    let node = read_node(tx, id).await?;
                    ensure!(
                        node.enabled && ancestors_enabled(tx, id).await?,
                        "responsible definition is disabled"
                    );
                }
            }
            ensure!(
                !previous.as_ref().is_some_and(|old| matches!(
                    old.value,
                    OrganizationControlValue::Escalation { resolved: true, .. }
                )),
                "resolved assistance requests are immutable"
            );
        }
        OrganizationControlValue::WorkflowImprovement {
            id,
            reason,
            template_id,
            template_revision,
            baseline_run_id,
            baseline_run_revision,
            candidate_run_id,
            candidate_run_revision,
            applied,
        } => {
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                baseline_run_id != candidate_run_id,
                "compare distinct workflow runs"
            );
            if let Some(old) = &previous {
                ensure!(
                    !matches!(
                        old.value,
                        OrganizationControlValue::WorkflowImprovement { applied: true, .. }
                    ),
                    "adopted workflow comparison is immutable"
                );
            }
            if *applied {
                let mut pending = value.clone();
                if let OrganizationControlValue::WorkflowImprovement { applied, .. } = &mut pending
                {
                    *applied = false;
                }
                ensure!(
                    previous.as_ref().is_some_and(|old| old.value == pending),
                    "save and review exact workflow comparison first"
                );
            }
            let before = comparison_run(tx, baseline_run_id, *baseline_run_revision).await?;
            let after = comparison_run(tx, candidate_run_id, *candidate_run_revision).await?;
            let current: String = sqlx::query_scalar(
                "SELECT data FROM organization_workflow_templates WHERE id=? AND revision=?",
            )
            .bind(template_id)
            .bind(template_revision)
            .fetch_one(&mut **tx)
            .await?;
            let template: codex_protocol::organization_workflow::OrganizationWorkflowTemplate =
                serde_json::from_str(&current)?;
            ensure!(
                after.preview.template == template
                    && before.preview.template != template
                    && before.preview.template.project_id == template.project_id
                    && before.task_ids.keys().eq(after.task_ids.keys()),
                "candidate must evaluate the current changed workflow with the same project and step keys"
            );
            for (key, task_id) in &before.task_ids {
                let old = comparison_evidence_for_task(tx, task_id).await?;
                let new = comparison_evidence_for_task(tx, &after.task_ids[key]).await?;
                ensure!(
                    old.1.scope.model == new.1.scope.model
                        && old.2.provider.is_some()
                        && old.2.provider == new.2.provider
                        && old.2.role_version_id == new.2.role_version_id,
                    "workflow evaluations changed model/provider/instructions or lack captured provenance"
                );
                compare_checks(&old.3, &new.3)?;
            }
            // Durable recommendation of this exact tested template; execution still requires fresh owner-reviewed scopes.
        }
        OrganizationControlValue::Improvement {
            id,
            reason,
            node,
            role_version_id,
            baseline_task_id,
            baseline_digest,
            candidate_task_id,
            candidate_digest,
            applied,
        } => {
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                baseline_task_id != candidate_task_id
                    && node.role_version_id.as_ref() != Some(role_version_id),
                "compare distinct evaluations and a fresh instruction version"
            );
            if let Some(old) = &previous {
                ensure!(
                    !matches!(
                        old.value,
                        OrganizationControlValue::Improvement { applied: true, .. }
                    ),
                    "adopted improvement is immutable"
                );
            }
            if *applied {
                let mut pending = value.clone();
                if let OrganizationControlValue::Improvement { applied, .. } = &mut pending {
                    *applied = false;
                }
                ensure!(
                    previous.as_ref().is_some_and(|old| old.value == pending),
                    "save and review exact measured improvement first"
                );
            }
            ensure!(
                read_node(tx, &node.id).await? == **node,
                "improvement target changed"
            );
            let baseline = comparison_evidence(tx, baseline_task_id, baseline_digest).await?;
            let candidate = comparison_evidence(tx, candidate_task_id, candidate_digest).await?;
            ensure!(
                baseline.0.project_id == candidate.0.project_id
                    && baseline.1.scope.model == candidate.1.scope.model
                    && baseline.2.provider.is_some()
                    && baseline.2.provider == candidate.2.provider,
                "evaluation project/model/provider differ or provenance is unavailable"
            );
            ensure!(
                baseline.2.role_version_id == node.role_version_id
                    && candidate.2.role_version_id.as_ref() == Some(role_version_id),
                "evaluate the actual captured baseline and candidate instructions"
            );
            compare_checks(&baseline.3, &candidate.3)?;
            if *applied {
                let mut updated = (**node).clone();
                updated.role_version_id = Some(role_version_id.clone());
                updated.revision = updated
                    .revision
                    .checked_add(1)
                    .context("node revision exhausted")?;
                super::definitions::write_node(tx, &updated).await?;
            }
        }
        OrganizationControlValue::Team {
            id,
            reason,
            expected_nodes,
            nodes,
            applied,
        } => {
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                (1..=32).contains(&nodes.len()) && expected_nodes.len() <= 64,
                "team proposal supports 1–32 changes and 64 captured definitions"
            );
            if let Some(old) = &previous {
                ensure!(
                    !matches!(
                        old.value,
                        OrganizationControlValue::Team { applied: true, .. }
                    ),
                    "applied team proposal is immutable"
                );
            }
            if *applied {
                let mut pending = value.clone();
                if let OrganizationControlValue::Team { applied, .. } = &mut pending {
                    *applied = false;
                }
                ensure!(
                    previous.as_ref().is_some_and(|old| old.value == pending),
                    "save and review the exact pending team proposal first"
                );
            }
            let mut bases = std::collections::BTreeMap::new();
            for node in expected_nodes {
                ensure!(
                    bases.insert(&node.id, node).is_none()
                        && read_node(tx, &node.id).await? == *node,
                    "team captured definition changed or repeated"
                );
            }
            let mut ids = BTreeSet::new();
            for node in nodes {
                ensure!(ids.insert(&node.id), "duplicate team target");
                if node.revision == 0 {
                    Uuid::parse_str(&node.id)?;
                    let exists: i64 = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM (SELECT id FROM organization_nodes UNION ALL SELECT id FROM organization_goals UNION ALL SELECT id FROM organization_projects UNION ALL SELECT id FROM organization_tasks UNION ALL SELECT id FROM organization_role_versions) WHERE id=?)",
                    )
                    .bind(&node.id)
                    .fetch_one(&mut **tx)
                    .await?;
                    ensure!(exists == 0, "fresh team identity already exists");
                } else {
                    ensure!(
                        bases
                            .get(&node.id)
                            .is_some_and(|base| base.revision == node.revision),
                        "capture each existing team target revision"
                    );
                }
            }
            for node in nodes {
                if let Some(parent) = &node.parent_id {
                    ensure!(
                        ids.contains(parent) || bases.contains_key(parent),
                        "capture the existing parent or include its fresh definition"
                    );
                }
            }
            if *applied {
                // Two passes permit arbitrary reparenting without exposing intermediate trees.
                for node in nodes {
                    let mut detached = node.clone();
                    detached.parent_id = None;
                    detached.revision = detached
                        .revision
                        .checked_add(1)
                        .context("node revision exhausted")?;
                    super::definitions::write_node(tx, &detached).await?;
                }
                for node in nodes {
                    let mut updated = node.clone();
                    updated.revision = updated
                        .revision
                        .checked_add(1)
                        .context("node revision exhausted")?;
                    super::definitions::write_node(tx, &updated).await?;
                }
            }
        }
        OrganizationControlValue::SafeRecovery {
            task_id,
            authorization_id,
            max_retries,
            ..
        } => {
            ensure!(
                (1..=2).contains(max_retries),
                "safe recovery supports at most two original-grant retries"
            );
            let data:String=sqlx::query_scalar("SELECT a.data FROM organization_authorizations a JOIN organization_tasks t ON t.authorization_id=a.id WHERE t.id=? AND a.id=? AND a.revoked=0").bind(task_id).bind(authorization_id).fetch_one(&mut **tx).await?;
            let grant: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
            ensure!(
                grant.parent_attempt_id.is_none() && grant.scope.max_attempts > 1,
                "safe recovery requires a root grant with original retry capacity"
            );
            if let Some(old) = &previous {
                ensure!(
                    matches!(&old.value,OrganizationControlValue::SafeRecovery{authorization_id:old_id,max_retries:old_max,..} if old_id==authorization_id && old_max==max_retries),
                    "recovery grant and ceiling are immutable; disable without resetting attempts"
                );
            }
        }
        OrganizationControlValue::Knowledge {
            note_id,
            body_sha256,
            reason,
            ..
        } => {
            text(reason, 4096)?;
            let data: String = sqlx::query_scalar("SELECT data FROM organization_notes WHERE id=?")
                .bind(note_id)
                .fetch_one(&mut **tx)
                .await?;
            let note: codex_protocol::organization_coordination::OrganizationNote =
                serde_json::from_str(&data)?;
            ensure!(
                *body_sha256 == format!("{:x}", sha2::Sha256::digest(note.body.as_bytes())),
                "knowledge source changed; inspect its exact body before review"
            );
        }
        OrganizationControlValue::Replan {
            id,
            project_id,
            reason,
            items,
            applied_handoff_ids,
        } => {
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                (1..=32).contains(&items.len()),
                "replans contain 1–32 checkpointed tasks"
            );
            let mut ids = BTreeSet::new();
            if let Some(old) = &previous {
                let OrganizationControlValue::Replan {
                    project_id: old_project,
                    applied_handoff_ids: old_handoffs,
                    ..
                } = &old.value
                else {
                    anyhow::bail!("invalid stored replan")
                };
                ensure!(
                    old_project == project_id && old_handoffs.is_empty(),
                    "applied replans are immutable"
                );
            }
            if applied_handoff_ids.is_empty() {
                check_project_edit(tx, project_id).await?;
                for item in items {
                    ensure!(ids.insert(&item.task_id), "duplicate replan task");
                    text(&item.instructions, 4096)?;
                    let task = read_task(tx, &item.task_id).await?;
                    ensure!(
                        task.project_id == *project_id
                            && task.revision == item.expected_revision
                            && task.state == "blocked",
                        "replan requires exact checkpointed blocked work in this project"
                    );
                    let latest:Option<String>=sqlx::query_scalar("SELECT id FROM organization_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1")
                        .bind(&item.task_id).fetch_optional(&mut **tx).await?;
                    ensure!(
                        latest.as_ref() == Some(&item.checkpoint_id),
                        "replan checkpoint changed"
                    );
                    ensure!(
                        read_node(tx, &item.owner_id).await?.revision
                            == item.expected_owner_revision,
                        "replan receiver changed"
                    );
                }
            } else {
                let old = previous
                    .as_ref()
                    .context("save the pending proposal before applying a replan")?;
                let mut pending = value.clone();
                if let OrganizationControlValue::Replan {
                    applied_handoff_ids,
                    ..
                } = &mut pending
                {
                    applied_handoff_ids.clear();
                }
                ensure!(
                    old.value == pending && applied_handoff_ids.len() == items.len(),
                    "applied replan must preserve the exact pending proposal"
                );
                for (item, handoff_id) in items.iter().zip(applied_handoff_ids) {
                    ensure!(ids.insert(handoff_id), "duplicate replan handoff receipt");
                    let data:String=sqlx::query_scalar("SELECT data FROM organization_handoffs WHERE id=? AND task_id=? AND checkpoint_id=?")
                        .bind(handoff_id).bind(&item.task_id).bind(&item.checkpoint_id).fetch_one(&mut **tx).await?;
                    let handoff: codex_protocol::organization_execution::OrganizationTaskHandoff =
                        serde_json::from_str(&data)?;
                    ensure!(
                        handoff.to_owner_id == item.owner_id
                            && handoff.to_node_revision == item.expected_owner_revision
                            && handoff.instructions == item.instructions,
                        "replan receipt does not match its owner-reviewed delta"
                    );
                }
            }
        }
        OrganizationControlValue::Supervision {
            series_id,
            goal,
            work_type,
            state,
        } => {
            text(work_type, 256)?;
            let data: String =
                sqlx::query_scalar("SELECT data FROM organization_workflow_series WHERE id=?")
                    .bind(series_id)
                    .fetch_one(&mut **tx)
                    .await?;
            let series: OrganizationWorkflowSeries = serde_json::from_str(&data)?;
            let expected_state = match series.state {
                codex_protocol::organization_schedule::OrganizationScheduleState::Active => {
                    OrganizationWorkflowState::Active
                }
                codex_protocol::organization_schedule::OrganizationScheduleState::Paused => {
                    OrganizationWorkflowState::Paused
                }
                codex_protocol::organization_schedule::OrganizationScheduleState::Cancelled => {
                    OrganizationWorkflowState::Cancelled
                }
                _ => anyhow::bail!(
                    "control the blocked/completed finite series before changing its supervision policy"
                ),
            };
            ensure!(
                *state == expected_state,
                "use the native supervision control to change execution state"
            );
            if let Some(old) = previous.as_ref() {
                let OrganizationControlValue::Supervision {
                    goal: old_goal,
                    work_type: old_type,
                    state: old_state,
                    ..
                } = &old.value
                else {
                    anyhow::bail!("invalid stored supervision control");
                };
                ensure!(
                    old_goal == goal
                        && old_type == work_type
                        && *old_state != OrganizationWorkflowState::Cancelled,
                    "policy content is immutable; cancel and review a fresh finite policy without resetting prior usage"
                );
            } else {
                ensure!(
                    *state == OrganizationWorkflowState::Active,
                    "new supervision must bind an active finite series"
                );
            }
            if *state == OrganizationWorkflowState::Active {
                check_goal(tx, goal).await?;
                ensure!(
                    read_project(tx, &series.preview.template.project_id)
                        .await?
                        .goal_id
                        .as_ref()
                        == Some(&goal.id),
                    "supervision goal must be linked to the captured project"
                );
            }
        }
        OrganizationControlValue::Global { limits, .. } => {
            if let Some(limits) = limits {
                validate_shared_ceilings(
                    limits.max_tokens,
                    limits.max_model_requests,
                    limits.max_active,
                )?;
                let active: i64 =
                    sqlx::query_scalar("SELECT active FROM organization_global_usage")
                        .fetch_one(&mut **tx)
                        .await?;
                ensure!(
                    active <= i64::from(limits.max_active),
                    "pause active work before lowering global concurrency below usage"
                );
            }
        }
        OrganizationControlValue::Priority {
            task_id,
            task_revision,
            priority,
        } => {
            ensure!(
                (-10..=10).contains(priority),
                "priority must be between -10 and 10"
            );
            let task = read_task(tx, task_id).await?;
            ensure!(
                task.revision == *task_revision,
                "priority task revision conflict"
            );
            ensure!(
                !matches!(task.state.as_str(), "accepted" | "cancelled"),
                "finished task priority is immutable"
            );
        }
        OrganizationControlValue::Project {
            project_id,
            project_revision,
            state,
            evidence,
        } => {
            let project = read_project(tx, project_id).await?;
            ensure!(
                project.revision == *project_revision,
                "project definition revision conflict"
            );
            let old = match previous.as_ref().map(|c| &c.value) {
                Some(OrganizationControlValue::Project { state, .. }) => *state,
                None => OrganizationProjectState::Active,
                _ => anyhow::bail!("invalid stored project control"),
            };
            ensure!(old != *state, "project is already in the requested state");
            ensure!(
                matches!(
                    (old, state),
                    (_, OrganizationProjectState::Active)
                        | (
                            OrganizationProjectState::Active,
                            OrganizationProjectState::Paused
                        )
                        | (
                            OrganizationProjectState::Active | OrganizationProjectState::Paused,
                            OrganizationProjectState::Completed
                        )
                        | (
                            OrganizationProjectState::Paused | OrganizationProjectState::Completed,
                            OrganizationProjectState::Archived
                        )
                ),
                "pause or complete before archiving; reopen before other changes"
            );
            if *state == OrganizationProjectState::Completed {
                ensure!(
                    !project.criteria.is_empty() && evidence.len() == project.criteria.len(),
                    "completion requires evidence for every project criterion"
                );
                let unfinished: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_tasks WHERE project_id=? AND state NOT IN ('accepted','cancelled')")
                    .bind(project_id).fetch_one(&mut **tx).await?;
                ensure!(
                    unfinished == 0,
                    "project has unfinished work; review or cancel it first"
                );
                let mut criteria = BTreeSet::new();
                for proof in evidence {
                    ensure!(
                        (proof.criterion as usize) < project.criteria.len()
                            && criteria.insert(proof.criterion),
                        "invalid or duplicate completion criterion"
                    );
                    text(&proof.explanation, 4096)?;
                    let accepted: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_tasks t JOIN organization_attempts a ON a.task_id=t.id AND a.authorization_id=t.authorization_id JOIN organization_results r ON r.attempt_id=a.id WHERE t.id=? AND t.project_id=? AND t.state='accepted' AND a.state='accepted' AND json_extract(r.data,'$.accepted')=1 AND r.digest=?")
                        .bind(&proof.task_id).bind(project_id).bind(&proof.result_digest).fetch_one(&mut **tx).await?;
                    ensure!(
                        accepted == 1,
                        "completion evidence must be an exact current accepted result in this project"
                    );
                }
            } else {
                ensure!(
                    evidence.is_empty(),
                    "completion evidence is only supplied when completing; prior evidence remains in history"
                );
            }
        }
    }
    persist(tx, previous, id, value).await
}

pub(super) async fn persist(
    tx: &mut Transaction<'_, Sqlite>,
    previous: Option<OrganizationControl>,
    id: String,
    value: OrganizationControlValue,
) -> Result<OrganizationControl> {
    ensure!(
        serde_json::to_vec(&value)?.len() <= 128 * 1024,
        "control content exceeds 128 KiB"
    );
    let control = OrganizationControl {
        id,
        revision: previous
            .map_or(0, |c| c.revision)
            .checked_add(1)
            .context("control revision exhausted")?,
        value,
    };
    sqlx::query("INSERT INTO organization_controls(id,revision,data) VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,data=excluded.data")
        .bind(&control.id).bind(control.revision).bind(serde_json::to_string(&OrganizationRecord::Control(control.clone()))?)
        .execute(&mut **tx).await?;
    let history = OrganizationControlHistory {
        id: Uuid::now_v7().to_string(),
        control: control.clone(),
    };
    sqlx::query(
        "INSERT INTO organization_control_history(id,control_id,revision,data) VALUES (?,?,?,?)",
    )
    .bind(&history.id)
    .bind(&control.id)
    .bind(control.revision)
    .bind(serde_json::to_string(&OrganizationRecord::ControlHistory(
        history,
    ))?)
    .execute(&mut **tx)
    .await?;
    Ok(control)
}

fn check_rollback_proposal(
    value: &OrganizationControlValue,
    previous: &Option<OrganizationControl>,
    applied: bool,
) -> Result<()> {
    ensure!(
        !previous.as_ref().is_some_and(|c| matches!(
            c.value,
            OrganizationControlValue::RoleRollback { applied: true, .. }
                | OrganizationControlValue::WorkflowRollback { applied: true, .. }
        )),
        "applied rollback is immutable"
    );
    if applied {
        let mut pending = value.clone();
        match &mut pending {
            OrganizationControlValue::RoleRollback { applied, .. }
            | OrganizationControlValue::WorkflowRollback { applied, .. } => *applied = false,
            _ => unreachable!(),
        };
        ensure!(
            previous.as_ref().is_some_and(|c| c.value == pending),
            "save and review the exact rollback proposal first"
        );
    }
    Ok(())
}

pub(super) async fn check_project_edit(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
) -> Result<()> {
    if let Some(control) = read(tx, &format!("project:{project_id}")).await? {
        ensure!(
            matches!(
                control.value,
                OrganizationControlValue::Project {
                    state: OrganizationProjectState::Active | OrganizationProjectState::Paused,
                    ..
                }
            ),
            "reopen the project before changing definitions"
        );
    }
    Ok(())
}

pub(super) async fn check_goal(
    tx: &mut Transaction<'_, Sqlite>,
    goal: &OrganizationGoal,
) -> Result<()> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_goals WHERE id=?")
        .bind(&goal.id)
        .fetch_one(&mut **tx)
        .await?;
    ensure!(
        serde_json::from_str::<OrganizationRecord>(&data)?
            == OrganizationRecord::Goal(goal.clone()),
        "supervision goal changed; review a new finite policy"
    );
    Ok(())
}

pub(super) async fn check_supervision(
    tx: &mut Transaction<'_, Sqlite>,
    series_id: &str,
) -> Result<()> {
    if let Some(policy) = read(tx, &format!("supervision:{series_id}")).await? {
        let OrganizationControlValue::Supervision { goal, state, .. } = policy.value else {
            anyhow::bail!("invalid stored supervision policy");
        };
        ensure!(
            state == OrganizationWorkflowState::Active,
            "supervision policy is paused or revoked"
        );
        check_goal(tx, &goal).await?;
    }
    Ok(())
}

pub(super) async fn check_dispatch(
    tx: &mut Transaction<'_, Sqlite>,
    grant: &OrganizationTaskAuthorization,
) -> Result<()> {
    ensure!(
        super::resource::pressure_ready(tx).await?,
        super::WorkflowPaused
    );
    if let Some(control) = read(tx, "global").await? {
        ensure!(
            !matches!(
                control.value,
                OrganizationControlValue::Global { paused: true, .. }
            ),
            super::WorkflowPaused
        );
    }
    let project_id: String =
        sqlx::query_scalar("SELECT project_id FROM organization_tasks WHERE id=?")
            .bind(&grant.task_id)
            .fetch_one(&mut **tx)
            .await?;
    if let Some(control) = read(tx, &format!("project:{project_id}")).await? {
        ensure!(
            matches!(
                control.value,
                OrganizationControlValue::Project {
                    state: OrganizationProjectState::Active,
                    ..
                }
            ),
            super::WorkflowPaused
        );
    }
    let task = read_task(tx, &grant.task_id).await?;
    let project = read_project(tx, &project_id).await?;
    check_goal_open(tx, &task, &project).await?;
    let policy_data: Option<String> = sqlx::query_scalar("SELECT c.data FROM organization_controls c JOIN organization_workflow_occurrences o ON c.id='supervision:'||o.series_id JOIN organization_workflow_tasks w ON w.run_id=o.run_id WHERE w.authorization_id=? LIMIT 1")
        .bind(grant.root_authorization_id.as_ref().unwrap_or(&grant.id)).fetch_optional(&mut **tx).await?;
    if let Some(data) = policy_data {
        let OrganizationRecord::Control(OrganizationControl {
            value: OrganizationControlValue::Supervision { goal, state, .. },
            ..
        }) = serde_json::from_str(&data)?
        else {
            anyhow::bail!("invalid stored supervision policy");
        };
        ensure!(
            state == OrganizationWorkflowState::Active && check_goal(tx, &goal).await.is_ok(),
            super::WorkflowPaused
        );
    }
    Ok(())
}

/// Completion closes new authorizations as well as dispatch. Existing ledgers remain.
pub(super) async fn check_goal_open(
    tx: &mut Transaction<'_, Sqlite>,
    task: &OrganizationTask,
    project: &OrganizationProjectPlan,
) -> Result<()> {
    let mut goal_id = task.goal_id.clone().or(project.goal_id.clone());
    let mut visited = BTreeSet::new();
    while let Some(id) = goal_id {
        ensure!(visited.insert(id.clone()), "invalid goal cycle");
        let data: String = sqlx::query_scalar("SELECT data FROM organization_goals WHERE id=?")
            .bind(&id)
            .fetch_one(&mut **tx)
            .await?;
        let OrganizationRecord::Goal(current) = serde_json::from_str(&data)? else {
            anyhow::bail!("invalid goal record")
        };
        super::goal::check_prerequisites(tx, &current).await?;
        if let Some(control) = read(tx, &format!("goal:{id}")).await? {
            ensure!(
                !matches!(control.value,OrganizationControlValue::GoalReview{goal,completed:true,..} if *goal==current),
                super::WorkflowPaused
            );
        }
        goal_id = current.parent_id;
    }
    Ok(())
}

/// Unknown terminal usage stays charged and fences the configured global pool.
pub(super) async fn check_global(
    tx: &mut Transaction<'_, Sqlite>,
    inference: bool,
    capacity: bool,
) -> Result<bool> {
    if (inference || capacity) && !super::resource::pressure_ready(tx).await? {
        return Ok(false);
    }
    let Some(control) = read(tx, "global").await? else {
        return Ok(true);
    };
    let OrganizationControlValue::Global { paused, limits } = control.value else {
        anyhow::bail!("invalid stored global control");
    };
    if paused && capacity {
        return Ok(false);
    }
    let Some(limits) = limits else {
        return Ok(true);
    };
    let (tokens, requests, active, unresolved): (i64, i64, i64, i64) = sqlx::query_as("SELECT reported_tokens,model_requests,active,unresolved_requests FROM organization_global_usage")
        .fetch_one(&mut **tx).await?;
    Ok(unresolved == 0
        && tokens < limits.max_tokens
        && (!inference || requests < i64::from(limits.max_model_requests))
        && (!capacity || active < i64::from(limits.max_active)))
}

pub(super) async fn comparison_evidence(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    digest: &str,
) -> Result<(
    OrganizationTask,
    OrganizationTaskAuthorization,
    codex_protocol::organization_execution::OrganizationTaskAttempt,
    codex_protocol::organization_execution::OrganizationTaskResult,
)> {
    let task = read_task(tx, task_id).await?;
    ensure!(
        task.state == "accepted",
        "evaluation requires exact owner-accepted results"
    );
    let (grant,attempt,result):(String,String,String)=sqlx::query_as("SELECT a.data,p.data,r.data FROM organization_tasks t JOIN organization_authorizations a ON a.id=t.authorization_id JOIN organization_attempts p ON p.task_id=t.id AND p.authorization_id=a.id JOIN organization_results r ON r.attempt_id=p.id WHERE t.id=? AND p.state='accepted' AND r.digest=? AND p.generation=(SELECT max(generation) FROM organization_attempts WHERE task_id=t.id)").bind(task_id).bind(digest).fetch_one(&mut **tx).await?;
    let result: codex_protocol::organization_execution::OrganizationTaskResult =
        serde_json::from_str(&result)?;
    ensure!(
        result.accepted && result.digest == digest,
        "evaluation result changed"
    );
    Ok((
        task,
        serde_json::from_str(&grant)?,
        serde_json::from_str(&attempt)?,
        result,
    ))
}

fn compare_checks(
    before: &codex_protocol::organization_execution::OrganizationTaskResult,
    after: &codex_protocol::organization_execution::OrganizationTaskResult,
) -> Result<()> {
    ensure!(
        !before.checks.is_empty() && before.checks.len() == after.checks.len(),
        "representative immutable checks required"
    );
    for (before, after) in before.checks.iter().zip(&after.checks) {
        ensure!(
            before.argv == after.argv
                && before.exit_code == Some(0)
                && after.exit_code == Some(0)
                && before
                    .immutable_inputs
                    .as_ref()
                    .is_some_and(|files| !files.is_empty())
                && before.immutable_inputs == after.immutable_inputs,
            "candidate must pass the same hash-pinned immutable checks; legacy evidence cannot qualify adoption"
        );
    }
    Ok(())
}
async fn comparison_run(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    revision: i64,
) -> Result<codex_protocol::organization_workflow::OrganizationWorkflowRun> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_workflow_runs WHERE id=?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let run: codex_protocol::organization_workflow::OrganizationWorkflowRun =
        serde_json::from_str(&data)?;
    ensure!(
        run.revision == revision && (1..=32).contains(&run.task_ids.len()),
        "workflow evaluation run changed or has invalid bounds"
    );
    Ok(run)
}
async fn comparison_evidence_for_task(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<(
    OrganizationTask,
    OrganizationTaskAuthorization,
    codex_protocol::organization_execution::OrganizationTaskAttempt,
    codex_protocol::organization_execution::OrganizationTaskResult,
)> {
    let digest:String=sqlx::query_scalar("SELECT r.digest FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id JOIN organization_tasks t ON t.id=p.task_id WHERE t.id=? AND t.authorization_id=p.authorization_id AND p.state='accepted' ORDER BY p.generation DESC LIMIT 1").bind(id).fetch_one(&mut **tx).await?;
    comparison_evidence(tx, id, &digest).await
}
