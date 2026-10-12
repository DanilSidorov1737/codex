use super::*;
use codex_protocol::organization::*;
use codex_protocol::organization_execution::*;
use std::io::Write;

#[path = "organization_coordination_tests.rs"]
mod coordination;

#[path = "organization_workflow_tests.rs"]
mod workflow;

#[path = "organization_handoff_tests.rs"]
mod handoff;

#[path = "organization_report_tests.rs"]
mod report;

#[path = "organization_preset_tests.rs"]
mod presets;

#[path = "organization_control_tests.rs"]
mod controls;

async fn remove_controls_migration_for_legacy_fixture(store: &OrganizationStore) -> Result<()> {
    for query in [
        "DROP VIEW organization_global_usage",
        "DROP TABLE organization_control_history",
        "DROP TABLE organization_controls",
        "ALTER TABLE organization_tasks DROP COLUMN queue_since",
    ] {
        sqlx::query(query).execute(&store.pool).await?;
    }
    Ok(())
}

async fn authorized_attempt(
    store: &OrganizationStore,
    approval: bool,
) -> Result<(String, OrganizationTaskAttempt)> {
    authorized_attempt_with_limit(store, approval, 2).await
}

async fn authorized_attempt_with_limit(
    store: &OrganizationStore,
    approval: bool,
    max_active: u32,
) -> Result<(String, OrganizationTaskAttempt)> {
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Executor", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(store, &node.id).await?;
    let task = execution_task(store, &node.id, &project).await?;
    let mut scope = execution_scope();
    scope.require_tool_approval = approval;
    store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, max_active)
        .await?
        .unwrap();
    Ok((worker, attempt))
}

