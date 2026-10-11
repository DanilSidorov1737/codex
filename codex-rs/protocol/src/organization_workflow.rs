//! Owner-controlled plan templates; saved content never grants execution authority.

use crate::organization::OrganizationTask;
use crate::organization_coordination::OrganizationPlanItem;
use crate::organization_execution::OrganizationExecutionScope;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationWorkflowKind {
    Templates,
    Runs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationWorkflowTemplate {
    pub id: String,
    pub project_id: String,
    pub label: String,
    #[ts(type = "number")]
    pub revision: i64,
    pub items: Vec<OrganizationPlanItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationWorkflowScope {
    pub key: String,
    pub scope: OrganizationExecutionScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationWorkflowPreviewRequest {
    pub id: String,
    pub template_id: String,
    #[ts(type = "number")]
    pub expected_revision: i64,
    pub scopes: Vec<OrganizationWorkflowScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationWorkflowPreview {
    pub id: String,
    pub template: OrganizationWorkflowTemplate,
    #[ts(type = "number")]
    pub project_revision: i64,
    #[ts(type = "Record<string, number>")]
    pub node_revisions: BTreeMap<String, i64>,
    pub policy_version: u32,
    pub scopes: Vec<OrganizationWorkflowScope>,
    #[ts(type = "number")]
    pub max_tokens: i64,
    pub max_model_requests: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationWorkflowRun {
    pub id: String,
    pub preview: OrganizationWorkflowPreview,
    pub task_ids: BTreeMap<String, String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OrganizationWorkflowView {
    Template {
        template: OrganizationWorkflowTemplate,
    },
    Run {
        run: OrganizationWorkflowRun,
        tasks: Vec<OrganizationTask>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum OrganizationWorkflowMutation {
    #[serde(rename_all = "camelCase")]
    Put {
        id: String,
        #[ts(type = "number | null")]
        expected_revision: Option<i64>,
        project_id: String,
        label: String,
        items: Vec<OrganizationPlanItem>,
    },
    Run {
        preview: Box<OrganizationWorkflowPreview>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_cannot_smuggle_scope_or_run_authority() {
        let mut value = serde_json::json!({"type":"put", "id":"id", "projectId":"p",
            "label":"Coding", "items":[], "expectedRevision":null});
        assert!(serde_json::from_value::<OrganizationWorkflowMutation>(value.clone()).is_ok());
        value["scope"] = serde_json::json!({"requireToolApproval":false});
        assert!(serde_json::from_value::<OrganizationWorkflowMutation>(value).is_err());
    }
}
