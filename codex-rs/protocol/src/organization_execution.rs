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

impl OrganizationTaskResult {
    /// Validate owner selection and keep the accepted artifact order. Omission
    /// retains the original apply-all contract; an explicit empty set is rejected.
    pub fn application_paths(
        &self,
        requested: Option<&[String]>,
    ) -> Result<Vec<String>, &'static str> {
        if requested.is_some_and(|paths| !(1..=64).contains(&paths.len())) {
            return Err("select 1–64 accepted artifact paths");
        }
        let paths = requested
            .map(<[String]>::to_vec)
            .unwrap_or_else(|| self.artifacts.iter().map(|a| a.path.clone()).collect());
        if !(1..=64).contains(&paths.len()) {
            return Err("select 1–64 accepted artifact paths");
        }
        let unique: std::collections::BTreeSet<_> = paths.iter().collect();
        if unique.len() != paths.len() {
            return Err("duplicate application path");
        }
        if !paths
            .iter()
            .all(|p| self.artifacts.iter().any(|a| &a.path == p))
        {
            return Err("application path is not an accepted artifact");
        }
        Ok(self
            .artifacts
            .iter()
            .filter(|a| unique.contains(&a.path))
            .map(|a| a.path.clone())
            .collect())
    }
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
    pub applications: Vec<OrganizationApplication>,
}

/// Owner-only application evidence. An uncertain operation is never replayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationApplication {
    pub id: String,
    pub digest: String,
    pub paths: Vec<String>,
    pub state: OrganizationApplicationState,
    pub receipt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationApplicationState {
    Pending,
    Completed,
    Uncertain,
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
    #[serde(rename_all = "camelCase")]
    Apply {
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        digest: String,
        /// Omit to apply all accepted artifacts. Explicit selection must be
        /// nonempty and unique; the receipt records exactly these whole files.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        paths: Option<Vec<String>>,
    },
}

#[cfg(test)]
mod application_tests {
    use super::*;

    #[test]
    fn selection_preserves_legacy_apply_and_rejects_invalid_paths() -> anyhow::Result<()> {
        let old = serde_json::json!({"type":"apply","taskId":"task","expectedRevision":7,"digest":"hash"});
        let mutation: OrganizationExecutionMutation = serde_json::from_value(old.clone())?;
        assert!(matches!(
            mutation,
            OrganizationExecutionMutation::Apply { paths: None, .. }
        ));
        assert_eq!(serde_json::to_value(mutation)?, old);
        let result = OrganizationTaskResult {
            attempt_id: "attempt".into(),
            artifacts: ["report.txt", "notes.md"]
                .into_iter()
                .map(|path| OrganizationArtifactEvidence {
                    path: path.into(),
                    sha256: "a".repeat(64),
                    bytes: 1,
                })
                .collect(),
            artifact_bases: vec![],
            checks: vec![],
            digest: "hash".into(),
            accepted: true,
        };
        assert_eq!(
            result.application_paths(None).map_err(anyhow::Error::msg)?,
            ["report.txt", "notes.md"]
        );
        assert_eq!(
            result
                .application_paths(Some(&["notes.md".into()]))
                .map_err(anyhow::Error::msg)?,
            ["notes.md"]
        );
        assert_eq!(
            result
                .application_paths(Some(&["notes.md".into(), "report.txt".into()]))
                .map_err(anyhow::Error::msg)?,
            ["report.txt", "notes.md"]
        );
        for invalid in [
            vec![],
            vec!["report.txt".into(); 2],
            vec!["../report.txt".into()],
            vec!["oracle.py".into()],
            vec!["report.txt".into(); 65],
        ] {
            assert!(result.application_paths(Some(&invalid)).is_err());
        }
        Ok(())
    }
}

impl OrganizationExecutionMutation {
    pub fn task_id(&self) -> &str {
        match self {
            Self::Authorize { task_id, .. }
            | Self::Stop { task_id, .. }
            | Self::DecideApproval { task_id, .. }
            | Self::Accept { task_id, .. }
            | Self::RequestChanges { task_id, .. }
            | Self::Apply { task_id, .. } => task_id,
        }
    }
}
