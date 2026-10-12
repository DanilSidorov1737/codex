use super::*;
use codex_protocol::organization_coordination::{
    OrganizationCoordinationCall, OrganizationNoteScope, OrganizationPlanItem,
};
use codex_protocol::organization_workflow::OrganizationSharedCeilings;

async fn set(
    store: &OrganizationStore,
    expected_revision: Option<i64>,
    value: OrganizationControlValue,
) -> Result<OrganizationControl> {
    let Some(OrganizationRecord::Control(control)) = store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision,
                value,
            },
            None,
        )
        .await?
    else {
        anyhow::bail!("expected owner control");
    };
    Ok(control)
}

fn global(paused: bool, requests: u32, tokens: i64, active: u32) -> OrganizationControlValue {
    OrganizationControlValue::Global {
        paused,
        limits: Some(OrganizationSharedCeilings {
            max_tokens: tokens,
            max_model_requests: requests,
            max_active: active,
        }),
    }
}

#[tokio::test]
async fn mission_versions_and_incomplete_goal_evidence_never_create_authority() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let mission = OrganizationControlValue::Mission {
        purpose: "Useful verified local work".into(),
        priorities: vec!["Finish declared outputs".into()],
        constraints: vec!["Native grants remain authoritative".into()],
    };
    let first = set(&store, None, mission.clone()).await?;
    assert_eq!(store.mission().await?.unwrap(), first);
    assert!(set(&store, None, mission.clone()).await.is_err());
    let next = set(&store, Some(first.revision), mission).await?;
    assert_eq!(next.revision, first.revision + 1);
    let Some(OrganizationRecord::Goal(goal)) = store
        .mutate_owner(
            OrganizationMutation::PutGoal {
                id: None,
                expected_revision: None,
                parent_id: None,
                objective: "Verified result".into(),
                criteria: vec!["Actual oracle passes".into()],
            },
            None,
        )
        .await?
    else {
        panic!("goal")
    };
    assert!(
        set(
            &store,
            None,
            OrganizationControlValue::GoalReview {
                goal: Box::new(goal.clone()),
                completed: true,
                evidence: vec![]
            }
        )
        .await
        .is_err()
    );
    let mut stale = goal.clone();
    stale.revision += 1;
    assert!(
        set(
            &store,
            None,
            OrganizationControlValue::GoalReview {
                goal: Box::new(stale),
                completed: false,
                evidence: vec![]
            }
        )
        .await
        .is_err()
    );
    set(
        &store,
        None,
        OrganizationControlValue::GoalReview {
            goal: Box::new(goal),
            completed: false,
            evidence: vec![],
        },
    )
    .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM organization_authorizations")
            .fetch_one(&store.pool)
            .await?,
        0
    );
    Ok(())
}

fn project(
    id: &str,
    state: OrganizationProjectState,
    evidence: Vec<OrganizationProjectEvidence>,
) -> OrganizationControlValue {
    OrganizationControlValue::Project {
        project_id: id.into(),
        project_revision: 1,
        state,
        evidence,
    }
}

