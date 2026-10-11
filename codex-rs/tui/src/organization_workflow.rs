//! Workflow views use the existing dashboard/task inspectors and scope forms.
use super::*;

pub(super) fn workflow_id(view: &OrganizationWorkflowView) -> String {
    match view {
        OrganizationWorkflowView::Template { template } => format!("workflow:{}", template.id),
        OrganizationWorkflowView::Run { run, .. } => format!("run:{}", run.id),
    }
}

impl Dashboard {
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