#[tokio::test]
async fn pending_approval_cannot_dispatch_after_reported_budget_exhaustion() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    assert!(
        store
            .prepare_action(
                &attempt.id,
                &worker,
                "obsolete",
                "shell_command",
                "{}",
                1001
            )
            .await
            .is_err()
    );
    let (sequence, _) = store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    let action = store
        .prepare_action(&attempt.id, &worker, "current", "exec_command", "{}", 1001)
        .await?;
    store
        .checkpoint_usage(&attempt.id, &worker, sequence, execution_scope().max_tokens)
        .await?;
    store
        .decide_approval(
            action.approval_id.as_ref().unwrap(),
            &action.action_hash,
            true,
            1002,
        )
        .await?;
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1002, 2)
            .await
            .is_err()
    );
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.actions[0].state, OrganizationActionState::Pending);
    assert_eq!(view.task.state, "waitingApproval");
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn exact_approval_single_dispatch_and_capacity_reacquisition() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    let action = store
        .prepare_action(
            &attempt.id,
            &worker,
            "call-1",
            "exec_command",
            "{\"command\":\"echo hello > report.txt\"}",
            1001,
        )
        .await?;
    assert!(!store.dispatch_action(&action.id, &worker, 1001, 1).await?);
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1001)
            .await
            .is_err()
    );
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.task.state, "waitingApproval");
    assert_eq!(view.approvals[0].arguments, action.arguments);
    assert!(
        store
            .decide_approval(
                action.approval_id.as_ref().unwrap(),
                "wrong-hash",
                true,
                1001
            )
            .await
            .is_err()
    );
    assert!(
        store
            .prepare_action(
                &attempt.id,
                &worker,
                "call-1",
                "exec_command",
                "{\"command\":\"different\"}",
                1001
            )
            .await
            .is_err()
    );
    let same = store
        .prepare_action(
            &attempt.id,
            &worker,
            "call-1",
            "exec_command",
            &action.arguments,
            1001,
        )
        .await?;
    assert_eq!(same.id, action.id);
    store
        .decide_approval(
            action.approval_id.as_ref().unwrap(),
            &action.action_hash,
            true,
            1002,
        )
        .await?;
    assert!(
        store
            .decide_approval(
                action.approval_id.as_ref().unwrap(),
                &action.action_hash,
                true,
                1002
            )
            .await
            .is_err()
    );
    // Waiting released the only running reservation; reacquisition respects it.
    let (other_worker, other) = authorized_attempt(&store, false).await?;
    assert!(!store.dispatch_action(&action.id, &worker, 1002, 1).await?);
    store.fail_attempt(&other.id, &other_worker).await?;
    assert!(
        !store.dispatch_action(&action.id, &worker, 1002, 1).await?,
        "failure alone does not prove the native runtime has stopped"
    );
    store
        .release_attempt_capacity(&other.id, &other_worker)
        .await?;
    assert!(store.dispatch_action(&action.id, &worker, 1002, 1).await?);
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1002, 1)
            .await
            .is_err()
    );
    store
        .finish_action(&attempt.id, &worker, "call-1", true, "exit=0")
        .await?;
    store
        .finish_action(&attempt.id, &worker, "call-1", true, "exit=0")
        .await?;
    assert!(
        store
            .prepare_action(
                &attempt.id,
                &worker,
                "call-1",
                "exec_command",
                &action.arguments,
                1003
            )
            .await
            .is_err()
    );
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.actions[0].state,
        OrganizationActionState::Completed
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn pause_restart_and_expiry_never_accept_or_replay_old_actions() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    let action = store
        .prepare_action(
            &attempt.id,
            &worker,
            "pending",
            "apply_patch",
            "*** Begin Patch\n*** End Patch",
            1001,
        )
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    store
        .stop_task(&attempt.task_id, view.task.revision, false)
        .await?;
    assert!(
        store
            .decide_approval(
                action.approval_id.as_ref().unwrap(),
                &action.action_hash,
                true,
                1002
            )
            .await
            .is_err()
    );
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1002, 2)
            .await
            .is_err()
    );
    assert!(
        store
            .heartbeat_attempt(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    let view = store.execution_view(&attempt.task_id).await?;
    assert!(view.authorization.unwrap().revoked);
    assert_eq!(
        view.approvals[0].state,
        OrganizationApprovalState::Superseded
    );
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let action = store
        .prepare_action(
            &attempt.id,
            &worker,
            "dispatched",
            "exec_command",
            "{}",
            1002,
        )
        .await?;
    assert!(store.dispatch_action(&action.id, &worker, 1002, 2).await?);
    assert_eq!(store.reconcile_interrupted_attempts().await?, 1);
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.actions[0].state,
        OrganizationActionState::Uncertain
    );
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1003, 2)
            .await
            .is_err()
    );
    // A late receipt records an observation without reviving execution.
    store
        .finish_action(
            &attempt.id,
            &worker,
            "dispatched",
            true,
            "observed completion",
        )
        .await?;
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.task.state,
        "needsReconciliation"
    );
    let backup = store.backup().await?;
    let recovery = Fixture::new();
    let restored = OrganizationStore::restore_into(&backup, &recovery.0).await?;
    assert_eq!(
        restored.execution_view(&attempt.task_id).await?.actions[0]
            .receipt
            .as_deref(),
        Some("observed completion")
    );
    restored.close().await;
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn pending_approval_survives_restart_but_requires_reconciliation() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    let action = store
        .prepare_action(&attempt.id, &worker, "pending", "exec_command", "{}", 1001)
        .await?;
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.reconcile_interrupted_attempts().await?, 1);
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.approvals[0].state, OrganizationApprovalState::Pending);
    assert_eq!(view.approvals[0].action_hash, action.action_hash);
    assert!(
        store
            .decide_approval(
                action.approval_id.as_ref().unwrap(),
                &action.action_hash,
                true,
                1002
            )
            .await
            .is_err()
    );
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1002, 2)
            .await
            .is_err()
    );
    assert_eq!(store.reconcile_interrupted_attempts().await?, 0);
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn action_limit_bounds_private_review_payloads() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    for index in 0..100 {
        let call = format!("bounded-{index}");
        let action = store
            .prepare_action(&attempt.id, &worker, &call, "exec_command", "{}", 1001)
            .await?;
        assert!(store.dispatch_action(&action.id, &worker, 1001, 2).await?);
        store
            .finish_action(&attempt.id, &worker, &call, true, "exit=0")
            .await?;
    }
    assert!(
        store
            .prepare_action(
                &attempt.id,
                &worker,
                "over-limit",
                "exec_command",
                "{}",
                1001
            )
            .await
            .is_err()
    );
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.actions.len(),
        100
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_actions")
        .fetch_one(&store.pool)
        .await?;
    assert_eq!(count, 100);
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn waiting_approval_keeps_capacity_until_inference_completion() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store
        .prepare_action(
            &attempt.id,
            &worker,
            "streamed-tool",
            "exec_command",
            "{}",
            1001,
        )
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert!(view.attempts[0].runtime_active);
    let child = execution_task(&store, &view.task.owner_id, &view.task.project_id).await?;
    store
        .authorize_task(&child.id, child.revision, execution_scope(), 1001)
        .await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 1)
            .await?
            .is_none()
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 20).await?;
    assert!(!store.execution_view(&attempt.task_id).await?.attempts[0].runtime_active);
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 1)
            .await?
            .is_some()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn paused_runtime_must_be_joined_before_new_authorization() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    store
        .stop_task(&attempt.task_id, view.task.revision, false)
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert!(view.attempts[0].runtime_active);
    assert!(
        store
            .authorize_task(
                &attempt.task_id,
                view.task.revision,
                execution_scope(),
                1002
            )
            .await
            .is_err()
    );
    assert!(
        store
            .release_attempt_capacity(&attempt.id, &Uuid::now_v7().to_string())
            .await
            .is_err()
    );
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    store
        .authorize_task(
            &attempt.task_id,
            view.task.revision,
            execution_scope(),
            1002,
        )
        .await?;
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn artifact_bases_are_frozen_with_runtime_binding() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let binding = OrganizationRuntimeBinding {
        workspace: "/tmp/isolated-task".into(),
        turn_id: None,
        artifact_bases: vec![OrganizationArtifactBase {
            captured: false,
            path: "report.txt".into(),
            sha256: None,
        }],
    };
    store
        .bind_attempt_runtime(&attempt.id, &worker, &binding, 1001)
        .await?;
    let mut changed = binding.clone();
    changed.artifact_bases[0].sha256 = Some("c".repeat(64));
    assert!(
        store
            .bind_attempt_runtime(&attempt.id, &worker, &changed, 1001)
            .await
            .is_err()
    );
    changed = binding.clone();
    changed.turn_id = Some("turn-identity".into());
    store
        .bind_attempt_runtime(&attempt.id, &worker, &changed, 1001)
        .await?;
    assert!(
        store
            .bind_attempt_runtime(&attempt.id, &worker, &binding, 1001)
            .await
            .is_err()
    );
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.attempts[0].artifact_bases,
        binding.artifact_bases
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn host_evidence_and_owner_digest_gate_result_acceptance() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    let scope = view.authorization.unwrap().scope;
    let artifacts: Vec<_> = scope
        .artifacts
        .iter()
        .map(|path| OrganizationArtifactEvidence {
            path: path.clone(),
            sha256: "a".repeat(64),
            bytes: 12,
        })
        .collect();
    let checks: Vec<_> = scope
        .checks
        .iter()
        .map(|argv| OrganizationCheckEvidence {
            immutable_inputs: None,
            argv: argv.clone(),
            exit_code: Some(0),
            output: "actual host output".into(),
            truncated: false,
        })
        .collect();
    let result = store
        .record_result(&attempt.id, &worker, artifacts, checks, 1001)
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.task.state, "needsReview");
    assert!(
        store
            .accept_result(&attempt.task_id, view.task.revision, "wrong-digest")
            .await
            .is_err()
    );
    assert!(
        store
            .accept_result(&attempt.task_id, view.task.revision - 1, &result.digest)
            .await
            .is_err()
    );
    store
        .accept_result(&attempt.task_id, view.task.revision, &result.digest)
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert!(view.result.unwrap().accepted);
    assert_eq!(view.task.state, "accepted");
    assert!(
        store
            .begin_application(
                &attempt.task_id,
                view.task.revision - 1,
                &result.digest,
                None
            )
            .await
            .is_err()
    );
    assert!(
        store
            .begin_application(&attempt.task_id, view.task.revision, "changed-digest", None)
            .await
            .is_err()
    );
    let selected = vec![result.artifacts[0].path.clone()];
    for invalid in [
        vec![],
        vec!["unknown-file".into()],
        vec![selected[0].clone(); 2],
    ] {
        assert!(
            store
                .begin_application(
                    &attempt.task_id,
                    view.task.revision,
                    &result.digest,
                    Some(&invalid)
                )
                .await
                .is_err()
        );
    }
    assert!(
        store
            .execution_view(&attempt.task_id)
            .await?
            .applications
            .is_empty()
    );
    let hunk = codex_protocol::organization_execution::OrganizationHunkSelection {
        path: selected[0].clone(),
        indices: vec![0],
        patch_sha256: "a".repeat(64),
        output_sha256: "b".repeat(64),
    };
    let mut invalid_hunk = hunk.clone();
    invalid_hunk.indices = vec![0, 0];
    assert!(
        store
            .begin_application_hunks(
                &attempt.task_id,
                view.task.revision,
                &result.digest,
                Some(&selected),
                Some(&[invalid_hunk])
            )
            .await
            .is_err()
    );
    let application = store
        .begin_application_hunks(
            &attempt.task_id,
            view.task.revision,
            &result.digest,
            Some(&selected),
            Some(std::slice::from_ref(&hunk)),
        )
        .await?;
    assert_eq!(application.hunks, Some(vec![hunk]));
    assert_eq!(application.paths, selected);
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.applications[0].state,
        OrganizationApplicationState::Pending
    );
    store.reconcile_interrupted_attempts().await?;
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.applications[0].state,
        OrganizationApplicationState::Uncertain
    );
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.applications[0].paths,
        selected
    );
    assert!(
        store
            .begin_application(&attempt.task_id, view.task.revision, &result.digest, None)
            .await
            .is_err()
    );
    assert!(
        store
            .settle_application(&application.id, true, "late stale completion")
            .await
            .is_err()
    );
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let scope = store
        .execution_view(&attempt.task_id)
        .await?
        .authorization
        .unwrap()
        .scope;
    let artifacts: Vec<_> = scope
        .artifacts
        .iter()
        .map(|path| OrganizationArtifactEvidence {
            path: path.clone(),
            sha256: "b".repeat(64),
            bytes: 8,
        })
        .collect();
    let checks: Vec<_> = scope
        .checks
        .iter()
        .map(|argv| OrganizationCheckEvidence {
            immutable_inputs: None,
            argv: argv.clone(),
            exit_code: Some(1),
            output: "regression failed".into(),
            truncated: false,
        })
        .collect();
    assert!(!checks.is_empty());
    assert!(
        store
            .record_result(&attempt.id, &worker, Vec::new(), checks.clone(), 1001)
            .await
            .is_err()
    );
    let result = store
        .record_result(&attempt.id, &worker, artifacts, checks, 1001)
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert!(
        store
            .accept_result(&attempt.task_id, view.task.revision, &result.digest)
            .await
            .is_err()
    );
    store
        .request_changes(
            &attempt.task_id,
            view.task.revision,
            "Fix the regression before review",
        )
        .await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.task.state, "paused");
    assert!(
        view.task
            .instructions
            .contains("Fix the regression before review")
    );
    assert!(
        store
            .accept_result(&attempt.task_id, view.task.revision, &result.digest)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}

