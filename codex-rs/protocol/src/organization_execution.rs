//! Owner authorization and host-observed task execution records.
//! Role instructions and model assertions cannot change these ceilings.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationExecutionScope {
    /// Files copied from the linked project into an isolated attempt workspace.
    pub input_files: Vec<String>,
    /// Input files mounted read-only, including owner-selected test oracles.
    pub immutable_files: Vec<String>,
    /// Files whose hashes must accompany the result presented for review.
    pub artifacts: Vec<String>,
    /// Owner-supplied argv, executed by the host in the same restricted workspace.
    pub checks: Vec<Vec<String>>,
    pub model: String,
    /// Reported aggregate token ceiling; one provider request may be in flight.
    #[ts(type = "number")]
    pub max_tokens: i64,
    pub max_seconds: u32,
    pub max_model_requests: u32,
    pub max_attempts: u32,
    pub require_tool_approval: bool,
    pub delegates: Vec<String>,
    pub max_children: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationTaskState {
    Proposed,
    Queued,
    Running,
    WaitingApproval,
    WaitingDependency,
    Paused,
    NeedsReconciliation,
    NeedsReview,
    Failed,
    Cancelled,
    Accepted,
}

impl OrganizationTaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingApproval => "waitingApproval",
            Self::WaitingDependency => "waitingDependency",
            Self::Paused => "paused",
            Self::NeedsReconciliation => "needsReconciliation",
            Self::NeedsReview => "needsReview",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Accepted => "accepted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskAuthorization {
    pub id: String,
    pub task_id: String,
    pub scope: OrganizationExecutionScope,
    #[ts(type = "number")]
    pub node_revision: i64,
    #[ts(type = "number")]
    pub project_revision: i64,
    #[ts(type = "number")]
    pub expires_at: i64,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskAttempt {
    pub id: String,
    pub task_id: String,
    pub authorization_id: String,
    #[ts(type = "number")]
    pub generation: i64,
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub workspace: Option<String>,
    pub state: OrganizationTaskState,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number")]
    pub lease_until: i64,
    #[ts(type = "number")]
    pub reported_tokens: i64,
    pub model_requests: u32,
    pub reported_model_requests: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationApprovalState {
    Pending,
    Approved,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationActionApproval {
    pub id: String,
    pub attempt_id: String,
    pub action_id: String,
    pub action_hash: String,
    pub tool: String,
    /// Exact arguments are private owner-review data, not audit/log payloads.
    pub arguments: String,
    pub state: OrganizationApprovalState,
    #[ts(type = "number")]
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationArtifactEvidence {
    pub path: String,
    pub sha256: String,
    #[ts(type = "number")]
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationCheckEvidence {
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub output: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskResult {
    pub attempt_id: String,
    pub artifacts: Vec<OrganizationArtifactEvidence>,
    pub checks: Vec<OrganizationCheckEvidence>,
    pub digest: String,
    pub accepted: bool,
}
