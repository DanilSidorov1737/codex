//! Owner authorization and host-observed task execution records.
//! Role instructions and model assertions cannot change these ceilings.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Explicitly version the host's admitted tools and coordination behavior.
/// Pre-coordination grants deserialize as version 1 and require reauthorization.
pub const ORGANIZATION_POLICY_VERSION: u32 = 2;

fn legacy_policy_version() -> u32 {
    1
}

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
    /// Explicit owner permission to accept internal children after all declared
    /// host checks pass. Final root acceptance always remains an owner decision.
    #[serde(default)]
    pub auto_accept_children: bool,
    /// Optional one-shot start time, UTC seconds since the Unix epoch.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub not_before: Option<i64>,
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
    #[serde(default = "legacy_policy_version")]
    pub policy_version: u32,
    #[serde(default)]
    pub parent_attempt_id: Option<String>,
    #[serde(default)]
    pub root_authorization_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskAttempt {
    pub id: String,
    pub task_id: String,
    pub authorization_id: String,
    #[serde(default)]
    pub role_version_id: Option<String>,
    #[ts(type = "number")]
    pub generation: i64,
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub workspace: Option<String>,
    #[serde(default)]
    pub artifact_bases: Vec<OrganizationArtifactBase>,
    #[serde(default)]
    pub runtime_active: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationActionState {
    Pending,
    Dispatched,
    Completed,
    Failed,
    Uncertain,
    Rejected,
}

impl OrganizationActionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Dispatched => "dispatched",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Uncertain => "uncertain",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationActionIntent {
    pub id: String,
    pub attempt_id: String,
    pub call_id: String,
    pub action_hash: String,
    pub tool: String,
    pub arguments: String,
    pub approval_id: Option<String>,
    pub state: OrganizationActionState,
    pub receipt: Option<String>,
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
pub struct OrganizationArtifactBase {
    pub path: String,
    /// None means the file did not exist when the isolated attempt was prepared.
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationRuntimeBinding {
    pub workspace: String,
    pub turn_id: Option<String>,
    pub artifact_bases: Vec<OrganizationArtifactBase>,
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
    pub artifact_bases: Vec<OrganizationArtifactBase>,
    pub checks: Vec<OrganizationCheckEvidence>,
    pub digest: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationExecutionView {
    pub task: super::organization::OrganizationTask,
    pub authorization: Option<OrganizationTaskAuthorization>,
    #[ts(type = "number")]
    pub authorization_used_tokens: i64,
    pub authorization_model_requests: u32,
    /// Recent attempts, newest first; each authorization permits at most three.
    pub attempts: Vec<OrganizationTaskAttempt>,
    pub actions: Vec<OrganizationActionIntent>,
    pub approvals: Vec<OrganizationActionApproval>,
    pub result: Option<OrganizationTaskResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum OrganizationExecutionMutation {
    #[serde(rename_all = "camelCase")]
    Authorize {
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        scope: OrganizationExecutionScope,
    },
    #[serde(rename_all = "camelCase")]
    Stop {
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        cancel: bool,
    },
    #[serde(rename_all = "camelCase")]
    DecideApproval {
        task_id: String,
        approval_id: String,
        expected_hash: String,
        approve: bool,
    },
    #[serde(rename_all = "camelCase")]
    Accept {
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        digest: String,
    },
    #[serde(rename_all = "camelCase")]
    RequestChanges {
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        instructions: String,
    },
}

impl OrganizationExecutionMutation {
    pub fn task_id(&self) -> &str {
        match self {
            Self::Authorize { task_id, .. }
            | Self::Stop { task_id, .. }
            | Self::DecideApproval { task_id, .. }
            | Self::Accept { task_id, .. }
            | Self::RequestChanges { task_id, .. } => task_id,
        }
    }
}
