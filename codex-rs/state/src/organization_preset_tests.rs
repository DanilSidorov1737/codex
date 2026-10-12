use super::*;
use codex_protocol::organization_coordination::OrganizationCoordinationCall;
use codex_protocol::organization_coordination::OrganizationNoteScope;

fn preset(record: Option<OrganizationRecord>) -> OrganizationPreset {
    let Some(OrganizationRecord::Preset(preset)) = record else {
        panic!("expected preset")
    };
    *preset
}

#[tokio::test]
async fn preset_snapshot_clone_is_atomic_fresh_and_copies_no_execution_authority() -> Result<()> {
    let home = Fixture::new();
    let store = OrganizationStore::open(&home.0).await?;
    let root = node_from(
        store
            .mutate_owner(
                node_mutation("User-chosen team", None, OrganizationNodeKind::Group),
                None,
            )
            .await?,
    );
    let team = node_from(
        store
            .mutate_owner(
                node_mutation(
                    "Extra group",
                    Some(root.id.clone()),
                    OrganizationNodeKind::Group,
                ),
                None,
            )
            .await?,
    );
    let outside = node_from(
        store
            .mutate_owner(
                node_mutation("Outside", None, OrganizationNodeKind::Group),
                None,
            )
            .await?,
    );
    let Some(OrganizationRecord::Role(role)) = store
        .mutate_owner(
            OrganizationMutation::CreateRoleVersion {
                role_id: None,
                expected_version: None,
                name: "Custom role".into(),
                instructions: "Bounded work".into(),
            },
            None,
        )
        .await?
    else {
        panic!("expected role")
    };
    let agent = node_from(
        store
            .mutate_owner(
                OrganizationMutation::PutNode {
                    id: None,
                    expected_revision: None,
                    parent_id: Some(root.id.clone()),
                    kind: OrganizationNodeKind::Agent,
                    label: "Arbitrary agent".into(),
                    role_version_id: Some(role.id.clone()),
                    model: Some("model".into()),
                    enabled: true,
                },
                None,
            )
            .await?,
    );
    for group in [&team.id, &outside.id] {
        store
            .mutate_owner(
                OrganizationMutation::SetMembership {
                    node_id: agent.id.clone(),
                    group_id: group.clone(),
                    present: true,
                },
                None,
            )
            .await?;
    }
    let project = execution_project(&store, &agent.id).await?;
    let task = execution_task(&store, &agent.id, &project).await?;
    let mut scope = execution_scope();
    scope.model = "model".into();
    let grant = store
        .authorize_task(&task.id, task.revision, scope, 1000)
        .await?;
    let worker = Uuid::now_v7().to_string();
    let attempt = store
        .claim_task(&worker, &Uuid::now_v7().to_string(), 1000, 2)
        .await?
        .unwrap();
    let private_body = "Private-only knowledge must never be in an organization preset";
    store
        .coordinate(
            &attempt.id,
            &worker,
            "private-note",
            OrganizationCoordinationCall::WriteNote {
                scope: OrganizationNoteScope::Agent,
                body: private_body.into(),
            },
            1001,
        )
        .await?;
    let saved = preset(
        store
            .mutate_owner(
                OrganizationMutation::SavePreset {
                    root_id: root.id.clone(),
                    expected_root_revision: root.revision,
                    label: "Optional custom organization".into(),
                },
                None,
            )
            .await?,
    );
    assert_eq!(saved.nodes.len(), 3);
    assert_eq!(saved.roles, vec![role.clone()]);
    assert_eq!(saved.memberships.len(), 1);
    assert!(
        saved
            .nodes
            .iter()
            .find(|node| node.id == root.id)
            .unwrap()
            .parent_id
            .is_none()
    );
    let serialized = serde_json::to_string(&saved)?;
    assert!(!serialized.contains(&grant.id) && !serialized.contains(&task.id));
    assert!(!serialized.contains(private_body) && !serialized.contains(&attempt.id));
    // Editing source definitions cannot change the captured snapshot.
    store
        .mutate_owner(
            OrganizationMutation::PutNode {
                id: Some(agent.id.clone()),
                expected_revision: Some(agent.revision),
                parent_id: Some(root.id.clone()),
                kind: OrganizationNodeKind::Agent,
                label: "Changed source".into(),
                role_version_id: Some(role.id.clone()),
                model: Some("new model".into()),
                enabled: false,
            },
            None,
        )
        .await?;
    let cloned = preset(
        store
            .mutate_owner(
                OrganizationMutation::ClonePreset {
                    id: saved.id.clone(),
                    expected_revision: saved.revision,
                    label: "Fresh named organization".into(),
                    parent_id: Some(outside.id.clone()),
                },
                None,
            )
            .await?,
    );
    let mapping = cloned.last_clone.as_ref().unwrap();
    assert_eq!(cloned.revision, 2);
    let OrganizationRecord::Node(copy) = store
        .read_definition(OrganizationRecordKind::Nodes, &mapping.node_ids[&agent.id])
        .await?
    else {
        panic!("node")
    };
    assert_eq!(copy.label, "Arbitrary agent");
    assert_eq!(copy.model.as_deref(), Some("model"));
    assert!(copy.enabled);
    assert_eq!(copy.revision, 1);
    assert_eq!(copy.parent_id.as_ref(), Some(&mapping.node_ids[&root.id]));
    assert_eq!(
        copy.role_version_id.as_ref(),
        Some(&mapping.role_ids[&role.id])
    );
    for (source, target) in &mapping.node_ids {
        assert_ne!(source, target);
    }
    for (source, target) in &mapping.role_ids {
        assert_ne!(source, target);
    }
    let OrganizationRecord::Role(copy_role) = store
        .read_definition(OrganizationRecordKind::Roles, &mapping.role_ids[&role.id])
        .await?
    else {
        panic!("role")
    };
    assert_ne!(copy_role.role_id, role.role_id);
    assert_eq!(copy_role.instructions, role.instructions);
    let members = store
        .list_definitions(OrganizationRecordKind::Memberships, None, 200)
        .await?
        .0;
    assert_eq!(members.len(), 3);
    assert!(members.iter().any(
        |record| matches!(record,OrganizationRecord::Membership(member)
        if member.node_id==copy.id && member.group_id==mapping.node_ids[&team.id])
    ));
    for (query, count) in [
        ("SELECT count(*) FROM organization_tasks", 1),
        ("SELECT count(*) FROM organization_authorizations", 1),
        ("SELECT count(*) FROM organization_attempts", 1),
        ("SELECT count(*) FROM organization_notes", 1),
    ] {
        let actual: i64 = sqlx::query_scalar(query).fetch_one(&store.pool).await?;
        assert_eq!(actual, count);
    }
    assert_eq!(
        store
            .search_memory(&project, OrganizationNoteScope::Agent, Some(&copy.id), "")
            .await?
            .matched_count,
        0
    );
    assert_eq!(
        store
            .search_memory(&project, OrganizationNoteScope::Agent, Some(&agent.id), "")
            .await?
            .matched_count,
        1
    );
    let nodes_before = store
        .list_definitions(OrganizationRecordKind::Nodes, None, 200)
        .await?
        .0;
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::ClonePreset {
                    id: saved.id.clone(),
                    expected_revision: 1,
                    label: "Stale clone".into(),
                    parent_id: None
                },
                None
            )
            .await
            .is_err()
    );
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::ClonePreset {
                    id: saved.id.clone(),
                    expected_revision: 2,
                    label: "Missing parent".into(),
                    parent_id: Some(Uuid::now_v7().to_string())
                },
                None
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .list_definitions(OrganizationRecordKind::Nodes, None, 200)
            .await?
            .0,
        nodes_before
    );
    assert_eq!(
        store
            .execution_view(&task.id)
            .await?
            .authorization
            .as_ref()
            .unwrap()
            .id,
        grant.id
    );
    assert_eq!(
        store
            .read_definition(OrganizationRecordKind::Presets, &saved.id)
            .await?,
        OrganizationRecord::Preset(Box::new(cloned))
    );
    Ok(())
}

