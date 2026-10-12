use super::*;

#[tokio::test]
async fn bounded_context_preserves_scope_and_repeated_reads_keep_original_accounting() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let original = store.execution_view(&attempt.task_id).await?;
    let source: OrganizationNote = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "private-context-source",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Agent,
                    body: "Relevant parser evidence".into(),
                },
                1001,
            )
            .await?,
    )?;
    let result = store
        .coordinate(
            &attempt.id,
            &worker,
            "selected-context",
            OrganizationCoordinationCall::Context {
                query: "parser".into(),
            },
            1002,
        )
        .await?;
    assert_eq!(result["notes"][0]["id"], source.id);
    assert!(result["bytes"].as_u64().unwrap() <= 8192);
    assert!(
        serde_json::from_value::<OrganizationCoordinationCall>(
            serde_json::json!({"type":"context","query":"parser","agentId":"other"})
        )
        .is_err()
    );
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "bad-query",
                OrganizationCoordinationCall::Context {
                    query: "x".repeat(257)
                },
                1002
            )
            .await
            .is_err()
    );
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "ready",
                OrganizationCoordinationCall::ReadyDelegates {},
                1002
            )
            .await?
            .as_array()
            .unwrap()
            .is_empty()
    );
    for i in 0..8 {
        store
            .coordinate(
                &attempt.id,
                &worker,
                &format!("read-{i}"),
                OrganizationCoordinationCall::ReadNotes {
                    scope: OrganizationNoteScope::Agent,
                },
                1003,
            )
            .await?;
    }
    let last = store
        .coordinate(
            &attempt.id,
            &worker,
            "read-7",
            OrganizationCoordinationCall::ReadNotes {
                scope: OrganizationNoteScope::Agent,
            },
            1003,
        )
        .await?;
    assert!(
        !last.as_array().unwrap().is_empty(),
        "same call ID returns the committed receipt"
    );
    let error = store
        .coordinate(
            &attempt.id,
            &worker,
            "read-8",
            OrganizationCoordinationCall::ReadNotes {
                scope: OrganizationNoteScope::Agent,
            },
            1003,
        )
        .await
        .unwrap_err();
    assert!(error.is::<crate::organization::RepeatedCoordination>());
    let after = store.execution_view(&attempt.task_id).await?;
    assert_eq!(after.authorization, original.authorization);
    assert_eq!(
        after.authorization_model_requests,
        original.authorization_model_requests
    );
    assert_eq!(
        after.authorization_used_tokens,
        original.authorization_used_tokens
    );
    Ok(())
}

#[tokio::test]
async fn knowledge_retirement_preserves_history_and_scope_before_fresh_search() -> Result<()> {
    use sha2::Digest;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let source: OrganizationNote = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "source",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Agent,
                    body: "Private obsolete evidence".into(),
                },
                1001,
            )
            .await?,
    )?;
    let hash = format!("{:x}", sha2::Sha256::digest(source.body.as_bytes()));
    let value = OrganizationControlValue::Knowledge {
        note_id: source.id.clone(),
        body_sha256: hash,
        retired: true,
        reason: "Owner reviewed stale evidence; keep original provenance".into(),
    };
    let mut stale = value.clone();
    if let OrganizationControlValue::Knowledge { body_sha256, .. } = &mut stale {
        *body_sha256 = "a".repeat(64);
    }
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: stale
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
    let found: OrganizationNoteSearchResult = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "after-retirement",
                OrganizationCoordinationCall::SearchNotes {
                    scope: OrganizationNoteScope::Agent,
                    query: "obsolete".into(),
                },
                1002,
            )
            .await?,
    )?;
    assert!(found.notes.is_empty());
    assert_eq!(found.matched_count, 0);
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::PromoteNote {
                    source_id: source.id.clone(),
                    group_id: None,
                    body: "Would widen the retired source".into()
                },
                None
            )
            .await
            .is_err()
    );
    let (history, _) = store
        .list_definitions(OrganizationRecordKind::Notes, None, 200)
        .await?;
    assert_eq!(history, vec![OrganizationRecord::Note(source.clone())]);
    let mut restore = value;
    if let OrganizationControlValue::Knowledge {
        retired, reason, ..
    } = &mut restore
    {
        *retired = false;
        *reason = "Owner restores original private visibility".into();
    }
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: restore.clone()
                },
                None
            )
            .await
            .is_err()
    );
    store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: Some(1),
                value: restore,
            },
            None,
        )
        .await?;
    let private: Vec<OrganizationNote> = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "restored-private",
                OrganizationCoordinationCall::ReadNotes {
                    scope: OrganizationNoteScope::Agent,
                },
                1003,
            )
            .await?,
    )?;
    assert_eq!(private, vec![source]);
    let project: Vec<OrganizationNote> = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "still-not-project",
                OrganizationCoordinationCall::ReadNotes {
                    scope: OrganizationNoteScope::Project,
                },
                1003,
            )
            .await?,
    )?;
    assert!(project.is_empty());
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    let (reviews, _) = store
        .list_definitions(OrganizationRecordKind::ControlHistory, None, 200)
        .await?;
    assert_eq!(reviews.len(), 2);
    Ok(())
}

