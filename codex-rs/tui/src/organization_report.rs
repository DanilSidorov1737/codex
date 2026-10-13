//! On-demand local projections and the Review inbox. No model calls or authority.
use super::*;

#[derive(Clone)]
pub(super) enum Focus {
    All,
    Project(String),
    Agent(String),
    Workflow(String),
}

/// Owner projection of existing canonical evidence; never creates agent authority.
pub(super) fn owner_overview(snapshot: &Snapshot, executive: Option<&str>) -> String {
    let mut text = String::from("OWNER OVERVIEW\n");
    match executive {
        None => text.push_str("Executive contact: direct owner control. Choose any enabled agent through Commands.\n"),
        Some(id) => match snapshot.records.iter().find_map(|r| match r {
            OrganizationRecord::Node(n) if n.id == id && n.kind == OrganizationNodeKind::Agent => Some(n),
            _ => None,
        }) {
            Some(node) => text.push_str(&format!("Executive contact: {} · {} · {}\nSelect this agent in Operations to open its existing conversation.\n", node.label, id, if node.enabled { "enabled" } else { "disabled; choose another contact" })),
            None => text.push_str(&format!("Executive contact {id}: unavailable in loaded records; refresh.\n")),
        },
    }
    text.push_str("\nMISSION\n");
    if let Some(control) = snapshot.records.iter().find_map(|r| match r {
        OrganizationRecord::Control(c)
            if matches!(c.value, OrganizationControlValue::Mission { .. }) =>
        {
            Some(c)
        }
        _ => None,
    }) && let OrganizationControlValue::Mission {
        purpose,
        priorities,
        constraints,
    } = &control.value
    {
        text.push_str(&format!(
            "Version {} · {}\nPriorities: {}\nConstraints: {}\n",
            control.revision,
            purpose,
            priorities.join("; "),
            constraints.join("; ")
        ));
    } else {
        text.push_str("Mission not configured. Commands → edit owner mission.\n");
    }
    text.push_str("\nOUTCOMES / OWNER DECISIONS\n");
    if let Some(report) = &snapshot.report {
        text.push_str(&format!("Captured audit revision {} · UTC {} · scope {}\nCanonical task states: {:?}\nRecorded checks: {} passed / {} failed\n", report.sequence, utc_time(report.generated_at), report.project_id.as_deref().unwrap_or("whole store"), report.totals.task_states, report.totals.checks_passed, report.totals.checks_failed));
        let decisions: Vec<_> = report.tasks.iter().filter_map(inbox_row).collect();
        for row in decisions.iter().take(32) {
            text.push_str(&format!("{} · {}\n", row.id, row.label));
        }
        text.push_str(&format!("{} decisions in loaded task evidence; first32 shown. Open Review for exact decisions.\n", decisions.len()));
        if let Some(usage) = &report.global_usage {
            text.push_str(&format!("Recorded usage: {} tokens / {} admitted requests / {} active / {} unresolved terminal requests.\n", usage.reported_tokens, usage.model_requests, usage.active, usage.unresolved_requests));
        } else {
            text.push_str("Recorded whole-store usage unavailable.\n");
        }
    } else {
        text.push_str("Canonical report unavailable; refresh before reviewing outcomes.\n");
    }
    text.push_str("\nESCALATIONS\n");
    let escalations: Vec<_> = snapshot
        .records
        .iter()
        .filter_map(|r| match r {
            OrganizationRecord::Control(c)
                if matches!(
                    c.value,
                    OrganizationControlValue::Escalation {
                        resolved: false,
                        ..
                    }
                ) =>
            {
                Some(c)
            }
            _ => None,
        })
        .collect();
    for control in escalations.iter().take(32) {
        if let OrganizationControlValue::Escalation {
            task_id,
            task_revision,
            assignee_id,
            reason,
            requested_action,
            ..
        } = &control.value
        {
            let current = snapshot.records.iter().any(|r| matches!(r,OrganizationRecord::Task(t) if t.id==*task_id && t.revision==*task_revision));
            text.push_str(&format!(
                "{} v{} · task {} v{} · responsibility {} · {}\n  {}\n  Requested: {}\n",
                control.id,
                control.revision,
                task_id,
                task_revision,
                assignee_id.as_deref().unwrap_or("owner"),
                if current {
                    "matches loaded task"
                } else {
                    "task changed or not loaded; inspect current evidence"
                },
                reason,
                requested_action
            ));
        }
    }
    text.push_str(&format!(
        "{} unresolved escalations in loaded definitions; first32 shown.\n",
        escalations.len()
    ));
    text.push_str("\nTRACE / QUEUE\n");
    let tasks: Vec<_> = snapshot
        .records
        .iter()
        .filter_map(|r| match r {
            OrganizationRecord::Task(t) => Some(t),
            _ => None,
        })
        .collect();
    for task in tasks
        .iter()
        .filter(|t| t.state != "accepted" && t.state != "cancelled")
        .take(32)
    {
        let unmet: Vec<_> = task
            .prerequisites
            .iter()
            .filter(|id| !tasks.iter().any(|t| &t.id == *id && t.state == "accepted"))
            .collect();
        let evidence = snapshot
            .report
            .as_ref()
            .and_then(|r| r.tasks.iter().find(|t| t.task_id == task.id));
        let grant = evidence.and_then(|t| t.budget.as_ref());
        text.push_str(&format!("Mission → goal {} → project {} → task {} v{} [{}]\n  Prerequisites awaiting accepted or missing evidence: {:?}\n  Grant {} · shared root {} · result {}\n  Actions/checks/review: codex organization execution-read {}\n",task.goal_id.as_deref().unwrap_or("unlinked"),task.project_id,task.id,task.revision,task.state,unmet,grant.map(|b|b.grant_id.as_str()).unwrap_or("unavailable"),grant.map(|b|b.root_id.as_str()).unwrap_or("unavailable"),evidence.and_then(|t|t.result_digest.as_deref()).unwrap_or("unavailable"),task.id));
    }
    text.push_str("Snapshot only; reopen after refresh. Missing evidence stays unavailable. No completion-time forecast inferred.\n");
    if snapshot.limited {
        text.push_str("Definition pages are partial; load more records for broader coverage.\n");
    }
    text
}
impl Focus {
    pub(super) fn project_id(&self) -> Option<&str> {
        if let Self::Project(id) = self {
            Some(id)
        } else {
            None
        }
    }
}
pub(super) fn focus(
    record: Option<&OrganizationRecord>,
    selected: Option<&str>,
    workflows: &[OrganizationWorkflowView],
) -> Focus {
    match record {
        Some(OrganizationRecord::Task(task)) => Focus::Project(task.project_id.clone()),
        Some(OrganizationRecord::Project(project)) => Focus::Project(project.id.clone()),
        Some(OrganizationRecord::Node(node)) => Focus::Agent(node.id.clone()),
        _ => {
            if let Some(id) = selected.and_then(|id| id.strip_prefix("run:")) {
                Focus::Workflow(id.into())
            } else {
                workflows
                    .iter()
                    .find_map(|view| match view {
                        OrganizationWorkflowView::Template { template }
                            if selected == Some(format!("workflow:{}", template.id).as_str()) =>
                        {
                            Some(Focus::Project(template.project_id.clone()))
                        }
                        _ => None,
                    })
                    .unwrap_or(Focus::All)
            }
        }
    }
}
pub(super) async fn load(
    handle: &AppServerRequestHandle,
    project: Option<&str>,
) -> anyhow::Result<OrganizationReport> {
    let mut report: Option<OrganizationReport> = None;
    let mut cursor = None;
    for page in 0..10 {
        let response: OrganizationReportResponse = handle
            .request_typed(ClientRequest::OrganizationReport {
                request_id: request_id(),
                params: OrganizationReportParams {
                    project_id: project.map(str::to_owned),
                    cursor: cursor.clone(),
                    limit: Some(200),
                },
            })
            .await?;
        let next = response.report.next_cursor.clone();
        if let Some(report) = &mut report {
            anyhow::ensure!(
                report.sequence == response.report.sequence,
                "State changed during report pagination; refresh to retry"
            );
            report.tasks.extend(response.report.tasks);
            report.usage.extend(response.report.usage);
            report.usage_truncated |= response.report.usage_truncated;
        } else {
            report = Some(response.report);
        }
        if next.is_none() {
            return report.ok_or_else(|| anyhow::anyhow!("Report unavailable"));
        }
        anyhow::ensure!(
            page < 9 && next != cursor,
            "Report exceeds 2,000 tasks; use paginated organization report CLI"
        );
        cursor = next;
    }
    anyhow::bail!("Report unavailable")
}
pub(super) fn inbox_row(task: &OrganizationTaskReport) -> Option<Row> {
    let mut needs = Vec::new();
    if task.pending_approvals > 0 {
        needs.push(format!("{} approval(s)", task.pending_approvals));
    }
    if task.state == "blocked" {
        needs.push(format!(
            "blocker: {}",
            task.blocker.as_deref().unwrap_or("inspect checkpoint")
        ));
    }
    if task.changes_requested_sequence.is_some() {
        needs.push("requested changes".into());
    }
    if task.state == "needsReview" && task.result_digest.is_some() {
        needs.push("result review".into());
    }
    if matches!(task.state.as_str(), "failed" | "needsReconciliation") {
        needs.push(format!("{}: owner decision", task.state));
    }
    (!needs.is_empty()).then(|| Row {
        id: task.task_id.clone(),
        label: format!("[{}] {} · {}", needs.join(" / "), task.title, task.task_id),
    })
}
pub(super) fn text(
    report: &OrganizationReport,
    focus: &Focus,
    prices: Option<&crate::organization_pricing::Prices>,
) -> String {
    let mut text = format!(
        "CANONICAL LOCAL SUMMARY · audit revision {} · {}\nNo model calls. z selected scope · Z whole store · C capabilities.\n",
        report.sequence,
        utc_time(report.generated_at)
    );
    if matches!(focus, Focus::All | Focus::Project(_)) {
        if let Some(pressure) = &report.resource_pressure {
            text.push_str(&format!("Resource pressure at {}: {} · disk {:?} bytes / minimum {} · available memory {:?} bytes / minimum {} · measured UTC {}\n",pressure.path,if pressure.ready {"admissions ready"}else{"admissions paused"},pressure.available_disk_bytes,pressure.min_disk_bytes,pressure.available_memory_bytes,pressure.min_available_memory_bytes,pressure.measured_at));
        }
        if let Some(u) = &report.global_usage {
            text.push_str(&format!("Whole organization: {} reported tokens · {} admitted requests · {} active · {} unresolved terminal requests\n", u.reported_tokens, u.model_requests, u.active, u.unresolved_requests));
        } else {
            text.push_str("Whole organization usage unavailable; refresh the daemon report.\n");
        }
        text.push_str(&format!("Scope: {}\nTask states: {:?}\nAction states: {:?}\nSchedules: {:?}\n{} pending approvals · {} requested changes\nRecorded checks: {} passed / {} failed\n{} reported tokens · {} admitted requests / {} usage receipts\n",report.project_id.as_deref().unwrap_or("whole store"),report.totals.task_states,report.totals.action_states,report.totals.schedule_states,report.totals.pending_approvals,report.totals.changes_requested,report.totals.checks_passed,report.totals.checks_failed,report.totals.reported_tokens,report.totals.model_requests,report.totals.reported_model_requests));
    }
    text.push_str("\nTASKS / EVIDENCE\n");
    let tasks: Vec<_> = report
        .tasks
        .iter()
        .filter(|task| match focus {
            Focus::Agent(id) => &task.owner_id == id,
            Focus::Workflow(id) => task.workflow_run_id.as_ref() == Some(id),
            _ => true,
        })
        .collect();
    let mut roots = BTreeSet::new();
    for task in tasks {
        text.push_str(&format!("{} · {} · revision {}\n  {} / owner {} / project {} / workflow {}\n  attempt {} · result {} · checkpoint {} · changes receipt {}\n  checks {} passed / {} failed · {} reported tokens · {}/{} requests reported\n  Evidence: codex organization execution-read {}\n",task.task_id,task.title,task.revision,task.state,task.owner_id,task.project_id,task.workflow_run_id.as_deref().unwrap_or("none"),task.attempt_id.as_deref().unwrap_or("none"),task.result_digest.as_deref().unwrap_or("none"),task.checkpoint_id.as_deref().unwrap_or("none"),task.changes_requested_sequence.map(|id|id.to_string()).unwrap_or("none".into()),task.checks_passed,task.checks_failed,task.reported_tokens,task.reported_model_requests,task.model_requests,task.task_id));
        if let Some(blocker) = &task.blocker {
            text.push_str(&format!("  Blocker: {blocker}\n"));
        }
        if let Some(b) = &task.budget {
            text.push_str(&format!(
                "  Grant {} · attempts {}/{} · expires {} · revoked {}\n",
                b.grant_id,
                b.task_attempts,
                b.task_max_attempts,
                utc_time(b.expires_at),
                b.revoked
            ));
            if roots.insert(&b.root_id) {
                text.push_str(&format!("  SHARED ROOT {} / {}: {} tokens and {} requests remaining (recorded usage); descendants share this ledger.\n",b.root_id,b.model,b.max_tokens.saturating_sub(b.used_tokens).max(0),b.max_model_requests.saturating_sub(b.model_requests).max(0)));
            }
        }
    }
    text.push_str("\nATTEMPT USAGE ATTRIBUTION (captured agent/provider/model; no root-ledger double count)\n");
    let usage: Vec<_> = report
        .usage
        .iter()
        .filter(|u| match focus {
            Focus::Agent(id) => u.owner_id.as_ref() == Some(id),
            Focus::Workflow(id) => u.workflow_run_id.as_ref() == Some(id),
            _ => true,
        })
        .collect();
    let mut groups: BTreeMap<String, (i64, i64, i64)> = BTreeMap::new();
    for u in &usage {
        let project = report
            .tasks
            .iter()
            .find(|task| task.task_id == u.task_id)
            .map(|task| task.project_id.as_str())
            .unwrap_or("unknown");
        for key in [
            format!(
                "agent {}",
                u.owner_id.as_deref().unwrap_or("unknown (historical)")
            ),
            format!("project {project}"),
            format!(
                "workflow {}",
                u.workflow_run_id.as_deref().unwrap_or("none")
            ),
        ] {
            let total = groups.entry(key).or_default();
            total.0 += u.reported_tokens;
            total.1 += u.model_requests;
            total.2 += u.reported_model_requests;
        }
    }
    text.push_str("Recorded totals by dimension (do not add dimensions):\n");
    for (key, (tokens, requests, reported)) in groups {
        text.push_str(&format!(
            "{key}: {tokens} reported tokens · {reported}/{requests} requests reported\n"
        ));
    }
    for u in usage {
        let estimate = u.provider.as_ref().and_then(|provider| {
            prices.and_then(|prices| {
                prices.reported_estimate(&provider.id, &u.model, u.reported_tokens)
            })
        });
        text.push_str(&format!(
            "{}\n",
            estimate.unwrap_or(
                "Cost estimate unavailable: no matching owner quote or historical provider unknown"
                    .into()
            )
        ));
        text.push_str(&format!("task {} · agent {} · workflow {} · {}/{} · {} reported tokens · {}/{} requests reported · {} attempt(s)\n",u.task_id,u.owner_id.as_deref().unwrap_or("unknown (historical)"),u.workflow_run_id.as_deref().unwrap_or("none"),u.provider.as_ref().map(|p|p.id.as_str()).unwrap_or("unknown (historical)"),u.model,u.reported_tokens,u.reported_model_requests,u.model_requests,u.attempts));
    }
    if report.usage_truncated {
        text.push_str(
            "Usage detail truncated; complete SQL totals above. Use smaller CLI task pages.\n",
        );
    }
    text.push_str("\nAGENT / MODEL EVIDENCE SCORECARDS\n");
    if let Some(cards) = &report.scorecards {
        for s in cards.entries.iter().filter(|s| match focus {
            Focus::All => true,
            Focus::Project(id) => &s.project_id == id,
            Focus::Agent(id) => s.owner_id.as_ref() == Some(id),
            Focus::Workflow(id) => s.workflow_run_id.as_ref() == Some(id),
        }) {
            text.push_str(&format!("agent {} · {}/{} · project {} · workflow {}\n  {} attempts · {} accepted results · {} failed · {} blocked · {} uncertain\n  checks: {} passed / {} failed · {} reported tokens · {} admitted / {} reported requests\n",s.owner_id.as_deref().unwrap_or("unavailable (historical)"),s.provider.as_ref().map(|p|p.id.as_str()).unwrap_or("unavailable (historical)"),s.model.as_deref().unwrap_or("unavailable (historical)"),s.project_id,s.workflow_run_id.as_deref().unwrap_or("none"),s.attempts,s.accepted_results,s.failed_attempts,s.blocked_attempts,s.uncertain_attempts,s.checks_passed,s.checks_failed,s.reported_tokens,s.model_requests,s.reported_model_requests));
        }
        if cards.truncated {
            text.push_str(
                "Scorecard groups truncated at 2,000; choose a project to narrow the report.\n",
            );
        }
        text.push_str("Evidence describes recorded outcomes and checks, not a quality grade. Missing attribution stays unavailable; no automatic model fallback.\n");
    } else {
        text.push_str("Scorecards unavailable from this daemon; refresh after updating it.\n");
    }
    text.push_str("\nUPCOMING LOCAL RUNS (first 16)\n");
    for run in &report.upcoming_runs {
        text.push_str(&format!(
            "{} · {} · project {} · {}\n",
            run.schedule_id,
            run.label,
            run.project_id,
            utc_time(run.due_at)
        ));
    }
    text.push_str("\nUnreported usage and provider billing are unknown. Limits fence admissions; already admitted usage can exceed a token ceiling. CPU, RAM and invoices are unavailable. No automatic retry or Apply.\n");
    text
}
pub(super) fn inventory_text(inventory: &OrganizationInventory) -> String {
    let mut text = String::from(
        "CONFIGURED CAPABILITIES / RECORDED EVIDENCE\nNo probes, installations or credential values.\n\nPROVIDERS\n",
    );
    for provider in &inventory.providers {
        if let Some(fingerprint) = &provider.endpoint_sha256 {
            text.push_str(&format!("Owner quote endpoint binding: {fingerprint}\n"));
        }
        text.push_str(&format!("{}{} · {} · {}\n  {}\n  {} requests / {} usage receipts · {} checked results · {} failed task attempts\n  models {:?} · attempt {}\n",provider.id,if provider.selected {" (selected)"} else {""},provider.origin.as_deref().unwrap_or("host-resolved endpoint; unknown"),provider.qualification,provider.credential_requirement,provider.observation.model_requests,provider.observation.reported_model_requests,provider.observation.checked_results,provider.observation.failed_attempts,provider.observation.models,provider.observation.latest_attempt_id.as_deref().unwrap_or("none")));
    }
    text.push_str("\nNATIVE TASK TOOLS\n");
    for tool in &inventory.tools {
        text.push_str(&format!(
            "{} · {} completed / {} failed / {} uncertain · attempt {}\n  {}\n",
            tool.name,
            tool.completed,
            tool.failed,
            tool.uncertain,
            tool.latest_attempt_id.as_deref().unwrap_or("none"),
            tool.permissions
        ));
    }
    text.push_str("\nCONFIGURED MCP SERVERS\n");
    for server in &inventory.mcp_servers {
        text.push_str(&format!(
            "{} · enabled {} · {} · admitted {}\n  {}\n",
            server.name,
            server.enabled,
            server.qualification,
            server.available_to_tasks,
            server.permissions
        ));
    }
    text.push_str(&inventory.limitations);
    text
}

