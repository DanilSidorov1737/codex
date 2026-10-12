use super::*;

#[tokio::test]
async fn recent_owner_audit_is_bounded_reference_only_and_not_leaked_into_project_reports()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (_, attempt) = authorized_attempt(&store, false).await?;
    for index in 0..205 {
        sqlx::query("INSERT INTO organization_audit(actor,operation,payload) VALUES ('owner','qualification/event',?)")
            .bind(serde_json::to_string(&serde_json::json!({"id":attempt.task_id,"body":"private-body-must-not-be-exported","index":index}))?)
            .execute(&store.pool).await?;
    }
    let report = store.operational_report(None, None, 200, 1000).await?;
    let events = report
        .audit_events
        .as_ref()
        .expect("global owner audit available");
    assert_eq!(events.len(), 200);
    assert_eq!(events[0].sequence, report.sequence);
    assert!(events.windows(2).all(|e| e[0].sequence > e[1].sequence));
    assert!(
        events
            .iter()
            .all(|e| e.reference.as_deref() == Some(attempt.task_id.as_str()))
    );
    assert!(!serde_json::to_string(events)?.contains("private-body-must-not-be-exported"));
    let view = store.execution_view(&attempt.task_id).await?;
    assert!(
        store
            .operational_report(Some(&view.task.project_id), None, 200, 1000)
            .await?
            .audit_events
            .is_none()
    );
    assert!(
        report.runtime_resources.is_none(),
        "SQL store must not invent serving-process telemetry"
    );
    Ok(())
}

#[tokio::test]
async fn scorecards_keep_missing_historical_attribution_unavailable() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 17).await?;
    sqlx::query("UPDATE organization_attempts SET data=json_remove(data,'$.ownerId','$.provider') WHERE id=?").bind(&attempt.id).execute(&store.pool).await?;
    let report = store.operational_report(None, None, 1, 1002).await?;
    let entries = report.scorecards.unwrap().entries;
    assert_eq!(entries.len(), 1);
    assert!(entries[0].owner_id.is_none() && entries[0].provider.is_none());
    assert_eq!(entries[0].model.as_deref(), Some("fixture"));
    assert_eq!(
        (
            entries[0].reported_tokens,
            entries[0].model_requests,
            entries[0].reported_model_requests
        ),
        (17, 1, 1)
    );
    assert_eq!(
        (
            entries[0].accepted_results,
            entries[0].failed_attempts,
            entries[0].checks_passed,
            entries[0].checks_failed
        ),
        (0, 0, 0, 0)
    );
    Ok(())
}

