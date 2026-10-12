//! Offline cold archival of unreferenced, effect-free failed workspaces.
//! Retain canonical ledgers and a durable rename intent; never delete results.
use super::*;
use anyhow::bail;
use codex_protocol::organization_execution::OrganizationTaskAttempt;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceRetentionPlan {
    pub database_id: String,
    pub project_id: String,
    pub before: i64,
    pub candidates: Vec<WorkspaceRetentionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceRetentionCandidate {
    pub attempt_id: String,
    pub workspace: String,
    pub tree_sha256: String,
    pub bytes: u64,
}

fn tree(path: &Path) -> Result<(String, u64)> {
    let mut files = BTreeMap::new();
    let mut queue = vec![(path.to_path_buf(), 0)];
    let mut total = 0u64;
    while let Some((directory, depth)) = queue.pop() {
        ensure!(depth <= 32, "retention depth exceeds 32");
        super::bundle::owned(&directory, true)?;
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let metadata = std::fs::symlink_metadata(entry.path())?;
            ensure!(
                !metadata.file_type().is_symlink(),
                "retention refuses symlinks"
            );
            if metadata.is_dir() {
                queue.push((entry.path(), depth + 1));
                continue;
            }
            super::bundle::owned(&entry.path(), false)?;
            ensure!(
                metadata.len() <= 8 * 1024 * 1024,
                "retention file exceeds 8 MiB"
            );
            total = total
                .checked_add(metadata.len())
                .context("retention size overflow")?;
            ensure!(
                total <= 64 * 1024 * 1024 && files.len() < 10000,
                "retention inventory exceeds bounds"
            );
            let data = super::bundle::read(&entry.path())?;
            ensure!(
                data.len() as u64 == metadata.len(),
                "retention file changed during capture"
            );
            files.insert(
                entry
                    .path()
                    .strip_prefix(path)?
                    .to_string_lossy()
                    .into_owned(),
                format!("{:x}", Sha256::digest(&data)),
            );
        }
    }
    Ok((
        format!("{:x}", Sha256::digest(serde_json::to_vec(&files)?)),
        total,
    ))
}

impl OrganizationStore {
    /// Offline caller must stop all runtime/editor writers; this protects the
    /// execution owner lock too. Results/bases/actions/notes/recovery never qualify.
    pub async fn retention_preview(
        &self,
        project_id: &str,
        before: i64,
    ) -> Result<WorkspaceRetentionPlan> {
        ensure!(before >= 0, "invalid retention time");
        let mut tx = self.pool.begin().await?;
        super::execution::read_project(&mut tx, project_id).await?;
        let rows:Vec<String>=sqlx::query_scalar("SELECT p.data FROM organization_attempts p JOIN organization_tasks t ON t.id=p.task_id WHERE t.project_id=? AND p.state IN ('failed','cancelled') AND p.occupied=0 AND json_extract(p.data,'$.startedAt')<? AND json_extract(p.data,'$.modelRequests')=COALESCE(json_extract(p.data,'$.reportedModelRequests'),0) AND NOT EXISTS(SELECT 1 FROM json_each(p.data,'$.artifactBases') WHERE json_extract(value,'$.sha256') IS NOT NULL) AND NOT EXISTS(SELECT 1 FROM organization_authorizations x WHERE x.parent_attempt_id=p.id) AND NOT EXISTS(SELECT 1 FROM organization_actions x WHERE x.attempt_id=p.id) AND NOT EXISTS(SELECT 1 FROM organization_results x WHERE x.attempt_id=p.id) AND NOT EXISTS(SELECT 1 FROM organization_checkpoints x WHERE x.attempt_id=p.id) AND NOT EXISTS(SELECT 1 FROM organization_notes x WHERE json_extract(x.data,'$.attemptId')=p.id) AND NOT EXISTS(SELECT 1 FROM organization_task_dependencies x WHERE x.prerequisite_id=t.id) AND NOT EXISTS(SELECT 1 FROM organization_workflow_tasks x WHERE x.task_id=t.id) AND NOT EXISTS(SELECT 1 FROM organization_workspace_archives x WHERE x.attempt_id=p.id) ORDER BY p.id LIMIT 32")
            .bind(project_id).bind(before).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        let mut candidates = Vec::new();
        for data in rows {
            let attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
            let Some(workspace) = attempt.workspace else {
                continue;
            };
            let path = Path::new(&workspace);
            #[cfg(unix)]
            let expected = std::env::temp_dir()
                .join(format!("codex-organization-{}", unsafe { libc::geteuid() }))
                .join(&attempt.id);
            #[cfg(not(unix))]
            let expected = PathBuf::new();
            if path != expected || std::fs::canonicalize(path)? != path {
                continue;
            }
            let (tree_sha256, bytes) = tree(path)?;
            candidates.push(WorkspaceRetentionCandidate {
                attempt_id: attempt.id,
                workspace,
                tree_sha256,
                bytes,
            });
        }
        Ok(WorkspaceRetentionPlan {
            database_id: self.database_id().await?,
            project_id: project_id.into(),
            before,
            candidates,
        })
    }

