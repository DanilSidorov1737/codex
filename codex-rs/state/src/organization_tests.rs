use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("codex-organization-{}", Uuid::now_v7())))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn durable_private_store_and_verified_restore() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let identity = store.database_id().await?;
    let sync: i64 = sqlx::query_scalar("PRAGMA synchronous")
        .fetch_one(&store.pool)
        .await?;
    assert_eq!(sync, 2);
    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&store.pool)
        .await?;
    assert_eq!(journal, "wal");
    check_file(&store.path())?;
    let backup = store.backup().await?;
    store.close().await;
    let reopened = OrganizationStore::open(&home.0).await?;
    assert_eq!(reopened.database_id().await?, identity);
    let recovery = Fixture::new();
    let restored = OrganizationStore::restore_into(&backup, &recovery.0).await?;
    assert_eq!(restored.database_id().await?, identity);
    assert!(
        OrganizationStore::restore_into(&backup, &recovery.0)
            .await
            .is_err()
    );
    restored.close().await;
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn corruption_and_missing_database_never_reset_state() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let path = store.path();
    store.close().await;
    std::fs::write(&path, b"corrupt canonical organization state")?;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    assert_eq!(
        std::fs::read(&path)?,
        b"corrupt canonical organization state"
    );
    std::fs::write(&path, b"")?;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    let empty = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&path).read_only(true))
        .await?;
    let tables: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE name = 'organization_store'")
            .fetch_one(&empty)
            .await?;
    assert_eq!(tables, 0, "a truncated store must not be reinitialized");
    empty.close().await;
    std::fs::remove_file(&path)?;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    assert!(!path.exists());
    Ok(())
}

#[tokio::test]
async fn incompatible_migration_and_bad_backup_are_rejected() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    sqlx::query("UPDATE _sqlx_migrations SET checksum = x'00'")
        .execute(&store.pool)
        .await?;
    assert!(store.backup().await.is_err());
    store.close().await;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    Ok(())
}

#[tokio::test]
async fn full_database_rolls_back_without_reset() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let id = store.database_id().await?;
    let pages: i64 = sqlx::query_scalar("PRAGMA page_count")
        .fetch_one(&store.pool)
        .await?;
    // PRAGMA assignment cannot bind; pages is an i64 read from SQLite, never input SQL.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "PRAGMA max_page_count = {pages}"
    )))
    .execute(&store.pool)
    .await?;
    let mut transaction = store.pool.begin().await?;
    let result = sqlx::query("UPDATE organization_store SET database_id = ?")
        .bind("x".repeat(1024 * 1024))
        .execute(&mut *transaction)
        .await;
    let error = result.expect_err("the real SQLite page limit must reject this write");
    assert!(
        error
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("13")),
        "{error}"
    );
    let _ = transaction.rollback().await;
    assert_eq!(store.database_id().await?, id);
    check_integrity(&store.pool).await?;
    store.close().await;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn unsafe_files_and_directories_are_rejected_without_chmod() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::symlink;
    let home = Fixture::new();
    std::fs::create_dir_all(&home.0)?;
    let outside = home.0.join("outside");
    std::fs::create_dir(&outside)?;
    symlink(&outside, home.0.join(DIRECTORY))?;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    std::fs::remove_file(home.0.join(DIRECTORY))?;
    std::fs::create_dir(home.0.join(DIRECTORY))?;
    std::fs::set_permissions(
        home.0.join(DIRECTORY),
        std::fs::Permissions::from_mode(0o755),
    )?;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    std::fs::set_permissions(
        home.0.join(DIRECTORY),
        std::fs::Permissions::from_mode(0o700),
    )?;
    let target = outside.join("state");
    std::fs::write(&target, b"keep human data")?;
    symlink(&target, home.0.join(DIRECTORY).join(DATABASE))?;
    assert!(OrganizationStore::open(&home.0).await.is_err());
    assert_eq!(std::fs::read(target)?, b"keep human data");
    Ok(())
}