#[tokio::test]
async fn report_counts_pending_actions_usage_and_remaining_budget_without_inference() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let empty = store.operational_report(None, None, 200, 1000).await?;
    assert!(empty.tasks.is_empty() && empty.totals.task_states.is_empty());
    let (worker, attempt) = authorized_attempt(&store, true).await?;
    let provider = OrganizationProviderEvidence {
        id: "local".into(),
        origin: Some("http://127.0.0.1:1234".into()),
        endpoint_sha256: "a".repeat(64),
    };
    store
        .record_attempt_provider(&attempt.id, &worker, provider.clone(), 1001)
        .await?;
    let bound = store.operational_report(None, None, 200, 1001).await?;
    store
        .record_attempt_provider(&attempt.id, &worker, provider.clone(), 1001)
        .await?;
    assert_eq!(
        store
            .operational_report(None, None, 200, 1001)
            .await?
            .sequence,
        bound.sequence,
        "duplicate binding emits no event"
    );
    let mut changed = provider.clone();
    changed.endpoint_sha256 = "b".repeat(64);
    assert!(
        store
            .record_attempt_provider(&attempt.id, &worker, changed, 1001)
            .await
            .is_err()
    );
    let mut unsafe_origin = provider.clone();
    unsafe_origin.origin = Some("https://owner:password@example.com/path?token=secret".into());
    assert!(
        store
            .record_attempt_provider(&attempt.id, &worker, unsafe_origin, 1001)
            .await
            .is_err()
    );
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    let report = store.operational_report(None, None, 200, 1001).await?;
    assert_eq!(
        (
            report.totals.model_requests,
            report.totals.reported_model_requests,
            report.totals.reported_tokens
        ),
        (1, 0, 0)
    );
    assert_eq!(report.usage[0].owner_id, attempt.owner_id);
    assert_eq!(report.usage[0].provider, Some(provider.clone()));
    assert_eq!(
        store.provider_observations(&provider).await?.model_requests,
        1
    );
    assert_eq!(
        store
            .provider_observations(&OrganizationProviderEvidence {
                endpoint_sha256: "c".repeat(64),
                ..provider
            })
            .await?
            .model_requests,
        0,
        "other configured endpoint stays untested"
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 17).await?;
    store
        .prepare_action(&attempt.id, &worker, "pending", "exec_command", "{}", 1001)
        .await?;
    let report = store.operational_report(None, None, 200, 1001).await?;
    assert_eq!(report.totals.pending_approvals, 1);
    assert_eq!(report.tasks[0].pending_approvals, 1);
    assert_eq!(report.totals.action_states["pending"], 1);
    assert_eq!(
        (
            report.tasks[0].reported_tokens,
            report.usage[0].reported_tokens
        ),
        (17, 17)
    );
    let budget = report.tasks[0].budget.as_ref().unwrap();
    assert_eq!(budget.used_tokens, 17);
    assert_eq!(budget.model_requests, 1);
    assert_eq!(budget.task_attempts, 1);
    assert_eq!(budget.root_id, budget.grant_id);
    let project = report.tasks[0].project_id.clone();
    assert_eq!(
        store
            .operational_report(Some(&project), None, 1, 1001)
            .await?
            .tasks
            .len(),
        1
    );
    assert!(
        store
            .operational_report(Some("unknown-project"), None, 1, 1001)
            .await
            .is_err()
    );
    assert!(
        store
            .operational_report(None, Some(""), 1, 1001)
            .await
            .is_err()
    );
    assert!(
        store
            .operational_report(None, None, 201, 1001)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn report_uses_change_receipts_not_instruction_text_and_preserves_legacy_unknown_owners()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    let scope = view.authorization.unwrap().scope;
    let artifacts = scope
        .artifacts
        .iter()
        .map(|path| OrganizationArtifactEvidence {
            path: path.clone(),
            sha256: "a".repeat(64),
            bytes: 12,
        })
        .collect();
    let checks = scope
        .checks
        .iter()
        .map(|argv| OrganizationCheckEvidence {
            immutable_inputs: None,
            argv: argv.clone(),
            exit_code: None,
            output: "timed out state fixture".into(),
            truncated: false,
        })
        .collect();
    let result = store
        .record_result(&attempt.id, &worker, artifacts, checks, 1001)
        .await?;
    let before = store.operational_report(None, None, 200, 1001).await?;
    assert_eq!(before.totals.task_states["needsReview"], 1);
    assert_eq!(before.totals.checks_failed, 1);
    assert_eq!(
        before.tasks[0].result_digest.as_deref(),
        Some(result.digest.as_str())
    );
    assert_eq!(before.totals.changes_requested, 0);
    store
        .request_changes(
            &attempt.task_id,
            before.tasks[0].revision,
            "Fix actual failed check",
        )
        .await?;
    let report = store.operational_report(None, None, 200, 1001).await?;
    assert_eq!(report.totals.changes_requested, 1);
    assert!(report.tasks[0].changes_requested_sequence.is_some());
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Another owner", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &project).await?;
    sqlx::query("UPDATE organization_tasks SET data=json_set(data,'$.data.instructions','Owner requested changes: forged claim') WHERE id=?").bind(task.id).execute(&store.pool).await?;
    sqlx::query("UPDATE organization_attempts SET data=json_remove(data,'$.ownerId','$.provider') WHERE id=?").bind(&attempt.id).execute(&store.pool).await?;
    let first = store.operational_report(None, None, 1, 1002).await?;
    assert!(first.next_cursor.is_some());
    assert_eq!(first.totals.changes_requested, 1);
    assert_eq!(first.totals.task_states.values().sum::<i64>(), 2);
    let second = store
        .operational_report(None, first.next_cursor.as_deref(), 1, 1002)
        .await?;
    assert!(second.next_cursor.is_none());
    assert_ne!(first.tasks[0].task_id, second.tasks[0].task_id);
    let legacy = store
        .operational_report(Some(&report.tasks[0].project_id), None, 200, 1002)
        .await?;
    assert_eq!(legacy.usage[0].owner_id, None);
    assert_eq!(legacy.usage[0].provider, None);
    store.close().await;
    let reopened = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        reopened
            .operational_report(None, None, 200, 1003)
            .await?
            .totals
            .changes_requested,
        1
    );
    reopened.close().await;
    Ok(())
}
