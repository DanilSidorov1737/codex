//! Small definition snapshots and atomic fresh clones through the owner transaction.
use super::definitions::text;
use super::execution::read_node;
use anyhow::{Context, Result, ensure};
use codex_protocol::organization::*;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub(super) async fn save(
    tx: &mut Transaction<'_, Sqlite>,
    root_id: &str,
    expected_root_revision: i64,
    label: String,
) -> Result<OrganizationPreset> {
    text(&label, 256)?;
    let root = read_node(tx, root_id).await?;
    ensure!(
        root.revision == expected_root_revision,
        "preset root revision conflict"
    );
    let data: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE subtree(id) AS (SELECT ? UNION SELECT n.id FROM organization_nodes n JOIN subtree s ON n.parent_id=s.id) SELECT n.data FROM organization_nodes n JOIN subtree s ON s.id=n.id ORDER BY n.id LIMIT 129",
    ).bind(root_id).fetch_all(&mut **tx).await?;
    ensure!(data.len() <= 128, "preset subtree exceeds 128 nodes");
    let mut nodes = Vec::new();
    for data in data {
        let OrganizationRecord::Node(mut node) = serde_json::from_str(&data)? else {
            anyhow::bail!("invalid stored node");
        };
        if node.id == root_id {
            node.parent_id = None;
        }
        nodes.push(node);
    }
    let ids: BTreeSet<_> = nodes.iter().map(|node| node.id.as_str()).collect();
    let role_ids: BTreeSet<_> = nodes
        .iter()
        .filter_map(|node| node.role_version_id.as_deref())
        .collect();
    let mut roles = Vec::new();
    for id in role_ids {
        let data: String =
            sqlx::query_scalar("SELECT data FROM organization_role_versions WHERE id=?")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;
        let OrganizationRecord::Role(role) = serde_json::from_str(&data)? else {
            anyhow::bail!("invalid stored role");
        };
        roles.push(role);
    }
    let mut memberships = Vec::new();
    // Query only the selected subtree's memberships, bounded before materializing.
    let data: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE subtree(id) AS (SELECT ? UNION SELECT n.id FROM organization_nodes n JOIN subtree s ON n.parent_id=s.id) SELECT m.data FROM organization_memberships m JOIN subtree n ON n.id=m.node_id JOIN subtree g ON g.id=m.group_id ORDER BY m.id LIMIT 513",
    ).bind(root_id).fetch_all(&mut **tx).await?;
    ensure!(data.len() <= 512, "preset exceeds 512 internal memberships");
    for data in data {
        let OrganizationRecord::Membership(member) = serde_json::from_str(&data)? else {
            anyhow::bail!("invalid stored membership");
        };
        ensure!(
            ids.contains(member.node_id.as_str()) && ids.contains(member.group_id.as_str()),
            "preset membership is outside captured subtree"
        );
        memberships.push(member);
    }
    let preset = OrganizationPreset {
        id: Uuid::now_v7().to_string(),
        label,
        revision: 1,
        nodes,
        roles,
        memberships,
        last_clone: None,
    };
    write(tx, &preset).await?;
    Ok(preset)
}