#[tokio::test]
async fn goal_completion_and_private_lessons_pin_current_accepted_evidence() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let agent = node_from(
        store
            .mutate_owner(
                node_mutation("Goal worker", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project_id = execution_project(&store, &agent.id).await?;
    let Some(OrganizationRecord::Goal(goal)) = store
        .mutate_owner(
            OrganizationMutation::PutGoal {
                id: None,
                expected_revision: None,
                parent_id: None,
                objective: "Verified outcome".into(),
                criteria: vec!["Oracle passed".into()],
            },
            None,
        )
        .await?
    else {
        panic!("goal")
    };
    store
        .mutate_owner(
            OrganizationMutation::LinkProject {
                project_id: project_id.clone(),
                expected_revision: Some(1),
                goal_id: Some(goal.id.clone()),
                owner_id: Some(agent.id.clone()),
                criteria: vec!["Oracle passed".into()],
            },
            Some(("Fixture".into(), vec!["/tmp/fixture-project".into()])),
        )
        .await?;
    let task = execution_task(&store, &agent.id, &project_id).await?;
    let mut scope = execution_scope();
    scope.require_tool_approval = false;
    store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    let view = store.execution_view(&attempt.task_id).await?;
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
                sha256: "a".repeat(64),
                bytes: 3,
            }],
            vec![OrganizationCheckEvidence {
                immutable_inputs: None,
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "native oracle evidence".into(),
                truncated: false,
            }],
            1002,
        )
        .await?;
    let current = store.execution_view(&attempt.task_id).await?;
    let proof = OrganizationProjectEvidence {
        criterion: 0,
        task_id: current.task.id.clone(),
        result_digest: result.digest.clone(),
        explanation: "Owner inspected oracle and output".into(),
    };
    let review = |evidence| OrganizationControlValue::GoalReview {
        goal: Box::new(goal.clone()),
        completed: true,
        evidence,
    };
    assert!(
        set(&store, None, review(vec![proof.clone()]))
            .await
            .is_err(),
        "review alone is not acceptance"
    );
    store
        .accept_result(&current.task.id, current.task.revision, &result.digest)
        .await?;
    assert!(
        set(&store, None, review(vec![proof.clone(), proof.clone()]))
            .await
            .is_err(),
        "duplicate criteria refuse"
    );
    assert!(
        set(
            &store,
            None,
            review(vec![OrganizationProjectEvidence {
                result_digest: "stale".into(),
                ..proof.clone()
            }])
        )
        .await
        .is_err()
    );
    let pending = execution_task(&store, &view.task.owner_id, &view.task.project_id).await?;
    assert!(
        set(&store, None, review(vec![proof.clone()]))
            .await
            .is_err(),
        "unfinished linked work prevents completion"
    );
    store
        .authorize_task(&pending.id, pending.revision, execution_scope(), 1003)
        .await?;
    let pending = store.execution_view(&pending.id).await?.task;
    store.stop_task(&pending.id, pending.revision, true).await?;
    let completed = set(&store, None, review(vec![proof.clone()])).await?;
    let another = execution_task(&store, &view.task.owner_id, &view.task.project_id).await?;
    assert!(
        store
            .authorize_task(&another.id, another.revision, execution_scope(), 1003)
            .await
            .is_err(),
        "completed goal fences new execution"
    );
    let current = store.execution_view(&view.task.id).await?;
    let lesson = |revision, digest: String| OrganizationMutation::ProposeLesson {
        task_id: current.task.id.clone(),
        expected_revision: revision,
        result_digest: digest,
        body: "Use a meaningful native oracle before acceptance".into(),
    };
    assert!(
        store
            .mutate_owner(
                lesson(current.task.revision - 1, result.digest.clone()),
                None
            )
            .await
            .is_err()
    );
    assert!(
        store
            .mutate_owner(lesson(current.task.revision, "stale".into()), None)
            .await
            .is_err()
    );
    let Some(OrganizationRecord::Note(note)) = store
        .mutate_owner(lesson(current.task.revision, result.digest.clone()), None)
        .await?
    else {
        panic!("lesson")
    };
    assert_eq!(note.author_id, view.task.owner_id);
    assert!(!note.owner_reviewed);
    assert_eq!(note.scope, OrganizationNoteScope::Agent);
    assert!(note.body.contains(&result.digest));
    assert_eq!(
        store
            .search_memory(
                &note.project_id,
                OrganizationNoteScope::Project,
                None,
                "oracle"
            )
            .await?
            .matched_count,
        0
    );
    assert_eq!(
        store
            .search_memory(
                &note.project_id,
                OrganizationNoteScope::Agent,
                Some(&note.author_id),
                "oracle"
            )
            .await?
            .notes,
        vec![note]
    );
    set(
        &store,
        Some(completed.revision),
        OrganizationControlValue::GoalReview {
            goal: Box::new(goal),
            completed: false,
            evidence: vec![proof],
        },
    )
    .await?;
    store
        .authorize_task(&another.id, another.revision, execution_scope(), 1003)
        .await?;
    let original = store.execution_view(&view.task.id).await?;
    assert_eq!(
        (
            original.authorization_model_requests,
            original.authorization_used_tokens
        ),
        (1, 9)
    );
    Ok(())
}