#[tokio::test]
async fn preset_bounds_stale_capture_and_invalid_snapshot_roll_back_without_partial_clone()
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
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SavePreset {
                    root_id: root.id.clone(),
                    expected_root_revision: 2,
                    label: "Stale".into()
                },
                None
            )
            .await
            .is_err()
    );
    let saved = preset(
        store
            .mutate_owner(
                OrganizationMutation::SavePreset {
                    root_id: root.id.clone(),
                    expected_root_revision: 1,
                    label: "Snapshot".into(),
                },
                None,
            )
            .await?,
    );
    let mut invalid = saved.clone();
    invalid.nodes[0].parent_id = Some(invalid.nodes[0].id.clone());
    sqlx::query("UPDATE organization_presets SET data=? WHERE id=?")
        .bind(serde_json::to_string(&OrganizationRecord::Preset(
            Box::new(invalid),
        ))?)
        .bind(&saved.id)
        .execute(&store.pool)
        .await?;
    let before = store
        .list_definitions(OrganizationRecordKind::Nodes, None, 200)
        .await?
        .0;
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::ClonePreset {
                    id: saved.id.clone(),
                    expected_revision: 1,
                    label: "Invalid cycle".into(),
                    parent_id: None
                },
                None
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .list_definitions(OrganizationRecordKind::Nodes, None, 200)
            .await?
            .0,
        before
    );
    for index in 0..128 {
        store
            .mutate_owner(
                node_mutation(
                    &format!("Node {index}"),
                    Some(root.id.clone()),
                    OrganizationNodeKind::Group,
                ),
                None,
            )
            .await?;
    }
    assert!(
        store
            .mutate_owner(
                OrganizationMutation::SavePreset {
                    root_id: root.id.clone(),
                    expected_root_revision: 1,
                    label: "Oversized".into()
                },
                None
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .list_definitions(OrganizationRecordKind::Presets, None, 200)
            .await?
            .0
            .len(),
        1
    );
    Ok(())
}