pub(super) fn model_advice(
    report: &OrganizationReport,
    provider: &OrganizationProviderEvidence,
) -> String {
    let mut text = String::from(
        "\nMODEL ADVICE · selected configured endpoint, captured bounded-runtime results\n",
    );
    let entries = report
        .scorecards
        .as_ref()
        .map(|s| s.entries.as_slice())
        .unwrap_or(&[]);
    let mut candidates: Vec<_> = entries
        .iter()
        .filter(|s| {
            s.provider.as_ref() == Some(provider)
                && s.model.is_some()
                && s.accepted_results > 0
                && s.checks_passed > 0
                && s.uncertain_attempts == 0
        })
        .collect();
    candidates.sort_by_key(|s| std::cmp::Reverse((s.accepted_results, s.checks_passed)));
    for s in candidates.iter().take(8) {
        text.push_str(&format!("Candidate {} · project {} · {} owner-accepted results · checks {} pass / {} fail · {} failed / {} blocked attempts · {} reported tokens / {}/{} requests reported\nCaptured endpoint SHA256 {}\n",s.model.as_deref().unwrap_or("unavailable"),s.project_id,s.accepted_results,s.checks_passed,s.checks_failed,s.failed_attempts,s.blocked_attempts,s.reported_tokens,s.reported_model_requests,s.model_requests,provider.endpoint_sha256));
    }
    if candidates.is_empty() {
        text.push_str("No checked, accepted candidate at this exact configured endpoint; recommendation unavailable.\n");
    }
    text.push_str("Counts describe captured tasks, not general model quality or a price ranking. Compatibility is revalidated for new grants; unavailable/Code Mode-only models may refuse. Choose explicitly in a scope; no fallback or model change is performed. Missing/unreported cost remains unavailable.\n");
    text
}

