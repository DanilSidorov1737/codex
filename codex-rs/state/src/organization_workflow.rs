//! Saved plans and immutable owner previews reuse ordinary task/grant transactions.

use super::OrganizationStore;
use super::coordination::validate_plan_items;
use super::definitions::text;
use super::execution::*;
use anyhow::{Context, Result, ensure};
use codex_protocol::organization::{OrganizationNodeKind, OrganizationRecord, OrganizationTask};
use codex_protocol::organization_execution::*;
use codex_protocol::organization_workflow::*;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

impl OrganizationStore {
    pub async fn preview_workflow(
        &self,
        request: OrganizationWorkflowPreviewRequest,
    ) -> Result<OrganizationWorkflowPreview> {
        let mut tx = self.pool.begin().await?;
        preview_in(&mut tx, request).await
    }

    pub async fn mutate_workflow(
        &self,
        mutation: OrganizationWorkflowMutation,
        now: i64,
    ) -> Result<OrganizationWorkflowView> {
        ensure!(
            now >= 0 && serde_json::to_vec(&mutation)?.len() <= 128 * 1024,
            "invalid workflow time or oversized mutation"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let view = match mutation {
            OrganizationWorkflowMutation::Put {
                id,
                expected_revision,
                project_id,
                label,
                items,
            } => {
                Uuid::parse_str(&id)?;
                text(&label, 256)?;
                let project = read_project(&mut tx, &project_id).await?;
                let existing: Option<(String, i64)> = sqlx::query_as(
                    "SELECT project_id,revision FROM organization_workflow_templates WHERE id=?",
                )
                .bind(&id)
                .fetch_optional(&mut *tx)
                .await?;
                let revision = match existing {
                    Some((previous_project, revision)) => {
                        ensure!(
                            previous_project == project.id && Some(revision) == expected_revision,
                            "workflow project or revision conflict"
                        );
                        revision
                            .checked_add(1)
                            .context("workflow revision exhausted")?
                    }
                    None => {
                        ensure!(
                            expected_revision.is_none(),
                            "new workflow must not have expectedRevision"
                        );
                        1
                    }
                };
                let items = validate_plan_items(items)?;
                for item in &items {
                    ensure!(
                        read_node(&mut tx, &item.owner_id).await?.kind
                            == OrganizationNodeKind::Agent,
                        "workflow owner must be an agent"
                    );
                }
                let template = OrganizationWorkflowTemplate {
                    id,
                    project_id,
                    label,
                    revision,
                    items,
                };
                sqlx::query("INSERT INTO organization_workflow_templates(id,project_id,revision,data) VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,data=excluded.data")
                    .bind(&template.id).bind(&template.project_id).bind(revision).bind(serde_json::to_string(&template)?).execute(&mut *tx).await?;
                owner_event(&mut tx, "workflow/put", &template.id).await?;
                OrganizationWorkflowView::Template { template }
            }
            OrganizationWorkflowMutation::Run { preview } => {
                let fresh = preview_in(
                    &mut tx,
                    OrganizationWorkflowPreviewRequest {
                        id: preview.id.clone(),
                        template_id: preview.template.id.clone(),
                        expected_revision: preview.template.revision,
                        scopes: preview.scopes.clone(),
                    },
                )
                .await?;
                ensure!(
                    *preview == fresh,
                    "workflow preview changed; inspect a fresh preview before authorizing"
                );
                let exists: i64 = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM organization_workflow_runs WHERE id=?)",
                )
                .bind(&preview.id)
                .fetch_one(&mut *tx)
                .await?;
                ensure!(
                    exists == 0,
                    "workflow run ID already exists; inspect it instead of retrying"
                );
                let project = read_project(&mut tx, &preview.template.project_id).await?;
                let mut run = OrganizationWorkflowRun {
                    id: preview.id.clone(),
                    preview: *preview,
                    task_ids: BTreeMap::new(),
                    created_at: now,
                };
                // FK parent precedes tasks; all rows are invisible until this transaction commits.
                sqlx::query("INSERT INTO organization_workflow_runs(id,template_id,project_id,data) VALUES (?,?,?,?)")
                    .bind(&run.id).bind(&run.preview.template.id).bind(&project.id).bind(serde_json::to_string(&run)?).execute(&mut *tx).await?;
                let mut tasks = Vec::new();
                for (item, step) in run.preview.template.items.iter().zip(&run.preview.scopes) {
                    let task = OrganizationTask {
                        id: Uuid::now_v7().to_string(),
                        project_id: project.id.clone(),
                        goal_id: project.goal_id.clone(),
                        owner_id: item.owner_id.clone(),
                        instructions: item.instructions.clone(),
                        criteria: item.criteria.clone(),
                        prerequisites: item
                            .prerequisites
                            .iter()
                            .map(|key| run.task_ids[key].clone())
                            .collect(),
                        state: "proposed".into(),
                        revision: 1,
                    };
                    sqlx::query("INSERT INTO organization_tasks(id,project_id,goal_id,owner_id,revision,state,data) VALUES (?,?,?,?,1,'proposed',?)")
                        .bind(&task.id).bind(&task.project_id).bind(&task.goal_id).bind(&task.owner_id)
                        .bind(serde_json::to_string(&OrganizationRecord::Task(task.clone()))?).execute(&mut *tx).await?;
                    for id in &task.prerequisites {
                        sqlx::query("INSERT INTO organization_task_dependencies(task_id,prerequisite_id) VALUES (?,?)")
                            .bind(&task.id).bind(id).execute(&mut *tx).await?;
                    }
                    let grant =
                        authorize_task_in(&mut tx, &task.id, 1, step.scope.clone(), now).await?;
                    sqlx::query("INSERT INTO organization_workflow_tasks(run_id,step_key,task_id,authorization_id) VALUES (?,?,?,?)")
                        .bind(&run.id).bind(&item.key).bind(&task.id).bind(&grant.id).execute(&mut *tx).await?;
                    run.task_ids.insert(item.key.clone(), task.id.clone());
                    tasks.push(read_task(&mut tx, &task.id).await?);
                }
                sqlx::query("UPDATE organization_workflow_runs SET data=? WHERE id=?")
                    .bind(serde_json::to_string(&run)?)
                    .bind(&run.id)
                    .execute(&mut *tx)
                    .await?;
                owner_event(&mut tx, "workflow/run", &run.id).await?;
                OrganizationWorkflowView::Run { run, tasks }
            }
        };
        tx.commit().await?;
        Ok(view)
    }

    pub async fn list_workflows(
        &self,
        kind: OrganizationWorkflowKind,
        project_id: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<(Vec<OrganizationWorkflowView>, Option<String>)> {
        ensure!((1..=200).contains(&limit), "workflow limit must be 1–200");
        if let Some(id) = project_id {
            text(id, 256)?;
        }
        if let Some(id) = cursor {
            text(id, 256)?;
        }
        let query = match kind {
            OrganizationWorkflowKind::Templates => {
                "SELECT id,data FROM organization_workflow_templates WHERE (? IS NULL OR project_id=?) AND (? IS NULL OR id>?) ORDER BY id LIMIT ?"
            }
            OrganizationWorkflowKind::Runs => {
                "SELECT id,data FROM organization_workflow_runs WHERE (? IS NULL OR project_id=?) AND (? IS NULL OR id>?) ORDER BY id LIMIT ?"
            }
        };
        let mut tx = self.pool.begin().await?;
        let rows: Vec<(String, String)> = sqlx::query_as(query)
            .bind(project_id)
            .bind(project_id)
            .bind(cursor)
            .bind(cursor)
            .bind(i64::from(limit) + 1)
            .fetch_all(&mut *tx)
            .await?;
        let next = (rows.len() > limit as usize).then(|| rows[limit as usize - 1].0.clone());
        let mut views = Vec::new();
        for (_, data) in rows.into_iter().take(limit as usize) {
            views.push(match kind {
                OrganizationWorkflowKind::Templates => OrganizationWorkflowView::Template {
                    template: serde_json::from_str(&data)?,
                },
                OrganizationWorkflowKind::Runs => {
                    let run: OrganizationWorkflowRun = serde_json::from_str(&data)?;
                    let mut tasks = Vec::new();
                    for item in &run.preview.template.items {
                        tasks.push(read_task(&mut tx, &run.task_ids[&item.key]).await?);
                    }
                    OrganizationWorkflowView::Run { run, tasks }
                }
            });
        }
        Ok((views, next))
    }

    /// Host-only artifact handoff. Agents cannot select another run or private workspace.
    pub async fn workflow_inputs(
        &self,
        task_id: &str,
    ) -> Result<Vec<(OrganizationTaskResult, OrganizationTaskAttempt)>> {
        let mut tx = self.pool.begin().await?;
        let data: Option<String> = sqlx::query_scalar("SELECT r.data FROM organization_workflow_tasks w JOIN organization_workflow_runs r ON r.id=w.run_id WHERE w.task_id=?")
            .bind(task_id).fetch_optional(&mut *tx).await?;
        let Some(data) = data else {
            return Ok(Vec::new());
        };
        let run: OrganizationWorkflowRun = serde_json::from_str(&data)?;
        let task = read_task(&mut tx, task_id).await?;
        let key = run
            .task_ids
            .iter()
            .find(|(_, id)| *id == task_id)
            .context("workflow task identity missing")?
            .0;
        let scope = &run
            .preview
            .scopes
            .iter()
            .find(|step| &step.key == key)
            .context("workflow scope missing")?
            .scope;
        let mut inputs = Vec::new();
        for id in task.prerequisites {
            ensure!(
                run.task_ids.values().any(|candidate| candidate == &id),
                "prerequisite outside this workflow run"
            );
            ensure!(
                read_task(&mut tx, &id).await?.state == "accepted",
                "workflow prerequisite is not accepted"
            );
            let data: String = sqlx::query_scalar("SELECT r.data FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id JOIN organization_tasks t ON t.id=p.task_id WHERE t.id=? AND p.authorization_id=t.authorization_id AND json_extract(r.data,'$.accepted')=1 ORDER BY p.generation DESC LIMIT 1")
                .bind(&id).fetch_one(&mut *tx).await?;
            let mut result: OrganizationTaskResult = serde_json::from_str(&data)?;
            result
                .artifacts
                .retain(|artifact| scope.input_files.contains(&artifact.path));
            if !result.artifacts.is_empty() {
                let data: String =
                    sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
                        .bind(&result.attempt_id)
                        .fetch_one(&mut *tx)
                        .await?;
                inputs.push((result, serde_json::from_str(&data)?));
            }
        }
        Ok(inputs)
    }
}

