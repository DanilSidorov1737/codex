use super::*;

#[cfg(unix)]
async fn retained_fixture(
    store: &OrganizationStore,
) -> Result<(String, OrganizationTaskAttempt, PathBuf)> {
    let (worker, attempt) = authorized_attempt(store, false).await?;
    let parent =
        std::env::temp_dir().join(format!("codex-organization-{}", unsafe { libc::geteuid() }));
    private_directory(&parent).await?;
    let path = parent.join(&attempt.id);
    private_directory(&path).await?;
    let mut file = private_file(&path.join("input.txt"), true)?;
    file.write_all(b"retained input")?;
    file.sync_all()?;
    store
        .bind_attempt_runtime(
            &attempt.id,
            &worker,
            &OrganizationRuntimeBinding {
                workspace: path.to_string_lossy().into_owned(),
                turn_id: None,
                artifact_bases: vec![OrganizationArtifactBase {
                    path: "report.txt".into(),
                    sha256: None,
                    captured: false,
                }],
            },
            1001,
        )
        .await?;
    Ok((worker, attempt, path))
}

#[cfg(unix)]
#[tokio::test]
async fn retention_preserves_active_unknown_and_base_referenced_workspaces() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt, path) = retained_fixture(&store).await?;
    let project = store
        .execution_view(&attempt.task_id)
        .await?
        .task
        .project_id;
    assert!(
        store
            .retention_preview(&project, 2000)
            .await?
            .candidates
            .is_empty()
    );
    store
        .admit_model_request(&attempt.id, &worker, 1002)
        .await?;
    store.fail_attempt(&attempt.id, &worker).await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    assert!(
        store
            .retention_preview(&project, 2000)
            .await?
            .candidates
            .is_empty()
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 3).await?;
    assert_eq!(
        store
            .retention_preview(&project, 2000)
            .await?
            .candidates
            .len(),
        1
    );
    let mut data = store.attempt_snapshot(&attempt.id).await?;
    data.artifact_bases[0].sha256 = Some("a".repeat(64));
    let mut tx = store.pool.begin().await?;
    super::super::execution::write_attempt(&mut tx, &data).await?;
    tx.commit().await?;
    assert!(
        store
            .retention_preview(&project, 2000)
            .await?
            .candidates
            .is_empty()
    );
    std::fs::remove_dir_all(path)?;
    store.close().await;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn exact_retention_preview_refuses_drift_and_archives_without_ledger_reset() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt, path) = retained_fixture(&store).await?;
    store.fail_attempt(&attempt.id, &worker).await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let plan = store
        .retention_preview(&before.task.project_id, 2000)
        .await?;
    assert_eq!(plan.candidates.len(), 1);
    std::fs::write(path.join("input.txt"), b"drift")?;
    assert!(store.archive_workspaces(&plan).await.is_err());
    assert!(path.exists());
    std::fs::write(path.join("input.txt"), b"retained input")?;
    store.archive_workspaces(&plan).await?;
    let after = store.execution_view(&attempt.task_id).await?;
    assert_eq!(after.authorization, before.authorization);
    assert_eq!(
        after.authorization_model_requests,
        before.authorization_model_requests
    );
    assert_eq!(after.task, before.task);
    assert!(!path.exists());
    let target = PathBuf::from(after.attempts[0].workspace.as_ref().unwrap());
    assert_eq!(std::fs::read(target.join("input.txt"))?, b"retained input");
    assert!(store.archive_workspaces(&plan).await.is_err());
    store.reconcile_workspace_archive(&attempt.id).await?;
    std::fs::remove_dir_all(target)?;
    store.close().await;
    Ok(())
}