pub(super) async fn inventory_advice(
    handle: &AppServerRequestHandle,
    inventory: &OrganizationInventory,
) -> anyhow::Result<String> {
    let mut text = inventory_text(inventory);
    let Some(selected) = inventory.providers.iter().find(|p| p.selected) else {
        text.push_str("\nModel advice unavailable: no selected configured provider.\n");
        return Ok(text);
    };
    let report = load(handle, None).await?;
    let source = selected.observation.latest_attempt_id.as_deref();
    let Some(task) = report
        .tasks
        .iter()
        .find(|t| t.attempt_id.as_deref() == source && source.is_some())
    else {
        text.push_str("\nModel advice unavailable: no current endpoint provenance in the bounded loaded report.\n");
        return Ok(text);
    };
    let response: OrganizationExecutionReadResponse = handle
        .request_typed(ClientRequest::OrganizationExecutionRead {
            request_id: request_id(),
            params: OrganizationExecutionReadParams {
                task_id: task.task_id.clone(),
            },
        })
        .await?;
    if let Some(provider) = response
        .view
        .attempts
        .iter()
        .find(|a| Some(a.id.as_str()) == source)
        .and_then(|a| a.provider.as_ref())
        .filter(|p| p.id == selected.id)
    {
        text.push_str(&model_advice(&report, provider));
    } else {
        text.push_str("\nModel advice unavailable: captured configured endpoint missing.\n");
    }
    Ok(text)
}

