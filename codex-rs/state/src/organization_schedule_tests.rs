use super::*;
use codex_protocol::organization_schedule::*;

async fn schedule(
    store: &OrganizationStore,
    scope: OrganizationExecutionScope,
    runs: u32,
    tokens: i64,
    requests: u32,
) -> Result<OrganizationScheduleView> {
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Interval owner", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(store, &node.id).await?;
    let task = execution_task(store, &node.id, &project).await?;
    store
        .mutate_schedule(
            OrganizationScheduleMutation::Create {
                id: Uuid::now_v7().to_string(),
                task_id: task.id,
                expected_revision: task.revision,
                scope: Box::new(scope),
                config: OrganizationScheduleConfig {
                    label: "Checked interval".into(),
                    interval_seconds: 10,
                    first_run_at: 1010,
                    ends_at: 2000,
                    max_runs: runs,
                    max_tokens: tokens,
                    max_model_requests: requests,
                },
            },
            1000,
        )
        .await
}

async fn view(store: &OrganizationStore, id: &str) -> Result<OrganizationScheduleView> {
    Ok(store
        .list_schedules(None, None, 200)
        .await?
        .0
        .into_iter()
        .find(|v| v.schedule.id == id)
        .unwrap())
}

async fn control(
    store: &OrganizationStore,
    id: &str,
    action: OrganizationScheduleControl,
    now: i64,
) -> Result<OrganizationScheduleView> {
    let current = view(store, id).await?;
    store
        .mutate_schedule(
            OrganizationScheduleMutation::Control {
                id: id.into(),
                expected_revision: current.schedule.revision,
                action,
            },
            now,
        )
        .await
}

async fn claim(
    store: &OrganizationStore,
    worker: &str,
    now: i64,
) -> Result<OrganizationTaskAttempt> {
    Ok(store
        .claim_task(worker, &Uuid::now_v7().to_string(), now, 4)
        .await?
        .unwrap())
}

#[tokio::test]
async fn recurring_occurrences_are_atomic_pinned_nonoverlapping_and_skip_missed_times() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let initial = schedule(&store, execution_scope(), 2, 1000, 10).await?;
    let id = &initial.schedule.id;
    let source = &initial.schedule.template;
    store
        .mutate_owner(
            OrganizationMutation::PutTask {
                id: Some(source.id.clone()),
                expected_revision: Some(source.revision),
                project_id: source.project_id.clone(),
                goal_id: None,
                owner_id: source.owner_id.clone(),
                instructions: "Changed source must not change authorized snapshot".into(),
                criteria: source.criteria.clone(),
                prerequisites: vec![],
            },
            None,
        )
        .await?;
    assert_eq!(store.materialize_schedules(1009).await?, 0);
    let (a, b) = tokio::join!(
        store.materialize_schedules(1010),
        store.materialize_schedules(1010)
    );
    assert_eq!(a? + b?, 1);
    let first = view(&store, id).await?;
    assert_eq!(first.occurrences.len(), 1);
    assert_eq!(
        store
            .execution_view(&first.occurrences[0].task_id)
            .await?
            .task
            .instructions,
        source.instructions
    );
    assert_eq!(store.materialize_schedules(1020).await?, 0); // queued occurrence counts as overlap
    assert_eq!(view(&store, id).await?.schedule.skipped_runs, 1);
    let worker = Uuid::now_v7().to_string();
    let attempt = claim(&store, &worker, 1021).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1021)
        .await?;
    assert_eq!(store.materialize_schedules(1030).await?, 0); // live usage is busy, not uncertain
    assert_eq!(
        view(&store, id).await?.schedule.state,
        OrganizationScheduleState::Active
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 10).await?;
    store.fail_attempt(&attempt.id, &worker).await?;
    assert_eq!(store.materialize_schedules(1040).await?, 0); // native cleanup still reserves room
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    assert_eq!(store.materialize_schedules(1072).await?, 0); // skip all overdue times, no catch-up
    assert_eq!(view(&store, id).await?.schedule.next_run_at, Some(1080));
    assert_eq!(store.materialize_schedules(1080).await?, 1);
    let final_view = view(&store, id).await?;
    assert_eq!(
        final_view.schedule.state,
        OrganizationScheduleState::Completed
    );
    assert_eq!(final_view.schedule.issued_runs, 2);
    assert_eq!(final_view.schedule.skipped_runs, 6);
    assert_ne!(
        final_view.occurrences[0].task_id,
        final_view.occurrences[1].task_id
    );
    assert_eq!(store.materialize_schedules(1070).await?, 0); // backward clock cannot replay
    store.close().await;
    let reopened = OrganizationStore::open(&home.0).await?;
    assert_eq!(view(&reopened, id).await?, final_view);
    assert_eq!(reopened.materialize_schedules(1080).await?, 0);
    let backup = reopened.backup().await?;
    let restored_home = Fixture::new();
    let restored = OrganizationStore::restore_into(&backup, &restored_home.0).await?;
    assert_eq!(view(&restored, id).await?, final_view);
    Ok(())
}

