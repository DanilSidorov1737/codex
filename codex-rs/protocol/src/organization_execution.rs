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
    Blocked,
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
            Self::Blocked => "blocked",
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
    pub owner_id: Option<String>,
    #[serde(default)]
    pub provider: Option<OrganizationProviderEvidence>,
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
    /// Actual model/tool activity; lease renewals are not progress.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub last_progress_at: Option<i64>,
    #[serde(default)]
    pub stall_seconds: Option<u32>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationProviderEvidence {
    pub id: String,
    /// Origin only; never credentials, request headers, path or query values.
    pub origin: Option<String>,
    /// Hash of the endpoint without URL credentials/query/fragment and wire API.
    pub endpoint_sha256: String,
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
    /// Host-captured hashes of the protected test inputs. Legacy evidence has
    /// no hashes and cannot qualify automatic comparison/adoption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immutable_inputs: Option<Vec<OrganizationArtifactEvidence>>,
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
    pub fn application_hunks(
        &self,
        paths: Option<&[String]>,
        requested: Option<&[OrganizationHunkSelection]>,
    ) -> Result<Vec<OrganizationHunkSelection>, &'static str> {
        let Some(requested) = requested else {
            return Ok(Vec::new());
        };
        let paths = self.application_paths(paths)?;
        if requested.is_empty() || requested.len() != paths.len() {
            return Err("hunk selection must cover exactly the selected paths");
        }
        let mut ordered = Vec::new();
        for path in paths {
            let selected: Vec<_> = requested.iter().filter(|s| s.path == path).collect();
            if selected.len() != 1 {
                return Err("duplicate or missing hunk path");
            }
            let selection = selected[0];
            if !(1..=256).contains(&selection.indices.len())
                || selection.indices.windows(2).any(|w| w[0] >= w[1])
                || [&selection.patch_sha256, &selection.output_sha256]
                    .iter()
                    .any(|s| {
                        s.len() != 64
                            || !s
                                .bytes()
                                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                    })
            {
                return Err("invalid hunk indices or hashes");
            }
            ordered.push(selection.clone());
        }
        Ok(ordered)
    }
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
    #[serde(default)]
    pub checkpoints: Vec<OrganizationTaskCheckpoint>,
    #[serde(default)]
    pub handoffs: Vec<OrganizationTaskHandoff>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskCheckpoint {
    pub id: String,
    pub task_id: String,
    pub project_id: String,
    pub owner_id: String,
    pub attempt_id: String,
    pub authorization_id: String,
    #[ts(type = "number")]
    pub created_at: i64,
    /// Progress text is reported; these canonical action IDs supply its evidence.
    pub completed_action_ids: Vec<String>,
    pub content: super::organization_coordination::OrganizationCheckpointContent,
    /// Host-hashed declared inputs and existing outputs; no agent-selected paths.
    pub files: Vec<OrganizationArtifactEvidence>,
    pub artifact_bases: Vec<OrganizationArtifactBase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTaskHandoff {
    pub id: String,
    pub task_id: String,
    pub checkpoint_id: String,
    pub authorization_id: String,
    pub from_owner_id: String,
    pub to_owner_id: String,
    #[ts(type = "number")]
    pub from_node_revision: i64,
    #[ts(type = "number")]
    pub to_node_revision: i64,
    pub instructions: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Owner-only application evidence. An uncertain operation is never replayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationApplication {
    pub id: String,
    pub digest: String,
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hunks: Option<Vec<OrganizationHunkSelection>>,
    pub state: OrganizationApplicationState,
    pub receipt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationHunkSelection {
    pub path: String,
    pub patch_sha256: String,
    /// Zero-based indices in the canonical diffy's complete patch, ascending.
    pub indices: Vec<u32>,
    pub output_sha256: String,
}

/// Shared canonical UTF-8 selection. All ranges refer to the unchanged base;
/// no fuzzy application, offset guessing or client-supplied patch is accepted.
pub fn selected_hunks(
    base: &str,
    artifact: &str,
    indices: &[u32],
) -> Result<(String, String), &'static str> {
    if base.len() > 1024 * 1024
        || artifact.len() > 1024 * 1024
        || base.contains('\0')
        || artifact.contains('\0')
    {
        return Err("hunk Apply supports nonbinary UTF-8 files up to 1 MiB");
    }
    let patch = diffy::create_patch(base, artifact);
    if indices.is_empty()
        || indices.len() > 256
        || indices.windows(2).any(|w| w[0] >= w[1])
        || indices.iter().any(|i| *i as usize >= patch.hunks().len())
    {
        return Err("select 1–256 existing distinct hunks in ascending order");
    }
    let lines: Vec<_> = base.split_inclusive('\n').collect();
    let mut cursor = 0;
    let mut output = String::new();
    for index in indices {
        let hunk = &patch.hunks()[*index as usize];
        let range = hunk.old_range();
        let start = if range.is_empty() {
            range.start()
        } else {
            range.start().saturating_sub(1)
        };
        if start < cursor || start + range.len() > lines.len() {
            return Err("invalid canonical hunk range");
        }
        for line in &lines[cursor..start] {
            output.push_str(line)
        }
        for line in hunk.lines() {
            match line {
                diffy::Line::Context(line) | diffy::Line::Insert(line) => output.push_str(line),
                diffy::Line::Delete(_) => {}
            }
        }
        cursor = start + range.len();
    }
    for line in &lines[cursor..] {
        output.push_str(line)
    }
    Ok((patch.to_string(), output))
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
    ApplyReplan {
        task_id: String,
        id: String,
        #[ts(type = "number")]
        expected_revision: i64,
    },
    #[serde(rename_all = "camelCase")]
    Handoff {
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        checkpoint_id: String,
        owner_id: String,
        #[ts(type = "number")]
        expected_owner_revision: i64,
        instructions: String,
    },
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        hunks: Option<Vec<OrganizationHunkSelection>>,
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
            Self::Handoff { task_id, .. } | Self::ApplyReplan { task_id, .. } => task_id,
        }
    }
}

#[cfg(test)]
mod hunk_tests {
    use super::*;
    #[test]
    fn exact_hunks_preserve_unselected_context_and_newline_edge_cases() {
        let middle = (0..30)
            .map(|i| format!("context {i}\n"))
            .collect::<String>();
        let base = format!("wrong first\n{middle}wrong last");
        let artifact = format!("correct first\n{middle}correct last\n");
        let (_, first) = selected_hunks(&base, &artifact, &[0]).unwrap();
        assert_eq!(first, format!("correct first\n{middle}wrong last"));
        assert_eq!(
            selected_hunks(&base, &artifact, &[0, 1]).unwrap().1,
            artifact
        );
        assert_eq!(selected_hunks("", "new\n", &[0]).unwrap().1, "new\n");
        assert_eq!(selected_hunks("gone\n", "", &[0]).unwrap().1, "");
        for indices in [vec![], vec![1, 0], vec![0, 0], vec![2]] {
            assert!(selected_hunks(&base, &artifact, &indices).is_err());
        }
        assert!(selected_hunks("binary\0", "text", &[0]).is_err());
    }
}