    pub async fn archive_workspaces(&self, plan: &WorkspaceRetentionPlan) -> Result<()> {
        let _owner = self.acquire_execution_owner()?;
        ensure!(
            *plan
                == self
                    .retention_preview(&plan.project_id, plan.before)
                    .await?,
            "retention preview changed; preserve receipts and recapture remaining work"
        );
        for candidate in &plan.candidates {
            let original = Path::new(&candidate.workspace);
            let parent = original.parent().context("workspace parent missing")?;
            let archives = parent.join("archives");
            private_directory(&archives).await?;
            let target = archives.join(&candidate.attempt_id);
            ensure!(
                !target.try_exists()?,
                "archive destination exists; inspect its receipt"
            );
            ensure!(
                tree(original)?.0 == candidate.tree_sha256,
                "workspace content changed"
            );
            sqlx::query("INSERT INTO organization_workspace_archives(attempt_id,original_path,archive_path,tree_sha256,state) VALUES (?,?,?,?,'planned')")
                .bind(&candidate.attempt_id).bind(&candidate.workspace).bind(target.to_str().context("non-UTF8 archive path")?).bind(&candidate.tree_sha256).execute(&self.pool).await?;
            if let Err(error) = std::fs::rename(original, &target) {
                sqlx::query("UPDATE organization_workspace_archives SET state='uncertain' WHERE attempt_id=?").bind(&candidate.attempt_id).execute(&self.pool).await?;
                return Err(error.into());
            }
            sync_directory(parent)?;
            sync_directory(&archives)?;
            self.finish_workspace_archive(&candidate.attempt_id).await?;
        }
        Ok(())
    }

    /// Evidence-only completion after a crash; never performs or repeats rename.
    pub async fn reconcile_workspace_archive(&self, attempt_id: &str) -> Result<()> {
        let _owner = self.acquire_execution_owner()?;
        self.finish_workspace_archive(attempt_id).await
    }

    async fn finish_workspace_archive(&self, attempt_id: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (original,target,hash,state):(String,String,String,String)=sqlx::query_as("SELECT original_path,archive_path,tree_sha256,state FROM organization_workspace_archives WHERE attempt_id=?").bind(attempt_id).fetch_one(&mut *tx).await?;
        if state == "completed" {
            return Ok(());
        }
        if std::fs::symlink_metadata(&original).is_ok()
            || tree(Path::new(&target)).map(|v| v.0).ok().as_deref() != Some(&hash)
        {
            bail!(
                "archive remains uncertain; retain both paths and inspect evidence, do not replay"
            );
        }
        let data: String = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
            .bind(attempt_id)
            .fetch_one(&mut *tx)
            .await?;
        let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
        ensure!(
            attempt.workspace.as_deref() == Some(&original),
            "archive workspace mapping changed"
        );
        attempt.workspace = Some(target);
        super::execution::write_attempt(&mut tx, &attempt).await?;
        sqlx::query(
            "UPDATE organization_workspace_archives SET state='completed' WHERE attempt_id=?",
        )
        .bind(attempt_id)
        .execute(&mut *tx)
        .await?;
        super::execution::owner_event(&mut tx, "workspace/archived", attempt_id).await?;
        tx.commit().await?;
        Ok(())
    }
}
