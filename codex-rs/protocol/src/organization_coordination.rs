//! Narrow agent operations: content and coordination, never owner administration.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationPlanItem {
    pub key: String,
    pub owner_id: String,
    pub instructions: String,
    pub criteria: Vec<String>,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationNoteScope {
    Project,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationNote {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub scope: OrganizationNoteScope,
    pub body: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationNoteSearchResult {
    pub notes: Vec<OrganizationNote>,
    pub matched_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMessage {
    pub id: String,
    pub root_authorization_id: String,
    pub project_id: String,
    pub sender_id: String,
    pub sender_task_id: String,
    pub recipient_task_id: String,
    pub body: String,
    #[ts(type = "number")]
    pub created_at: i64,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum OrganizationCoordinationCall {
    #[serde(rename_all = "camelCase")]
    Wait {
        task_ids: Vec<String>,
    },
    ProposePlan {
        items: Vec<OrganizationPlanItem>,
    },
    #[serde(rename_all = "camelCase")]
    Delegate {
        task_id: String,
    },
    #[serde(rename_all = "camelCase")]
    WriteNote {
        scope: OrganizationNoteScope,
        body: String,
    },
    ReadNotes {
        scope: OrganizationNoteScope,
    },
    SearchNotes {
        scope: OrganizationNoteScope,
        query: String,
    },
    #[serde(rename_all = "camelCase")]
    Send {
        task_id: String,
        body: String,
    },
    Inbox {},
    #[serde(rename_all = "camelCase")]
    Acknowledge {
        message_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_search_cannot_select_identity_and_legacy_read_is_unchanged() {
        let search =
            serde_json::json!({"type":"searchNotes","scope":"agent","query":"Überprüfung"});
        assert!(serde_json::from_value::<OrganizationCoordinationCall>(search.clone()).is_ok());
        for field in ["projectId", "agentId", "attemptId", "worker"] {
            let mut spoofed = search.clone();
            spoofed[field] = "other".into();
            assert!(serde_json::from_value::<OrganizationCoordinationCall>(spoofed).is_err());
        }
        assert_eq!(
            serde_json::to_value(OrganizationCoordinationCall::ReadNotes {
                scope: OrganizationNoteScope::Project,
            })
            .unwrap(),
            serde_json::json!({"type":"readNotes","scope":"project"})
        );
    }
}