#[tokio::test]
async fn recurring_pause_revokes_exact_approvals_resume_never_revives_old_work() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let initial = schedule(&store, execution_scope(), 3, 1000, 10).await?;
    let id = &initial.schedule.id;
    store.materialize_schedules(1010).await?;
    let worker = Uuid::now_v7().to_string();
    let old = claim(&store, &worker, 1010).await?;
    let action = store
        .prepare_action(&old.id, &worker, "exact", "exec_command", "{}", 1011)
        .await?;
    let approval = action.approval_id.unwrap();
    let paused = control(&store, id, OrganizationScheduleControl::Pause, 1012).await?;
    assert_eq!(paused.schedule.state, OrganizationScheduleState::Paused);
    assert!(
        store
            .admit_model_request(&old.id, &worker, 1012)
            .await
            .is_err()
    );
    assert!(
        store
            .decide_approval(&approval, &action.action_hash, true, 1012)
            .await
            .is_err()
    );
    assert_eq!(store.materialize_schedules(1040).await?, 0);
    assert!(
        store
            .mutate_schedule(
                OrganizationScheduleMutation::Control {
                    id: id.clone(),
                    expected_revision: 1,
                    action: OrganizationScheduleControl::Resume
                },
                1040
            )
            .await
            .is_err()
    );
    store.release_attempt_capacity(&old.id, &worker).await?;
    let resumed = control(&store, id, OrganizationScheduleControl::Resume, 1040).await?;
    assert_eq!(resumed.schedule.next_run_at, Some(1050));
    assert_eq!(resumed.schedule.skipped_runs, 3);
    assert!(
        store
            .admit_model_request(&old.id, &worker, 1041)
            .await
            .is_err()
    );
    let stopped = store.execution_view(&old.task_id).await?;
    assert!(
        store
            .authorize_task(&old.task_id, stopped.task.revision, execution_scope(), 1041)
            .await
            .is_err()
    );
    store.materialize_schedules(1050).await?;
    let fresh = claim(&store, &worker, 1050).await?;
    assert_ne!(fresh.task_id, old.task_id);
    control(&store, id, OrganizationScheduleControl::Cancel, 1051).await?;
    assert!(
        store
            .admit_model_request(&fresh.id, &worker, 1051)
            .await
            .is_err()
    );
    assert_eq!(store.materialize_schedules(1060).await?, 0);
    assert!(
        control(&store, id, OrganizationScheduleControl::Resume, 1060)
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn recurring_aggregate_admission_counts_descendants_and_preserves_late_usage() -> Result<()> {
    use codex_protocol::organization_coordination::*;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let helper = node_from(
        store
            .mutate_owner(
                node_mutation("Helper", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let mut scope = execution_scope();
    scope.delegates = vec![helper.id.clone()];
    scope.max_children = 1;
    scope.max_model_requests = 10;
    let initial = schedule(&store, scope, 4, 15, 3).await?;
    let id = &initial.schedule.id;
    store.materialize_schedules(1010).await?;
    let worker = Uuid::now_v7().to_string();
    let root = claim(&store, &worker, 1010).await?;
    store.admit_model_request(&root.id, &worker, 1010).await?;
    store.checkpoint_usage(&root.id, &worker, 1, 10).await?;
    let plan = store
        .coordinate(
            &root.id,
            &worker,
            "plan",
            OrganizationCoordinationCall::ProposePlan {
                items: vec![OrganizationPlanItem {
                    key: "child".into(),
                    owner_id: helper.id,
                    instructions: "Child".into(),
                    criteria: vec!["Checked".into()],
                    prerequisites: vec![],
                }],
            },
            1011,
        )
        .await?;
    let task_id = plan[0]["id"].as_str().unwrap();
    store
        .coordinate(
            &root.id,
            &worker,
            "delegate",
            OrganizationCoordinationCall::Delegate {
                task_id: task_id.into(),
            },
            1011,
        )
        .await?;
    let child = claim(&store, &worker, 1011).await?;
    store.admit_model_request(&child.id, &worker, 1011).await?;
    control(&store, id, OrganizationScheduleControl::Pause, 1012).await?;
    assert!(
        store
            .admit_model_request(&child.id, &worker, 1012)
            .await
            .is_err()
    );
    store.checkpoint_usage(&child.id, &worker, 1, 8).await?; // billable late report after revoke
    let current = view(&store, id).await?;
    assert_eq!((current.used_tokens, current.model_requests), (18, 2));
    assert!(
        control(&store, id, OrganizationScheduleControl::Resume, 1020)
            .await
            .is_err()
    );
    assert_eq!(store.materialize_schedules(1020).await?, 0);
    // A second series proves the aggregate request cap is independent of per-run cap.
    let mut scope = execution_scope();
    scope.max_model_requests = 10;
    let other = schedule(&store, scope, 2, 1000, 1).await?;
    store.release_attempt_capacity(&root.id, &worker).await?;
    store.release_attempt_capacity(&child.id, &worker).await?;
    store.materialize_schedules(1010).await?;
    let attempt = claim(&store, &worker, 1010).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1010)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 1).await?;
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1011)
            .await
            .is_err()
    );
    store.fail_attempt(&attempt.id, &worker).await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    assert_eq!(store.materialize_schedules(1020).await?, 0);
    assert_eq!(
        view(&store, &other.schedule.id).await?.schedule.state,
        OrganizationScheduleState::Completed
    );
    Ok(())
}