#[tokio::test]
async fn resolving_assistance_neither_changes_task_nor_replays_an_attempt() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let request = |task: &OrganizationTask| OrganizationControlValue::Escalation {
        task_id: task.id.clone(),
        task_revision: task.revision,
        assignee_id: None,
        reason: "Owner assistance required".into(),
        requested_action: "Inspect captured evidence".into(),
        resolved: false,
    };
    assert!(
        set(&store, None, request(&before.task)).await.is_err(),
        "ordinary running work is not a blocker"
    );
    store.fail_attempt(&attempt.id, &worker).await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    let blocked = store.execution_view(&attempt.task_id).await?;
    assert!(
        set(&store, None, request(&before.task)).await.is_err(),
        "stale task refuses"
    );
    let pending = set(&store, None, request(&blocked.task)).await?;
    let mut resolved = pending.value.clone();
    if let OrganizationControlValue::Escalation { resolved, .. } = &mut resolved {
        *resolved = true;
    }
    let mut changed = resolved.clone();
    if let OrganizationControlValue::Escalation {
        requested_action, ..
    } = &mut changed
    {
        *requested_action = "Authorize more work".into();
    }
    assert!(
        set(&store, Some(pending.revision), changed).await.is_err(),
        "resolution pins the exact request"
    );
    let done = set(&store, Some(pending.revision), resolved.clone()).await?;
    assert!(
        set(&store, Some(done.revision), resolved).await.is_err(),
        "resolved requests retain immutable history"
    );
    assert_eq!(store.execution_view(&attempt.task_id).await?, blocked);
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 4)
            .await?
            .is_none()
    );
    Ok(())
}

async fn queued(
    store: &OrganizationStore,
    node: &str,
    project: &str,
    now: i64,
) -> Result<OrganizationTask> {
    let task = execution_task(store, node, project).await?;
    store
        .authorize_task(&task.id, task.revision, execution_scope(), now)
        .await?;
    Ok(store.execution_view(&task.id).await?.task)
}

#[tokio::test]
async fn global_claim_and_request_races_reserve_exact_capacity_without_reset() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let agent = node_from(
        store
            .mutate_owner(
                node_mutation("Race", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &agent.id).await?;
    queued(&store, &agent.id, &project, 1000).await?;
    queued(&store, &agent.id, &project, 1000).await?;
    set(&store, None, global(false, 1, 100, 1)).await?;
    let worker = Uuid::now_v7().to_string();
    let t1 = Uuid::now_v7().to_string();
    let t2 = Uuid::now_v7().to_string();
    let (a, b) = tokio::join!(
        store.claim_task(&worker, &t1, 1000, 4),
        store.claim_task(&worker, &t2, 1000, 4)
    );
    let a = a?;
    let b = b?;
    assert_eq!(usize::from(a.is_some()) + usize::from(b.is_some()), 1);
    let first = a.or(b).unwrap();
    set(&store, Some(1), global(false, 1, 100, 2)).await?;
    let second = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 4)
        .await?
        .unwrap();
    let (a, b) = tokio::join!(
        store.admit_model_request(&first.id, &worker, 1001),
        store.admit_model_request(&second.id, &worker, 1001)
    );
    assert_eq!(
        usize::from(a.is_ok()) + usize::from(b.is_ok()),
        1,
        "one shared request reservation across separate roots"
    );
    let charged = if a.is_ok() { &first } else { &second };
    store.checkpoint_usage(&charged.id, &worker, 1, 11).await?;
    let view = store.execution_view(&charged.task_id).await?;
    store
        .stop_task(&charged.task_id, view.task.revision, false)
        .await?;
    store.release_attempt_capacity(&charged.id, &worker).await?;
    let task = store.execution_view(&charged.task_id).await?.task;
    store
        .authorize_task(&task.id, task.revision, execution_scope(), 1002)
        .await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none(),
        "a fresh/revoked grant cannot reset global usage"
    );
    let u = store
        .operational_report(None, None, 200, 1002)
        .await?
        .global_usage
        .unwrap();
    assert_eq!((u.model_requests, u.reported_tokens), (1, 11));
    Ok(())
}