/// Advice only, computed after scoped retrieval. Never retires or promotes notes.
pub(super) fn freshness(notes: &[OrganizationNote], now: i64) -> String {
    let mut text = String::from("\nMEMORY REVIEW ADVICE · loaded scoped matches only\n");
    for (index, note) in notes.iter().enumerate() {
        if now.saturating_sub(note.created_at) > 30 * 24 * 3600 {
            text.push_str(&format!(
                "{}: older than 30 days; verify against current evidence.\n",
                note.id
            ));
        }
        for prior in &notes[..index] {
            if note.project_id != prior.project_id || note.scope != prior.scope {
                continue;
            }
            if note.body == prior.body {
                text.push_str(&format!(
                    "{} / {}: identical text; review duplication.\n",
                    prior.id, note.id
                ));
            } else if note.source_note_id.as_deref().unwrap_or(&note.id)
                == prior.source_note_id.as_deref().unwrap_or(&prior.id)
            {
                text.push_str(&format!(
                    "{} / {}: differing versions of one source; review possible conflict.\n",
                    prior.id, note.id
                ));
            }
        }
    }
    text.push_str("Age/identity/text comparisons are suggestions, not semantic truth. O reviews a correction/promotion; originals and history remain.\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_freshness_advice_does_not_merge_or_retire_notes() {
        let note: OrganizationNote = serde_json::from_value(serde_json::json!({"id":"source","projectId":"project","authorId":"agent","taskId":"task","attemptId":"attempt","scope":"agent","body":"Original evidence","createdAt":1,"ownerReviewed":false})).unwrap();
        let duplicate = OrganizationNote {
            id: "duplicate".into(),
            ..note.clone()
        };
        let correction = OrganizationNote {
            id: "correction".into(),
            source_note_id: Some(note.id.clone()),
            body: "Changed evidence".into(),
            ..note.clone()
        };
        let unrelated = OrganizationNote {
            id: "unrelated".into(),
            project_id: "other-project".into(),
            ..note.clone()
        };
        let notes = vec![note, duplicate, correction, unrelated];
        let before = notes.clone();
        let text = freshness(&notes, 31 * 24 * 3600);
        assert!(text.contains("source / duplicate: identical"));
        assert!(text.contains("source / correction: differing versions"));
        assert!(!text.contains("source / unrelated:"));
        assert!(text.contains("older than 30 days"));
        assert_eq!(notes, before);
    }
    fn task(state: &str) -> OrganizationTaskReport {
        OrganizationTaskReport {
            task_id: "task".into(),
            project_id: "project".into(),
            owner_id: "agent".into(),
            workflow_run_id: None,
            title: "Untrusted title: requested changes".into(),
            state: state.into(),
            revision: 1,
            attempt_id: None,
            result_digest: None,
            checkpoint_id: None,
            blocker: None,
            changes_requested_sequence: None,
            pending_approvals: 0,
            checks_passed: 0,
            checks_failed: 0,
            reported_tokens: 0,
            model_requests: 0,
            reported_model_requests: 0,
            budget: None,
        }
    }
    #[test]
    fn inbox_uses_canonical_receipts_and_never_instruction_claims() {
        assert!(inbox_row(&task("proposed")).is_none());
        assert!(inbox_row(&task("accepted")).is_none());
        let mut t = task("waitingApproval");
        t.pending_approvals = 2;
        assert!(inbox_row(&t).unwrap().label.contains("2 approval(s)"));
        t = task("paused");
        assert!(inbox_row(&t).is_none());
        t.changes_requested_sequence = Some(5);
        assert!(inbox_row(&t).unwrap().label.contains("requested changes"));
        t = task("needsReview");
        assert!(inbox_row(&t).is_none());
        t.result_digest = Some("exact-digest".into());
        assert!(inbox_row(&t).unwrap().label.contains("result review"));
        for state in ["blocked", "failed", "needsReconciliation"] {
            assert!(inbox_row(&task(state)).is_some());
        }
    }
    #[test]
    fn summaries_expose_unknown_usage_and_deduplicate_shared_root_budgets() {
        let budget = OrganizationBudgetReport {
            grant_id: "grant".into(),
            root_id: "root".into(),
            model: "model".into(),
            max_tokens: 100,
            used_tokens: 25,
            max_model_requests: 4,
            model_requests: 2,
            task_attempts: 1,
            task_max_attempts: 2,
            expires_at: 100,
            revoked: false,
        };
        let mut a = task("accepted");
        a.budget = Some(budget.clone());
        let mut b = a.clone();
        b.task_id = "child".into();
        b.budget.as_mut().unwrap().grant_id = "child-grant".into();
        let report = OrganizationReport {
            resource_pressure: None,
            sequence: 3,
            generated_at: 0,
            project_id: None,
            totals: OrganizationReportTotals::default(),
            global_usage: Some(OrganizationGlobalUsage::default()),
            scorecards: None,
            audit_events: None,
            runtime_resources: None,
            tasks: vec![a, b],
            usage: vec![OrganizationUsageReport {
                task_id: "task".into(),
                workflow_run_id: None,
                owner_id: None,
                provider: None,
                model: "model".into(),
                reported_tokens: 10,
                model_requests: 1,
                reported_model_requests: 1,
                attempts: 1,
            }],
            usage_truncated: false,
            upcoming_runs: vec![],
            next_cursor: None,
        };
        let text = text(&report, &Focus::All, None);
        assert_eq!(text.matches("SHARED ROOT root").count(), 1);
        assert!(text.contains("75 tokens and 2 requests remaining"));
        assert!(text.contains("unknown (historical)"));
        assert!(text.contains("execution-read child"));
    }
}