#[tokio::test]
async fn recurring_restart_skips_overdue_and_blocks_interrupted_or_changed_scope() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let safe = schedule(&store, execution_scope(), 3, 1000, 10).await?;
    store.reconcile_schedules(1027).await?;
    assert_eq!(
        view(&store, &safe.schedule.id).await?.schedule.next_run_at,
        Some(1030)
    );
    assert_eq!(
        view(&store, &safe.schedule.id).await?.schedule.skipped_runs,
        2
    );
    store.materialize_schedules(1030).await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = claim(&store, &worker, 1030).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1030)
        .await?;
    store.reconcile_interrupted_attempts().await?;
    store.reconcile_schedules(1031).await?;
    assert_eq!(
        view(&store, &safe.schedule.id).await?.schedule.state,
        OrganizationScheduleState::Blocked
    );
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1031)
            .await
            .is_err()
    );
    assert!(
        control(
            &store,
            &safe.schedule.id,
            OrganizationScheduleControl::Resume,
            1031
        )
        .await
        .is_err()
    );
    let changed = schedule(&store, execution_scope(), 3, 1000, 10).await?;
    let node = match store
        .read_definition(
            OrganizationRecordKind::Nodes,
            &changed.schedule.template.owner_id,
        )
        .await?
    {
        OrganizationRecord::Node(n) => n,
        _ => unreachable!(),
    };
    store
        .mutate_owner(
            OrganizationMutation::PutNode {
                id: Some(node.id),
                expected_revision: Some(node.revision),
                parent_id: None,
                kind: OrganizationNodeKind::Agent,
                label: "New role".into(),
                role_version_id: None,
                model: None,
                enabled: true,
            },
            None,
        )
        .await?;
    assert_eq!(store.materialize_schedules(1010).await?, 0);
    assert_eq!(
        view(&store, &changed.schedule.id).await?.schedule.state,
        OrganizationScheduleState::Blocked
    );
    assert!(
        control(
            &store,
            &changed.schedule.id,
            OrganizationScheduleControl::Resume,
            1011
        )
        .await
        .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn recurring_migration_validation_and_pagination_preserve_existing_state() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Preserved", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    // Simulate a genuine version-6 store: later schemas must also be absent.
    remove_controls_migration_for_legacy_fixture(&store).await?;
    sqlx::query("DROP TABLE organization_presets")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP VIEW organization_shared_usage")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP VIEW organization_project_run_usage")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_occurrences")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_shared_limits")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_series")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_handoffs")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_checkpoints")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_tasks")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_runs")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_workflow_templates")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_schedule_occurrences")
        .execute(&store.pool)
        .await?;
    sqlx::query("DROP TABLE organization_schedules")
        .execute(&store.pool)
        .await?;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version>=7")
        .execute(&store.pool)
        .await?;
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Nodes, &node.id)
            .await?,
        OrganizationRecord::Node(node)
    );
    let a = schedule(&store, execution_scope(), 1, 100, 2).await?;
    let b = schedule(&store, execution_scope(), 1, 100, 2).await?;
    let (first, cursor) = store.list_schedules(None, None, 1).await?;
    let (second, last) = store.list_schedules(None, cursor.as_deref(), 1).await?;
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_ne!(first[0].schedule.id, second[0].schedule.id);
    assert!(last.is_none());
    assert_eq!(
        store
            .list_schedules(Some(&a.schedule.template.project_id), None, 100)
            .await?
            .0
            .len(),
        1
    );
    let base = OrganizationScheduleMutation::Create {
        id: Uuid::now_v7().to_string(),
        task_id: a.schedule.template.id.clone(),
        expected_revision: a.schedule.template.revision,
        scope: Box::new(execution_scope()),
        config: a.schedule.config.clone(),
    };
    for (field, value) in [
        ("intervalSeconds", serde_json::json!(0)),
        ("maxRuns", serde_json::json!(1001)),
        ("endsAt", serde_json::json!(1000 + 32 * 86400)),
        ("firstRunAt", serde_json::json!(999)),
        ("maxTokens", serde_json::json!(0)),
        ("maxModelRequests", serde_json::json!(0)),
        ("label", serde_json::json!("bad\nlabel")),
    ] {
        let mut json = serde_json::to_value(&base)?;
        json["config"][field] = value;
        assert!(
            store
                .mutate_schedule(serde_json::from_value(json)?, 1000)
                .await
                .is_err()
        );
    }
    let mut duplicate = serde_json::to_value(&base)?;
    duplicate["id"] = serde_json::json!(b.schedule.id);
    assert!(
        store
            .mutate_schedule(serde_json::from_value(duplicate)?, 1000)
            .await
            .is_err()
    );
    assert_eq!(store.list_schedules(None, None, 100).await?.0.len(), 2);
    Ok(())
}