#[tokio::test]
async fn global_pause_parks_quiescent_work_preserves_grants_and_late_receipts() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    set(&store, None, global(true, 2, 20, 1)).await?;
    let error = store
        .admit_model_request(&attempt.id, &worker, 1002)
        .await
        .unwrap_err();
    assert!(error.is::<WorkflowPaused>());
    assert!(
        !store
            .workflow_dispatch_ready(&attempt.id, &worker, 1002, 4)
            .await?
    );
    assert!(
        store.attempt_snapshot(&attempt.id).await?.runtime_active,
        "in-flight work retains capacity"
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 9).await?;
    assert!(
        !store
            .workflow_dispatch_ready(&attempt.id, &worker, 1002, 4)
            .await?
    );
    assert!(!store.attempt_snapshot(&attempt.id).await?.runtime_active);
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 4)
            .await?
            .is_none()
    );
    assert!(
        set(&store, None, global(false, 2, 20, 1)).await.is_err(),
        "stale emergency resume refuses"
    );
    set(&store, Some(1), global(false, 2, 20, 1)).await?;
    assert!(
        store
            .workflow_dispatch_ready(&attempt.id, &worker, 1003, 4)
            .await?
    );
    assert_eq!(
        store.execution_view(&attempt.task_id).await?.authorization,
        before.authorization
    );
    store
        .admit_model_request(&attempt.id, &worker, 1003)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 2, 25).await?;
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1004)
            .await
            .is_err(),
        "reported token overshoot still fences the next admission"
    );
    let current = store.execution_view(&attempt.task_id).await?;
    store
        .stop_task(&attempt.task_id, current.task.revision, true)
        .await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    let u = store
        .operational_report(None, None, 200, 1004)
        .await?
        .global_usage
        .unwrap();
    assert_eq!(
        (
            u.model_requests,
            u.reported_tokens,
            u.active,
            u.unresolved_requests
        ),
        (2, 25, 0, 0)
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    let (history, _) = store
        .list_definitions(OrganizationRecordKind::ControlHistory, None, 200)
        .await?;
    assert_eq!(history.len(), 2);
    assert_eq!(
        store
            .operational_report(None, None, 200, 1005)
            .await?
            .global_usage
            .unwrap(),
        u
    );
    Ok(())
}

#[tokio::test]
async fn priorities_age_deterministically_and_paused_projects_do_not_fill_candidates() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Arbitrary", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let paused = execution_project(&store, &node.id).await?;
    for _ in 0..40 {
        let task = queued(&store, &node.id, &paused, 1000).await?;
        set(
            &store,
            None,
            OrganizationControlValue::Priority {
                task_id: task.id,
                task_revision: task.revision,
                priority: 10,
            },
        )
        .await?;
    }
    set(
        &store,
        None,
        project(&paused, OrganizationProjectState::Paused, vec![]),
    )
    .await?;
    let live = execution_project(&store, &node.id).await?;
    let old = queued(&store, &node.id, &live, 1000).await?;
    set(
        &store,
        None,
        OrganizationControlValue::Priority {
            task_id: old.id.clone(),
            task_revision: old.revision,
            priority: -10,
        },
    )
    .await?;
    let new = queued(&store, &node.id, &live, 2301).await?;
    set(
        &store,
        None,
        OrganizationControlValue::Priority {
            task_id: new.id.clone(),
            task_revision: new.revision,
            priority: 10,
        },
    )
    .await?;
    let grant = store.execution_view(&old.id).await?.authorization;
    assert!(
        set(
            &store,
            Some(1),
            OrganizationControlValue::Priority {
                task_id: old.id.clone(),
                task_revision: old.revision - 1,
                priority: 1
            }
        )
        .await
        .is_err()
    );
    assert!(
        set(
            &store,
            Some(1),
            OrganizationControlValue::Priority {
                task_id: old.id.clone(),
                task_revision: old.revision,
                priority: 11
            }
        )
        .await
        .is_err()
    );
    assert_eq!(store.execution_view(&old.id).await?.authorization, grant);
    let worker = Uuid::now_v7().to_string();
    let claim = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 2301, 1)
        .await?
        .unwrap();
    assert_eq!(
        claim.task_id, old.id,
        "age beats fresh highest priority and 40 paused candidates are filtered before LIMIT"
    );
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 2301, 1)
            .await?
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn project_completion_requires_exact_accepted_criteria_and_history_survives_reopen()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let task = store.execution_view(&attempt.task_id).await?.task;
    let proof = OrganizationProjectEvidence {
        criterion: 0,
        task_id: task.id.clone(),
        result_digest: "wrong".into(),
        explanation: "Owner-reviewed output".into(),
    };
    assert!(
        set(
            &store,
            None,
            project(
                &task.project_id,
                OrganizationProjectState::Completed,
                vec![proof.clone()]
            )
        )
        .await
        .is_err()
    );
    let paused = set(
        &store,
        None,
        project(&task.project_id, OrganizationProjectState::Paused, vec![]),
    )
    .await?;
    assert!(
        !store
            .workflow_dispatch_ready(&attempt.id, &worker, 1001, 2)
            .await?
    );
    assert!(!store.attempt_snapshot(&attempt.id).await?.runtime_active);
    set(
        &store,
        Some(paused.revision),
        project(&task.project_id, OrganizationProjectState::Active, vec![]),
    )
    .await?;
    assert!(
        store
            .workflow_dispatch_ready(&attempt.id, &worker, 1001, 2)
            .await?
    );
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
                sha256: "a".repeat(64),
                bytes: 3,
            }],
            vec![OrganizationCheckEvidence {
                immutable_inputs: None,
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "Native state evidence".into(),
                truncated: false,
            }],
            1002,
        )
        .await?;
    let current = store.execution_view(&task.id).await?;
    store
        .accept_result(&task.id, current.task.revision, &result.digest)
        .await?;
    assert!(
        set(
            &store,
            Some(2),
            project(
                &task.project_id,
                OrganizationProjectState::Completed,
                vec![proof.clone()]
            )
        )
        .await
        .is_err()
    );
    let proof = OrganizationProjectEvidence {
        result_digest: result.digest.clone(),
        ..proof
    };
    let completed = set(
        &store,
        Some(2),
        project(
            &task.project_id,
            OrganizationProjectState::Completed,
            vec![proof.clone()],
        ),
    )
    .await?;
    assert!(
        execution_task(&store, &task.owner_id, &task.project_id)
            .await
            .is_err(),
        "completed definitions stay closed until owner reopen"
    );
    set(
        &store,
        Some(completed.revision),
        project(&task.project_id, OrganizationProjectState::Archived, vec![]),
    )
    .await?;
    set(
        &store,
        Some(4),
        project(&task.project_id, OrganizationProjectState::Active, vec![]),
    )
    .await?;
    assert!(
        execution_task(&store, &task.owner_id, &task.project_id)
            .await
            .is_ok()
    );
    let view = store.execution_view(&task.id).await?;
    assert!(view.result.unwrap().accepted);
    assert_eq!(
        (
            view.authorization_model_requests,
            view.authorization_used_tokens
        ),
        (1, 9)
    );
    let (history, _) = store
        .list_definitions(OrganizationRecordKind::ControlHistory, None, 200)
        .await?;
    assert_eq!(history.len(), 5);
    assert!(history.iter().any(|r| matches!(r,OrganizationRecord::ControlHistory(h) if matches!(&h.control.value,OrganizationControlValue::Project {state:OrganizationProjectState::Completed,evidence,..} if evidence==&vec![proof.clone()]))));
    Ok(())
}

