//! Offline recovery bundles. Credentials/configuration and original projects are
//! excluded. The owner must stop all runtime writers, not just organization work.
use super::*;
use codex_protocol::organization_execution::OrganizationTaskAttempt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::Component;

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
    executable: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    version: u32,
    database_id: String,
    source_home: String,
    source_sqlite_home: String,
    workspaces: BTreeMap<String, String>,
    files: Vec<Entry>,
}

fn relative(path: &str) -> Result<&Path> {
    let path = Path::new(path);
    ensure!(
        !path.as_os_str().is_empty()
            && path
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
        "unsafe bundle path"
    );
    ensure!(
        path.components().collect::<PathBuf>().as_os_str() == path.as_os_str(),
        "noncanonical bundle path"
    );
    Ok(path)
}

fn owned(path: &Path, directory: bool) -> Result<std::fs::Metadata> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        !metadata.file_type().is_symlink()
            && if directory {
                metadata.is_dir()
            } else {
                metadata.is_file()
            },
        "bundle source must be regular: {}",
        path.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() }
                && metadata.mode() & 0o022 == 0
                && (directory || metadata.nlink() == 1),
            "unsafe source ownership, permissions or hard link: {}",
            path.display()
        );
    }
    Ok(metadata)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let metadata = owned(path, false)?;
    ensure!(
        metadata.len() <= MAX_FILE_BYTES,
        "bundle file exceeds 64 MiB"
    );
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut bytes = Vec::new();
    options
        .open(path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_FILE_BYTES,
        "bundle file grew beyond 64 MiB"
    );
    Ok(bytes)
}

fn create_directory(path: &Path) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(path)
        .context("bundle/restore destination must be fresh")?;
    Ok(())
}

fn write(root: &Path, name: &str, bytes: &[u8], executable: bool) -> Result<()> {
    let target = root.join(relative(name)?);
    let parent = target.parent().context("bundle parent missing")?;
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(parent)?;
    // Every existing component must remain private and never be a symlink.
    let mut ancestor = parent;
    while ancestor.starts_with(root) {
        owned(ancestor, true)?;
        if ancestor == root {
            break;
        }
        ancestor = ancestor.parent().context("bundle parent missing")?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(if executable { 0o700 } else { 0o600 })
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut file = options.open(target)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn capture(root: &Path, source: &Path, name: &str, manifest: &mut Manifest) -> Result<()> {
    let bytes = read(source)?;
    let total = manifest
        .files
        .iter()
        .try_fold(bytes.len() as u64, |total, file| {
            total
                .checked_add(file.bytes)
                .context("bundle size overflow")
        })?;
    ensure!(
        manifest.files.len() < MAX_FILES && total <= MAX_BYTES,
        "bundle exceeds 10,000 files or 512 MiB"
    );
    let executable = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            owned(source, false)?.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            false
        }
    };
    write(root, name, &bytes, executable)?;
    manifest.files.push(Entry {
        path: name.into(),
        bytes: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        executable,
    });
    Ok(())
}

fn capture_tree(
    root: &Path,
    source: &Path,
    name: &str,
    manifest: &mut Manifest,
    depth: usize,
) -> Result<()> {
    ensure!(depth <= 32, "bundle directory nesting exceeds 32");
    owned(source, true)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let child = format!(
            "{name}/{}",
            entry
                .file_name()
                .to_str()
                .context("non-UTF-8 bundle filename")?
        );
        if entry.file_type()?.is_dir() {
            capture_tree(root, &entry.path(), &child, manifest, depth + 1)?;
        } else {
            capture(root, &entry.path(), &child, manifest)?;
        }
    }
    Ok(())
}

async fn sqlite_check(path: &Path) -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .read_only(true)
                .create_if_missing(false),
        )
        .await?;
    let result = check_integrity(&pool).await;
    pool.close().await;
    result
}

