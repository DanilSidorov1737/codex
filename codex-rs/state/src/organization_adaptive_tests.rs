use super::*;

#[tokio::test]
async fn profile_versions_materialize_exact_grants_and_reject_stale_authorization() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (_, attempt) = authorized_attempt(&store, false).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    let task = execution_task(&store, &view.task.owner_id, &view.task.project_id).await?;
    let id = Uuid::now_v7().to_string();
    let profile = |tokens| {
        let mut scope = execution_scope();
        scope.max_tokens = tokens;
        OrganizationControlValue::ExecutionProfile {
            id: id.clone(),
            label: "Coding".into(),
            policy_version: ORGANIZATION_POLICY_VERSION,
            scope: Box::new(scope),
        }
    };
    let first = store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: None,
                value: profile(100),
            },
            None,
        )
        .await?;
    let Some(OrganizationRecord::Control(first)) = first else {
        panic!("profile")
    };
    let second = store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: Some(1),
                value: profile(90),
            },
            None,
        )
        .await?;
    let Some(OrganizationRecord::Control(second)) = second else {
        panic!("profile")
    };
    assert!(
        store
            .authorize_profile(&task.id, task.revision, &first, 1001)
            .await
            .is_err()
    );
    assert!(
        store
            .execution_view(&task.id)
            .await?
            .authorization
            .is_none()
    );
    let grant = store
        .authorize_profile(&task.id, task.revision, &second, 1001)
        .await?;
    assert_eq!(grant.scope.max_tokens, 90);
    store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: Some(2),
                value: profile(200),
            },
            None,
        )
        .await?;
    assert_eq!(
        store.execution_view(&task.id).await?.authorization,
        Some(grant)
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn uncertainty_inspection_is_pinned_immutable_and_never_releases_a_reservation() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.reconcile_interrupted_attempts().await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let captured = before.attempts[0].clone();
    let value = OrganizationControlValue::Reconciliation {
        id: Uuid::now_v7().to_string(),
        attempt_id: attempt.id.clone(),
        request_sequence: captured.model_requests,
        fingerprint: uncertainty_fingerprint(&captured, &before.actions),
        evidence: "No provider completion receipt; retain charge and fence".into(),
    };
    let mut changed = value.clone();
    if let OrganizationControlValue::Reconciliation { fingerprint, .. } = &mut changed {
        *fingerprint = "0".repeat(64);
    }
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: changed
                },
                None
            )
            .await
            .is_err()
    );
    store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: None,
                value: value.clone(),
            },
            None,
        )
        .await?;
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: Some(1),
                    value
                },
                None
            )
            .await
            .is_err()
    );
    assert_eq!(store.execution_view(&attempt.task_id).await?, before);
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}