#[tokio::test]
async fn global_pool_counts_descendants_once_and_unknown_usage_remains_fenced() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let agent = node_from(
        store
            .mutate_owner(
                node_mutation("Root", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let helper = node_from(
        store
            .mutate_owner(
                node_mutation("Helper", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &agent.id).await?;
    let root_task = execution_task(&store, &agent.id, &project).await?;
    let mut scope = execution_scope();
    scope.delegates = vec![helper.id.clone()];
    scope.max_children = 1;
    store
        .authorize_task(&root_task.id, root_task.revision, scope, 1000)
        .await?;
    set(&store, None, global(false, 3, 100, 2)).await?;
    let worker = Uuid::now_v7().to_string();
    let root = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 4)
        .await?
        .unwrap();
    let tasks: Vec<OrganizationTask> = serde_json::from_value(
        store
            .coordinate(
                &root.id,
                &worker,
                "plan",
                OrganizationCoordinationCall::ProposePlan {
                    items: vec![OrganizationPlanItem {
                        key: "helper".into(),
                        owner_id: helper.id,
                        instructions: "Prepare granted report".into(),
                        criteria: vec!["Verified".into()],
                        prerequisites: vec![],
                    }],
                },
                1001,
            )
            .await?,
    )?;
    store
        .coordinate(
            &root.id,
            &worker,
            "delegate",
            OrganizationCoordinationCall::Delegate {
                task_id: tasks[0].id.clone(),
            },
            1001,
        )
        .await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    assert!(
        set(&store, Some(1), global(false, 3, 100, 1))
            .await
            .is_err(),
        "cannot lower below active usage"
    );
    store.admit_model_request(&child.id, &worker, 1002).await?;
    store.checkpoint_usage(&child.id, &worker, 1, 15).await?;
    let u = store
        .operational_report(None, None, 200, 1002)
        .await?
        .global_usage
        .unwrap();
    assert_eq!((u.model_requests, u.reported_tokens, u.active), (1, 15, 2));
    store.admit_model_request(&child.id, &worker, 1003).await?;
    store.fail_attempt(&child.id, &worker).await?;
    store.release_attempt_capacity(&child.id, &worker).await?;
    let other_project = execution_project(&store, &agent.id).await?;
    queued(&store, &agent.id, &other_project, 1004).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1004, 4)
            .await?
            .is_none(),
        "terminal unknown request fences the whole configured pool"
    );
    let u = store
        .operational_report(Some(&other_project), None, 200, 1004)
        .await?
        .global_usage
        .unwrap();
    assert_eq!(
        (u.model_requests, u.reported_tokens, u.unresolved_requests),
        (2, 15, 1)
    );
    store.checkpoint_usage(&child.id, &worker, 2, 25).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1005, 4)
            .await?
            .is_some(),
        "late exact receipt clears uncertainty without replay"
    );
    assert_eq!(
        store
            .operational_report(None, None, 200, 1005)
            .await?
            .global_usage
            .unwrap()
            .reported_tokens,
        25
    );
    Ok(())
}