impl OrganizationStore {
    /// This lock excludes organization execution. All other runtime writers must
    /// also be stopped by the owner; this is not a hot cross-database snapshot.
    pub async fn backup_bundle(&self, codex_home: &Path, destination: &Path) -> Result<PathBuf> {
        ensure!(
            cfg!(target_os = "linux"),
            "recovery bundles are qualified on Linux"
        );
        let _owner = self.acquire_execution_owner()?;
        let codex_home = std::fs::canonicalize(codex_home)?;
        let sqlite_home =
            std::fs::canonicalize(self.directory.parent().context("SQLite home missing")?)?;
        owned(&codex_home, true)?;
        owned(&sqlite_home, true)?;
        let destination = if destination.is_absolute() {
            destination.to_owned()
        } else {
            std::env::current_dir()?.join(destination)
        };
        ensure!(
            !destination.starts_with(&codex_home) && !destination.starts_with(&sqlite_home),
            "bundle destination must be outside state homes"
        );
        create_directory(&destination)?;
        let destination = std::fs::canonicalize(destination)?;
        ensure!(
            !destination.starts_with(&codex_home) && !destination.starts_with(&sqlite_home),
            "bundle destination resolves inside a state home"
        );
        let mut manifest = Manifest {
            version: 1,
            database_id: self.database_id().await?,
            source_home: codex_home.to_str().context("non-UTF-8 home")?.into(),
            source_sqlite_home: sqlite_home
                .to_str()
                .context("non-UTF-8 SQLite home")?
                .into(),
            workspaces: BTreeMap::new(),
            files: Vec::new(),
        };
        let snapshot = self.backup().await?;
        capture(
            &destination,
            &snapshot,
            "organization/organization.sqlite",
            &mut manifest,
        )?;
        let config = crate::SqliteConfig::from_sqlite_home(
            codex_utils_absolute_path::AbsolutePathBuf::try_from(sqlite_home)?,
        );
        for db in config.runtime_db_paths() {
            if !db.path.try_exists()? {
                continue;
            }
            owned(&db.path, false)?;
            let temporary = destination.join(format!("{}.partial", Uuid::now_v7()));
            private_file(&temporary, true)?;
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&db.path)
                        .read_only(true)
                        .create_if_missing(false),
                )
                .await?;
            let result = sqlx::query("VACUUM INTO ?")
                .bind(temporary.to_str().context("non-UTF-8 staging")?)
                .execute(&pool)
                .await;
            pool.close().await;
            result?;
            sqlite_check(&temporary).await?;
            capture(
                &destination,
                &temporary,
                &format!(
                    "runtime/{}",
                    db.path
                        .file_name()
                        .context("database filename missing")?
                        .to_str()
                        .context("database filename not UTF-8")?
                ),
                &mut manifest,
            )?;
            std::fs::remove_file(temporary)?;
        }
        for directory in ["sessions", "archived_sessions"] {
            let path = codex_home.join(directory);
            if path.try_exists()? {
                capture_tree(&destination, &path, directory, &mut manifest, 0)?;
            }
        }
        let attempts: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE json_extract(data,'$.workspace') IS NOT NULL").fetch_all(&self.pool).await?;
        for data in attempts {
            let attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
            for base in &attempt.artifact_bases {
                let Some(hash) = &base.sha256 else {
                    continue;
                };
                ensure!(valid_base_hash(hash), "invalid captured artifact base hash");
                let name = format!("organization-artifact-bases/{hash}");
                let source = codex_home.join(&name);
                if manifest.files.iter().any(|f| f.path == name) {
                    continue;
                }
                // Historical attempts only captured hashes. Missing legacy bytes
                // stay unavailable; never infer a base from today's owner file.
                if !source.try_exists()? {
                    continue;
                }
                owned(&codex_home.join("organization-artifact-bases"), true)?;
                ensure!(
                    format!("{:x}", Sha256::digest(read(&source)?)) == *hash,
                    "artifact base integrity mismatch"
                );
                capture(&destination, &source, &name, &mut manifest)?;
            }
            let path = attempt.workspace.context("workspace missing")?;
            Uuid::parse_str(&attempt.id)?;
            ensure!(Path::new(&path).is_absolute(), "workspace must be absolute");
            capture_tree(
                &destination,
                Path::new(&path),
                &format!("workspaces/{}", attempt.id),
                &mut manifest,
                0,
            )?;
            manifest.workspaces.insert(attempt.id, path);
        }
        write(
            &destination,
            "manifest.json",
            &serde_json::to_vec_pretty(&manifest)?,
            false,
        )?;
        sync_directory(&destination)?;
        Ok(destination)
    }

    /// Verify the complete manifest before creating a fresh destination. Old
    /// grants, IDs, uncertain effects, accounting and Apply receipts remain intact.
    pub async fn restore_bundle(bundle: &Path, destination: &Path) -> Result<Self> {
        owned(bundle, true)?;
        let manifest: Manifest = serde_json::from_slice(&read(&bundle.join("manifest.json"))?)?;
        ensure!(
            manifest.version == 1
                && !manifest.files.is_empty()
                && manifest.files.len() <= MAX_FILES,
            "unsupported/empty/oversized bundle"
        );
        ensure!(
            Path::new(&manifest.source_home).is_absolute()
                && Path::new(&manifest.source_sqlite_home).is_absolute(),
            "invalid source homes"
        );
        let mut paths = BTreeSet::new();
        let mut total = 0u64;
        let runtime_names: BTreeSet<_> = crate::SqliteConfig::from_sqlite_home(
            codex_utils_absolute_path::AbsolutePathBuf::try_from(PathBuf::from(
                &manifest.source_sqlite_home,
            ))?,
        )
        .runtime_db_paths()
        .into_iter()
        .map(|db| {
            Ok(format!(
                "runtime/{}",
                db.path
                    .file_name()
                    .context("database filename missing")?
                    .to_string_lossy()
            ))
        })
        .collect::<Result<_>>()?;
        for entry in &manifest.files {
            let path = relative(&entry.path)?;
            ensure!(paths.insert(entry.path.clone()), "duplicate manifest path");
            ensure!(
                entry.path == "organization/organization.sqlite"
                    || entry
                        .path
                        .strip_prefix("organization-artifact-bases/")
                        .is_some_and(valid_base_hash)
                    || runtime_names.contains(&entry.path)
                    || entry.path.starts_with("sessions/")
                    || entry.path.starts_with("archived_sessions/")
                    || manifest
                        .workspaces
                        .keys()
                        .any(|id| entry.path.starts_with(&format!("workspaces/{id}/"))),
                "undeclared bundle content"
            );
            total = total
                .checked_add(entry.bytes)
                .context("bundle size overflow")?;
            ensure!(total <= MAX_BYTES, "bundle exceeds 512 MiB");
            let mut current = bundle.to_owned();
            for component in path.parent().context("file parent missing")?.components() {
                current.push(component);
                owned(&current, true)?;
            }
            let bytes = read(&bundle.join(path))?;
            ensure!(
                bytes.len() as u64 == entry.bytes
                    && format!("{:x}", Sha256::digest(&bytes)) == entry.sha256,
                "bundle integrity mismatch: {}",
                entry.path
            );
            if runtime_names.contains(&entry.path) {
                sqlite_check(&bundle.join(path)).await?;
            }
        }
        ensure!(
            paths.contains("organization/organization.sqlite"),
            "business snapshot missing"
        );
        validate_snapshot(&bundle.join("organization/organization.sqlite")).await?;
        for (id, path) in &manifest.workspaces {
            Uuid::parse_str(id)?;
            ensure!(
                Path::new(path).is_absolute(),
                "invalid workspace provenance"
            );
        }
        // Reject an inventory which omits a canonical attempt's workspace.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(bundle.join("organization/organization.sqlite"))
                    .read_only(true),
            )
            .await?;
        let identity: String = sqlx::query_scalar("SELECT database_id FROM organization_store")
            .fetch_one(&pool)
            .await?;
        let canonical: Vec<(String,String)> = sqlx::query_as("SELECT id,json_extract(data,'$.workspace') FROM organization_attempts WHERE json_extract(data,'$.workspace') IS NOT NULL ORDER BY id").fetch_all(&pool).await?;
        let attempts: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_attempts")
            .fetch_all(&pool)
            .await?;
        let mut bases = BTreeSet::new();
        for data in attempts {
            let attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
            bases.extend(attempt.artifact_bases.into_iter().filter_map(|b| b.sha256));
        }
        for entry in &manifest.files {
            if let Some(hash) = entry.path.strip_prefix("organization-artifact-bases/") {
                ensure!(
                    bases.contains(hash) && entry.sha256 == hash && entry.bytes <= 8 * 1024 * 1024,
                    "undeclared or changed captured artifact base"
                );
            }
        }
        pool.close().await;
        ensure!(
            identity == manifest.database_id
                && canonical.into_iter().collect::<BTreeMap<_, _>>() == manifest.workspaces,
            "bundle identity/workspace inventory mismatch"
        );
        create_directory(destination)?;
        let destination = std::fs::canonicalize(destination)?;
        for entry in &manifest.files {
            if entry.path == "organization/organization.sqlite" {
                continue;
            }
            let bytes = read(&bundle.join(relative(&entry.path)?))?;
            ensure!(
                bytes.len() as u64 == entry.bytes
                    && format!("{:x}", Sha256::digest(&bytes)) == entry.sha256,
                "bundle changed during restore"
            );
            let target = entry.path.strip_prefix("runtime/").unwrap_or(&entry.path);
            write(&destination, target, &bytes, entry.executable)?;
        }
        for id in manifest.workspaces.keys() {
            let path = destination.join("workspaces").join(id);
            if !path.exists() {
                let mut builder = std::fs::DirBuilder::new();
                builder.recursive(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                builder.create(path)?;
            }
        }
        let restored = Self::restore_into(
            &bundle.join("organization/organization.sqlite"),
            &destination,
        )
        .await?;
        let _owner = restored.acquire_execution_owner()?;
        let mut tx = restored.pool.begin_with("BEGIN IMMEDIATE").await?;
        for id in manifest.workspaces.keys() {
            let data: String =
                sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
            let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
            attempt.workspace = Some(
                destination
                    .join("workspaces")
                    .join(id)
                    .to_str()
                    .context("non-UTF-8 destination")?
                    .into(),
            );
            super::execution::write_attempt(&mut tx, &attempt).await?;
        }
        tx.commit().await?;
        // Native runtime indexes point at the relocated JSONLs/workspaces. Audit
        // text and project CWDs remain historical/owner originals.
        for entry in &manifest.files {
            if let Some(name) = entry.path.strip_prefix("runtime/") {
                let path = destination.join(name);
                let pool = SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect_with(SqliteConnectOptions::new().filename(&path))
                    .await?;
                for (pragma, column, select, update) in [
                    (
                        "PRAGMA table_info(threads)",
                        "rollout_path",
                        "SELECT DISTINCT rollout_path FROM threads",
                        "UPDATE threads SET rollout_path=? WHERE rollout_path=?",
                    ),
                    (
                        "PRAGMA table_info(threads)",
                        "cwd",
                        "SELECT DISTINCT cwd FROM threads",
                        "UPDATE threads SET cwd=? WHERE cwd=?",
                    ),
                    (
                        "PRAGMA table_info(rollout_migration_skipped_rollouts)",
                        "rollout_path",
                        "SELECT DISTINCT rollout_path FROM rollout_migration_skipped_rollouts",
                        "UPDATE rollout_migration_skipped_rollouts SET rollout_path=? WHERE rollout_path=?",
                    ),
                ] {
                    let columns = sqlx::query(pragma).fetch_all(&pool).await?;
                    if !columns
                        .iter()
                        .any(|row| row.get::<String, _>("name") == column)
                    {
                        continue;
                    }
                    let values: Vec<String> = sqlx::query_scalar(select).fetch_all(&pool).await?;
                    for value in values {
                        let rewritten = if column == "rollout_path" {
                            Path::new(&value)
                                .strip_prefix(&manifest.source_home)
                                .ok()
                                .map(|suffix| destination.join(suffix))
                        } else {
                            manifest.workspaces.iter().find_map(|(id, old)| {
                                Path::new(&value).strip_prefix(old).ok().map(|suffix| {
                                    let base = destination.join("workspaces").join(id);
                                    if suffix.as_os_str().is_empty() {
                                        base
                                    } else {
                                        base.join(suffix)
                                    }
                                })
                            })
                        };
                        if let Some(rewritten) = rewritten {
                            sqlx::query(update)
                                .bind(
                                    rewritten
                                        .to_str()
                                        .context("non-UTF-8 restored runtime path")?,
                                )
                                .bind(&value)
                                .execute(&pool)
                                .await?;
                        }
                    }
                }
                check_integrity(&pool).await?;
                pool.close().await;
            }
        }
        check_integrity(&restored.pool).await?;
        sync_directory(&destination)?;
        Ok(restored)
    }
}

fn valid_base_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
