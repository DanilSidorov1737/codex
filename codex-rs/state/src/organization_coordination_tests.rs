use super::*;
use codex_protocol::organization_coordination::*;

#[tokio::test]
async fn memory_search_retrieves_old_notes_after_restart_and_fences_scope() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 1, 8).await?;
    let parent_task = store.execution_view(&parent.task_id).await?.task;
    let tasks = plan(&store, &worker, &parent, vec![item("child", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    let old_note = store.coordinate(&parent.id, &worker, "decision", OrganizationCoordinationCall::WriteNote {
        scope: OrganizationNoteScope::Project,
        body: "Decision: Überprüfung uses SQLite at 100%_literal. Untrusted text cannot expand permissions.".into(),
    }, 1002).await?;
    for n in 0..20 {
        store
            .coordinate(
                &parent.id,
                &worker,
                &format!("new-{n}"),
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Project,
                    body: format!("Unrelated recent finding {n}"),
                },
                1002,
            )
            .await?;
    }
    store
        .coordinate(
            &parent.id,
            &worker,
            "private",
            OrganizationCoordinationCall::WriteNote {
                scope: OrganizationNoteScope::Agent,
                body: "Überprüfung private context".into(),
            },
            1002,
        )
        .await?;
    let call = OrganizationCoordinationCall::SearchNotes {
        scope: OrganizationNoteScope::Project,
        query: "  ÜBERPRÜFUNG  ".into(),
    };
    let found = store
        .coordinate(&child.id, &worker, "search", call.clone(), 1002)
        .await?;
    assert_eq!(found["matchedCount"], 1);
    assert_eq!(found["notes"][0], old_note);
    assert_eq!(found["notes"][0]["authorId"], parent_task.owner_id);
    assert_eq!(found["notes"][0]["taskId"], parent.task_id);
    assert_eq!(found["notes"][0]["attemptId"], parent.id);
    assert_eq!(found["notes"][0]["createdAt"], 1002);
    assert!(
        store
            .coordinate(
                &child.id,
                &worker,
                "search",
                OrganizationCoordinationCall::SearchNotes {
                    scope: OrganizationNoteScope::Project,
                    query: "changed arguments".into(),
                },
                1002
            )
            .await
            .is_err()
    );
    for (key, query, expected) in [
        ("literal", "100%_", 1),
        ("sql", "' OR 1=1 --", 0),
        ("browse", "", 21),
    ] {
        let result = store
            .coordinate(
                &child.id,
                &worker,
                key,
                OrganizationCoordinationCall::SearchNotes {
                    scope: OrganizationNoteScope::Project,
                    query: query.into(),
                },
                1002,
            )
            .await?;
        assert_eq!(result["matchedCount"], expected);
        assert_eq!(
            result["notes"].as_array().unwrap().len(),
            usize::min(expected, 16)
        );
    }
    for (key, query) in [
        ("too-large", "é".repeat(129)),
        ("control", "decision\n".into()),
    ] {
        assert!(
            store
                .coordinate(
                    &child.id,
                    &worker,
                    key,
                    OrganizationCoordinationCall::SearchNotes {
                        scope: OrganizationNoteScope::Project,
                        query,
                    },
                    1002
                )
                .await
                .is_err()
        );
    }
    let private = store
        .coordinate(
            &child.id,
            &worker,
            "private-search",
            OrganizationCoordinationCall::SearchNotes {
                scope: OrganizationNoteScope::Agent,
                query: "Überprüfung".into(),
            },
            1002,
        )
        .await?;
    assert_eq!(private, serde_json::json!({"notes": [], "matchedCount": 0}));
    let (other_worker, other) = authorized_attempt_with_limit(&store, false, 4).await?;
    assert_eq!(
        store
            .coordinate(
                &other.id,
                &other_worker,
                "other-project",
                call.clone(),
                1002
            )
            .await?,
        private
    );
    assert!(
        store
            .search_memory(
                &parent_task.project_id,
                OrganizationNoteScope::Project,
                Some(&helper),
                ""
            )
            .await
            .is_err()
    );
    assert!(
        store
            .search_memory(
                &parent_task.project_id,
                OrganizationNoteScope::Agent,
                None,
                ""
            )
            .await
            .is_err()
    );
    assert!(
        store
            .search_memory("missing", OrganizationNoteScope::Project, None, "")
            .await
            .is_err()
    );
    assert!(
        store
            .search_memory(
                &parent_task.project_id,
                OrganizationNoteScope::Agent,
                Some("missing"),
                ""
            )
            .await
            .is_err()
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        store
            .coordinate(&child.id, &worker, "search", call.clone(), 1002)
            .await?,
        found
    );
    let owner = store
        .search_memory(
            &parent_task.project_id,
            OrganizationNoteScope::Project,
            None,
            "überprüfung",
        )
        .await?;
    assert_eq!(serde_json::to_value(owner)?, found);
    assert_eq!(
        store
            .search_memory(
                &parent_task.project_id,
                OrganizationNoteScope::Agent,
                Some(&parent_task.owner_id),
                "überprüfung"
            )
            .await?
            .matched_count,
        1
    );
    let fresh_task = execution_task(&store, &parent_task.owner_id, &parent_task.project_id).await?;
    store
        .authorize_task(&fresh_task.id, fresh_task.revision, execution_scope(), 1003)
        .await?;
    let fresh = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 4)
        .await?
        .unwrap();
    assert_eq!(fresh.task_id, fresh_task.id);
    assert_eq!(
        store
            .coordinate(&fresh.id, &worker, "fresh-search", call, 1003)
            .await?,
        found
    );
    let current = store.execution_view(&fresh.task_id).await?;
    store
        .stop_task(&fresh.task_id, current.task.revision, false)
        .await?;
    assert!(
        store
            .coordinate(
                &fresh.id,
                &worker,
                "fresh-search",
                OrganizationCoordinationCall::SearchNotes {
                    scope: OrganizationNoteScope::Project,
                    query: "  ÜBERPRÜFUNG  ".into(),
                },
                1003
            )
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn scheduled_task_is_not_claimed_early_and_attempt_pins_role_and_policy() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let Some(OrganizationRecord::Role(role)) = store
        .mutate_owner(
            OrganizationMutation::CreateRoleVersion {
                role_id: None,
                expected_version: None,
                name: "Navigator".into(),
                instructions: "Check real evidence".into(),
            },
            None,
        )
        .await?
    else {
        unreachable!()
    };
    let mut mutation = node_mutation("Independent explorer", None, OrganizationNodeKind::Agent);
    let OrganizationMutation::PutNode {
        role_version_id, ..
    } = &mut mutation
    else {
        unreachable!()
    };
    *role_version_id = Some(role.id.clone());
    let node = node_from(store.mutate_owner(mutation, None).await?);
    let project = execution_project(&store, &node.id).await?;
    let task = execution_task(&store, &node.id, &project).await?;
    let mut scope = execution_scope();
    scope.not_before = Some(1100);
    let grant = store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    assert_eq!(grant.policy_version, ORGANIZATION_POLICY_VERSION);
    let worker = Uuid::now_v7().to_string();
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1099, 2)
            .await?
            .is_none()
    );
    assert!(store.execution_view(&task.id).await?.attempts.is_empty());
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1100, 2)
        .await?
        .unwrap();
    assert_eq!(attempt.role_version_id, Some(role.id));
    assert_eq!(attempt.authorization_id, grant.id);
    // An older persisted host policy is never silently upgraded to new tools.
    let mut data = serde_json::to_value(&grant)?;
    data.as_object_mut().unwrap().remove("policyVersion");
    sqlx::query("UPDATE organization_authorizations SET data=? WHERE id=?")
        .bind(data.to_string())
        .bind(&grant.id)
        .execute(&store.pool)
        .await?;
    assert!(
        store
            .admit_model_request(&attempt.id, &worker, 1101)
            .await
            .is_err()
    );
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "legacy",
                OrganizationCoordinationCall::Inbox {},
                1101
            )
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn four_waiting_roots_reserve_space_for_children_without_admitting_a_fifth_root() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 1, 8).await?;
    let tasks = plan(&store, &worker, &parent, vec![item("child", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    store
        .wait_for_children(&parent.id, &worker, &[tasks[0].id.clone()], 1001)
        .await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 2)
        .await?
        .unwrap();
    assert_eq!(child.task_id, tasks[0].id);
    for _ in 0..3 {
        let (owner, attempt) = authorized_attempt(&store, true).await?;
        store
            .prepare_action(
                &attempt.id,
                &owner,
                "wait-review",
                "exec_command",
                "{}",
                1001,
            )
            .await?;
    }
    let view = store.execution_view(&parent.task_id).await?;
    let fifth = execution_task(&store, &view.task.owner_id, &view.task.project_id).await?;
    store
        .authorize_task(&fifth.id, fifth.revision, execution_scope(), 1001)
        .await?;
    let live_roots:i64=sqlx::query_scalar("SELECT count(*) FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id WHERE a.parent_attempt_id IS NULL AND p.state IN ('running','waitingApproval','waitingDependency')").fetch_one(&store.pool).await?;
    assert_eq!(live_roots, 4);
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none()
    );
    assert_eq!(store.execution_view(&fifth.id).await?.task.state, "queued");
    assert_eq!(store.execution_view(&tasks[0].id).await?.attempts.len(), 1);
    store.close().await;
    Ok(())
}