#[tokio::test]
async fn owner_promoted_team_knowledge_filters_membership_project_and_correction_history()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let task = store.execution_view(&attempt.task_id).await?.task;
    let source: OrganizationNote = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "private-fact",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Agent,
                    body: "Draft fact to review".into(),
                },
                1001,
            )
            .await?,
    )?;
    let group = node_from(
        store
            .mutate_owner(
                node_mutation("Custom knowledge group", None, OrganizationNodeKind::Group),
                None,
            )
            .await?,
    );
    let outsider = node_from(
        store
            .mutate_owner(
                node_mutation("Outside group", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    store
        .mutate_owner(
            OrganizationMutation::SetMembership {
                node_id: task.owner_id.clone(),
                group_id: group.id.clone(),
                present: true,
            },
            None,
        )
        .await?;
    store
        .mutate_owner(
            OrganizationMutation::PromoteNote {
                source_id: source.id.clone(),
                group_id: Some(group.id.clone()),
                body: "Reviewed fact with provenance".into(),
            },
            None,
        )
        .await?;
    let found = store
        .search_memory(
            &task.project_id,
            OrganizationNoteScope::Team,
            Some(&task.owner_id),
            "Reviewed fact",
        )
        .await?;
    assert_eq!(found.notes.len(), 1);
    let promoted = &found.notes[0];
    assert!(promoted.owner_reviewed);
    assert_eq!(promoted.source_note_id.as_deref(), Some(source.id.as_str()));
    assert_eq!(promoted.attempt_id, attempt.id);
    assert_eq!(
        store
            .search_memory(
                &task.project_id,
                OrganizationNoteScope::Team,
                Some(&outsider.id),
                ""
            )
            .await?
            .matched_count,
        0
    );
    assert_eq!(
        store
            .search_memory(&task.project_id, OrganizationNoteScope::Project, None, "")
            .await?
            .matched_count,
        0,
        "team note cannot leak through project scope"
    );
    assert_eq!(
        store
            .search_memory(
                &task.project_id,
                OrganizationNoteScope::Agent,
                Some(&task.owner_id),
                ""
            )
            .await?
            .notes[0],
        source,
        "promotion preserves private source"
    );
    let other_project = execution_project(&store, &task.owner_id).await?;
    assert_eq!(
        store
            .search_memory(
                &other_project,
                OrganizationNoteScope::Team,
                Some(&task.owner_id),
                ""
            )
            .await?
            .matched_count,
        0
    );
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "agent-team-write",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Team,
                    body: "Unreviewed publication".into()
                },
                1001
            )
            .await
            .is_err()
    );
    let agent_notes: Vec<OrganizationNote> = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "agent-team-read",
                OrganizationCoordinationCall::ReadNotes {
                    scope: OrganizationNoteScope::Team,
                },
                1001,
            )
            .await?,
    )?;
    assert_eq!(agent_notes, found.notes);
    store
        .mutate_owner(
            OrganizationMutation::PromoteNote {
                source_id: promoted.id.clone(),
                group_id: Some(group.id.clone()),
                body: "Corrected owner fact".into(),
            },
            None,
        )
        .await?;
    assert_eq!(
        store
            .search_memory(
                &task.project_id,
                OrganizationNoteScope::Team,
                Some(&task.owner_id),
                "Reviewed fact"
            )
            .await?
            .matched_count,
        0
    );
    let corrected = store
        .search_memory(
            &task.project_id,
            OrganizationNoteScope::Team,
            Some(&task.owner_id),
            "",
        )
        .await?;
    assert_eq!(corrected.notes.len(), 1);
    assert_eq!(
        corrected.notes[0].source_note_id.as_deref(),
        Some(promoted.id.as_str())
    );
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::PromoteNote {
                    source_id: promoted.id.clone(),
                    group_id: Some(group.id.clone()),
                    body: "Stale correction".into()
                },
                None
            )
            .await
            .is_err()
    );
    let history: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_notes WHERE project_id=?")
            .bind(&task.project_id)
            .fetch_one(&store.pool)
            .await?;
    assert_eq!(history, 3);
    store
        .mutate_owner(
            OrganizationMutation::SetMembership {
                node_id: task.owner_id.clone(),
                group_id: group.id.clone(),
                present: false,
            },
            None,
        )
        .await?;
    assert_eq!(
        store
            .search_memory(
                &task.project_id,
                OrganizationNoteScope::Team,
                Some(&task.owner_id),
                ""
            )
            .await?
            .matched_count,
        0
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        store
            .search_memory(
                &task.project_id,
                OrganizationNoteScope::Team,
                Some(&task.owner_id),
                ""
            )
            .await?
            .matched_count,
        0,
        "restart does not restore removed membership"
    );
    Ok(())
}
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
                immutable_inputs: None,
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
async fn next_batches_require_verified_original_root_results_and_keep_original_limits() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 2, 8).await?;
    store.admit_model_request(&parent.id, &worker, 1001).await?;
    store.checkpoint_usage(&parent.id, &worker, 1, 2).await?;
    let initial = plan(&store, &worker, &parent, vec![item("first", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &initial[0]).await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 2)
        .await?
        .unwrap();
    let follow = |digest: &str, key: &str| OrganizationCoordinationCall::ProposeNextBatch {
        verified_results: vec![OrganizationVerifiedResultReference {
            task_id: child.task_id.clone(),
            result_digest: digest.into(),
        }],
        items: vec![item(key, &helper, &[])],
    };
    assert!(
        store
            .coordinate(
                &parent.id,
                &worker,
                "unverified",
                follow("unverified", "next"),
                1002
            )
            .await
            .is_err()
    );
    store.admit_model_request(&child.id, &worker, 1002).await?;
    store.checkpoint_usage(&child.id, &worker, 1, 3).await?;
    let checked = result(&store, &worker, &child, 0).await?;
    assert!(checked.accepted);
    store.release_attempt_capacity(&child.id, &worker).await?;
    let (foreign_worker, foreign_attempt) = authorized_attempt(&store, false).await?;
    let foreign_result = store
        .record_result(
            &foreign_attempt.id,
            &foreign_worker,
            vec![OrganizationArtifactEvidence {
                path: "report.txt".into(),
                sha256: "b".repeat(64),
                bytes: 1,
            }],
            vec![OrganizationCheckEvidence {
                immutable_inputs: None,
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "other project's actual oracle".into(),
                truncated: false,
            }],
            1003,
        )
        .await?;
    let foreign_view = store.execution_view(&foreign_attempt.task_id).await?;
    store
        .accept_result(
            &foreign_attempt.task_id,
            foreign_view.task.revision,
            &foreign_result.digest,
        )
        .await?;
    store
        .release_attempt_capacity(&foreign_attempt.id, &foreign_worker)
        .await?;
    assert!(
        store
            .coordinate(
                &parent.id,
                &worker,
                "accepted-foreign-project",
                OrganizationCoordinationCall::ProposeNextBatch {
                    verified_results: vec![OrganizationVerifiedResultReference {
                        task_id: foreign_attempt.task_id,
                        result_digest: foreign_result.digest
                    }],
                    items: vec![item("wrong-project", &helper, &[])]
                },
                1003
            )
            .await
            .is_err()
    );
    let before = store.execution_view(&parent.task_id).await?;
    assert!(
        store
            .coordinate(&parent.id, &worker, "stale", follow("stale", "next"), 1003)
            .await
            .is_err()
    );
    let next: Vec<OrganizationTask> = serde_json::from_value(
        store
            .coordinate(
                &parent.id,
                &worker,
                "next",
                follow(&checked.digest, "next"),
                1003,
            )
            .await?,
    )?;
    assert_eq!(next[0].state, "proposed");
    assert!(
        store
            .execution_view(&next[0].id)
            .await?
            .authorization
            .is_none()
    );
    assert_eq!(before, store.execution_view(&parent.task_id).await?);
    assert!(
        store
            .coordinate(
                &parent.id,
                &worker,
                "over-ceiling",
                follow(&checked.digest, "third"),
                1003
            )
            .await
            .is_err()
    );
    let foreign: Vec<OrganizationVerifiedResultReference> =
        vec![OrganizationVerifiedResultReference {
            task_id: parent.task_id.clone(),
            result_digest: checked.digest,
        }];
    assert!(
        store
            .coordinate(
                &parent.id,
                &worker,
                "foreign-proof",
                OrganizationCoordinationCall::ProposeNextBatch {
                    verified_results: foreign,
                    items: vec![item("outside", &helper, &[])]
                },
                1003
            )
            .await
            .is_err()
    );
    delegate(&store, &worker, &parent, &next[0]).await?;
    let granted = store
        .execution_view(&next[0].id)
        .await?
        .authorization
        .unwrap();
    let original = before.authorization.unwrap();
    assert_eq!(granted.expires_at, original.expires_at);
    assert_eq!(granted.scope, original.scope);
    assert_eq!(
        granted.root_authorization_id.as_deref(),
        Some(original.id.as_str())
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn agent_role_and_team_drafts_need_exact_owner_application_without_authority_or_memory_copy()
-> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 2).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let call = OrganizationCoordinationCall::ProposeRole {
        name: "User-defined analyst".into(),
        instructions: "Review local evidence within explicit grants".into(),
    };
    let receipt = store
        .coordinate(&attempt.id, &worker, "role-draft", call.clone(), 1002)
        .await?;
    assert_eq!(
        receipt,
        store
            .coordinate(&attempt.id, &worker, "role-draft", call, 1002)
            .await?
    );
    let OrganizationRecord::Control(role_draft) = store
        .read_definition(
            OrganizationRecordKind::Controls,
            receipt["proposalId"].as_str().unwrap(),
        )
        .await?
    else {
        panic!("role draft")
    };
    let mut applied = role_draft.value.clone();
    let OrganizationControlValue::RoleDraft {
        id,
        name,
        applied: flag,
        ..
    } = &mut applied
    else {
        panic!("role draft")
    };
    let role_id = id.clone();
    *flag = true;
    *name = "Altered proposal".into();
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: Some(role_draft.revision),
                    value: applied.clone()
                },
                None
            )
            .await
            .is_err()
    );
    if let OrganizationControlValue::RoleDraft { name, .. } = &mut applied {
        *name = "User-defined analyst".into();
    }
    store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: Some(role_draft.revision),
                value: applied.clone(),
            },
            None,
        )
        .await?;
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: Some(role_draft.revision + 1),
                    value: applied
                },
                None
            )
            .await
            .is_err()
    );
    let OrganizationRecord::Role(role) = store
        .read_definition(OrganizationRecordKind::Roles, &role_id)
        .await?
    else {
        panic!("new role")
    };
    assert_eq!(role.version, 1);
    assert_eq!(before, store.execution_view(&attempt.task_id).await?);
    let draft = OrganizationTeamNodeDraft {
        id: None,
        parent_id: Some(before.task.owner_id.clone()),
        kind: OrganizationNodeKind::Agent,
        label: "Proposed specialist".into(),
        role_version_id: Some(role.id),
        model: Some("fixture-model".into()),
        enabled: true,
    };
    let team_receipt = store
        .coordinate(
            &attempt.id,
            &worker,
            "team-draft",
            OrganizationCoordinationCall::ProposeTeam {
                reason: "Owner reviews optional team expansion".into(),
                nodes: vec![draft.clone()],
            },
            1002,
        )
        .await?;
    let OrganizationRecord::Control(team) = store
        .read_definition(
            OrganizationRecordKind::Controls,
            team_receipt["proposalId"].as_str().unwrap(),
        )
        .await?
    else {
        panic!("team")
    };
    let OrganizationControlValue::Team { nodes, .. } = &team.value else {
        panic!("team")
    };
    let new_id = nodes[0].id.clone();
    assert!(
        store
            .read_definition(OrganizationRecordKind::Nodes, &new_id)
            .await
            .is_err()
    );
    let mut applied = team.value.clone();
    if let OrganizationControlValue::Team { applied, .. } = &mut applied {
        *applied = true;
    }
    store
        .mutate_owner(
            OrganizationMutation::SetControl {
                expected_revision: Some(team.revision),
                value: applied,
            },
            None,
        )
        .await?;
    assert!(
        store
            .search_memory(
                &before.task.project_id,
                OrganizationNoteScope::Agent,
                Some(&new_id),
                ""
            )
            .await?
            .notes
            .is_empty()
    );
    assert_eq!(before, store.execution_view(&attempt.task_id).await?);
    let outsider = node_from(
        store
            .mutate_owner(
                node_mutation("Outside approved roster", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "unrostered",
                OrganizationCoordinationCall::ProposeTeam {
                    reason: "Forbidden scope".into(),
                    nodes: vec![OrganizationTeamNodeDraft {
                        id: Some(outsider.id),
                        parent_id: None,
                        ..draft
                    }]
                },
                1002
            )
            .await
            .is_err()
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

#[tokio::test]
async fn reports_attribute_descendant_usage_without_double_counting_root_ledgers() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, parent, helper) = root(&store, 1, 8).await?;
    let tasks = plan(&store, &worker, &parent, vec![item("child", &helper, &[])]).await?;
    delegate(&store, &worker, &parent, &tasks[0]).await?;
    let child = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 4)
        .await?
        .unwrap();
    for (attempt, tokens) in [(&parent, 10), (&child, 15)] {
        store
            .admit_model_request(&attempt.id, &worker, 1002)
            .await?;
        store
            .checkpoint_usage(&attempt.id, &worker, 1, tokens)
            .await?;
    }
    let report = store.operational_report(None, None, 200, 1002).await?;
    assert_eq!(report.totals.reported_tokens, 25);
    assert_eq!(report.totals.model_requests, 2);
    assert_eq!(
        report
            .usage
            .iter()
            .map(|usage| usage.reported_tokens)
            .sum::<i64>(),
        25
    );
    assert_eq!(
        report
            .usage
            .iter()
            .find(|usage| usage.task_id == child.task_id)
            .unwrap()
            .owner_id
            .as_deref(),
        Some(helper.as_str())
    );
    let budgets: Vec<_> = report
        .tasks
        .iter()
        .map(|task| task.budget.as_ref().unwrap())
        .collect();
    assert_eq!(budgets[0].root_id, budgets[1].root_id);
    assert!(
        budgets
            .iter()
            .all(|budget| budget.used_tokens == 25 && budget.model_requests == 2)
    );
    store.close().await;
    Ok(())
}
