use super::OrganizationStore;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use sqlx::Sqlite;
use sqlx::Transaction;
use uuid::Uuid;

impl OrganizationStore {
    /// Owner-only mutation seam. The app-server authenticates owner transport;
    /// future agent operations must remain separate from this administrative API.
    pub async fn mutate_owner(
        &self,
        mutation: OrganizationMutation,
        project_descriptor: Option<(String, Vec<String>)>,
    ) -> Result<Option<OrganizationRecord>> {
        ensure!(
            serde_json::to_vec(&mutation)?.len() <= 128 * 1024,
            "organization mutation too large"
        );
        let removed_id = match &mutation {
            OrganizationMutation::SetMembership {
                node_id,
                group_id,
                present: false,
            } => Some(format!("{node_id}/{group_id}")),
            _ => None,
        };
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (operation, record) = match mutation {
            OrganizationMutation::SavePreset {
                root_id,
                expected_root_revision,
                label,
            } => {
                let preset =
                    super::presets::save(&mut tx, &root_id, expected_root_revision, label).await?;
                (
                    "preset/save",
                    Some(OrganizationRecord::Preset(Box::new(preset))),
                )
            }
            OrganizationMutation::ClonePreset {
                id,
                expected_revision,
                label,
                parent_id,
            } => {
                let preset =
                    super::presets::clone(&mut tx, &id, expected_revision, label, parent_id)
                        .await?;
                (
                    "preset/clone",
                    Some(OrganizationRecord::Preset(Box::new(preset))),
                )
            }
            OrganizationMutation::PromoteNote {
                source_id,
                group_id,
                body,
            } => {
                super::coordination::promote_note(&mut tx, &source_id, group_id, body).await?;
                ("note/promote", None)
            }
            OrganizationMutation::PutNode {
                id,
                expected_revision,
                parent_id,
                kind,
                label,
                role_version_id,
                model,
                enabled,
            } => {
                text(&label, 256)?;
                ensure!(
                    kind == OrganizationNodeKind::Agent
                        || (role_version_id.is_none() && model.is_none()),
                    "groups do not have execution roles/models"
                );
                if let Some(model) = &model {
                    text(model, 256)?;
                }
                let (id, revision) = next_revision(
                    &mut tx,
                    OrganizationRecordKind::Nodes,
                    id,
                    expected_revision,
                )
                .await?;
                let old_kind: Option<String> =
                    sqlx::query_scalar("SELECT kind FROM organization_nodes WHERE id = ?")
                        .bind(&id)
                        .fetch_optional(&mut *tx)
                        .await?;
                let kind_name = match kind {
                    OrganizationNodeKind::Agent => "agent",
                    OrganizationNodeKind::Group => "group",
                };
                ensure!(
                    old_kind.as_deref().is_none_or(|old| old == kind_name),
                    "node kind is immutable"
                );
                let cycle: i64 = sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT ? UNION SELECT n.parent_id FROM organization_nodes n JOIN ancestors a ON n.id = a.id WHERE n.parent_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id = ?)")
                    .bind(&parent_id).bind(&id).fetch_one(&mut *tx).await?;
                ensure!(cycle == 0, "organization hierarchy cycle rejected");
                let record = OrganizationRecord::Node(OrganizationNode {
                    id: id.clone(),
                    parent_id: parent_id.clone(),
                    kind,
                    label,
                    role_version_id: role_version_id.clone(),
                    model,
                    enabled,
                    revision,
                });
                sqlx::query("INSERT INTO organization_nodes(id,parent_id,kind,role_version_id,revision,data) VALUES (?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET parent_id=excluded.parent_id,role_version_id=excluded.role_version_id,revision=excluded.revision,data=excluded.data")
                    .bind(id).bind(parent_id).bind(kind_name).bind(role_version_id).bind(revision).bind(serde_json::to_string(&record)?).execute(&mut *tx).await?;
                ("node/put", Some(record))
            }
            OrganizationMutation::CreateRoleVersion {
                role_id,
                expected_version,
                name,
                instructions,
            } => {
                text(&name, 256)?;
                text(&instructions, 64 * 1024)?;
                let version = if let Some(id) = &role_id {
                    let current: Option<i64> = sqlx::query_scalar(
                        "SELECT max(version) FROM organization_role_versions WHERE role_id = ?",
                    )
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                    ensure!(
                        current.is_some() && current == expected_version,
                        "role version conflict or unknown role"
                    );
                    current
                        .context("unknown role")?
                        .checked_add(1)
                        .context("role version exhausted")?
                } else {
                    ensure!(
                        expected_version.is_none(),
                        "new role must not have expectedVersion"
                    );
                    1
                };
                let id = Uuid::now_v7().to_string();
                let role_id = role_id.unwrap_or_else(|| Uuid::now_v7().to_string());
                let record = OrganizationRecord::Role(OrganizationRoleVersion {
                    id: id.clone(),
                    role_id: role_id.clone(),
                    name,
                    instructions,
                    version,
                });
                sqlx::query("INSERT INTO organization_role_versions(id,role_id,version,data) VALUES (?,?,?,?)")
                    .bind(id).bind(role_id).bind(version).bind(serde_json::to_string(&record)?).execute(&mut *tx).await?;
                ("role/version", Some(record))
            }
            OrganizationMutation::PutGoal {
                id,
                expected_revision,
                parent_id,
                objective,
                criteria,
            } => {
                text(&objective, 16 * 1024)?;
                validate_criteria(&criteria)?;
                let (id, revision) = next_revision(
                    &mut tx,
                    OrganizationRecordKind::Goals,
                    id,
                    expected_revision,
                )
                .await?;
                let cycle: i64 = sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT ? UNION SELECT g.parent_id FROM organization_goals g JOIN ancestors a ON g.id = a.id WHERE g.parent_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id = ?)")
                    .bind(&parent_id).bind(&id).fetch_one(&mut *tx).await?;
                ensure!(cycle == 0, "goal hierarchy cycle rejected");
                let record = OrganizationRecord::Goal(OrganizationGoal {
                    id: id.clone(),
                    parent_id: parent_id.clone(),
                    objective,
                    criteria,
                    revision,
                });
                sqlx::query("INSERT INTO organization_goals(id,parent_id,revision,data) VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET parent_id=excluded.parent_id,revision=excluded.revision,data=excluded.data")
                    .bind(id).bind(parent_id).bind(revision).bind(serde_json::to_string(&record)?).execute(&mut *tx).await?;
                ("goal/put", Some(record))
            }
            OrganizationMutation::LinkProject {
                project_id,
                expected_revision,
                goal_id,
                owner_id,
                criteria,
            } => {
                validate_criteria(&criteria)?;
                text(&project_id, 256)?;
                let (name, roots) = project_descriptor
                    .context("project must be verified against the existing project store")?;
                if let Some(owner) = &owner_id {
                    require_agent(&mut tx, owner).await?;
                }
                let existing: Option<i64> =
                    sqlx::query_scalar("SELECT revision FROM organization_projects WHERE id = ?")
                        .bind(&project_id)
                        .fetch_optional(&mut *tx)
                        .await?;
                ensure!(existing == expected_revision, "project revision conflict");
                let revision = existing
                    .unwrap_or(0)
                    .checked_add(1)
                    .context("project revision exhausted")?;
                let record = OrganizationRecord::Project(OrganizationProjectPlan {
                    id: project_id.clone(),
                    name,
                    roots,
                    goal_id: goal_id.clone(),
                    owner_id: owner_id.clone(),
                    criteria,
                    revision,
                });
                sqlx::query("INSERT INTO organization_projects(id,goal_id,owner_id,revision,data) VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET goal_id=excluded.goal_id,owner_id=excluded.owner_id,revision=excluded.revision,data=excluded.data")
                    .bind(project_id).bind(goal_id).bind(owner_id).bind(revision).bind(serde_json::to_string(&record)?).execute(&mut *tx).await?;
                ("project/link", Some(record))
            }
            OrganizationMutation::PutTask {
                id,
                expected_revision,
                project_id,
                goal_id,
                owner_id,
                instructions,
                criteria,
                prerequisites,
            } => {
                text(&instructions, 64 * 1024)?;
                validate_criteria(&criteria)?;
                ensure!(prerequisites.len() <= 256, "too many task prerequisites");
                require_agent(&mut tx, &owner_id).await?;
                let (id, revision) = next_revision(
                    &mut tx,
                    OrganizationRecordKind::Tasks,
                    id,
                    expected_revision,
                )
                .await?;
                let existing: Option<(String, String)> =
                    sqlx::query_as("SELECT state, project_id FROM organization_tasks WHERE id = ?")
                        .bind(&id)
                        .fetch_optional(&mut *tx)
                        .await?;
                ensure!(
                    existing.as_ref().is_none_or(
                        |(state, project)| state == "proposed" && project == &project_id
                    ),
                    "only proposed tasks may be edited, without moving between projects"
                );
                for prerequisite in &prerequisites {
                    let prerequisite_project: Option<String> = sqlx::query_scalar(
                        "SELECT project_id FROM organization_tasks WHERE id = ?",
                    )
                    .bind(prerequisite)
                    .fetch_optional(&mut *tx)
                    .await?;
                    ensure!(
                        prerequisite_project.as_deref() == Some(project_id.as_str()),
                        "prerequisite must exist in the same project"
                    );
                    let cycle: i64 = sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT ? UNION SELECT d.prerequisite_id FROM organization_task_dependencies d JOIN ancestors a ON d.task_id = a.id) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id = ?)")
                        .bind(prerequisite).bind(&id).fetch_one(&mut *tx).await?;
                    ensure!(cycle == 0, "task dependency cycle rejected");
                }
                let record = OrganizationRecord::Task(OrganizationTask {
                    id: id.clone(),
                    project_id: project_id.clone(),
                    goal_id: goal_id.clone(),
                    owner_id: owner_id.clone(),
                    instructions,
                    criteria,
                    prerequisites: prerequisites.clone(),
                    state: "proposed".into(),
                    revision,
                });
                sqlx::query("INSERT INTO organization_tasks(id,project_id,goal_id,owner_id,revision,state,data) VALUES (?,?,?,?,?,'proposed',?) ON CONFLICT(id) DO UPDATE SET project_id=excluded.project_id,goal_id=excluded.goal_id,owner_id=excluded.owner_id,revision=excluded.revision,data=excluded.data")
                    .bind(&id).bind(project_id).bind(goal_id).bind(owner_id).bind(revision).bind(serde_json::to_string(&record)?).execute(&mut *tx).await?;
                sqlx::query("DELETE FROM organization_task_dependencies WHERE task_id = ?")
                    .bind(&id)
                    .execute(&mut *tx)
                    .await?;
                for prerequisite in prerequisites {
                    sqlx::query("INSERT INTO organization_task_dependencies(task_id,prerequisite_id) VALUES (?,?)").bind(&id).bind(prerequisite).execute(&mut *tx).await?;
                }
                ("task/put", Some(record))
            }
            OrganizationMutation::SetMembership {
                node_id,
                group_id,
                present,
            } => {
                let kind: Option<String> =
                    sqlx::query_scalar("SELECT kind FROM organization_nodes WHERE id = ?")
                        .bind(&group_id)
                        .fetch_optional(&mut *tx)
                        .await?;
                ensure!(
                    kind.as_deref() == Some("group"),
                    "membership target must be a group"
                );
                ensure!(node_id != group_id, "node cannot belong to itself");
                let id = format!("{node_id}/{group_id}");
                if present {
                    let record = OrganizationRecord::Membership(OrganizationMembership {
                        id: id.clone(),
                        node_id: node_id.clone(),
                        group_id: group_id.clone(),
                    });
                    sqlx::query("INSERT INTO organization_memberships(id,node_id,group_id,data) VALUES (?,?,?,?) ON CONFLICT(id) DO NOTHING")
                        .bind(id).bind(node_id).bind(group_id).bind(serde_json::to_string(&record)?).execute(&mut *tx).await?;
                    ("membership/add", Some(record))
                } else {
                    sqlx::query("DELETE FROM organization_memberships WHERE id = ?")
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    ("membership/remove", None)
                }
            }
        };
        // Audit references, not role instructions, user text or credentials.
        let reference = record.as_ref().map(record_id).or(removed_id.as_deref());
        sqlx::query("INSERT INTO organization_audit(actor,operation,payload) VALUES ('owner',?,?)")
            .bind(operation)
            .bind(serde_json::to_string(
                &serde_json::json!({"id": reference}),
            )?)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(record)
    }

    pub async fn read_definition(
        &self,
        kind: OrganizationRecordKind,
        id: &str,
    ) -> Result<OrganizationRecord> {
        text(id, 256)?;
        let query = match kind {
            OrganizationRecordKind::Nodes => "SELECT data FROM organization_nodes WHERE id=?",
            OrganizationRecordKind::Roles => {
                "SELECT data FROM organization_role_versions WHERE id=?"
            }
            OrganizationRecordKind::Goals => "SELECT data FROM organization_goals WHERE id=?",
            OrganizationRecordKind::Projects => "SELECT data FROM organization_projects WHERE id=?",
            OrganizationRecordKind::Tasks => "SELECT data FROM organization_tasks WHERE id=?",
            OrganizationRecordKind::Memberships => {
                "SELECT data FROM organization_memberships WHERE id=?"
            }
            OrganizationRecordKind::Presets => "SELECT data FROM organization_presets WHERE id=?",
        };
        let data: String = sqlx::query_scalar(query)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(serde_json::from_str(&data)?)
    }

    pub async fn list_definitions(
        &self,
        kind: OrganizationRecordKind,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<(Vec<OrganizationRecord>, Option<String>)> {
        ensure!(
            (1..=200).contains(&limit),
            "limit must be between 1 and 200"
        );
        if let Some(cursor) = &cursor {
            text(cursor, 256)?;
        }
        let query = match kind {
            OrganizationRecordKind::Nodes => {
                "SELECT data FROM organization_nodes WHERE id > ? ORDER BY id LIMIT ?"
            }
            OrganizationRecordKind::Roles => {
                "SELECT data FROM organization_role_versions WHERE id > ? ORDER BY id LIMIT ?"
            }
            OrganizationRecordKind::Goals => {
                "SELECT data FROM organization_goals WHERE id > ? ORDER BY id LIMIT ?"
            }
            OrganizationRecordKind::Projects => {
                "SELECT data FROM organization_projects WHERE id > ? ORDER BY id LIMIT ?"
            }
            OrganizationRecordKind::Tasks => {
                "SELECT data FROM organization_tasks WHERE id > ? ORDER BY id LIMIT ?"
            }
            OrganizationRecordKind::Memberships => {
                "SELECT data FROM organization_memberships WHERE id > ? ORDER BY id LIMIT ?"
            }
            OrganizationRecordKind::Presets => {
                "SELECT data FROM organization_presets WHERE id > ? ORDER BY id LIMIT ?"
            }
        };
        let rows: Vec<String> = sqlx::query_scalar(query)
            .bind(cursor.unwrap_or_default())
            .bind(i64::from(limit) + 1)
            .fetch_all(&self.pool)
            .await?;
        let has_more = rows.len() > limit as usize;
        let data: Vec<OrganizationRecord> = rows
            .into_iter()
            .take(limit as usize)
            .map(|data| serde_json::from_str(&data))
            .collect::<std::result::Result<_, _>>()?;
        let next = if has_more {
            data.last().map(|record| record_id(record).to_owned())
        } else {
            None
        };
        Ok((data, next))
    }
}

