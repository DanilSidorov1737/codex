//! Authoritative organization state, separate from rebuildable runtime metadata.
//!
//! Opening fails on corruption, incompatible migrations or unsafe paths. Backups
//! are consistent SQLite snapshots; restoration creates a new store, never replaces
//! a live database. The execution host must exclude this state from agent sandboxes.

#![expect(
    clippy::disallowed_methods,
    reason = "canonical SQLite pools require FULL durability without runtime rebuild recovery"
)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteSynchronous;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx_macros::migrate!("./organization_migrations");
const DIRECTORY: &str = "organization";
const DATABASE: &str = "organization.sqlite";
const MARKER: &str = "initialized";

#[path = "organization_definitions.rs"]
mod definitions;

#[path = "organization_execution.rs"]
mod execution;

#[path = "organization_actions.rs"]
mod actions;

#[path = "organization_coordination.rs"]
mod coordination;

#[path = "organization_schedule.rs"]
mod schedule;

#[path = "organization_workflow.rs"]
mod workflow;

#[path = "organization_workflow_series.rs"]
mod workflow_series;

#[path = "organization_handoff.rs"]
mod handoff;

#[path = "organization_report.rs"]
mod report;

#[path = "organization_resource.rs"]
mod resource;

#[path = "organization_bundle.rs"]
mod bundle;

#[derive(Clone, Debug)]
pub struct OrganizationStore {
    pool: SqlitePool,
    directory: PathBuf,
}

/// Only admission before an effect can be retried when the owner resumes a run.
#[derive(Debug)]
pub struct WorkflowPaused;

impl std::fmt::Display for WorkflowPaused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("workflow is paused")
    }
}

impl std::error::Error for WorkflowPaused {}

impl OrganizationStore {
    /// One execution owner per canonical store, including embedded app-servers
    /// using different Codex homes. Definitions remain ordinary transactional writes.
    /// Keep this file open for the service lifetime; never unlink a live lock.
    pub fn acquire_execution_owner(&self) -> Result<File> {
        let path = self.directory.join("execution.lock");
        let file = match private_file(&path, true) {
            Ok(file) => file,
            Err(error) if path.exists() => private_file(&path, false).map_err(|_| error)?,
            Err(error) => return Err(error),
        };
        file.try_lock()
            .context("another service owns organization execution")?;
        Ok(file)
    }

    pub async fn open(sqlite_home: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(sqlite_home).await?;
        let directory = sqlite_home.join(DIRECTORY);
        private_directory(&directory).await?;
        let path = directory.join(DATABASE);
        if !path.try_exists()? {
            ensure!(
                tokio::fs::read_dir(&directory)
                    .await?
                    .next_entry()
                    .await?
                    .is_none(),
                "organization database is missing; preserve state and restore a verified backup"
            );
            private_file(&path, true)?;
        }
        check_file(&path)?;
        for suffix in ["-wal", "-shm"] {
            let sidecar = directory.join(format!("{DATABASE}{suffix}"));
            if std::fs::symlink_metadata(&sidecar).is_ok() {
                check_file(&sidecar)?;
            }
        }
        if directory.join(MARKER).try_exists()? {
            check_file(&directory.join(MARKER))?;
            // Validate read-only before SQLite can modify an initialized file.
            // A zero-byte truncated database otherwise looks like a healthy new DB.
            validate_snapshot(&path).await?;
        }
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(Duration::from_secs(5));
        // One writer serializes short business transactions. No inference or tool
        // work may run while a transaction holds this connection.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .with_context(|| {
                format!("opening {}; no automatic reset is allowed", path.display())
            })?;
        check_integrity(&pool).await?;
        MIGRATOR
            .run(&pool)
            .await
            .context("organization migration failed; preserve the database")?;
        let database_id: String =
            sqlx::query_scalar("SELECT database_id FROM organization_store WHERE singleton = 1")
                .fetch_one(&pool)
                .await?;
        ensure!(
            !database_id.is_empty(),
            "missing organization database identity"
        );
        if !directory.join(MARKER).try_exists()? {
            private_file(&directory.join(MARKER), true)?.sync_all()?;
            sync_directory(&directory)?;
        } else {
            check_file(&directory.join(MARKER))?;
        }
        Ok(Self { pool, directory })
    }

    pub fn path(&self) -> PathBuf {
        self.directory.join(DATABASE)
    }

    pub async fn database_id(&self) -> Result<String> {
        Ok(
            sqlx::query_scalar("SELECT database_id FROM organization_store WHERE singleton = 1")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// VACUUM INTO includes committed WAL contents and creates a consistent snapshot.
    /// Backups stay under the private state directory and never overwrite a file.
    pub async fn backup(&self) -> Result<PathBuf> {
        let backups = self.directory.join("backups");
        private_directory(&backups).await?;
        let staging = backups.join(format!("{}.partial", Uuid::now_v7()));
        private_file(&staging, true)?;
        let result = async {
            sqlx::query("VACUUM INTO ?")
                .bind(staging.to_str().context("backup path is not UTF-8")?)
                .execute(&self.pool)
                .await?;
            validate_snapshot(&staging).await?;
            private_file(&staging, false)?.sync_all()?;
            let destination = staging.with_extension("sqlite");
            tokio::fs::rename(&staging, &destination).await?;
            sync_directory(&backups)?;
            Ok(destination)
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&staging).await;
        }
        result
    }

    /// Restore only into a fresh organization directory. The caller must stop the
    /// old service and explicitly select the recovered home after verification.
    pub async fn restore_into(backup: &Path, sqlite_home: &Path) -> Result<Self> {
        validate_snapshot(backup).await?;
        tokio::fs::create_dir_all(sqlite_home).await?;
        let directory = sqlite_home.join(DIRECTORY);
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&directory)
            .context("restore destination must be a fresh organization directory")?;
        private_directory(&directory).await?;
        let staging = directory.join("restore.partial");
        let mut source = private_file(backup, false)?;
        let mut target = private_file(&staging, true)?;
        std::io::copy(&mut source, &mut target)?;
        target.sync_all()?;
        validate_snapshot(&staging).await?;
        // A concurrent service opening this fresh home must not be overwritten.
        // Publish without replacement, then remove the staging name so nlink is 1.
        tokio::fs::hard_link(&staging, directory.join(DATABASE)).await?;
        tokio::fs::remove_file(&staging).await?;
        sync_directory(&directory)?;
        Self::open(sqlite_home).await
    }
}