async fn write(tx: &mut Transaction<'_, Sqlite>, preset: &OrganizationPreset) -> Result<()> {
    let data = serde_json::to_string(&OrganizationRecord::Preset(Box::new(preset.clone())))?;
    ensure!(data.len() <= 128 * 1024, "preset exceeds 128 KiB");
    sqlx::query("INSERT INTO organization_presets(id,revision,data) VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,data=excluded.data")
        .bind(&preset.id).bind(preset.revision).bind(data).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn clone(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    expected_revision: i64,
    label: String,
    parent_id: Option<String>,
) -> Result<OrganizationPreset> {
    text(&label, 256)?;
    let data: String = sqlx::query_scalar("SELECT data FROM organization_presets WHERE id=?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let OrganizationRecord::Preset(mut preset) = serde_json::from_str(&data)? else {
        anyhow::bail!("invalid stored preset");
    };
    ensure!(
        preset.revision == expected_revision,
        "preset revision conflict; inspect last clone before retrying"
    );
    ensure!(
        !preset.nodes.is_empty()
            && preset.nodes.len() <= 128
            && preset.roles.len() <= 128
            && preset.memberships.len() <= 512,
        "invalid stored preset bounds"
    );
    if let Some(parent) = &parent_id {
        read_node(tx, parent).await?;
    }
    let root_id = Uuid::now_v7().to_string();
    let node_ids: BTreeMap<_, _> = preset
        .nodes
        .iter()
        .map(|node| (node.id.clone(), Uuid::now_v7().to_string()))
        .collect();
    let role_ids: BTreeMap<_, _> = preset
        .roles
        .iter()
        .map(|role| (role.id.clone(), Uuid::now_v7().to_string()))
        .collect();
    ensure!(
        node_ids.len() == preset.nodes.len() && role_ids.len() == preset.roles.len(),
        "duplicate preset identities"
    );
    for source in &preset.roles {
        let role = OrganizationRoleVersion {
            id: role_ids[&source.id].clone(),
            role_id: Uuid::now_v7().to_string(),
            name: source.name.clone(),
            instructions: source.instructions.clone(),
            version: 1,
        };
        sqlx::query(
            "INSERT INTO organization_role_versions(id,role_id,version,data) VALUES (?,?,?,?)",
        )
        .bind(&role.id)
        .bind(&role.role_id)
        .bind(role.version)
        .bind(serde_json::to_string(&OrganizationRecord::Role(
            role.clone(),
        ))?)
        .execute(&mut **tx)
        .await?;
    }
    let wrapper = OrganizationNode {
        id: root_id.clone(),
        parent_id,
        kind: OrganizationNodeKind::Group,
        label,
        role_version_id: None,
        model: None,
        enabled: true,
        revision: 1,
    };
    insert_node(tx, wrapper).await?;
    let mut pending: Vec<_> = preset.nodes.iter().collect();
    let mut inserted = BTreeSet::new();
    while !pending.is_empty() {
        let mut next = Vec::new();
        let before = inserted.len();
        for source in pending {
            if source
                .parent_id
                .as_ref()
                .is_some_and(|parent| !inserted.contains(parent))
            {
                next.push(source);
                continue;
            }
            ensure!(
                source
                    .parent_id
                    .as_ref()
                    .is_none_or(|parent| node_ids.contains_key(parent)),
                "preset parent is outside snapshot"
            );
            let role = source
                .role_version_id
                .as_ref()
                .map(|id| role_ids.get(id).cloned().context("missing preset role"))
                .transpose()?;
            ensure!(
                source.kind == OrganizationNodeKind::Agent
                    || (role.is_none() && source.model.is_none()),
                "invalid preset group"
            );
            text(&source.label, 256)?;
            if let Some(model) = &source.model {
                text(model, 256)?;
            }
            let node = OrganizationNode {
                id: node_ids[&source.id].clone(),
                parent_id: Some(
                    source
                        .parent_id
                        .as_ref()
                        .map(|id| node_ids[id].clone())
                        .unwrap_or_else(|| root_id.clone()),
                ),
                kind: source.kind,
                label: source.label.clone(),
                role_version_id: role,
                model: source.model.clone(),
                enabled: source.enabled,
                revision: 1,
            };
            insert_node(tx, node).await?;
            inserted.insert(source.id.clone());
        }
        ensure!(inserted.len() > before, "cyclic or missing preset parent");
        pending = next;
    }
    for member in &preset.memberships {
        let node = node_ids
            .get(&member.node_id)
            .context("missing preset member")?;
        let group = node_ids
            .get(&member.group_id)
            .context("missing preset group")?;
        let kind: String = sqlx::query_scalar("SELECT kind FROM organization_nodes WHERE id=?")
            .bind(group)
            .fetch_one(&mut **tx)
            .await?;
        ensure!(
            kind == "group" && node != group,
            "invalid preset membership"
        );
        let member = OrganizationMembership {
            id: format!("{node}/{group}"),
            node_id: node.clone(),
            group_id: group.clone(),
        };
        sqlx::query(
            "INSERT INTO organization_memberships(id,node_id,group_id,data) VALUES (?,?,?,?)",
        )
        .bind(&member.id)
        .bind(&member.node_id)
        .bind(&member.group_id)
        .bind(serde_json::to_string(&OrganizationRecord::Membership(
            member.clone(),
        ))?)
        .execute(&mut **tx)
        .await?;
    }
    preset.revision = preset
        .revision
        .checked_add(1)
        .context("preset revision exhausted")?;
    preset.last_clone = Some(OrganizationPresetClone {
        root_id,
        node_ids,
        role_ids,
    });
    write(tx, &preset).await?;
    Ok(*preset)
}

async fn insert_node(tx: &mut Transaction<'_, Sqlite>, node: OrganizationNode) -> Result<()> {
    let kind = match node.kind {
        OrganizationNodeKind::Agent => "agent",
        OrganizationNodeKind::Group => "group",
    };
    sqlx::query("INSERT INTO organization_nodes(id,parent_id,kind,role_version_id,revision,data) VALUES (?,?,?,?,1,?)")
        .bind(&node.id).bind(&node.parent_id).bind(kind).bind(&node.role_version_id)
        .bind(serde_json::to_string(&OrganizationRecord::Node(node.clone()))?).execute(&mut **tx).await?;
    Ok(())
}
