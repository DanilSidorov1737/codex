//! Bounded owner-only projections of canonical records, never model summaries.
use crate::organization_execution::OrganizationProviderEvidence;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationGlobalUsage {
    #[ts(type = "number")]
    pub reported_tokens: i64,
    #[ts(type = "number")]
    pub model_requests: i64,
    #[ts(type = "number")]
    pub active: i64,
    #[ts(type = "number")]
    pub unresolved_requests: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationReportTotals {
    #[ts(type = "Record<string, number>")]
    pub task_states: BTreeMap<String, i64>,
    #[ts(type = "Record<string, number>")]
    pub action_states: BTreeMap<String, i64>,
    #[ts(type = "Record<string, number>")]
    pub schedule_states: BTreeMap<String, i64>,
    #[ts(type = "number")]
    pub pending_approvals: i64,
    #[ts(type = "number")]
    pub changes_requested: i64,
    #[ts(type = "number")]
    pub reported_tokens: i64,
    #[ts(type = "number")]
    pub model_requests: i64,
    #[ts(type = "number")]
    pub reported_model_requests: i64,
    #[ts(type = "number")]
    pub checks_passed: i64,
    #[ts(type = "number")]
    pub checks_failed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationBudgetReport {
    pub grant_id: String,
    pub root_id: String,
    pub model: String,
    #[ts(type = "number")]
    pub max_tokens: i64,
    #[ts(type = "number")]
    pub used_tokens: i64,
    #[ts(type = "number")]
    pub max_model_requests: i64,
    #[ts(type = "number")]
    pub model_requests: i64,
    #[ts(type = "number")]
    pub task_attempts: i64,
    pub task_max_attempts: u32,
    #[ts(type = "number")]
    pub expires_at: i64,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskReport {
    pub task_id: String,
    pub project_id: String,
    pub owner_id: String,
    pub workflow_run_id: Option<String>,
    pub title: String,
    pub state: String,
    #[ts(type = "number")]
    pub revision: i64,
    pub attempt_id: Option<String>,
    pub result_digest: Option<String>,
    pub checkpoint_id: Option<String>,
    pub blocker: Option<String>,
    #[ts(type = "number | null")]
    pub changes_requested_sequence: Option<i64>,
    #[ts(type = "number")]
    pub pending_approvals: i64,
    #[ts(type = "number")]
    pub checks_passed: i64,
    #[ts(type = "number")]
    pub checks_failed: i64,
    #[ts(type = "number")]
    pub reported_tokens: i64,
    #[ts(type = "number")]
    pub model_requests: i64,
    #[ts(type = "number")]
    pub reported_model_requests: i64,
    pub budget: Option<OrganizationBudgetReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationUsageReport {
    pub task_id: String,
    pub workflow_run_id: Option<String>,
    /// Captured at claim time. Historical records are explicitly unknown.
    pub owner_id: Option<String>,
    pub provider: Option<OrganizationProviderEvidence>,
    pub model: String,
    #[ts(type = "number")]
    pub reported_tokens: i64,
    #[ts(type = "number")]
    pub model_requests: i64,
    #[ts(type = "number")]
    pub reported_model_requests: i64,
    #[ts(type = "number")]
    pub attempts: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationUpcomingRun {
    pub schedule_id: String,
    pub project_id: String,
    pub label: String,
    #[ts(type = "number")]
    pub due_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationReport {
    /// Canonical audit revision; paginated readers must not combine revisions.
    #[ts(type = "number")]
    pub sequence: i64,
    #[ts(type = "number")]
    pub generated_at: i64,
    pub project_id: Option<String>,
    /// Complete totals for the selected project/store, independent of pagination.
    pub totals: OrganizationReportTotals,
    /// Whole-store root ledgers, independent of the selected project/task page.
    #[serde(default)]
    pub global_usage: Option<OrganizationGlobalUsage>,
    pub tasks: Vec<OrganizationTaskReport>,
    /// At most 2,000 grouped observations for this task page.
    pub usage: Vec<OrganizationUsageReport>,
    pub usage_truncated: bool,
    pub upcoming_runs: Vec<OrganizationUpcomingRun>,
    pub next_cursor: Option<String>,
}
