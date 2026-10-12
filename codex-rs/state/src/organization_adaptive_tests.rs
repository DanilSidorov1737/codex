use super::*;

#[tokio::test]
async fn research_receipts_keep_findings_assumptions_and_live_scope_separate() -> Result<()> {
    use codex_protocol::organization_coordination::*;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let make = |path: &str| OrganizationCoordinationCall::RecordResearch {
        citations: vec![OrganizationResearchCitation {
            path: path.into(),
            sha256: "a".repeat(64),
            start_line: 1,
            end_line: 2,
        }],
        findings: vec!["The declared local source reports a failure".into()],
        assumptions: vec!["The larger workload may behave similarly; unverified".into()],
    };
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "foreign-research",
                make("outside.txt"),
                1001
            )
            .await
            .is_err()
    );
    let receipt = store
        .coordinate(&attempt.id, &worker, "research", make("report.txt"), 1001)
        .await?;
    let evidence: OrganizationControl = serde_json::from_value(receipt["evidence"].clone())?;
    let OrganizationControlValue::ResearchEvidence {
        findings,
        assumptions,
        source_owner_id,
        ..
    } = &evidence.value
    else {
        panic!("research")
    };
    assert_ne!(findings, assumptions);
    assert_eq!(attempt.owner_id.as_ref(), Some(source_owner_id));
    assert!(
        set_control(&store, Some(evidence.revision), evidence.value.clone())
            .await
            .is_err(),
        "owner cannot rewrite host citation evidence"
    );
    assert_eq!(
        store
            .coordinate(&attempt.id, &worker, "research", make("report.txt"), 1001)
            .await?,
        receipt
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn artifact_publication_pins_accepted_bytes_and_recipients_without_copying_authority()
-> Result<()> {
    use codex_protocol::organization_coordination::*;
    use sha2::Digest;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let body = "Exact accepted output, with an unverified claim";
    let artifact = OrganizationArtifactEvidence {
        path: "report.txt".into(),
        sha256: format!("{:x}", sha2::Sha256::digest(body.as_bytes())),
        bytes: body.len() as u64,
    };
    let result = store
        .record_result(
            &attempt.id,
            &worker,
            vec![artifact.clone()],
            vec![OrganizationCheckEvidence {
                immutable_inputs: None,
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "oracle passed".into(),
                truncated: false,
            }],
            1001,
        )
        .await?;
    let pending = store.execution_view(&attempt.task_id).await?;
    store
        .accept_result(&attempt.task_id, pending.task.revision, &result.digest)
        .await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    let source = store.execution_view(&attempt.task_id).await?;
    let (reader_worker, reader) = authorized_attempt(&store, false).await?;
    let recipient = store.execution_view(&reader.task_id).await?;
    let id = Uuid::now_v7().to_string();
    let share = OrganizationControlValue::ArtifactShare {
        id: id.clone(),
        source_task_id: source.task.id.clone(),
        source_revision: source.task.revision,
        result_digest: result.digest.clone(),
        source_project_id: source.task.project_id.clone(),
        source_owner_id: attempt.owner_id.clone().unwrap(),
        artifact,
        body: Some(body.into()),
        destinations: vec![recipient.task.project_id.clone()],
    };
    let read = OrganizationCoordinationCall::ReadSharedArtifact {
        share_id: id.clone(),
    };
    assert!(
        store
            .coordinate(
                &reader.id,
                &reader_worker,
                "unpublished",
                read.clone(),
                1002
            )
            .await
            .is_err()
    );
    let mut forged = share.clone();
    if let OrganizationControlValue::ArtifactShare { body, .. } = &mut forged {
        *body = Some("Replacement claim".into());
    }
    assert!(set_control(&store, None, forged).await.is_err());
    let published = set_control(&store, None, share.clone()).await?;
    let received = store
        .coordinate(&reader.id, &reader_worker, "approved", read.clone(), 1002)
        .await?;
    assert_eq!(received["publication"]["value"]["body"], body);
    assert_eq!(
        received["publication"]["value"]["sourceProjectId"],
        source.task.project_id
    );
    assert_eq!(received["executionAuthority"], false);
    let (other_worker, other) = authorized_attempt(&store, false).await?;
    assert!(
        store
            .coordinate(&other.id, &other_worker, "unapproved", read.clone(), 1002)
            .await
            .is_err()
    );
    let mut revoked = share.clone();
    if let OrganizationControlValue::ArtifactShare { destinations, .. } = &mut revoked {
        destinations.clear();
    }
    set_control(&store, Some(published.revision), revoked).await?;
    assert!(
        store
            .coordinate(&reader.id, &reader_worker, "revoked", read, 1002)
            .await
            .is_err()
    );
    assert_eq!(source, store.execution_view(&source.task.id).await?);
    assert_eq!(
        recipient.authorization,
        store.execution_view(&reader.task_id).await?.authorization
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn knowledge_synthesis_rechecks_sources_and_never_replaces_or_promotes_them() -> Result<()> {
    use codex_protocol::organization_coordination::*;
    use sha2::Digest;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 3).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let source: OrganizationNote = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "source",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Agent,
                    body: "Parser evidence, with an unresolved edge case".into(),
                },
                1002,
            )
            .await?,
    )?;
    let reference = OrganizationNoteReference {
        note_id: source.id.clone(),
        body_sha256: format!("{:x}", sha2::Sha256::digest(source.body.as_bytes())),
    };
    let (other_worker, other) = authorized_attempt(&store, false).await?;
    let private: OrganizationNote = serde_json::from_value(
        store
            .coordinate(
                &other.id,
                &other_worker,
                "private",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Agent,
                    body: "Private other-project context".into(),
                },
                1002,
            )
            .await?,
    )?;
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "foreign-synthesis",
                OrganizationCoordinationCall::ProposeKnowledge {
                    sources: vec![OrganizationNoteReference {
                        note_id: private.id,
                        body_sha256: format!("{:x}", sha2::Sha256::digest(private.body.as_bytes()))
                    }],
                    body: "Unauthorized summary".into(),
                    conflicts: vec![]
                },
                1002
            )
            .await
            .is_err()
    );
    let receipt = store
        .coordinate(
            &attempt.id,
            &worker,
            "synthesis",
            OrganizationCoordinationCall::ProposeKnowledge {
                sources: vec![reference.clone()],
                body: "Verify the parser edge case independently".into(),
                conflicts: vec![
                    "An earlier note claims all edge cases are covered; this is not verified"
                        .into(),
                ],
            },
            1002,
        )
        .await?;
    let OrganizationRecord::Control(proposal) = store
        .read_definition(
            OrganizationRecordKind::Controls,
            receipt["proposalId"].as_str().unwrap(),
        )
        .await?
    else {
        panic!("proposal")
    };
    let OrganizationControlValue::KnowledgeProposal { id, .. } = &proposal.value else {
        panic!("proposal")
    };
    assert!(
        store
            .read_definition(OrganizationRecordKind::Notes, id)
            .await
            .is_err()
    );
    let mut reviewed = proposal.value.clone();
    if let OrganizationControlValue::KnowledgeProposal { reviewed, .. } = &mut reviewed {
        *reviewed = true;
    }
    let retirement = |retired| OrganizationControlValue::Knowledge {
        note_id: source.id.clone(),
        body_sha256: reference.body_sha256.clone(),
        retired,
        reason: "Owner reviews source currency".into(),
    };
    set_control(&store, None, retirement(true)).await?;
    assert!(
        set_control(&store, Some(proposal.revision), reviewed.clone())
            .await
            .is_err(),
        "retired source cannot be silently consolidated"
    );
    set_control(&store, Some(1), retirement(false)).await?;
    set_control(&store, Some(proposal.revision), reviewed.clone()).await?;
    assert!(
        set_control(&store, Some(proposal.revision + 1), reviewed)
            .await
            .is_err(),
        "review cannot duplicate a private synthesis"
    );
    let OrganizationRecord::Note(summary) = store
        .read_definition(OrganizationRecordKind::Notes, id)
        .await?
    else {
        panic!("private summary")
    };
    assert_eq!(summary.scope, OrganizationNoteScope::Agent);
    assert!(
        summary.body.contains("not proof of truth")
            && summary.body.contains(&reference.body_sha256)
    );
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Notes, &source.id)
            .await?,
        OrganizationRecord::Note(source)
    );
    assert_eq!(before, store.execution_view(&attempt.task_id).await?);
    assert!(
        store
            .search_memory(
                &summary.project_id,
                OrganizationNoteScope::Project,
                None,
                "edge case"
            )
            .await?
            .notes
            .is_empty()
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn workspace_library_requires_explicit_projects_current_reviewed_public_sources_and_acl_before_search()
-> Result<()> {
    use codex_protocol::organization_coordination::*;
    use sha2::Digest;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (worker, attempt) = authorized_attempt(&store, false).await?;
    let view = store.execution_view(&attempt.task_id).await?;
    let source: OrganizationNote = serde_json::from_value(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "private-source",
                OrganizationCoordinationCall::WriteNote {
                    scope: OrganizationNoteScope::Agent,
                    body: "Shared checksum evidence".into(),
                },
                1001,
            )
            .await?,
    )?;
    let (other_worker, other) = authorized_attempt(&store, false).await?;
    let other_view = store.execution_view(&other.task_id).await?;
    let excluded = execution_project(&store, &view.task.owner_id).await?;
    let reference = |note: &OrganizationNote| OrganizationNoteReference {
        note_id: note.id.clone(),
        body_sha256: format!("{:x}", sha2::Sha256::digest(note.body.as_bytes())),
    };
    let projects = vec![
        view.task.project_id.clone(),
        other_view.task.project_id.clone(),
    ];
    assert!(
        set_control(
            &store,
            None,
            OrganizationControlValue::WorkspaceLibrary {
                projects: projects.clone(),
                sources: vec![reference(&source)]
            }
        )
        .await
        .is_err(),
        "private notes cannot be published"
    );
    store
        .mutate_owner(
            OrganizationMutation::PromoteNote {
                source_id: source.id.clone(),
                group_id: None,
                body: source.body.clone(),
            },
            None,
        )
        .await?;
    let published = store
        .search_memory(
            &view.task.project_id,
            OrganizationNoteScope::Project,
            None,
            "checksum",
        )
        .await?
        .notes
        .remove(0);
    set_control(
        &store,
        None,
        OrganizationControlValue::WorkspaceLibrary {
            projects,
            sources: vec![reference(&published)],
        },
    )
    .await?;
    let shared = store
        .search_memory(
            &other_view.task.project_id,
            OrganizationNoteScope::Project,
            None,
            "checksum",
        )
        .await?;
    assert_eq!(shared.notes, vec![published.clone()]);
    assert_eq!(shared.matched_count, 1);
    assert_eq!(
        store
            .search_memory(&excluded, OrganizationNoteScope::Project, None, "checksum")
            .await?
            .matched_count,
        0
    );
    let context = store
        .coordinate(
            &other.id,
            &other_worker,
            "library-context",
            OrganizationCoordinationCall::Context {
                query: "checksum".into(),
            },
            1002,
        )
        .await?;
    assert!(
        context["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note["id"] == published.id)
    );
    set_control(
        &store,
        None,
        OrganizationControlValue::Knowledge {
            note_id: published.id.clone(),
            body_sha256: reference(&published).body_sha256,
            retired: true,
            reason: "Owner withdrew this source".into(),
        },
    )
    .await?;
    assert!(
        store
            .search_memory(
                &other_view.task.project_id,
                OrganizationNoteScope::Project,
                None,
                "checksum"
            )
            .await?
            .notes
            .is_empty()
    );
    set_control(
        &store,
        Some(1),
        OrganizationControlValue::WorkspaceLibrary {
            projects: vec![],
            sources: vec![],
        },
    )
    .await?;
    assert_eq!(
        store
            .search_memory(
                &other_view.task.project_id,
                OrganizationNoteScope::Project,
                None,
                "checksum"
            )
            .await?
            .matched_count,
        0
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn meeting_follow_up_pins_accepted_source_bytes_preserves_dissent_and_grants_no_authority()
-> Result<()> {
    use sha2::Digest;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let agent = node_from(
        store
            .mutate_owner(
                node_mutation("Meeting source owner", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project = execution_project(&store, &agent.id).await?;
    let task = execution_task(&store, &agent.id, &project).await?;
    let mut scope = execution_scope();
    scope.artifacts = vec!["discussion/discussion-conclusion.json".into()];
    store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    let source_body=serde_json::json!({"conclusions":["Verify the implementation"],"agreements":["Use local evidence"],"dissent":["The benchmark omits large inputs"],"evidence":["oracle.py"],"followUpProposals":["Test larger inputs"]}).to_string();
    let result = store
        .record_result(
            &attempt.id,
            &worker,
            vec![OrganizationArtifactEvidence {
                path: "discussion/discussion-conclusion.json".into(),
                sha256: format!("{:x}", sha2::Sha256::digest(source_body.as_bytes())),
                bytes: source_body.len() as u64,
            }],
            vec![OrganizationCheckEvidence {
                immutable_inputs: None,
                argv: execution_scope().checks[0].clone(),
                exit_code: Some(0),
                output: "conclusion oracle passed".into(),
                truncated: false,
            }],
            1001,
        )
        .await?;
    let source = store.execution_view(&attempt.task_id).await?;
    let mutation = |revision, body: String| OrganizationMutation::MeetingFollowUp {
        source_task_id: source.task.id.clone(),
        expected_source_revision: revision,
        result_digest: result.digest.clone(),
        source_body: body,
        owner_id: source.task.owner_id.clone(),
        goal_id: source.task.goal_id.clone(),
        instructions: "Verify larger inputs and retain the dissent".into(),
        criteria: vec!["An independent large-input oracle passes".into()],
    };
    assert!(
        store
            .mutate_owner(mutation(source.task.revision, source_body.clone()), None)
            .await
            .is_err(),
        "review alone does not accept the conclusion"
    );
    store
        .accept_result(&source.task.id, source.task.revision, &result.digest)
        .await?;
    let accepted = store.execution_view(&source.task.id).await?;
    assert!(
        store
            .mutate_owner(mutation(source.task.revision, source_body.clone()), None)
            .await
            .is_err(),
        "source revision is exact"
    );
    assert!(
        store
            .mutate_owner(
                mutation(
                    accepted.task.revision,
                    source_body.replace("large inputs", "small inputs")
                ),
                None
            )
            .await
            .is_err(),
        "cannot replace evidence or dissent"
    );
    let Some(OrganizationRecord::Task(draft)) = store
        .mutate_owner(mutation(accepted.task.revision, source_body), None)
        .await?
    else {
        panic!("linked draft")
    };
    assert_eq!(draft.prerequisites, vec![source.task.id]);
    assert!(
        draft.instructions.contains("benchmark omits large inputs")
            && draft.instructions.contains(&result.digest)
    );
    assert!(
        store
            .execution_view(&draft.id)
            .await?
            .authorization
            .is_none()
    );
    assert_eq!(accepted, store.execution_view(&attempt.task_id).await?);
    store.close().await;
    Ok(())
}

async fn goal(store: &OrganizationStore, parent_id: Option<String>) -> Result<OrganizationGoal> {
    let Some(OrganizationRecord::Goal(goal)) = store
        .mutate_owner(
            OrganizationMutation::PutGoal {
                id: None,
                expected_revision: None,
                parent_id,
                objective: "Verified milestone".into(),
                criteria: vec!["Oracle passed".into()],
            },
            None,
        )
        .await?
    else {
        panic!("goal")
    };
    Ok(goal)
}

async fn bind_goal(
    store: &OrganizationStore,
    project_id: &str,
    agent: &str,
    goal: &OrganizationGoal,
) -> Result<()> {
    store
        .mutate_owner(
            OrganizationMutation::LinkProject {
                project_id: project_id.into(),
                expected_revision: Some(1),
                goal_id: Some(goal.id.clone()),
                owner_id: Some(agent.into()),
                criteria: vec!["Oracle passed".into()],
            },
            Some(("Fixture".into(), vec!["/tmp/fixture-project".into()])),
        )
        .await?;
    Ok(())
}

async fn set_control(
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
        panic!("control")
    };
    Ok(control)
}

#[tokio::test]
async fn goal_prerequisites_pin_owner_completion_and_pause_without_sharing_authority() -> Result<()>
{
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let agent = node_from(
        store
            .mutate_owner(
                node_mutation("Configured owner", None, OrganizationNodeKind::Agent),
                None,
            )
            .await?,
    );
    let project_a = execution_project(&store, &agent.id).await?;
    let project_b = execution_project(&store, &agent.id).await?;
    let a = goal(&store, None).await?;
    let b = goal(&store, None).await?;
    bind_goal(&store, &project_a, &agent.id, &a).await?;
    bind_goal(&store, &project_b, &agent.id, &b).await?;
    let metadata = |goal: &OrganizationGoal,
                    project: &str,
                    prerequisite: &OrganizationGoal,
                    other_project: &str,
                    approved| OrganizationControlValue::GoalMetadata {
        goal: Box::new(goal.clone()),
        project_id: project.into(),
        responsible_node_id: agent.id.clone(),
        prerequisites: vec![OrganizationGoalDependency {
            goal: Box::new(prerequisite.clone()),
            project_id: other_project.into(),
            cross_project_approved: approved,
        }],
    };
    assert!(
        set_control(
            &store,
            None,
            metadata(&a, &project_a, &b, &project_b, false)
        )
        .await
        .is_err()
    );
    set_control(&store, None, metadata(&a, &project_a, &b, &project_b, true)).await?;
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::PutGoal {
                    id: Some(a.id.clone()),
                    expected_revision: Some(a.revision),
                    parent_id: Some(b.id.clone()),
                    objective: a.objective.clone(),
                    criteria: a.criteria.clone()
                },
                None
            )
            .await
            .is_err(),
        "reparenting must also reject a combined dependency cycle"
    );
    assert!(
        set_control(&store, None, metadata(&b, &project_b, &a, &project_a, true))
            .await
            .is_err(),
        "explicit dependency cycle"
    );
    let child = goal(&store, Some(a.id.clone())).await?;
    assert!(
        set_control(
            &store,
            None,
            metadata(&child, &project_a, &a, &project_a, false)
        )
        .await
        .is_err(),
        "child depending on parent completion deadlocks"
    );
    let waiting = execution_task(&store, &agent.id, &project_a).await?;
    assert!(
        store
            .authorize_task(&waiting.id, waiting.revision, execution_scope(), 1000)
            .await
            .is_err()
    );
    assert!(
        store
            .execution_view(&waiting.id)
            .await?
            .authorization
            .is_none()
    );
    let proof_task = execution_task(&store, &agent.id, &project_b).await?;
    store
        .authorize_task(&proof_task.id, proof_task.revision, execution_scope(), 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    store
        .admit_model_request(&attempt.id, &worker, 1001)
        .await?;
    store.checkpoint_usage(&attempt.id, &worker, 1, 3).await?;
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
                output: "oracle passed".into(),
                truncated: false,
            }],
            1002,
        )
        .await?;
    let current = store.execution_view(&proof_task.id).await?;
    store
        .accept_result(&proof_task.id, current.task.revision, &result.digest)
        .await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    set_control(
        &store,
        None,
        OrganizationControlValue::GoalReview {
            goal: Box::new(b.clone()),
            completed: true,
            evidence: vec![OrganizationProjectEvidence {
                criterion: 0,
                task_id: proof_task.id,
                result_digest: result.digest,
                explanation: "Owner reviewed actual oracle".into(),
            }],
        },
    )
    .await?;
    store
        .authorize_task(&waiting.id, waiting.revision, execution_scope(), 1003)
        .await?;
    let next = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1003, 2)
        .await?
        .unwrap();
    store.admit_model_request(&next.id, &worker, 1003).await?;
    let before = store.execution_view(&waiting.id).await?;
    store
        .mutate_owner(
            OrganizationMutation::PutGoal {
                id: Some(b.id),
                expected_revision: Some(b.revision),
                parent_id: None,
                objective: "Changed prerequisite".into(),
                criteria: b.criteria,
            },
            None,
        )
        .await?;
    assert!(
        !store
            .workflow_dispatch_ready(&next.id, &worker, 1004, 2)
            .await?
    );
    assert!(
        store
            .admit_model_request(&next.id, &worker, 1004)
            .await
            .is_err()
    );
    let after = store.execution_view(&waiting.id).await?;
    assert_eq!(before.authorization, after.authorization);
    assert_eq!(before.token_reservations, after.token_reservations);
    assert_eq!(
        before.authorization_model_requests,
        after.authorization_model_requests
    );
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn business_deadlines_validate_exact_target_without_extending_execution() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (_, attempt) = authorized_attempt(&store, false).await?;
    let before = store.execution_view(&attempt.task_id).await?;
    let deadline = |revision, offset| OrganizationControlValue::Deadline {
        kind: OrganizationDeadlineKind::Task,
        target_id: before.task.id.clone(),
        target_revision: revision,
        due_at: Some(2_000_000_000),
        display_offset_minutes: offset,
    };
    assert!(
        set_control(&store, None, deadline(before.task.revision - 1, 0))
            .await
            .is_err()
    );
    assert!(
        set_control(&store, None, deadline(before.task.revision, 841))
            .await
            .is_err()
    );
    let saved = set_control(&store, None, deadline(before.task.revision, -240)).await?;
    assert_eq!(before, store.execution_view(&attempt.task_id).await?);
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Controls, &saved.id)
            .await?,
        OrganizationRecord::Control(saved)
    );
    assert_eq!(before, store.execution_view(&attempt.task_id).await?);
    store.close().await;
    Ok(())
}