async fn root(
    store: &OrganizationStore,
    children: u32,
    requests: u32,
) -> Result<(String, OrganizationTaskAttempt, String)> {
    let lead = node_from(
        store
            .mutate_owner(
                node_mutation("Cartographer", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let helper = node_from(
        store
            .mutate_owner(
                node_mutation("Proofreader", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(store, &lead.id).await?;
    let task = execution_task(store, &lead.id, &project).await?;
    let mut scope = execution_scope();
    scope.delegates = vec![helper.id.clone()];
    scope.max_children = children;
    scope.max_model_requests = requests;
    scope.require_tool_approval = false;
    scope.auto_accept_children = true;
    store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    Ok((worker, attempt, helper.id))
}

fn item(key: &str, owner: &str, prerequisites: &[&str]) -> OrganizationPlanItem {
    OrganizationPlanItem {
        key: key.into(),
        owner_id: owner.into(),
        instructions: "Produce a checked report".into(),
        criteria: vec!["Owner-declared check passes".into()],
        prerequisites: prerequisites.iter().map(|key| (*key).into()).collect(),
    }
}

async fn plan(
    store: &OrganizationStore,
    worker: &str,
    parent: &OrganizationTaskAttempt,
    items: Vec<OrganizationPlanItem>,
) -> Result<Vec<OrganizationTask>> {
    Ok(serde_json::from_value(
        store
            .coordinate(
                &parent.id,
                worker,
                "plan",
                OrganizationCoordinationCall::ProposePlan { items },
                1001,
            )
            .await?,
    )?)
}

async fn delegate(
    store: &OrganizationStore,
    worker: &str,
    parent: &OrganizationTaskAttempt,
    task: &OrganizationTask,
) -> Result<()> {
    store
        .coordinate(
            &parent.id,
            worker,
            &format!("delegate-{}", task.id),
            OrganizationCoordinationCall::Delegate {
                task_id: task.id.clone(),
            },
            1001,
        )
        .await?;
    Ok(())
}

async fn result(
    store: &OrganizationStore,
    worker: &str,
    attempt: &OrganizationTaskAttempt,
    exit_code: i32,
) -> Result<OrganizationTaskResult> {
    store
        .record_result(
            &attempt.id,
            worker,
            vec![OrganizationArtifactEvidence {
                path: "report.txt".into(),
                sha256: "a".repeat(64),
                bytes: 1,
            }],
            vec![OrganizationCheckEvidence {
                argv: vec!["python3".into(), "oracle.py".into()],
                exit_code: Some(exit_code),
                output: "observed check".into(),
                truncated: false,
            }],
            1003,
        )
        .await
}

#[tokio::test]
async fn planner_is_atomic_bounded_and_deduplicated() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 2, 8).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_tasks")
        .fetch_one(&store.pool)
        .await?;
    for items in [
        vec![item("a", &helper, &["b"]), item("b", &helper, &["a"])],
        vec![
            item("valid", &helper, &[]),
            item("broken", &helper, &["missing"]),
        ],
        vec![item("a", &helper, &[]), item("a", &helper, &[])],
        vec![item("a", "not-authorized", &[])],
    ] {
        assert!(
            store
                .coordinate(
                    &parent.id,
                    &worker,
                    "invalid",
                    OrganizationCoordinationCall::ProposePlan { items },
                    1001
                )
                .await
                .is_err()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM organization_tasks")
                .fetch_one(&store.pool)
                .await?,
            before
        );
    }
    let items = vec![
        item("second", &helper, &["first"]),
        item("first", &helper, &[]),
    ];
    let tasks = plan(&store, &worker, &parent, items.clone()).await?;
    assert_eq!(tasks[1].prerequisites, vec![tasks[0].id.clone()]);
    assert_eq!(plan(&store, &worker, &parent, items).await?, tasks);
    assert!(
        plan(
            &store,
            &worker,
            &parent,
            vec![item("changed", &helper, &[])]
        )
        .await
        .is_err()
    );
    assert!(
        store
            .coordinate(
                &parent.id,
                &worker,
                "over-limit",
                OrganizationCoordinationCall::ProposePlan {
                    items: vec![item("extra", &helper, &[])]
                },
                1001
            )
            .await
            .is_err()
    );
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    delegate(&store, &worker, &parent, &tasks[1]).await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    assert_eq!(child.task_id, tasks[0].id);
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
            .await?
            .is_none()
    );
    assert!(result(&store, &worker, &child, 0).await?.accepted);
    store.release_attempt_capacity(&child.id, &worker).await?;
    assert_eq!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 4)
            .await?
            .unwrap()
            .task_id,
        tasks[1].id
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn children_race_against_one_root_request_and_token_budget() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 2, 2).await?;
    store.admit_model_request(&parent.id, &worker, 1001).await?;
    store.checkpoint_usage(&parent.id, &worker, 1, 20).await?;
    let tasks = plan(
        &store,
        &worker,
        &parent,
        vec![item("one", &helper, &[]), item("two", &helper, &[])],
    )
    .await?;
    for task in &tasks {
        delegate(&store, &worker, &parent, task).await?;
    }
    let one = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    let two = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    let (a, b) = tokio::join!(
        store.admit_model_request(&one.id, &worker, 1002),
        store.admit_model_request(&two.id, &worker, 1002)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let admitted = if a.is_ok() { &one } else { &two };
    store
        .checkpoint_usage(&admitted.id, &worker, 1, 120)
        .await?;
    store
        .checkpoint_usage(&admitted.id, &worker, 1, 120)
        .await?;
    assert!(
        store
            .checkpoint_usage(&admitted.id, &worker, 1, 121)
            .await
            .is_err()
    );
    let view = store.execution_view(&parent.task_id).await?;
    assert_eq!(view.authorization_model_requests, 2);
    assert_eq!(view.authorization_used_tokens, 140);
    let child_view = store.execution_view(&admitted.task_id).await?;
    assert_eq!(child_view.authorization_used_tokens, 140);
    assert_eq!(child_view.authorization_model_requests, 2);
    assert_eq!(child_view.attempts[0].reported_tokens, 120);
    assert!(
        store
            .admit_model_request(&parent.id, &worker, 1003)
            .await
            .is_err()
    );
    assert!(
        store
            .prepare_action(&two.id, &worker, "overbudget", "exec_command", "{}", 1003)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn streamed_wait_yields_only_after_usage_then_reacquires_after_native_cleanup() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 1, 8).await?;
    let tasks = plan(&store, &worker, &parent, vec![item("child", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    store.admit_model_request(&parent.id, &worker, 1001).await?;
    store
        .wait_for_children(&parent.id, &worker, &[tasks[0].id.clone()], 1001)
        .await?;
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1001, 1)
            .await?
            .is_none()
    );
    store.checkpoint_usage(&parent.id, &worker, 1, 20).await?;
    store.heartbeat_attempt(&parent.id, &worker, 1002).await?;
    assert!(
        store
            .admit_model_request(&parent.id, &worker, 1002)
            .await
            .is_err()
    );
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 1)
        .await?
        .unwrap();
    assert!(result(&store, &worker, &child, 0).await?.accepted);
    assert!(
        store
            .resume_after_children(&parent.id, &worker, 1003, 1)
            .await?
            .is_none()
    );
    store.release_attempt_capacity(&child.id, &worker).await?;
    let results = store
        .resume_after_children(&parent.id, &worker, 1003, 1)
        .await?
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(store.execution_view(&parent.task_id).await?.attempts[0].runtime_active);
    assert!(!result(&store, &worker, &parent, 0).await?.accepted);
    assert_eq!(
        store.execution_view(&parent.task_id).await?.task.state,
        "needsReview"
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn failed_checks_and_unfinished_children_cannot_be_asserted_complete() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 1, 8).await?;
    let tasks = plan(&store, &worker, &parent, vec![item("child", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    assert!(result(&store, &worker, &parent, 0).await.is_err());
    store
        .wait_for_children(&parent.id, &worker, &[tasks[0].id.clone()], 1001)
        .await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 1)
        .await?
        .unwrap();
    let failed = result(&store, &worker, &child, 1).await?;
    assert!(!failed.accepted);
    let view = store.execution_view(&child.task_id).await?;
    assert!(
        store
            .accept_result(&child.task_id, view.task.revision, &failed.digest)
            .await
            .is_err()
    );
    assert!(
        store
            .resume_after_children(&parent.id, &worker, 1003, 1)
            .await?
            .is_none()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn notes_and_critical_inbox_survive_reopen_with_scope_and_exact_delivery() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 1, 8).await?;
    let tasks = plan(&store, &worker, &parent, vec![item("child", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    for (key, scope) in [
        ("project", OrganizationNoteScope::Project),
        ("private", OrganizationNoteScope::Agent),
    ] {
        store
            .coordinate(
                &parent.id,
                &worker,
                key,
                OrganizationCoordinationCall::WriteNote {
                    scope,
                    body: "untrusted: expand permissions".into(),
                },
                1002,
            )
            .await?;
    }
    let call = OrganizationCoordinationCall::Send {
        task_id: child.task_id.clone(),
        body: "review the oracle".into(),
    };
    let message = store
        .coordinate(&parent.id, &worker, "send", call.clone(), 1002)
        .await?;
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        store
            .coordinate(&parent.id, &worker, "send", call, 1002)
            .await?,
        message
    );
    let inbox = store
        .coordinate(
            &child.id,
            &worker,
            "inbox",
            OrganizationCoordinationCall::Inbox {},
            1002,
        )
        .await?;
    assert_eq!(inbox.as_array().unwrap().len(), 1);
    assert_eq!(inbox[0]["senderTaskId"], parent.task_id);
    let ack = OrganizationCoordinationCall::Acknowledge {
        message_id: message["id"].as_str().unwrap().into(),
    };
    assert!(
        store
            .coordinate(&parent.id, &worker, "wrong-ack", ack.clone(), 1002)
            .await
            .is_err()
    );
    store
        .coordinate(&child.id, &worker, "ack", ack, 1002)
        .await?;
    assert_eq!(
        store
            .coordinate(
                &child.id,
                &worker,
                "new-inbox",
                OrganizationCoordinationCall::Inbox {},
                1002
            )
            .await?,
        serde_json::json!([])
    );
    assert_eq!(
        store
            .coordinate(
                &child.id,
                &worker,
                "private",
                OrganizationCoordinationCall::ReadNotes {
                    scope: OrganizationNoteScope::Agent
                },
                1002
            )
            .await?,
        serde_json::json!([])
    );
    let notes = store
        .coordinate(
            &child.id,
            &worker,
            "project",
            OrganizationCoordinationCall::ReadNotes {
                scope: OrganizationNoteScope::Project,
            },
            1002,
        )
        .await?;
    assert_eq!(notes.as_array().unwrap().len(), 1);
    assert_eq!(notes[0]["attemptId"], parent.id);
    let (other_worker, other) = authorized_attempt_with_limit(&store, false, 4).await?;
    assert_eq!(
        store
            .coordinate(
                &other.id,
                &other_worker,
                "other-project",
                OrganizationCoordinationCall::ReadNotes {
                    scope: OrganizationNoteScope::Project
                },
                1002
            )
            .await?,
        serde_json::json!([])
    );
    assert!(
        store
            .coordinate(
                &other.id,
                &other_worker,
                "send",
                OrganizationCoordinationCall::Send {
                    task_id: child.task_id.clone(),
                    body: "cross project".into()
                },
                1002
            )
            .await
            .is_err()
    );
    let pending = store
        .coordinate(
            &parent.id,
            &worker,
            "pending-on-restart",
            OrganizationCoordinationCall::Send {
                task_id: child.task_id.clone(),
                body: "Retain through reconciliation".into(),
            },
            1002,
        )
        .await?;
    assert_eq!(store.reconcile_interrupted_attempts().await?, 3);
    assert!(
        store
            .coordinate(
                &child.id,
                &worker,
                "stale",
                OrganizationCoordinationCall::Inbox {},
                1002
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM organization_messages")
            .fetch_one(&store.pool)
            .await?,
        2
    );
    let recovered = store.execution_view(&child.task_id).await?;
    store
        .authorize_task(
            &child.task_id,
            recovered.task.revision,
            execution_scope(),
            1003,
        )
        .await?;
    let fresh = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 2)
        .await?
        .unwrap();
    assert_eq!(fresh.task_id, child.task_id);
    let inbox = store
        .coordinate(
            &fresh.id,
            &worker,
            "recovered-inbox",
            OrganizationCoordinationCall::Inbox {},
            1003,
        )
        .await?;
    assert_eq!(inbox.as_array().unwrap().len(), 1);
    assert_eq!(inbox[0]["id"], pending["id"]);
    assert_ne!(inbox[0]["rootAuthorizationId"], fresh.authorization_id);
    let notes = store
        .coordinate(
            &fresh.id,
            &worker,
            "recovered-notes",
            OrganizationCoordinationCall::ReadNotes {
                scope: OrganizationNoteScope::Project,
            },
            1003,
        )
        .await?;
    assert_eq!(notes.as_array().unwrap().len(), 1);
    store
        .coordinate(
            &fresh.id,
            &worker,
            "recovered-ack",
            OrganizationCoordinationCall::Acknowledge {
                message_id: pending["id"].as_str().unwrap().into(),
            },
            1003,
        )
        .await?;
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn parent_pause_fences_child_and_late_usage_still_debits_root() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 2, 8).await?;
    let tasks = plan(
        &store,
        &worker,
        &parent,
        vec![item("child", &helper, &[]), item("queued", &helper, &[])],
    )
    .await?;
    for task in &tasks {
        delegate(&store, &worker, &parent, task).await?;
    }
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 2)
        .await?
        .unwrap();
    store.admit_model_request(&child.id, &worker, 1002).await?;
    let action = store
        .prepare_action(&child.id, &worker, "intent", "exec_command", "{}", 1002)
        .await?;
    let view = store.execution_view(&parent.task_id).await?;
    store
        .stop_task(&parent.task_id, view.task.revision, false)
        .await?;
    assert!(
        store
            .dispatch_action(&action.id, &worker, 1003, 4)
            .await
            .is_err()
    );
    assert!(
        store
            .heartbeat_attempt(&child.id, &worker, 1003)
            .await
            .is_err()
    );
    assert!(
        store
            .coordinate(
                &child.id,
                &worker,
                "inbox",
                OrganizationCoordinationCall::Inbox {},
                1003
            )
            .await
            .is_err()
    );
    assert!(
        store
            .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 4)
            .await?
            .is_none()
    );
    store.checkpoint_usage(&child.id, &worker, 1, 15).await?;
    assert_eq!(
        store
            .execution_view(&parent.task_id)
            .await?
            .authorization_used_tokens,
        15
    );
    assert!(
        store
            .admit_model_request(&child.id, &worker, 1003)
            .await
            .is_err()
    );
    store.close().await;
    Ok(())
}
