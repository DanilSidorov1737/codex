//! Owner-only configured capabilities and canonical observations. No readiness probes.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationProviderObservation {
    #[ts(type = "number")]
    pub model_requests: i64,
    #[ts(type = "number")]
    pub reported_model_requests: i64,
    #[ts(type = "number")]
    pub checked_results: i64,
    /// Task outcomes, not proof of a provider compatibility failure.
    #[ts(type = "number")]
    pub failed_attempts: i64,
    pub models: Vec<String>,
    pub latest_attempt_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationProviderCapability {
    pub id: String,
    pub selected: bool,
    pub origin: Option<String>,
    pub credential_requirement: String,
    pub qualification: String,
    pub observation: OrganizationProviderObservation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationToolCapability {
    pub name: String,
    pub permissions: String,
    #[ts(type = "number")]
    pub completed: i64,
    #[ts(type = "number")]
    pub failed: i64,
    #[ts(type = "number")]
    pub uncertain: i64,
    pub latest_attempt_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMcpCapability {
    pub name: String,
    pub enabled: bool,
    pub available_to_tasks: bool,
    pub qualification: String,
    pub permissions: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationInventory {
    pub providers: Vec<OrganizationProviderCapability>,
    pub tools: Vec<OrganizationToolCapability>,
    pub mcp_servers: Vec<OrganizationMcpCapability>,
    pub limitations: String,
}