#[tokio::test]
async fn conversation_prompts_are_atomic_scoped_drafts_and_replies_survive_restart() -> Result<()> {
    use codex_protocol::organization_coordination::OrganizationCoordinationCall;
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let (other_worker, other) = authorized_attempt(&store, false).await?;
    let other_view = store.execution_view(&other.task_id).await?;
    let prompt = |id, expected_revision, prompt: &str| OrganizationMutation::ConversationPrompt {
        id,
        expected_revision,
        project_id: other_view.task.project_id.clone(),
        agent_id: other_view.task.owner_id.clone(),
        topic: "Local planning".into(),
        prompt: prompt.into(),
    };
    let Some(OrganizationRecord::Control(first)) = store
        .mutate_owner(prompt(None, None, "Explain the next step"), None)
        .await?
    else {
        panic!("conversation")
    };
    let OrganizationControlValue::Conversation { id, entries, .. } = &first.value else {
        panic!("conversation")
    };
    let task_id = entries[0].task_id.clone();
    let conversation_id = id.clone();
    let draft = store.execution_view(&task_id).await?;
    assert_eq!(draft.task.state, "proposed");
    assert!(draft.authorization.is_none());
    assert!(
        store
            .mutate_owner(
                prompt(Some(conversation_id.clone()), Some(1), "Do more"),
                None
            )
            .await
            .is_err()
    );
    store
        .admit_model_request(&other.id, &other_worker, 1001)
        .await?;
    assert!(
        store
            .coordinate(
                &other.id,
                &other_worker,
                "foreign-reply",
                OrganizationCoordinationCall::ConversationReply {
                    body: "spoofed reply".into()
                },
                1001
            )
            .await
            .is_err()
    );
    store
        .authorize_task(&task_id, draft.task.revision, execution_scope(), 1001)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1002, 2)
        .await?
        .unwrap();
    assert_eq!(attempt.task_id, task_id);
    store
        .admit_model_request(&attempt.id, &worker, 1002)
        .await?;
    let reply = store
        .coordinate(
            &attempt.id,
            &worker,
            "reply",
            OrganizationCoordinationCall::ConversationReply {
                body: "Draft a bounded task; request owner review".into(),
            },
            1002,
        )
        .await?;
    assert_eq!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "reply",
                OrganizationCoordinationCall::ConversationReply {
                    body: "Draft a bounded task; request owner review".into()
                },
                1002
            )
            .await?,
        reply
    );
    assert!(
        store
            .coordinate(
                &attempt.id,
                &worker,
                "overwrite",
                OrganizationCoordinationCall::ConversationReply {
                    body: "replaced".into()
                },
                1002
            )
            .await
            .is_err()
    );
    store.checkpoint_usage(&attempt.id, &worker, 1, 3).await?;
    store.fail_attempt(&attempt.id, &worker).await?;
    store.release_attempt_capacity(&attempt.id, &worker).await?;
    let record = store
        .read_definition(OrganizationRecordKind::Controls, &first.id)
        .await?;
    let OrganizationRecord::Control(current) = record else {
        unreachable!()
    };
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SetControl {
                    expected_revision: Some(current.revision),
                    value: current.value.clone()
                },
                None
            )
            .await
            .is_err()
    );
    store.close().await;
    let store = OrganizationStore::open(&home.0).await?;
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Controls, &first.id)
            .await?,
        OrganizationRecord::Control(current.clone())
    );
    let Some(OrganizationRecord::Control(next)) = store
        .mutate_owner(
            prompt(
                Some(conversation_id),
                Some(current.revision),
                "Turn that into a proposal",
            ),
            None,
        )
        .await?
    else {
        unreachable!()
    };
    let OrganizationControlValue::Conversation { entries, .. } = next.value else {
        unreachable!()
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0].reply_attempt_id.as_deref(),
        Some(attempt.id.as_str())
    );
    let next_task = store.execution_view(&entries[1].task_id).await?;
    assert!(next_task.authorization.is_none());
    assert!(next_task.task.instructions.contains("Draft a bounded task"));
    store.close().await;
    Ok(())
}

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
