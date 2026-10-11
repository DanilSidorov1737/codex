//! On-demand local projections and the Review inbox. No model calls or authority.
use super::*;

#[derive(Clone)]
pub(super) enum Focus {
    All,
    Project(String),
    Agent(String),
    Workflow(String),
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

#[cfg(test)]
mod tests {
    use super::*;
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
            sequence: 3,
            generated_at: 0,
            project_id: None,
            totals: OrganizationReportTotals::default(),
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
