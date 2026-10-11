//! Workflow views use the existing dashboard/task inspectors and scope forms.
use super::*;

pub(super) fn workflow_id(view: &OrganizationWorkflowView) -> String {
    match view {
        OrganizationWorkflowView::Series { series, .. } => format!("series:{}", series.id),
        OrganizationWorkflowView::Template { template } => format!("workflow:{}", template.id),
        OrganizationWorkflowView::Run { run, .. } => format!("run:{}", run.id),
        OrganizationWorkflowView::Limit { limit, .. } => format!("limit:{}", limit.id),
    }
}

impl Dashboard {
    pub(super) fn accepted_plan_items(
        &self,
    ) -> anyhow::Result<(String, Vec<OrganizationPlanItem>)> {
        let view = self
            .detail
            .as_ref()
            .filter(|view| Some(&view.task.id) == self.selected.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Wait for the selected planner's details"))?;
        let result = view
            .result
            .as_ref()
            .filter(|result| result.accepted && view.task.state == "accepted")
            .ok_or_else(|| {
                anyhow::anyhow!("Accept the planner's exact result before importing it")
            })?;
        let artifact = result
            .artifacts
            .iter()
            .find(|artifact| artifact.path == "plan.json" && artifact.bytes <= 65536)
            .ok_or_else(|| {
                anyhow::anyhow!("An accepted plan.json artifact of at most 64 KiB is required")
            })?;
        let workspace = view
            .attempts
            .iter()
            .find(|attempt| attempt.id == result.attempt_id)
            .and_then(|attempt| attempt.workspace.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Planner workspace unavailable"))?;
        let preview = Preview::new(
            self.path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Owner home unavailable"))?,
            Path::new(workspace),
            &artifact.path,
            Some(artifact),
        )?;
        let items: Vec<OrganizationPlanItem> =
            serde_json::from_slice(&std::fs::read(preview.path())?)?;
        anyhow::ensure!(
            (1..=32).contains(&items.len()),
            "Plan must contain 1–32 steps"
        );
        Ok((view.task.project_id.clone(), items))
    }
    pub(super) fn selected_workflow(&self) -> Option<&OrganizationWorkflowView> {
        self.snapshot
            .workflows
            .iter()
            .find(|view| Some(&workflow_id(view)) == self.selected.as_ref())
    }

    pub(super) fn selected_template(&self) -> Option<&OrganizationWorkflowTemplate> {
        match self.selected_workflow()? {
            OrganizationWorkflowView::Template { template } => Some(template),
            _ => None,
        }
    }

    /// Capture existing task content, never its grants, results or attempt identities.
    pub(super) fn workflow_items(&self) -> anyhow::Result<(String, Vec<OrganizationPlanItem>)> {
        let Some(OrganizationRecord::Task(task)) = self.record() else {
            anyhow::bail!("Select the final task to save its prerequisite chain as a workflow")
        };
        let tasks: BTreeMap<_, _> = self
            .snapshot
            .records
            .iter()
            .filter_map(|record| match record {
                OrganizationRecord::Task(task) => Some((task.id.as_str(), task)),
                _ => None,
            })
            .collect();
        let mut stack = vec![task.id.as_str()];
        let mut keys = BTreeSet::new();
        let mut items = Vec::new();
        while let Some(id) = stack.pop() {
            if !keys.insert(id) {
                continue;
            }
            anyhow::ensure!(
                keys.len() <= 32,
                "Workflow templates contain at most 32 tasks"
            );
            let step = tasks.get(id).ok_or_else(|| {
                anyhow::anyhow!("Prerequisite {id} is not loaded; refresh or use the CLI")
            })?;
            anyhow::ensure!(
                step.project_id == task.project_id,
                "Workflow prerequisites must belong to one project"
            );
            stack.extend(step.prerequisites.iter().map(String::as_str));
            items.push(OrganizationPlanItem {
                key: step.id.clone(),
                owner_id: step.owner_id.clone(),
                instructions: step.instructions.clone(),
                criteria: step.criteria.clone(),
                prerequisites: step.prerequisites.clone(),
            });
        }
        Ok((task.project_id.clone(), items))
    }

    pub(super) fn workflow_details(&self) -> String {
        let Some(view) = self.selected_workflow() else {
            return String::new();
        };
        let (template, run) = match view {
            OrganizationWorkflowView::Series { series, usage } => {
                return format!(
                    "Workflow series: {}\n{}\nState: {} · revision {}\nTrigger: {:?}\nNext due: {}\nEnd: {}\nIssued: {} / {} · skipped: {}\nReported tokens: {} / {}\nRequests: {} / {}\n{}\n\np pauses future invocations; current runs use their own controls.\nu resumes future invocations without resetting usage.\nx cancels unfinished series work. No automatic acceptance or Apply.\nCreate finite series with owner workflow-mutate JSON.\n",
                    series.config.label,
                    series.id,
                    series.state.as_str(),
                    series.revision,
                    series.trigger,
                    series
                        .next_run_at
                        .map(utc_time)
                        .unwrap_or_else(|| "None".into()),
                    utc_time(series.config.ends_at),
                    series.issued_runs,
                    series.config.max_runs,
                    series.skipped_runs,
                    usage.reported_tokens,
                    series.config.max_tokens,
                    usage.model_requests,
                    series.config.max_model_requests,
                    series
                        .reason
                        .as_deref()
                        .unwrap_or("Pinned scopes; one active invocation")
                );
            }
            OrganizationWorkflowView::Limit { limit, usage } => {
                return format!(
                    "Shared resource limits: {}\nProject: {}\nWorkflow run: {}\nRevision: {}\n\nReported tokens: {} / {}\nAdmitted model requests: {} / {}\nActive executions: {} / {}\n\nb edits these exact limits. Consumed usage includes historical root grants and is never reset by pause, handoff or reauthorization. A token ceiling stops new admission after reported usage; in-flight requests may exceed it. Cost estimates are not monetary caps.\n",
                    limit.id,
                    limit.project_id,
                    limit.run_id.as_deref().unwrap_or("Entire project"),
                    limit.revision,
                    usage.reported_tokens,
                    limit.max_tokens,
                    usage.model_requests,
                    limit.max_model_requests,
                    usage.active,
                    limit.max_active
                );
            }
            OrganizationWorkflowView::Template { template } => (template, None),
            OrganizationWorkflowView::Run { run, tasks } => {
                (&run.preview.template, Some((run, tasks)))
            }
        };
        let mut text = format!(
            "Workflow: {}\nTemplate: {} · revision {}\nProject: {}\n\n",
            template.label, template.id, template.revision, template.project_id
        );
        if let Some((run, _)) = run {
            text.push_str(&format!(
                "Run: {}\nState: {:?} · revision {}\np pause · u continue · x cancel (explicit confirmation)\nCreated: {}\nFixed grant ceilings: {} tokens / {} requests\n\n",
                run.id,
                run.state,
                run.revision,
                utc_time(run.created_at),
                run.preview.max_tokens,
                run.preview.max_model_requests
            ));
            if let Some(limits) = &run.preview.shared_limits {
                text.push_str(&format!("Initial shared caps: {} tokens / {} requests / {} active\nCurrent limits have their own canonical row; b edits this run's current caps.\n\n", limits.max_tokens, limits.max_model_requests, limits.max_active));
            }
        } else {
            text.push_str("Saved content grants no authority. E edits steps / assignments / prerequisites / criteria. s previews distinct per-step grants.\n\n");
        }
        for item in &template.items {
            text.push_str(&format!(
                "Step: {}\nAssigned agent: {}\n{}\nCriteria:\n{}\nRequires: {}\n",
                item.key,
                item.owner_id,
                item.instructions,
                item.criteria.join("\n"),
                if item.prerequisites.is_empty() {
                    "None".into()
                } else {
                    item.prerequisites.join(", ")
                }
            ));
            if let Some((run, tasks)) = run {
                let task = tasks.iter().find(|task| task.id == run.task_ids[&item.key]);
                if let Some(task) = task {
                    text.push_str(&format!("Task: {} · [{}]\n", task.id, task.state));
                }
            }
            text.push('\n');
        }
        text.push_str("Prerequisites require exact owner acceptance. Failed or interrupted work blocks dependents.\nSelect a run task to inspect approvals/results or Follow in actual Neovim.\nAcceptance and Apply remain separate owner decisions.\n");
        text
    }
}
