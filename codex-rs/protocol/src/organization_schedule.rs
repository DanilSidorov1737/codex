//! Finite owner-authorized interval schedules. Every occurrence gets a fresh grant.

use super::organization::OrganizationTask;
use super::organization_execution::OrganizationExecutionScope;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationScheduleConfig {
    pub label: String,
    pub interval_seconds: u32,
    #[ts(type = "number")]
    pub first_run_at: i64,
    #[ts(type = "number")]
    pub ends_at: i64,
    pub max_runs: u32,
    #[ts(type = "number")]
    pub max_tokens: i64,
    pub max_model_requests: u32,
}

impl OrganizationScheduleConfig {
    pub fn validate(&self, now: i64) -> Result<(), &'static str> {
        if self.label.trim().is_empty()
            || self.label.len() > 256
            || self.label.chars().any(char::is_control)
        {
            return Err("schedule label must be 1–256 bytes without controls");
        }
        if now < 0
            || self.first_run_at < now
            || self.first_run_at >= self.ends_at
            || self.ends_at > now.saturating_add(31 * 86400)
        {
            return Err("schedule needs a future first run and end within 31 days");
        }
        if !(10..=31 * 86400).contains(&self.interval_seconds)
            || !(1..=1000).contains(&self.max_runs)
            || !(1..=10_000_000).contains(&self.max_tokens)
            || !(1..=100_000).contains(&self.max_model_requests)
        {
            return Err("schedule interval, run or aggregate limits are out of range");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationScheduleState {
    Active,
    Paused,
    Cancelled,
    Completed,
    Blocked,
}

impl OrganizationScheduleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationSchedule {
    pub id: String,
    pub template: OrganizationTask,
    pub scope: OrganizationExecutionScope,
    pub config: OrganizationScheduleConfig,
    #[ts(type = "number")]
    pub node_revision: i64,
    #[ts(type = "number")]
    pub project_revision: i64,
    pub policy_version: u32,
    pub state: OrganizationScheduleState,
    #[ts(type = "number")]
    pub revision: i64,
    #[ts(type = "number | null")]
    pub next_run_at: Option<i64>,
    pub issued_runs: u32,
    pub skipped_runs: u32,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationScheduleOccurrence {
    pub task_id: String,
    pub authorization_id: String,
    #[ts(type = "number")]
    pub due_at: i64,
    pub task_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationScheduleView {
    pub schedule: OrganizationSchedule,
    #[ts(type = "number")]
    pub used_tokens: i64,
    pub model_requests: u32,
    /// Newest 16 issued occurrences; older task evidence remains in the task store.
    pub occurrences: Vec<OrganizationScheduleOccurrence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum OrganizationScheduleControl {
    Pause,
    Resume,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum OrganizationScheduleMutation {
    #[serde(rename_all = "camelCase")]
    Create {
        /// Owner-selected UUID prevents an unknown RPC outcome creating a second series.
        id: String,
        task_id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        scope: Box<OrganizationExecutionScope>,
        config: OrganizationScheduleConfig,
    },
    #[serde(rename_all = "camelCase")]
    Control {
        id: String,
        #[ts(type = "number")]
        expected_revision: i64,
        action: OrganizationScheduleControl,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schedule_wire_contract_rejects_unbounded_and_spoofed_inputs() {
        let control = json!({"type":"control","id":"series","expectedRevision":2,"action":"pause"});
        let mutation: OrganizationScheduleMutation =
            serde_json::from_value(control.clone()).unwrap();
        assert_eq!(serde_json::to_value(mutation).unwrap(), control);
        for field in ["authorId", "rootAuthorizationId", "workerId", "usedTokens"] {
            let mut invalid = control.clone();
            invalid[field] = json!("spoofed");
            assert!(serde_json::from_value::<OrganizationScheduleMutation>(invalid).is_err());
        }
        let mut config = OrganizationScheduleConfig {
            label: "Routine".into(),
            interval_seconds: 60,
            first_run_at: 1001,
            ends_at: 1000 + 31 * 86400,
            max_runs: 1000,
            max_tokens: 10_000_000,
            max_model_requests: 100_000,
        };
        assert!(config.validate(1000).is_ok());
        config.interval_seconds = 9;
        assert!(config.validate(1000).is_err());
        config.interval_seconds = 10;
        assert!(config.validate(1000).is_ok());
        config.first_run_at = 999;
        assert!(config.validate(1000).is_err());
        config.first_run_at = config.ends_at;
        assert!(config.validate(1000).is_err());
        config.first_run_at = 1001;
        config.ends_at += 1;
        assert!(config.validate(1000).is_err());
        config.ends_at -= 1;
        config.label = "é".repeat(129);
        assert!(config.validate(1000).is_err());
    }
}