struct Fixture(PathBuf);

#[tokio::test]
async fn execution_owner_lock_prevents_second_service_reconciliation() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let first = store.acquire_execution_owner()?;
    let other = OrganizationStore::open(&home.0).await?;
    assert!(other.acquire_execution_owner().is_err());
    drop(first);
    let second = other.acquire_execution_owner()?;
    assert!(store.acquire_execution_owner().is_err());
    drop(second);
    other.close().await;
    store.close().await;
    Ok(())
}

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
async fn reviewed_autonomy_floor_refuses_older_runtime_without_changing_grants() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (_, attempt) = authorized_attempt(&store, false).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let legacy = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Borrowed(&super::MIGRATOR.migrations[..14]),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    assert!(matches!(
        legacy.run(&store.pool).await,
        Err(sqlx::migrate::MigrateError::VersionMissing(15))
    ));
    assert_eq!(store.execution_view(&attempt.task_id).await?, before);
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

fn execution_scope() -> OrganizationExecutionScope {
    OrganizationExecutionScope {
        input_files: vec!["input.txt".into(), "oracle.py".into()],
        immutable_files: vec!["oracle.py".into()],
        artifacts: vec!["report.txt".into()],
        checks: vec![vec!["python3".into(), "oracle.py".into()]],
        model: "fixture".into(),
        max_tokens: 100,
        max_seconds: 120,
        max_model_requests: 2,
        max_attempts: 1,
        require_tool_approval: true,
        delegates: Vec::new(),
        max_children: 0,
        auto_accept_children: false,
        not_before: None,
    }
}