async fn check_integrity(pool: &SqlitePool) -> Result<()> {
    let checks: Vec<String> = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_all(pool)
        .await?;
    ensure!(
        checks == ["ok"],
        "organization database integrity failed: {checks:?}; do not reset"
    );
    let foreign_key_violation = sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(pool)
        .await?;
    ensure!(
        foreign_key_violation.is_none(),
        "organization foreign key integrity failed"
    );
    Ok(())
}

async fn validate_snapshot(path: &Path) -> Result<()> {
    check_file(path)?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .read_only(true)
                .create_if_missing(false)
                .foreign_keys(true),
        )
        .await?;
    let result = async {
        check_integrity(&pool).await?;
        // Validate every migration/checksum before copying. Never run migrations
        // against the source backup or accept a database from a newer binary.
        let applied: Vec<(i64, Vec<u8>, bool)> = sqlx::query_as(
            "SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&pool)
        .await?;
        ensure!(
            !applied.is_empty() && applied.len() <= MIGRATOR.migrations.len(),
            "backup migration set is incompatible"
        );
        for ((version, checksum, success), expected) in
            applied.iter().zip(MIGRATOR.migrations.iter())
        {
            ensure!(
                *version == expected.version
                    && checksum.as_slice() == expected.checksum.as_ref()
                    && *success,
                "backup migration checksum/version is incompatible"
            );
        }
        let _: String =
            sqlx::query_scalar("SELECT database_id FROM organization_store WHERE singleton = 1")
                .fetch_one(&pool)
                .await?;
        Ok(())
    }
    .await;
    pool.close().await;
    result
}

async fn private_directory(path: &Path) -> Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "unsafe organization directory"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o777 == 0o700,
                "organization directory must be owned by the current user with mode 0700"
            );
        }
    }
    codex_uds::prepare_private_socket_directory(path).await?;
    Ok(())
}

fn check_file(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "unsafe organization file: {}",
        path.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() }
                && metadata.mode() & 0o777 == 0o600
                && metadata.nlink() == 1,
            "organization file must be current-user owned, mode 0600, and not hard-linked: {}",
            path.display()
        );
    }
    Ok(())
}

fn private_file(path: &Path, create: bool) -> Result<File> {
    if !create {
        check_file(path)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(create).create_new(create);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = options.open(path)?;
    check_file(path)?;
    Ok(file)
}

fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "organization_tests.rs"]
mod tests;