#[tokio::test]
async fn team_proposals_apply_atomic_definitions_and_refuse_cycles_stale_changes_and_replay()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let root = node_from(
        store
            .mutate_owner(
                node_mutation("Root", None, OrganizationNodeKind::Group),
                None,
            )
            .await?,
    );
    let child = node_from(
        store
            .mutate_owner(
                node_mutation("Worker", Some(root.id.clone()), OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let mut renamed = root.clone();
    renamed.label = "Reviewed root".into();
    let mut retired = child.clone();
    retired.enabled = false;
    let fresh = OrganizationNode {
        id: Uuid::now_v7().to_string(),
        parent_id: Some(root.id.clone()),
        kind: OrganizationNodeKind::Agent,
        label: "Specialist".into(),
        role_version_id: None,
        model: None,
        enabled: true,
        revision: 0,
    };
    let proposal = OrganizationControlValue::Team {
        id: Uuid::now_v7().to_string(),
        reason: "Definitions only".into(),
        expected_nodes: vec![root.clone(), child.clone()],
        nodes: vec![renamed, retired, fresh.clone()],
        applied: false,
    };
    let c = set(&store, None, proposal.clone()).await?;
    assert!(
        store
            .read_definition(OrganizationRecordKind::Nodes, &fresh.id)
            .await
            .is_err()
    );
    let mut applied = proposal;
    if let OrganizationControlValue::Team { applied, .. } = &mut applied {
        *applied = true;
    }
    set(&store, Some(c.revision), applied.clone()).await?;
    assert!(set(&store, Some(2), applied).await.is_err());
    let root = node_from(Some(
        store
            .read_definition(OrganizationRecordKind::Nodes, &root.id)
            .await?,
    ));
    let child = node_from(Some(
        store
            .read_definition(OrganizationRecordKind::Nodes, &child.id)
            .await?,
    ));
    assert_eq!(root.label, "Reviewed root");
    assert!(!child.enabled);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM organization_authorizations")
            .fetch_one(&store.pool)
            .await?,
        0
    );
    let mut cyclic = root.clone();
    cyclic.parent_id = Some(child.id.clone());
    let mut p = OrganizationControlValue::Team {
        id: Uuid::now_v7().to_string(),
        reason: "Cycle".into(),
        expected_nodes: vec![root.clone(), child],
        nodes: vec![cyclic],
        applied: false,
    };
    let c = set(&store, None, p.clone()).await?;
    if let OrganizationControlValue::Team { applied, .. } = &mut p {
        *applied = true;
    }
    assert!(set(&store, Some(c.revision), p).await.is_err());
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Nodes, &root.id)
            .await?,
        OrganizationRecord::Node(root.clone())
    );
    let mut p = OrganizationControlValue::Team {
        id: Uuid::now_v7().to_string(),
        reason: "Stale".into(),
        expected_nodes: vec![root.clone()],
        nodes: vec![root.clone()],
        applied: false,
    };
    let c = set(&store, None, p.clone()).await?;
    store
        .mutate_owner(
            OrganizationMutation::PutNode {
                id: Some(root.id.clone()),
                expected_revision: Some(root.revision),
                parent_id: None,
                kind: root.kind,
                label: "Concurrent edit".into(),
                role_version_id: None,
                model: None,
                enabled: true,
            },
            None,
        )
        .await?;
    if let OrganizationControlValue::Team { applied, .. } = &mut p {
        *applied = true;
    }
    assert!(set(&store, Some(c.revision), p).await.is_err());
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn safe_startup_retries_preserve_grants_require_cleanup_and_refuse_unknown_provider_effects()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Worker", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &project).await?;
    let mut scope = execution_scope();
    scope.max_attempts = 3;
    let grant = store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let a = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    store.fail_attempt(&a.id, &worker).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1010, 2)
            .await?
            .is_none()
    );
    set(
        &store,
        None,
        OrganizationControlValue::SafeRecovery {
            task_id: task.id.clone(),
            authorization_id: grant.id.clone(),
            max_retries: 1,
            enabled: true,
        },
    )
    .await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1010, 2)
            .await?
            .is_none()
    );
    store.release_attempt_capacity(&a.id, &worker).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1009, 2)
            .await?
            .is_none()
    );
    let b = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1010, 2)
        .await?
        .unwrap();
    assert_eq!(b.authorization_id, grant.id);
    assert!(
        store
            .admit_model_request(&a.id, &worker, 1010)
            .await
            .is_err()
    );
    store.fail_attempt(&b.id, &worker).await?;
    store.release_attempt_capacity(&b.id, &worker).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1020, 2)
            .await?
            .is_none()
    );
    let view = store.execution_view(&task.id).await?;
    assert_eq!(view.authorization, Some(grant));
    assert_eq!(view.attempts.len(), 2);
    assert_eq!(view.authorization_model_requests, 0);
    let other = execution_task(&store, &node.id, &project).await?;
    let mut scope = execution_scope();
    scope.max_attempts = 2;
    let grant = store
        .authorize_task(&other.id, other.revision, scope, 1020)
        .await?;
    set(
        &store,
        None,
        OrganizationControlValue::SafeRecovery {
            task_id: other.id.clone(),
            authorization_id: grant.id,
            max_retries: 1,
            enabled: true,
        },
    )
    .await?;
    let a = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1020, 2)
        .await?
        .unwrap();
    store.admit_model_request(&a.id, &worker, 1020).await?;
    store.fail_attempt(&a.id, &worker).await?;
    store.release_attempt_capacity(&a.id, &worker).await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1030, 2)
            .await?
            .is_none()
    );
    assert_eq!(
        store
            .execution_view(&other.id)
            .await?
            .authorization_model_requests,
        1
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn cross_project_sharing_requires_current_reviewed_project_source_and_preserves_origin()
-> Result<()> {
    use codex_protocol::organization_coordination::OrganizationNoteScope;
    use sha2::Digest;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let node = node_from(
        store
            .mutate_owner(
                node_mutation("Writer", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let source = execution_project(&store, &node.id).await?;
    let target = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &source).await?;
    store
        .authorize_task(&task.id, task.revision, execution_scope(), 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let a = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    store
        .coordinate(
            &a.id,
            &worker,
            "private",
            OrganizationCoordinationCall::WriteNote {
                scope: OrganizationNoteScope::Agent,
                body: "Private context must remain private".into(),
            },
            1001,
        )
        .await?;
    let private = store
        .search_memory(&source, OrganizationNoteScope::Agent, Some(&node.id), "")
        .await?
        .notes
        .remove(0);
    let share = |note: &codex_protocol::organization_coordination::OrganizationNote, revision| {
        OrganizationMutation::ShareNote {
            source_id: note.id.clone(),
            source_sha256: format!("{:x}", sha2::Sha256::digest(note.body.as_bytes())),
            target_project_id: target.clone(),
            expected_project_revision: revision,
            body: "Explicit reviewed useful fact".into(),
        }
    };
    assert!(store.mutate_owner(share(&private, 1), None).await.is_err());
    store
        .mutate_owner(
            OrganizationMutation::PromoteNote {
                source_id: private.id.clone(),
                group_id: None,
                body: "Reviewed public fact".into(),
            },
            None,
        )
        .await?;
    let public = store
        .search_memory(&source, OrganizationNoteScope::Project, None, "")
        .await?
        .notes
        .remove(0);
    assert!(store.mutate_owner(share(&public, 99), None).await.is_err());
    store.mutate_owner(share(&public, 1), None).await?;
    let copied = store
        .search_memory(&target, OrganizationNoteScope::Project, None, "")
        .await?
        .notes
        .remove(0);
    assert!(copied.owner_reviewed);
    assert_eq!(copied.source_note_id, Some(public.id.clone()));
    assert_eq!(copied.task_id, public.task_id);
    assert_eq!(copied.attempt_id, public.attempt_id);
    assert_eq!(
        store
            .search_memory(&source, OrganizationNoteScope::Project, None, "")
            .await?
            .notes,
        vec![public.clone()]
    );
    assert!(
        store
            .search_memory(
                &target,
                OrganizationNoteScope::Project,
                None,
                "Private context"
            )
            .await?
            .notes
            .is_empty()
    );
    let mut changed = share(&public, 1);
    if let OrganizationMutation::ShareNote { source_sha256, .. } = &mut changed {
        *source_sha256 = "f".repeat(64);
    }
    assert!(store.mutate_owner(changed, None).await.is_err());
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn instruction_adoption_requires_captured_roles_identical_immutable_oracles_and_exact_review()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let baseline = node_from(
        store
            .mutate_owner(
                node_mutation("Baseline", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let Some(OrganizationRecord::Role(role)) = store
        .mutate_owner(
            OrganizationMutation::CreateRoleVersion {
                role_id: None,
                expected_version: None,
                name: "Candidate".into(),
                instructions: "Check parser edge cases".into(),
            },
            None,
        )
        .await?
    else {
        panic!("role")
    };
    let Some(OrganizationRecord::Node(candidate)) = store
        .mutate_owner(
            OrganizationMutation::PutNode {
                id: None,
                expected_revision: None,
                parent_id: None,
                kind: OrganizationNodeKind::Agent,
                label: "Candidate evaluator".into(),
                role_version_id: Some(role.id.clone()),
                model: None,
                enabled: true,
            },
            None,
        )
        .await?
    else {
        panic!("node")
    };
    let project = execution_project(&store, &baseline.id).await?;
    let worker = Uuid::now_v7().to_string();
    let mut evaluated = Vec::new();
    for (index, owner) in [&baseline.id, &candidate.id, &candidate.id]
        .iter()
        .enumerate()
    {
        let task = execution_task(&store, owner, &project).await?;
        store
            .authorize_task(&task.id, task.revision, execution_scope(), 1000)
            .await?;
        let a = store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
            .await?
            .unwrap();
        store
            .record_attempt_provider(
                &a.id,
                &worker,
                OrganizationProviderEvidence {
                    id: "fixture".into(),
                    origin: None,
                    endpoint_sha256: "b".repeat(64),
                },
                1001,
            )
            .await?;
        store.admit_model_request(&a.id, &worker, 1001).await?;
        store
            .checkpoint_usage(&a.id, &worker, 1, if index == 0 { 20 } else { 10 })
            .await?;
        let inputs = Some(vec![OrganizationArtifactEvidence {
            path: "oracle.py".into(),
            sha256: if index == 2 {
                "c".repeat(64)
            } else {
                "a".repeat(64)
            },
            bytes: 4,
        }]);
        let result = store
            .record_result(
                &a.id,
                &worker,
                vec![OrganizationArtifactEvidence {
                    path: "report.txt".into(),
                    sha256: "d".repeat(64),
                    bytes: 2,
                }],
                vec![OrganizationCheckEvidence {
                    immutable_inputs: inputs,
                    argv: execution_scope().checks[0].clone(),
                    exit_code: Some(0),
                    output: "state-only fixture oracle".into(),
                    truncated: false,
                }],
                1002,
            )
            .await?;
        let view = store.execution_view(&task.id).await?;
        store
            .accept_result(&task.id, view.task.revision, &result.digest)
            .await?;
        store.release_attempt_capacity(&a.id, &worker).await?;
        evaluated.push((task.id, result.digest));
    }
    let propose = |candidate: usize| OrganizationControlValue::Improvement {
        id: Uuid::now_v7().to_string(),
        reason: "Representative parser edge cases and measured reported tokens".into(),
        node: Box::new(baseline.clone()),
        role_version_id: role.id.clone(),
        baseline_task_id: evaluated[0].0.clone(),
        baseline_digest: evaluated[0].1.clone(),
        candidate_task_id: evaluated[candidate].0.clone(),
        candidate_digest: evaluated[candidate].1.clone(),
        applied: false,
    };
    assert!(
        set(&store, None, propose(2)).await.is_err(),
        "changed test oracle cannot qualify comparison"
    );
    let mut p = propose(1);
    let c = set(&store, None, p.clone()).await?;
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Nodes, &baseline.id)
            .await?,
        OrganizationRecord::Node(baseline.clone())
    );
    if let OrganizationControlValue::Improvement { applied, .. } = &mut p {
        *applied = true;
    }
    set(&store, Some(c.revision), p.clone()).await?;
    assert!(set(&store, Some(2), p).await.is_err());
    let updated = node_from(Some(
        store
            .read_definition(OrganizationRecordKind::Nodes, &baseline.id)
            .await?,
    ));
    assert_eq!(updated.role_version_id, Some(role.id));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM organization_authorizations")
            .fetch_one(&store.pool)
            .await?,
        3,
        "adoption grants no execution authority"
    );
    store.close().await;
    Ok(())
}
