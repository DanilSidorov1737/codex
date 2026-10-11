//! Durable organization definitions. Identities are independent of runtime threads;
//! role names, parent relationships and memberships never grant authority.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationNodeKind {
    Agent,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: OrganizationNodeKind,
    pub label: String,
    pub role_version_id: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
    #[ts(type = "number")]
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationRoleVersion {
    pub id: String,
    pub role_id: String,
    pub name: String,
    pub instructions: String,
    #[ts(type = "number")]
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationGoal {
    pub id: String,
    pub parent_id: Option<String>,
    pub objective: String,
    pub criteria: Vec<String>,
    #[ts(type = "number")]
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationProjectPlan {
    pub id: String,
    pub name: String,
    pub roots: Vec<String>,
    pub goal_id: Option<String>,
    pub owner_id: Option<String>,
    pub criteria: Vec<String>,
    #[ts(type = "number")]
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTask {
    pub id: String,
    pub project_id: String,
    pub goal_id: Option<String>,
    pub owner_id: String,
    pub instructions: String,
    pub criteria: Vec<String>,
    pub prerequisites: Vec<String>,
    pub state: String,
    #[ts(type = "number")]
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMembership {
    pub id: String,
    pub node_id: String,
    pub group_id: String,
}

/// Definition-only snapshot. No task, grant, memory or runtime identity is copied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPreset {
    pub id: String,
    pub label: String,
    #[ts(type = "number")]
    pub revision: i64,
    pub nodes: Vec<OrganizationNode>,
    pub roles: Vec<OrganizationRoleVersion>,
    pub memberships: Vec<OrganizationMembership>,
    pub last_clone: Option<OrganizationPresetClone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPresetClone {
    pub root_id: String,
    pub node_ids: std::collections::BTreeMap<String, String>,
    pub role_ids: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationRecordKind {
    Nodes,
    Roles,
    Goals,
    Projects,
    Tasks,
    Memberships,
    Presets,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum OrganizationRecord {
    Node(OrganizationNode),
    Role(OrganizationRoleVersion),
    Goal(OrganizationGoal),
    Project(OrganizationProjectPlan),
    Task(OrganizationTask),
    Membership(OrganizationMembership),
    Preset(Box<OrganizationPreset>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum OrganizationMutation {
    #[serde(rename_all = "camelCase")]
    SavePreset {
        root_id: String,
        #[ts(type = "number")]
        expected_root_revision: i64,
        label: String,
    },
    #[serde(rename_all = "camelCase")]
    ClonePreset {
        id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        label: String,
        parent_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    PromoteNote {
        source_id: String,
        group_id: Option<String>,
        body: String,
    },
    #[serde(rename_all = "camelCase")]
    PutNode {
        id: Option<String>,
        #[ts(type = "number | null")]
        expected_revision: Option<i64>,
        parent_id: Option<String>,
        kind: OrganizationNodeKind,
        label: String,
        role_version_id: Option<String>,
        model: Option<String>,
        enabled: bool,
    },
    #[serde(rename_all = "camelCase")]
    CreateRoleVersion {
        role_id: Option<String>,
        #[ts(type = "number | null")]
        expected_version: Option<i64>,
        name: String,
        instructions: String,
    },
    #[serde(rename_all = "camelCase")]
    PutGoal {
        id: Option<String>,
        #[ts(type = "number | null")]
        expected_revision: Option<i64>,
        parent_id: Option<String>,
        objective: String,
        criteria: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    LinkProject {
        project_id: String,
        #[ts(type = "number | null")]
        expected_revision: Option<i64>,
        goal_id: Option<String>,
        owner_id: Option<String>,
        criteria: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    PutTask {
        id: Option<String>,
        #[ts(type = "number | null")]
        expected_revision: Option<i64>,
        project_id: String,
        goal_id: Option<String>,
        owner_id: String,
        instructions: String,
        criteria: Vec<String>,
        prerequisites: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    SetMembership {
        node_id: String,
        group_id: String,
        present: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    #[test]
    fn mutation_schema_matches_actual_json_field_names() {
        let schema = serde_json::to_value(schemars::schema_for!(OrganizationMutation)).unwrap();
        let examples = [
            json!({"type":"savePreset","rootId":"root","expectedRootRevision":1,"label":"Team"}),
            json!({"type":"clonePreset","id":"preset","expectedRevision":1,"label":"Team copy","parentId":null}),
            json!({"type":"putNode","kind":"agent","label":"One","enabled":true}),
            json!({"type":"createRoleVersion","name":"Researcher","instructions":"Read sources"}),
            json!({"type":"putGoal","objective":"Report","criteria":["Cited"]}),
            json!({"type":"linkProject","projectId":"project","criteria":["Accepted"]}),
            json!({"type":"putTask","projectId":"project","ownerId":"agent","instructions":"Report","criteria":["Cited"],"prerequisites":[]}),
            json!({"type":"setMembership","nodeId":"agent","groupId":"group","present":true}),
        ];
        for example in examples {
            let mutation: OrganizationMutation = serde_json::from_value(example).unwrap();
            let serialized = serde_json::to_value(mutation).unwrap();
            let variant = schema["oneOf"]
                .as_array()
                .unwrap()
                .iter()
                .find(|variant| variant["properties"]["type"]["enum"][0] == serialized["type"])
                .unwrap();
            assert_eq!(
                variant["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<BTreeSet<_>>(),
                serialized
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<BTreeSet<_>>()
            );
        }
    }
}
