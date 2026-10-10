use super::*;
use codex_protocol::organization::*;

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
async fn concurrent_restores_never_replace_the_winning_destination() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let identity = store.database_id().await?;
    let backup = store.backup().await?;
    let recovery = Fixture::new();
    let (first, second) = tokio::join!(
        OrganizationStore::restore_into(&backup, &recovery.0),
        OrganizationStore::restore_into(&backup, &recovery.0)
    );
    assert_ne!(
        first.is_ok(),
        second.is_ok(),
        "exactly one restore owns the destination"
    );
    let restored = first.or(second)?;
    assert_eq!(restored.database_id().await?, identity);
    check_file(&restored.path())?;
    restored.close().await;
    store.close().await;
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

fn node_mutation(
    label: &str,
    parent_id: Option<String>,
    kind: OrganizationNodeKind,
) -> OrganizationMutation {
    OrganizationMutation::PutNode {
        id: None,
        expected_revision: None,
        parent_id,
        kind,
        label: label.into(),
        role_version_id: None,
        model: None,
        enabled: true,
    }
}

fn node_from(record: Option<OrganizationRecord>) -> OrganizationNode {
    let Some(OrganizationRecord::Node(node)) = record else {
        panic!("expected node");
    };
    node
}

#[tokio::test]
async fn arbitrary_forest_memberships_revision_race_and_cycles() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let solo = node_from(
        store
            .mutate_owner(
                node_mutation("Solo", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let group = node_from(
        store
            .mutate_owner(
                node_mutation("Research collective", None, OrganizationNodeKind::Group),
                None,
            )
            .await?,
    );
    let child = node_from(
        store
            .mutate_owner(
                node_mutation(
                    "任意 role",
                    Some(group.id.clone()),
                    OrganizationNodeKind::Agent,
                ),
                None,
            )
            .await?,
    );
    store
        .mutate_owner(
            OrganizationMutation::SetMembership {
                node_id: solo.id.clone(),
                group_id: group.id.clone(),
                present: true,
            },
            None,
        )
        .await?;
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::PutNode {
                    id: Some(group.id.clone()),
                    expected_revision: Some(1),
                    parent_id: Some(child.id.clone()),
                    kind: group.kind,
                    label: group.label.clone(),
                    role_version_id: None,
                    model: None,
                    enabled: true
                },
                None
            )
            .await
            .is_err()
    );
    let rename = OrganizationMutation::PutNode {
        id: Some(solo.id.clone()),
        expected_revision: Some(1),
        parent_id: None,
        kind: solo.kind,
        label: "Renamed standalone".into(),
        role_version_id: None,
        model: None,
        enabled: true,
    };
    let (first, second) = tokio::join!(
        store.mutate_owner(rename.clone(), None),
        store.mutate_owner(rename, None)
    );
    assert_ne!(
        first.is_ok(),
        second.is_ok(),
        "only one expected-revision edit may win"
    );
    let backup = store.backup().await?;
    let restored_home = Fixture::new();
    let restored = OrganizationStore::restore_into(&backup, &restored_home.0).await?;
    let (nodes, _) = restored
        .list_definitions(OrganizationRecordKind::Nodes, None, 200)
        .await?;
    assert_eq!(nodes.len(), 3);
    assert!(nodes.iter().any(|record| matches!(record, OrganizationRecord::Node(node) if node.id == solo.id && node.revision == 2)));
    let (members, _) = restored
        .list_definitions(OrganizationRecordKind::Memberships, None, 200)
        .await?;
    assert_eq!(members.len(), 1);
    assert!(
        restored
            .list_definitions(OrganizationRecordKind::Nodes, None, 0)
            .await
            .is_err()
    );
    restored.close().await;
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn role_versions_are_immutable_and_paginated() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let first = store
        .mutate_owner(
            OrganizationMutation::CreateRoleVersion {
                role_id: None,
                expected_version: None,
                name: "My role".into(),
                instructions: "Read local materials".into(),
            },
            None,
        )
        .await?;
    let Some(OrganizationRecord::Role(first)) = first else {
        panic!("expected role");
    };
    let second = OrganizationMutation::CreateRoleVersion {
        role_id: Some(first.role_id.clone()),
        expected_version: Some(1),
        name: "My changed role".into(),
        instructions: "Write a report".into(),
    };
    store.mutate_owner(second.clone(), None).await?;
    assert!(store.mutate_owner(second, None).await.is_err());
    let (page, cursor) = store
        .list_definitions(OrganizationRecordKind::Roles, None, 1)
        .await?;
    assert_eq!(page, vec![OrganizationRecord::Role(first)]);
    let (page, cursor) = store
        .list_definitions(OrganizationRecordKind::Roles, cursor, 1)
        .await?;
    assert!(matches!(&page[0], OrganizationRecord::Role(role) if role.version == 2));
    assert!(cursor.is_none());
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn goal_project_task_graph_validation_is_atomic_and_durable() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let agent = node_from(
        store
            .mutate_owner(
                node_mutation("Documenter", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let goal = store
        .mutate_owner(
            OrganizationMutation::PutGoal {
                id: None,
                expected_revision: None,
                parent_id: None,
                objective: "Explain supplied documents".into(),
                criteria: vec!["Report cites the supplied sources".into()],
            },
            None,
        )
        .await?;
    let Some(OrganizationRecord::Goal(goal)) = goal else {
        panic!("expected goal");
    };
    let link = OrganizationMutation::LinkProject {
        project_id: "existing-project-identity".into(),
        expected_revision: None,
        goal_id: Some(goal.id.clone()),
        owner_id: Some(agent.id.clone()),
        criteria: goal.criteria.clone(),
    };
    assert!(store.mutate_owner(link.clone(), None).await.is_err());
    store
        .mutate_owner(
            link,
            Some((
                "Actual project".into(),
                vec![home.0.to_string_lossy().into()],
            )),
        )
        .await?;
    let task = OrganizationMutation::PutTask {
        id: None,
        expected_revision: None,
        project_id: "existing-project-identity".into(),
        goal_id: Some(goal.id),
        owner_id: agent.id,
        instructions: "Write the document".into(),
        criteria: vec!["The output file exists".into()],
        prerequisites: Vec::new(),
    };
    let first = store.mutate_owner(task.clone(), None).await?;
    let Some(OrganizationRecord::Task(first)) = first else {
        panic!("expected task");
    };
    let OrganizationMutation::PutTask {
        project_id,
        goal_id,
        owner_id,
        instructions,
        criteria,
        ..
    } = task
    else {
        unreachable!();
    };
    let second = store
        .mutate_owner(
            OrganizationMutation::PutTask {
                id: None,
                expected_revision: None,
                project_id: project_id.clone(),
                goal_id: goal_id.clone(),
                owner_id: owner_id.clone(),
                instructions: "Review the document".into(),
                criteria: criteria.clone(),
                prerequisites: vec![first.id.clone()],
            },
            None,
        )
        .await?;
    let Some(OrganizationRecord::Task(second)) = second else {
        panic!("expected task");
    };
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::PutTask {
                    id: Some(first.id.clone()),
                    expected_revision: Some(1),
                    project_id,
                    goal_id,
                    owner_id,
                    instructions,
                    criteria,
                    prerequisites: vec![second.id]
                },
                None
            )
            .await
            .is_err()
    );
    let (tasks, _) = store
        .list_definitions(OrganizationRecordKind::Tasks, None, 200)
        .await?;
    assert_eq!(tasks.len(), 2);
    assert!(tasks.contains(&OrganizationRecord::Task(first)));
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_audit")
        .fetch_one(&store.pool)
        .await?;
    assert_eq!(
        audits, 5,
        "rejected edits must not partially commit business changes"
    );
    store.close().await;
    Ok(())
}