async fn execution_task(
    store: &OrganizationStore,
    owner: &str,
    project_id: &str,
) -> Result<OrganizationTask> {
    let record = store
        .mutate_owner(
            OrganizationMutation::PutTask {
                id: None,
                expected_revision: None,
                project_id: project_id.into(),
                goal_id: None,
                owner_id: owner.into(),
                instructions: "Prepare a checked report".into(),
                criteria: vec!["Owner accepts the report".into()],
                prerequisites: Vec::new(),
            },
            None,
        )
        .await?;
    let Some(OrganizationRecord::Task(task)) = record else {
        panic!("expected task");
    };
    Ok(task)
}

async fn execution_project(store: &OrganizationStore, owner: &str) -> Result<String> {
    let id = Uuid::now_v7().to_string();
    store
        .mutate_owner(
            OrganizationMutation::LinkProject {
                project_id: id.clone(),
                expected_revision: None,
                goal_id: None,
                owner_id: Some(owner.into()),
                criteria: vec!["Owner accepts the result".into()],
            },
            Some(("Fixture".into(), vec!["/tmp/fixture-project".into()])),
        )
        .await?;
    Ok(id)
}

#[tokio::test]
async fn execution_claim_race_reserves_capacity_once() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("One", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &node.id).await?;
    for _ in 0..3 {
        let task = execution_task(&store, &node.id, &project).await?;
        store
            .authorize_task(&task.id, task.revision, execution_scope(), 1000)
            .await?;
    }
    let worker = Uuid::now_v7().to_string();
    let a = Uuid::now_v7().to_string();
    let b = Uuid::now_v7().to_string();
    let (a, b) = tokio::join!(
        store.claim_task(&worker, &a, 1000, 1),
        store.claim_task(&worker, &b, 1000, 1)
    );
    assert_ne!(a?.is_some(), b?.is_some());
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM organization_attempts),(SELECT count(*) FROM organization_outbox WHERE kind='attempt/claimed'),(SELECT count(*) FROM organization_audit WHERE operation='attempt/claimed')").fetch_one(&store.pool).await?;
    assert_eq!(
        counts,
        (1, 1, 1),
        "claim, reservation, audit and outbox commit together"
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn execution_budget_accounts_usage_once_and_blocks_unreported_work() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("One", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &project).await?;
    let authorization = store
        .authorize_task(&task.id, task.revision, execution_scope(), 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    assert_eq!(
        store
            .admit_model_request(&attempt.id, &worker, 1001)
            .await?,
        (1, 0)
    );
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1001)
            .await
            .is_err()
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 60).await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 60).await?;
    assert_eq!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await?,
        (2, 60)
    );
    store.checkpoint_usage(&attempt.id, &worker, 2, 101).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.authorization_used_tokens, 101);
    assert_eq!(view.authorization_model_requests, 2);
    assert!(
        store
            .prepare_action(
                &attempt.id,
                &worker,
                "after-overage",
                "exec_command",
                "{}",
                1003
            )
            .await
            .is_err()
    );
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1003)
            .await
            .is_err()
    );
    assert!(
        store
            .checkpoint_usage(&attempt.id, &worker, 2, 99)
            .await
            .is_err()
    );
    let used: i64 =
        sqlx::query_scalar("SELECT used_tokens FROM organization_authorizations WHERE id=?")
            .bind(authorization.id)
            .fetch_one(&store.pool)
            .await?;
    assert_eq!(
        used, 101,
        "record the in-flight overage rather than hiding it"
    );
    let checkpoints: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM organization_outbox WHERE kind='usage/checkpoint'",
    )
    .fetch_one(&store.pool)
    .await?;
    assert_eq!(
        checkpoints, 2,
        "duplicate usage reports must not double charge or emit twice"
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn execution_pause_restart_and_restore_fence_old_workers() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let group = node_from(
        store
            .mutate_owner(
                node_mutation("Custom group", None, OrganizationNodeKind::Group),
                None,
            )
            .await?,
    );
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("One", Some(group.id.clone()), OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &project).await?;
    store
        .authorize_task(&task.id, task.revision, execution_scope(), 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store
        .mutate_owner(
            OrganizationMutation::PutNode {
                id: Some(group.id),
                expected_revision: Some(group.revision),
                parent_id: None,
                kind: OrganizationNodeKind::Group,
                label: group.label,
                role_version_id: None,
                model: None,
                enabled: false,
            },
            None,
        )
        .await?;
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    assert_eq!(store.reconcile_interrupted_attempts().await?, 1);
    assert_eq!(store.reconcile_interrupted_attempts().await?, 0);
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 80).await?;
    assert!(
        store
            .checkpoint_usage(&attempt.id, &Uuid::now_v7().to_string(), 1, 80)
            .await
            .is_err()
    );
    let backup = store.backup().await?;
    let recovery = Fixture::new();
    let restored = OrganizationStore::restore_into(&backup, &recovery.0).await?;
    let task_state: String = sqlx::query_scalar("SELECT state FROM organization_tasks WHERE id=?")
        .bind(task.id)
        .fetch_one(&restored.pool)
        .await?;
    assert_eq!(task_state, "needsReconciliation");
    assert!(
        restored
            .claim_task(
                &Uuid::now_v7().to_string(),
                &Uuid::now_v7().to_string(),
                1002,
                2
            )
            .await?
            .is_none()
    );
    assert!(
        restored
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    restored.close().await;
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn execution_invalid_scope_and_stale_authorization_are_atomic() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("One", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &project).await?;
    let mut scope = execution_scope();
    scope
        .input_files
        .push("../organization/organization.sqlite".into());
    assert!(
        store
            .authorize_task(&task.id, task.revision, scope, 1000)
            .await
            .is_err()
    );
    assert!(
        store
            .authorize_task(&task.id, task.revision + 1, execution_scope(), 1000)
            .await
            .is_err()
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM organization_authorizations),(SELECT count(*) FROM organization_outbox)").fetch_one(&store.pool).await?;
    assert_eq!(counts, (0, 0));
    store
        .authorize_task(&task.id, task.revision, execution_scope(), 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1030)
            .await
            .is_err(),
        "expired leases cannot execute"
    );
    store.close().await;
    Ok(())
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
#[path = "organization_schedule_tests.rs"]
mod schedules;

#[tokio::test]
async fn stall_health_exempts_approval_waits_and_fences_uncertain_effects_without_reset()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 9).await?;
    let action = store
        .prepare_action(&attempt.id, &worker, "stall", "exec_command", "{}", 1001)
        .await?;
    assert!(
        !store
            .observe_attempt_health(&attempt.id, &worker, Some(1001), Some(10), 100, 1002)
            .await?,
        "owner waits are not stalls"
    );
    store
        .decide_approval(
            action.approval_id.as_ref().unwrap(),
            &action.action_hash,
            true,
            1002,
        )
        .await?;
    assert!(store.dispatch_action(&action.id, &worker, 1002, 2).await?);
    assert!(
        store
            .observe_attempt_health(&attempt.id, &worker, Some(1001), Some(10), 0, 1002)
            .await?
    );
    assert!(
        store
            .observe_attempt_health(&attempt.id, &worker, Some(1001), Some(10), 10, 1012)
            .await
            .is_err()
    );
    let view = store.execution_view(&attempt.task_id).await?;
    assert_eq!(view.task.state, "failed");
    assert_eq!(view.actions[0].state, OrganizationActionState::Uncertain);
    assert_eq!(
        (
            view.authorization_used_tokens,
            view.authorization_model_requests
        ),
        (9, 1)
    );
    assert_eq!(view.attempts[0].last_progress_at, Some(1001));
    assert!(
        view.attempts[0]
            .stop_reason
            .as_deref()
            .unwrap()
            .contains("stall")
    );
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1013, 2)
            .await
            .is_err()
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(store.execution_view(&attempt.task_id).await?, view);
    assert_eq!(store.reconcile_interrupted_attempts().await?, 0);
    Ok(())
}