async fn preview_in(
    tx: &mut Transaction<'_, Sqlite>,
    request: OrganizationWorkflowPreviewRequest,
) -> Result<OrganizationWorkflowPreview> {
    ensure!(
        serde_json::to_vec(&request)?.len() <= 128 * 1024,
        "workflow preview exceeds 128 KiB"
    );
    Uuid::parse_str(&request.id)?;
    text(&request.template_id, 256)?;
    let data: String =
        sqlx::query_scalar("SELECT data FROM organization_workflow_templates WHERE id=?")
            .bind(&request.template_id)
            .fetch_one(&mut **tx)
            .await?;
    let template: OrganizationWorkflowTemplate = serde_json::from_str(&data)?;
    ensure!(
        template.revision == request.expected_revision,
        "workflow template revision conflict"
    );
    let mut scopes = BTreeMap::new();
    for step in request.scopes {
        ensure!(
            !scopes.contains_key(&step.key),
            "duplicate workflow scope key"
        );
        validate_scope(&step.scope)?;
        scopes.insert(step.key.clone(), step);
    }
    ensure!(
        scopes.len() == template.items.len(),
        "each workflow step needs exactly one scope"
    );
    let project = read_project(tx, &template.project_id).await?;
    let mut preview = OrganizationWorkflowPreview {
        id: request.id,
        template,
        project_revision: project.revision,
        node_revisions: BTreeMap::new(),
        policy_version: ORGANIZATION_POLICY_VERSION,
        scopes: Vec::new(),
        max_tokens: 0,
        max_model_requests: 0,
    };
    for item in &preview.template.items {
        let step = scopes
            .get(&item.key)
            .context("missing workflow scope key")?;
        let task = OrganizationTask {
            id: String::new(),
            project_id: project.id.clone(),
            goal_id: project.goal_id.clone(),
            owner_id: item.owner_id.clone(),
            instructions: item.instructions.clone(),
            criteria: item.criteria.clone(),
            prerequisites: Vec::new(),
            state: "proposed".into(),
            revision: 1,
        };
        let (node, _) = scope_definitions(tx, &task, &step.scope).await?;
        preview.node_revisions.insert(node.id, node.revision);
        for id in &step.scope.delegates {
            let node = read_node(tx, id).await?;
            preview.node_revisions.insert(node.id, node.revision);
        }
        let mut imported = HashSet::new();
        for key in &item.prerequisites {
            let producer = scopes.get(key).context("missing prerequisite scope")?;
            for path in &producer.scope.artifacts {
                if step.scope.input_files.contains(path) {
                    ensure!(
                        !step.scope.immutable_files.contains(path),
                        "workflow cannot replace immutable input"
                    );
                    ensure!(imported.insert(path), "ambiguous workflow input producers");
                }
            }
        }
        preview.max_tokens = preview
            .max_tokens
            .checked_add(step.scope.max_tokens)
            .context("workflow token total overflow")?;
        preview.max_model_requests = preview
            .max_model_requests
            .checked_add(step.scope.max_model_requests)
            .context("workflow request total overflow")?;
        preview.scopes.push(step.clone());
    }
    Ok(preview)
}

pub(super) async fn reject_workflow_reauthorization(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
) -> Result<()> {
    let workflow: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM organization_workflow_tasks w LEFT JOIN organization_authorizations a ON a.root_id=w.authorization_id WHERE w.task_id=? OR a.task_id=?)")
        .bind(task_id).bind(task_id).fetch_one(&mut **tx).await?;
    ensure!(
        workflow == 0,
        "workflow invocation limits cannot be reset; review a fresh workflow run instead"
    );
    Ok(())
}