pub(super) fn record_id(record: &OrganizationRecord) -> &str {
    match record {
        OrganizationRecord::Node(record) => &record.id,
        OrganizationRecord::Role(record) => &record.id,
        OrganizationRecord::Goal(record) => &record.id,
        OrganizationRecord::Project(record) => &record.id,
        OrganizationRecord::Task(record) => &record.id,
        OrganizationRecord::Membership(record) => &record.id,
        OrganizationRecord::Preset(record) => &record.id,
    }
}

pub(super) fn text(value: &str, maximum: usize) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= maximum && !value.contains('\0'),
        "text must be nonempty and at most {maximum} bytes without NUL"
    );
    Ok(())
}

pub(super) fn validate_criteria(criteria: &[String]) -> Result<()> {
    ensure!(
        !criteria.is_empty() && criteria.len() <= 64,
        "provide between 1 and 64 acceptance criteria"
    );
    for criterion in criteria {
        text(criterion, 4096)?;
    }
    Ok(())
}

async fn require_agent(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<()> {
    let kind: Option<String> =
        sqlx::query_scalar("SELECT kind FROM organization_nodes WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    ensure!(
        kind.as_deref() == Some("agent"),
        "task/project owner must be an existing agent"
    );
    Ok(())
}

async fn next_revision(
    tx: &mut Transaction<'_, Sqlite>,
    kind: OrganizationRecordKind,
    id: Option<String>,
    expected: Option<i64>,
) -> Result<(String, i64)> {
    let Some(id) = id else {
        ensure!(
            expected.is_none(),
            "new definition must not have expectedRevision"
        );
        return Ok((Uuid::now_v7().to_string(), 1));
    };
    let query = match kind {
        OrganizationRecordKind::Nodes => "SELECT revision FROM organization_nodes WHERE id = ?",
        OrganizationRecordKind::Goals => "SELECT revision FROM organization_goals WHERE id = ?",
        OrganizationRecordKind::Tasks => "SELECT revision FROM organization_tasks WHERE id = ?",
        _ => anyhow::bail!("unsupported revision record kind"),
    };
    let revision: Option<i64> = sqlx::query_scalar(query)
        .bind(&id)
        .fetch_optional(&mut **tx)
        .await?;
    ensure!(
        revision.is_some() && revision == expected,
        "definition revision conflict or unknown id"
    );
    Ok((
        id,
        revision
            .context("unknown definition")?
            .checked_add(1)
            .context("definition revision exhausted")?,
    ))
}