#[tokio::test]
async fn full_bundle_restores_evidence_runtime_paths_and_uncertain_accounting_without_credentials()
-> Result<()> {
    use sha2::{Digest, Sha256};
    let home = Fixture::new();
    let bundle = Fixture::new();
    let destination = Fixture::new();
    let workspace = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    private_directory(&workspace.0).await?;
    private_file(&workspace.0.join("report.txt"), true)?.write_all(b"checked evidence")?;
    private_file(&home.0.join("auth.json"), true)?.write_all(b"excluded test credential")?;
    let base_hash = format!("{:x}", Sha256::digest(b"common captured base"));
    let base_dir = home.0.join("organization-artifact-bases");
    private_directory(&base_dir).await?;
    private_file(&base_dir.join(&base_hash), true)?.write_all(b"common captured base")?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    store
        .bind_attempt_runtime(
            &attempt.id,
            &worker,
            &OrganizationRuntimeBinding {
                workspace: workspace.0.to_str().unwrap().into(),
                turn_id: None,
                artifact_bases: vec![OrganizationArtifactBase {
                    captured: true,
                    path: "report.txt".into(),
                    sha256: Some(base_hash.clone()),
                }],
            },
            1001,
        )
        .await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 9).await?;
    let result = store
        .record_result(
            &attempt.id,
            &worker,
            vec![OrganizationArtifactEvidence {
                path: "report.txt".into(),
                sha256: format!("{:x}", Sha256::digest(b"checked evidence")),
                bytes: 16,
            }],
            vec![OrganizationCheckEvidence {
                immutable_inputs: None,
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "actual state test".into(),
                truncated: false,
            }],
            1002,
        )
        .await?;
    let v = store.execution_view(&attempt.task_id).await?;
    store
        .accept_result(&attempt.task_id, v.task.revision, &result.digest)
        .await?;
    let (other_worker, other) = authorized_attempt(&store, false).await?;
    store
        .admit_model_request(&other.id, &other_worker, 1003)
        .await?;
    let action = store
        .prepare_action(
            &other.id,
            &other_worker,
            "interrupted",
            "exec_command",
            "{}",
            1003,
        )
        .await?;
    store
        .dispatch_action(&action.id, &other_worker, 1003, 2)
        .await?;
    private_directory(&home.0.join("sessions")).await?;
    let rollout = home.0.join("sessions/thread.jsonl");
    private_file(&rollout, true)?.write_all(b"{\"type\":\"test receipt\"}\n")?;
    let config = crate::SqliteConfig::from_sqlite_home(
        codex_utils_absolute_path::AbsolutePathBuf::try_from(home.0.clone())?,
    );
    private_file(&config.state_db_path(), true)?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(config.state_db_path()))
        .await?;
    sqlx::query(
        "CREATE TABLE threads(id TEXT PRIMARY KEY,rollout_path TEXT NOT NULL,cwd TEXT NOT NULL)",
    )
    .execute(&pool)
    .await?;
    sqlx::query("INSERT INTO threads VALUES('test',?,?)")
        .bind(rollout.to_str())
        .bind(workspace.0.to_str())
        .execute(&pool)
        .await?;
    pool.close().await;
    let lock = store.acquire_execution_owner()?;
    assert!(store.backup_bundle(&home.0, &bundle.0).await.is_err());
    assert!(
        !bundle.0.exists(),
        "live execution refused before bundle creation"
    );
    drop(lock);
    store.backup_bundle(&home.0, &bundle.0).await?;
    assert!(!bundle.0.join("auth.json").exists());
    let restored = OrganizationStore::restore_bundle(&bundle.0, &destination.0).await?;
    assert_eq!(restored.database_id().await?, store.database_id().await?);
    assert_eq!(
        std::fs::read(
            destination
                .0
                .join("organization-artifact-bases")
                .join(&base_hash)
        )?,
        b"common captured base"
    );
    let view = restored.execution_view(&attempt.task_id).await?;
    assert_eq!(view.task.state, "accepted");
    assert_eq!(view.result.as_ref().unwrap().digest, result.digest);
    assert_eq!(
        (
            view.authorization_used_tokens,
            view.authorization_model_requests
        ),
        (9, 1)
    );
    let relocated = PathBuf::from(view.attempts[0].workspace.as_ref().unwrap());
    assert!(relocated.starts_with(&destination.0));
    assert_eq!(
        std::fs::read(relocated.join("report.txt"))?,
        b"checked evidence"
    );
    let config = crate::SqliteConfig::from_sqlite_home(
        codex_utils_absolute_path::AbsolutePathBuf::try_from(destination.0.clone())?,
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(config.state_db_path()))
        .await?;
    let (path, cwd): (String, String) = sqlx::query_as("SELECT rollout_path,cwd FROM threads")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        path,
        destination
            .0
            .join("sessions/thread.jsonl")
            .to_str()
            .unwrap()
    );
    assert_eq!(cwd, relocated.to_str().unwrap());
    pool.close().await;
    assert_eq!(restored.reconcile_interrupted_attempts().await?, 1);
    let view = restored.execution_view(&other.task_id).await?;
    assert_eq!(view.task.state, "needsReconciliation");
    assert_eq!(view.authorization_model_requests, 1);
    assert_eq!(view.actions[0].state, OrganizationActionState::Uncertain);
    assert!(
        OrganizationStore::restore_bundle(&bundle.0, &destination.0)
            .await
            .is_err(),
        "cannot replace a recovered home"
    );
    let missing = Fixture::new();
    let manifest_path = bundle.0.join("manifest.json");
    let original_manifest = std::fs::read(&manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_slice(&original_manifest)?;
    manifest["files"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["path"] != format!("organization-artifact-bases/{base_hash}"));
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    assert!(
        OrganizationStore::restore_bundle(&bundle.0, &missing.0)
            .await
            .is_err(),
        "omitted required common base refuses restoration"
    );
    assert!(
        !missing.0.exists(),
        "required inventory is checked before destination creation"
    );
    std::fs::write(&manifest_path, original_manifest)?;
    std::fs::remove_file(base_dir.join(&base_hash))?;
    assert!(
        store
            .backup_bundle(&home.0, &Fixture::new().0)
            .await
            .is_err(),
        "newly captured base loss refuses full backup"
    );
    restored.close().await;
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn full_bundle_refuses_corruption_and_path_injection_before_destination_creation()
-> Result<()> {
    let home = Fixture::new();
    let bundle = Fixture::new();
    let bad = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    store.backup_bundle(&home.0, &bundle.0).await?;
    let manifest = bundle.0.join("manifest.json");
    let original = std::fs::read(&manifest)?;
    let mut value: serde_json::Value = serde_json::from_slice(&original)?;
    value["files"][0]["path"] = "../escape".into();
    std::fs::write(&manifest, serde_json::to_vec(&value)?)?;
    assert!(
        OrganizationStore::restore_bundle(&bundle.0, &bad.0)
            .await
            .is_err()
    );
    assert!(!bad.0.exists());
    std::fs::write(&manifest, &original)?;
    std::fs::write(
        bundle.0.join("organization/organization.sqlite"),
        b"corrupt",
    )?;
    assert!(
        OrganizationStore::restore_bundle(&bundle.0, &bad.0)
            .await
            .is_err()
    );
    assert!(!bad.0.exists());
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn full_bundle_refuses_workspace_symlinks_instead_of_reading_owner_files() -> Result<()> {
    let home = Fixture::new();
    let workspace = Fixture::new();
    let bundle = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    private_directory(&workspace.0).await?;
    std::os::unix::fs::symlink(
        home.0.join("organization/organization.sqlite"),
        workspace.0.join("leak"),
    )?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    store
        .bind_attempt_runtime(
            &attempt.id,
            &worker,
            &OrganizationRuntimeBinding {
                workspace: workspace.0.to_str().unwrap().into(),
                turn_id: None,
                artifact_bases: vec![OrganizationArtifactBase {
                    captured: false,
                    path: "report.txt".into(),
                    sha256: None,
                }],
            },
            1001,
        )
        .await?;
    assert!(store.backup_bundle(&home.0, &bundle.0).await.is_err());
    assert!(!bundle.0.join("manifest.json").exists());
    Ok(())
}
