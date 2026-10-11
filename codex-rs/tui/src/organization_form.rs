//! Fixed owner forms reuse the existing textarea and typed administration contracts.
use super::*;
use crate::bottom_pane::{TextArea, TextAreaState};
use crossterm::event::KeyModifiers;
use ratatui::layout::Margin;

pub(super) enum Kind {
    Agent,
    Group,
    NodeEdit(OrganizationNode),
    Role(Option<OrganizationRoleVersion>),
    Membership(OrganizationNode),
    Goal,
    Project,
    Task,
    GoalPlan(OrganizationGoal),
    Selection {
        project_id: String,
        selection: crate::organization_editor::Selection,
    },
    Handoff {
        task: OrganizationTask,
        checkpoint: OrganizationTaskCheckpoint,
        receivers: Vec<OrganizationNode>,
    },
    Authorize(OrganizationTask),
    Schedule(OrganizationTask),
    WorkflowSave {
        project_id: String,
        items: Vec<OrganizationPlanItem>,
        previous: Option<OrganizationWorkflowTemplate>,
    },
    WorkflowRun {
        template: OrganizationWorkflowTemplate,
    },
    SharedLimits {
        project_id: String,
        run_id: Option<String>,
        previous: Option<OrganizationSharedLimit>,
    },
    Apply {
        task: OrganizationTask,
        result: OrganizationTaskResult,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(form: &mut Form, index: usize, value: &str) {
        form.fields[index].input.set_text_clearing_elements(value);
    }

    fn task() -> OrganizationTask {
        OrganizationTask {
            id: "original-task".into(),
            project_id: "project".into(),
            goal_id: None,
            owner_id: "agent".into(),
            instructions: "Repair addition".into(),
            criteria: vec!["Oracle passes".into()],
            prerequisites: vec![],
            state: "proposed".into(),
            revision: 7,
        }
    }

    #[test]
    fn goal_planner_is_only_a_proposal_and_pins_the_goal() {
        let goal = OrganizationGoal {
            id: "goal".into(),
            parent_id: None,
            objective: "Write a tested handbook".into(),
            criteria: vec!["Reviewable chapters".into()],
            revision: 3,
        };
        let project = OrganizationProjectPlan {
            id: "p".into(),
            name: "Handbook".into(),
            roots: vec![],
            goal_id: Some(goal.id.clone()),
            owner_id: Some("owner".into()),
            criteria: vec![],
            revision: 1,
        };
        let records = vec![OrganizationRecord::Project(project), OrganizationRecord::Goal(goal.clone()), serde_json::from_value(serde_json::json!({"type":"node","data":{"id":"owner","parentId":null,"kind":"agent","label":"Writer","roleVersionId":null,"model":null,"enabled":true,"revision":1}})).unwrap()];
        let form = Form::new(Kind::GoalPlan(goal), &records, Some(&records[1]), "model");
        let Command::Definition(OrganizationMutation::PutTask {
            id: None,
            goal_id,
            instructions,
            criteria,
            ..
        }) = form.command().unwrap()
        else {
            panic!("planner must only create a proposed task")
        };
        assert_eq!(goal_id.as_deref(), Some("goal"));
        assert!(
            instructions.contains("plan.json")
                && instructions.contains("Write a tested handbook")
                && instructions.contains("separately approve")
        );
        assert!(criteria[0].contains("acyclic"));
    }

    #[test]
    fn shared_limit_form_pins_target_and_revision_without_resetting_usage() {
        let previous = OrganizationSharedLimit {
            id: "run:r".into(),
            project_id: "p".into(),
            run_id: Some("r".into()),
            revision: 8,
            max_tokens: 100,
            max_model_requests: 2,
            max_active: 1,
        };
        let mut form = Form::new(
            Kind::SharedLimits {
                project_id: "p".into(),
                run_id: Some("r".into()),
                previous: Some(previous),
            },
            &[],
            None,
            "model",
        );
        set(&mut form, 0, "200");
        let Command::Workflow(OrganizationWorkflowMutation::SetLimits {
            project_id,
            run_id,
            expected_revision,
            max_tokens,
            max_model_requests,
            max_active,
        }) = form.command().unwrap()
        else {
            panic!("limits")
        };
        assert_eq!(
            (project_id, run_id, expected_revision),
            ("p".into(), Some("r".into()), Some(8))
        );
        assert_eq!((max_tokens, max_model_requests, max_active), (200, 2, 1));
        set(&mut form, 1, "");
        assert!(form.command().is_err());
    }

    #[test]
    fn selection_draft_pins_source_project_and_creates_only_proposed_content() {
        let capture = crate::organization_editor::Selection {
            buffer: 4,
            path: "/tmp/source/café.py".into(),
            changedtick: 8,
            modified: true,
            mode: "v".into(),
            positions: vec![[[4, 1, 1, 0], [4, 1, 5, 0]]],
            text: "café".into(),
            sha256: "a".repeat(64),
        };
        let projects = ["source-project", "other-project"].map(|id| {
            OrganizationRecord::Project(OrganizationProjectPlan {
                id: id.into(),
                name: id.into(),
                roots: vec!["/tmp/source".into()],
                goal_id: None,
                owner_id: Some("agent".into()),
                criteria: vec![],
                revision: 1,
            })
        });
        let mut form = Form::new(
            Kind::Selection {
                project_id: "source-project".into(),
                selection: capture.clone(),
            },
            &projects,
            None,
            "model",
        );
        assert_eq!(form.fields[0].choices.len(), 1);
        set(&mut form, 1, "agent");
        set(&mut form, 3, "Repair addition");
        set(&mut form, 4, "Oracle passes");
        for purpose in 0..4 {
            form.fields[6].choice = purpose;
            let Command::Definition(OrganizationMutation::PutTask {
                project_id,
                instructions,
                ..
            }) = form.command().unwrap()
            else {
                panic!("not a proposed task")
            };
            assert_eq!(project_id, "source-project");
            assert!(instructions.contains("café"));
            assert!(instructions.contains("changedtick"));
            assert!(instructions.contains("grants no paths/tools"));
            assert_eq!(form.selection(), Some(&capture));
        }
    }

    #[test]
    fn node_edit_preserves_identity_kind_and_root_parent_while_retaining_history() {
        let other = OrganizationRecord::Node(OrganizationNode {
            id: "other".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Other".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 1,
        });
        for kind in [OrganizationNodeKind::Agent, OrganizationNodeKind::Group] {
            let node = OrganizationNode {
                id: "captured".into(),
                parent_id: None,
                kind,
                label: "Custom level".into(),
                role_version_id: None,
                model: None,
                enabled: true,
                revision: 9,
            };
            let mut form = Form::new(
                Kind::NodeEdit(node.clone()),
                &[other.clone(), OrganizationRecord::Node(node.clone())],
                Some(&other),
                "model",
            );
            assert!(!form.fields[1].choices.iter().any(|(id, _)| id == &node.id));
            form.fields[if kind == OrganizationNodeKind::Group {
                2
            } else {
                4
            }]
            .choice = 1;
            let Command::Definition(OrganizationMutation::PutNode {
                id,
                expected_revision,
                parent_id,
                kind: captured_kind,
                enabled,
                model,
                role_version_id,
                ..
            }) = form.command().unwrap()
            else {
                panic!("missing edit")
            };
            assert_eq!((id, expected_revision), (Some(node.id), Some(9)));
            assert_eq!(captured_kind, kind);
            assert_eq!(
                parent_id, None,
                "editing a root must not adopt another selected node"
            );
            assert!(!enabled);
            assert_eq!((model, role_version_id), (None, None));
        }
    }

    #[test]
    fn role_and_membership_forms_capture_versions_and_ids_without_execution_grants() {
        let role = OrganizationRoleVersion {
            id: "old-version".into(),
            role_id: "role".into(),
            version: 4,
            name: "My role".into(),
            instructions: "Original".into(),
        };
        let mut form = Form::new(Kind::Role(Some(role)), &[], None, "model");
        set(&mut form, 1, "New instructions");
        assert!(
            matches!(form.command().unwrap(),Command::Definition(OrganizationMutation::CreateRoleVersion {role_id:Some(id),expected_version:Some(4),instructions,..}) if id=="role" && instructions=="New instructions")
        );
        let node = OrganizationNode {
            id: "member".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Any level".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 2,
        };
        let group = OrganizationRecord::Node(OrganizationNode {
            id: "group".into(),
            kind: OrganizationNodeKind::Group,
            ..node.clone()
        });
        let mut form = Form::new(Kind::Membership(node), &[group], None, "model");
        for present in [true, false] {
            form.fields[1].choice = usize::from(!present);
            assert!(
                matches!(form.command().unwrap(),Command::Definition(OrganizationMutation::SetMembership {node_id,group_id,present:captured}) if node_id=="member" && group_id=="group" && captured==present)
            );
        }
    }

    #[test]
    fn workflow_edit_pins_revision_and_run_keeps_distinct_step_scopes() {
        let items = vec![
            OrganizationPlanItem {
                key: "first".into(),
                owner_id: "agent".into(),
                instructions: "Inspect".into(),
                criteria: vec!["Cited".into()],
                prerequisites: vec![],
            },
            OrganizationPlanItem {
                key: "second".into(),
                owner_id: "agent".into(),
                instructions: "Write".into(),
                criteria: vec!["Checked".into()],
                prerequisites: vec!["first".into()],
            },
        ];
        let template = OrganizationWorkflowTemplate {
            id: "template".into(),
            project_id: "project".into(),
            label: "Custom flow".into(),
            revision: 8,
            items: items.clone(),
        };
        let mut form = Form::new(
            Kind::WorkflowSave {
                project_id: "project".into(),
                items,
                previous: Some(template.clone()),
            },
            &[],
            None,
            "model",
        );
        set(&mut form, 9, "Owner-reviewed evidence");
        let Command::Workflow(OrganizationWorkflowMutation::Put {
            id,
            expected_revision,
            items,
            ..
        }) = form.command().unwrap()
        else {
            panic!("missing edit")
        };
        assert_eq!((id, expected_revision), ("template".into(), Some(8)));
        assert_eq!(items[1].criteria, vec!["Owner-reviewed evidence"]);
        assert_eq!(items[1].prerequisites, vec!["first"]);
        form.key(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE));
        assert_eq!(form.fields.len(), 16);
        assert!(
            form.command().is_err(),
            "a blank added step cannot be saved"
        );
        form.key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
        assert_eq!(form.fields.len(), 11);
        assert!(form.command().is_ok());
        let mut form = Form::new(Kind::WorkflowRun { template }, &[], None, "model");
        set(&mut form, 0, "first.txt");
        set(&mut form, 2, "first.txt");
        set(&mut form, 5, "100");
        set(&mut form, 14, "second.txt");
        set(&mut form, 16, "second.txt");
        set(&mut form, 19, "200");
        let Command::WorkflowPreview(request) = form.command().unwrap() else {
            panic!("missing preview")
        };
        assert_eq!(request.scopes[0].scope.input_files, vec!["first.txt"]);
        assert_eq!(request.scopes[1].scope.input_files, vec!["second.txt"]);
        assert_eq!(request.scopes[0].scope.max_tokens, 100);
        assert_eq!(request.scopes[1].scope.max_tokens, 200);
        assert!(request.shared_limits.is_none());
        set(&mut form, 28, "250");
        assert!(
            form.command().is_err(),
            "a partial shared ceiling cannot be confirmed"
        );
        set(&mut form, 29, "3");
        set(&mut form, 30, "1");
        let Command::WorkflowPreview(request) = form.command().unwrap() else {
            panic!("preview")
        };
        assert_eq!(
            request.shared_limits,
            Some(OrganizationSharedCeilings {
                max_tokens: 250,
                max_model_requests: 3,
                max_active: 1
            })
        );
        assert_eq!(request.scopes[1].scope.max_tokens, 200);
        set(&mut form, 16, "../escape");
        assert!(form.command().is_err(), "each step validates its own paths");
        for (w, h) in [(1, 1), (50, 20), (160, 48)] {
            let area = Rect::new(0, 0, w, h);
            form.render(area, &mut Buffer::empty(area));
        }
    }

    #[test]
    fn authorization_captures_task_and_rejects_invalid_paths_budgets_and_checks() {
        let mut form = Form::new(Kind::Authorize(task()), &[], None, "native-model");
        set(&mut form, 0, "math.py\noracle.py");
        set(&mut form, 1, "oracle.py");
        set(&mut form, 2, "math.py");
        set(&mut form, 3, "[[\"python3\",\"oracle.py\"]]");
        let Command::Execution(OrganizationExecutionMutation::Authorize {
            task_id,
            expected_revision,
            scope,
        }) = form.command().unwrap()
        else {
            panic!("missing scope")
        };
        assert_eq!(task_id, "original-task");
        assert_eq!(expected_revision, 7);
        assert_eq!(scope.checks, vec![vec!["python3", "oracle.py"]]);
        assert!(scope.require_tool_approval);
        assert_eq!(scope.max_children, 0);
        for invalid in ["../math.py", "/math.py", "math.py\nmath.py"] {
            set(&mut form, 2, invalid);
            assert!(form.command().is_err());
        }
        set(&mut form, 2, "math.py");
        set(&mut form, 5, "0");
        assert!(form.command().is_err());
        set(&mut form, 5, "10000");
        set(&mut form, 3, "python3 oracle.py");
        assert!(form.command().is_err());
        set(&mut form, 3, "[]");
        form.fields[12].choice = 1;
        assert!(form.command().is_err());
    }

    #[test]
    fn handoff_form_captures_checkpoint_task_and_receiver_revisions_without_new_grant() {
        let mut task = task();
        task.state = "blocked".into();
        let node = OrganizationNode {
            id: "receiver".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Custom receiver".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 9,
        };
        let checkpoint = OrganizationTaskCheckpoint {
            id: "pinned-checkpoint".into(),
            task_id: task.id.clone(),
            project_id: task.project_id.clone(),
            owner_id: task.owner_id.clone(),
            attempt_id: "source".into(),
            authorization_id: "original-grant".into(),
            created_at: 1,
            completed_action_ids: vec!["completed-effect".into()],
            content: codex_protocol::organization_coordination::OrganizationCheckpointContent {
                completed_steps: vec![],
                remaining_steps: vec!["Finish report".into()],
                blocker: Some("Owner decision".into()),
            },
            files: vec![],
            artifact_bases: vec![],
        };
        let mut form = Form::new(
            Kind::Handoff {
                task: task.clone(),
                checkpoint,
                receivers: vec![node],
            },
            &[],
            None,
            "fixture",
        );
        assert!(form.command().is_err());
        set(&mut form, 1, "Continue only remaining work");
        let Command::Execution(OrganizationExecutionMutation::Handoff {
            task_id,
            expected_revision,
            checkpoint_id,
            owner_id,
            expected_owner_revision,
            instructions,
        }) = form.command().unwrap()
        else {
            panic!("missing handoff")
        };
        assert_eq!((task_id, expected_revision), (task.id, 7));
        assert_eq!(
            (checkpoint_id, owner_id, expected_owner_revision),
            ("pinned-checkpoint".into(), "receiver".into(), 9)
        );
        assert_eq!(instructions, "Continue only remaining work");
        set(&mut form, 1, &"a".repeat(4097));
        assert!(form.command().is_err());
    }

    #[test]
    fn project_reuses_one_identity_key_and_requires_an_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut form = Form::new(Kind::Project, &[], None, "model");
        assert!(form.command().is_err());
        set(&mut form, 0, "Local project");
        set(&mut form, 1, root.path().to_str().unwrap());
        set(&mut form, 4, "Meaningful check passes\nOwner accepts");
        let Command::Project {
            params: first,
            criteria,
            ..
        } = form.command().unwrap()
        else {
            panic!("missing project")
        };
        let Command::Project { params: second, .. } = form.command().unwrap() else {
            panic!("missing project")
        };
        assert_eq!(first.idempotency_key, second.idempotency_key);
        assert_eq!(criteria.len(), 2);
        assert_eq!(first.roots[0].path.as_path(), root.path());
        set(&mut form, 1, root.path().join("missing").to_str().unwrap());
        assert!(form.command().is_err());
    }

    #[test]
    fn recurring_form_freezes_template_identity_scope_and_finite_limits() {
        let mut form = Form::new(Kind::Schedule(task()), &[], None, "native-model");
        let scope = OrganizationExecutionScope {
            input_files: vec!["oracle.py".into()],
            immutable_files: vec!["oracle.py".into()],
            artifacts: vec!["report.txt".into()],
            checks: vec![vec!["python3".into(), "oracle.py".into()]],
            model: "native-model".into(),
            max_tokens: 1000,
            max_seconds: 120,
            max_model_requests: 2,
            max_attempts: 1,
            require_tool_approval: true,
            delegates: vec![],
            max_children: 0,
            auto_accept_children: false,
            not_before: Some(1),
        };
        form.prefill_scope(&scope);
        let Command::Schedule(first) = form.command().unwrap() else {
            panic!("missing schedule")
        };
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(match form.command().unwrap() {
                Command::Schedule(m) => m,
                _ => panic!("missing schedule"),
            })
            .unwrap()
        );
        let OrganizationScheduleMutation::Create {
            id,
            task_id,
            expected_revision,
            scope: captured,
            config,
        } = first
        else {
            panic!("missing create")
        };
        assert!(!id.is_empty());
        assert_eq!(task_id, "original-task");
        assert_eq!(expected_revision, 7);
        assert_eq!(captured.not_before, None);
        assert!(captured.require_tool_approval);
        assert_eq!(captured.checks, scope.checks);
        assert_eq!(
            (
                config.max_runs,
                config.max_tokens,
                config.max_model_requests
            ),
            (3, 30000, 15)
        );
        for (index, value) in [(15, "0"), (16, "1"), (17, "1001"), (18, "0"), (19, "0")] {
            let old = form.fields[index].value().to_string();
            set(&mut form, index, value);
            assert!(form.command().is_err());
            set(&mut form, index, &old);
        }
        for (w, h) in [(1, 1), (50, 20), (160, 48)] {
            let area = Rect::new(0, 0, w, h);
            form.render(area, &mut Buffer::empty(area));
        }
    }

    #[test]
    fn workflow_forms_save_content_only_and_preview_fixed_scopes_with_stable_identity() {
        let items = vec![OrganizationPlanItem {
            key: "inspect".into(),
            owner_id: "agent".into(),
            instructions: "Inspect café".into(),
            criteria: vec!["Cited".into()],
            prerequisites: vec![],
        }];
        let mut form = Form::new(
            Kind::WorkflowSave {
                project_id: "project".into(),
                items: items.clone(),
                previous: None,
            },
            &[],
            None,
            "model",
        );
        set(&mut form, 0, "Research café");
        let Command::Workflow(OrganizationWorkflowMutation::Put {
            id,
            project_id,
            items: saved,
            ..
        }) = form.command().unwrap()
        else {
            panic!("expected content-only template")
        };
        assert_eq!(project_id, "project");
        assert_eq!(saved, items);
        let Command::Workflow(OrganizationWorkflowMutation::Put { id: again, .. }) =
            form.command().unwrap()
        else {
            panic!("expected template")
        };
        assert_eq!(id, again);
        let template = OrganizationWorkflowTemplate {
            id,
            project_id,
            label: "Research café".into(),
            revision: 3,
            items,
        };
        let mut form = Form::new(Kind::WorkflowRun { template }, &[], None, "model");
        set(&mut form, 0, "oracle.py");
        set(&mut form, 1, "oracle.py");
        set(&mut form, 2, "report.txt");
        set(&mut form, 3, "[[\"python3\",\"oracle.py\"]]");
        let Command::WorkflowPreview(request) = form.command().unwrap() else {
            panic!("expected read-only preview")
        };
        assert_eq!(request.expected_revision, 3);
        assert_eq!(request.scopes[0].key, "inspect");
        assert!(request.scopes[0].scope.require_tool_approval);
        for (w, h) in [(1, 1), (50, 20), (160, 48)] {
            let area = Rect::new(0, 0, w, h);
            form.render(area, &mut Buffer::empty(area));
        }
    }

    #[test]
    fn multiline_unicode_draft_and_choice_ids_survive_navigation() {
        let records = vec![OrganizationRecord::Node(OrganizationNode {
            id: "stable-agent".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Arbitrary researcher".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 1,
        })];
        let mut form = Form::new(Kind::Task, &records, None, "model");
        set(&mut form, 0, "project");
        form.focus = 3;
        form.paste("Read facts — café\nCite sources");
        form.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        form.paste("Contains exact facts");
        let Command::Definition(OrganizationMutation::PutTask {
            owner_id,
            instructions,
            ..
        }) = form.command().unwrap()
        else {
            panic!("missing task")
        };
        assert_eq!(owner_id, "stable-agent");
        assert_eq!(instructions, "Read facts — café\nCite sources");
        for size in [(1, 1), (50, 20), (160, 44)] {
            let area = Rect::new(0, 0, size.0, size.1);
            form.render(area, &mut Buffer::empty(area));
        }
    }

    #[test]
    fn apply_selection_preserves_exact_paths_and_requires_a_nonempty_set() {
        let result = OrganizationTaskResult {
            attempt_id: "attempt".into(),
            artifacts: ["report.txt", " notes café.md "]
                .into_iter()
                .map(|p| OrganizationArtifactEvidence {
                    path: p.into(),
                    sha256: "a".repeat(64),
                    bytes: 1,
                })
                .collect(),
            artifact_bases: vec![],
            checks: vec![],
            digest: "exact-result".into(),
            accepted: true,
        };
        let mut form = Form::new(
            Kind::Apply {
                task: task(),
                result,
            },
            &[],
            None,
            "unused",
        );
        form.key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        let Command::Execution(OrganizationExecutionMutation::Apply {
            task_id,
            expected_revision,
            digest,
            paths,
        }) = form.command().unwrap()
        else {
            panic!("missing apply")
        };
        assert_eq!(task_id, "original-task");
        assert_eq!(expected_revision, 7);
        assert_eq!(digest, "exact-result");
        assert_eq!(paths, Some(vec![" notes café.md ".into()]));
        form.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        form.key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(form.command().is_err());
        for size in [(1, 1), (50, 20), (160, 48)] {
            let area = Rect::new(0, 0, size.0, size.1);
            form.render(area, &mut Buffer::empty(area));
        }
    }
}

struct Field {
    label: String,
    hint: &'static str,
    input: TextArea,
    choices: Vec<(String, String)>,
    choice: usize,
}

impl Field {
    fn text(label: impl Into<String>, hint: &'static str, value: &str) -> Self {
        let mut input = TextArea::new();
        input.set_text_clearing_elements(value);
        Self {
            label: clean(&label.into()),
            hint,
            input,
            choices: vec![],
            choice: 0,
        }
    }
    fn select(
        label: impl Into<String>,
        choices: Vec<(String, String)>,
        selected: Option<&str>,
    ) -> Self {
        let choice = choices
            .iter()
            .position(|(id, _)| Some(id.as_str()) == selected)
            .unwrap_or(0);
        Self {
            choices,
            choice,
            ..Self::text(label, "Left / Right choose", "")
        }
    }
    fn value(&self) -> &str {
        self.choices
            .get(self.choice)
            .map(|(id, _)| id.as_str())
            .unwrap_or(self.input.text().trim())
    }
}

pub(super) struct Form {
    kind: Kind,
    fields: Vec<Field>,
    focus: usize,
    project_key: String,
    pub(super) error: String,
}

impl Form {
    fn scope_fields(model: &str) -> Vec<Field> {
        vec![
            Field::text("Input files", "Project-relative paths, one per line", ""),
            Field::text(
                "Immutable inputs",
                "Read-only inputs, including test oracles; one per line",
                "",
            ),
            Field::text(
                "Output artifacts",
                "Project-relative paths, one per line",
                "",
            ),
            Field::text(
                "Checks (argv JSON)",
                "Example: [[\"python3\",\"oracle.py\"]]; no shell parsing",
                "[]",
            ),
            Field::text("Model", "Explicit model for this grant", model),
            Field::text(
                "Token ceiling",
                "Reported tokens; an admitted request can exceed this",
                "10000",
            ),
            Field::text("Time limit (seconds)", "1–3600", "240"),
            Field::text("Model requests", "1–100; provider retries disabled", "5"),
            Field::text("Attempts", "1–3", "1"),
            Field::select(
                "Approve each tool action",
                vec![
                    ("true".into(), "Yes".into()),
                    (
                        "false".into(),
                        "No — routine in-scope tools preauthorized".into(),
                    ),
                ],
                None,
            ),
            Field::text(
                "Allowed delegates",
                "Optional agent IDs, one per line; hierarchy grants no authority",
                "",
            ),
            Field::text(
                "Maximum child tasks",
                "0 disables delegation; maximum 32",
                "0",
            ),
            Field::select(
                "Accept checked children",
                vec![
                    ("false".into(), "No".into()),
                    ("true".into(), "Yes — declared checks required".into()),
                ],
                None,
            ),
            Field::text(
                "Start time (UTC epoch)",
                "Optional one-shot time; blank starts when ready",
                "",
            ),
        ]
    }
    fn step_fields(item: &OrganizationPlanItem, agents: &[(String, String)]) -> Vec<Field> {
        let mut agents = agents.to_vec();
        if !item.owner_id.is_empty() && !agents.iter().any(|(id, _)| id == &item.owner_id) {
            agents.push((
                item.owner_id.clone(),
                format!("{} (not enabled / not loaded)", item.owner_id),
            ));
        }
        vec![
            Field::text(
                "Step key",
                "Unique key; dependencies refer to these keys",
                &item.key,
            ),
            Field::select("Assigned agent", agents, Some(&item.owner_id)),
            Field::text(
                "Instructions",
                "What this step should do",
                &item.instructions,
            ),
            Field::text(
                "Success criteria",
                "One criterion per line",
                &item.criteria.join("\n"),
            ),
            Field::text(
                "Prerequisite keys",
                "One key per line; no cycles",
                &item.prerequisites.join("\n"),
            ),
        ]
    }

    pub(super) fn new(
        kind: Kind,
        records: &[OrganizationRecord],
        selected: Option<&OrganizationRecord>,
        model: &str,
    ) -> Self {
        let optional = |mut choices: Vec<(String, String)>| {
            choices.insert(0, (String::new(), "None".into()));
            choices
        };
        let nodes: Vec<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Node(n) => Some((n.id.clone(), n.label.clone())),
                _ => None,
            })
            .collect();
        let agents: Vec<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Node(n)
                    if n.kind == OrganizationNodeKind::Agent && n.enabled =>
                {
                    Some((n.id.clone(), n.label.clone()))
                }
                _ => None,
            })
            .collect();
        let goals: Vec<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Goal(g) => Some((g.id.clone(), g.objective.clone())),
                _ => None,
            })
            .collect();
        let project = selected.and_then(|r| match r {
            OrganizationRecord::Project(p) => Some(p),
            OrganizationRecord::Task(t) => records.iter().find_map(|r| match r {
                OrganizationRecord::Project(p) if p.id == t.project_id => Some(p),
                _ => None,
            }),
            _ => None,
        });
        let goal_id = selected
            .and_then(|r| match r {
                OrganizationRecord::Goal(g) => Some(g.id.as_str()),
                _ => None,
            })
            .or_else(|| project.and_then(|p| p.goal_id.as_deref()));
        let owner = project.and_then(|p| p.owner_id.as_deref());
        let mut fields = match &kind {
            Kind::Handoff {
                task, receivers, ..
            } => vec![
                Field::select(
                    "Receiver",
                    receivers
                        .iter()
                        .map(|node| (node.id.clone(), node.label.clone()))
                        .collect(),
                    Some(&task.owner_id),
                ),
                Field::text(
                    "Owner resolution",
                    "Resolve the blocker; original permissions and remaining limits apply",
                    "",
                ),
            ],
            Kind::Apply { result, .. } => result
                .artifacts
                .iter()
                .map(|artifact| {
                    Field::select(
                        artifact.path.clone(),
                        vec![
                            (artifact.path.clone(), "[x] Apply this file".into()),
                            (String::new(), "[ ] Leave unchanged".into()),
                        ],
                        None,
                    )
                })
                .collect(),
            Kind::Role(previous) => vec![
                Field::text("Role name", "Your own reusable role; no execution permission is granted", previous.as_ref().map(|role|role.name.as_str()).unwrap_or("")),
                Field::text("Role instructions", "Creates an immutable version; bind it to a node separately", previous.as_ref().map(|role|role.instructions.as_str()).unwrap_or("")),
            ],
            Kind::Membership(node) => vec![
                Field::select("Group", records.iter().filter_map(|record|match record {
                    OrganizationRecord::Node(group) if group.kind==OrganizationNodeKind::Group && group.id!=node.id => {
                        let present=records.iter().any(|record|matches!(record,OrganizationRecord::Membership(member) if member.node_id==node.id && member.group_id==group.id));
                        Some((group.id.clone(),format!("{}{}",group.label,if present { " (member)" } else { "" })))
                    },_=>None }).collect(),None),
                Field::select("Membership",vec![("true".into(),"Add / keep membership".into()),("false".into(),"Remove membership".into())],None),
            ],
            Kind::Group | Kind::NodeEdit(OrganizationNode {kind:OrganizationNodeKind::Group,..}) => {
                let node=match &kind { Kind::NodeEdit(node)=>Some(node),_=>None };
                vec![Field::text("Group name","A passive container; agents may have any hierarchy level",node.map(|node|node.label.as_str()).unwrap_or("")),
                    Field::select("Parent",optional(nodes.into_iter().filter(|(id,_)|node.is_none_or(|node|id!=&node.id)).collect()),
                        if let Some(node)=node { node.parent_id.as_deref() } else {selected.and_then(|record|match record { OrganizationRecord::Node(node)=>Some(node.id.as_str()),_=>None })}),
                    Field::select("State",vec![("true".into(),"Enabled".into()),("false".into(),"Retired / disabled (records retained)".into())],Some(if node.is_none_or(|node|node.enabled) { "true" } else { "false" }))]
            },
            Kind::Agent | Kind::NodeEdit(_) => {
                let node=match &kind { Kind::NodeEdit(node)=>Some(node),_=>None };
                vec![
                Field::text("Agent name", "Your own name and hierarchy", node.map(|node|node.label.as_str()).unwrap_or("")),
                Field::select(
                    "Parent",
                    optional(nodes.into_iter().filter(|(id,_)|node.is_none_or(|node|id!=&node.id)).collect()),
                    if let Some(node)=node {node.parent_id.as_deref()} else {selected.and_then(|r| match r {
                        OrganizationRecord::Node(n) => Some(n.id.as_str()),
                        _ => None,
                    })},
                ),
                Field::select(
                    "Role version",
                    optional(
                        records
                            .iter()
                            .filter_map(|r| match r {
                                OrganizationRecord::Role(role) => Some((
                                    role.id.clone(),
                                    format!("{} v{}", role.name, role.version),
                                )),
                                _ => None,
                            })
                            .collect(),
                    ),
                    node.and_then(|node|node.role_version_id.as_deref()),
                ),
                Field::text("Model", "Blank uses the configured default", node.and_then(|node|node.model.as_deref()).unwrap_or("")),
                Field::select("State",vec![("true".into(),"Enabled".into()),("false".into(),"Retired / disabled (records retained)".into())],Some(if node.is_none_or(|node|node.enabled) { "true" } else { "false" })),
            ]},
            Kind::Goal => vec![
                Field::text("Objective", "Describe the desired outcome", ""),
                Field::text("Success criteria", "One criterion per line", ""),
                Field::select("Parent goal", optional(goals), goal_id),
            ],
            Kind::Project => vec![
                Field::text("Project name", "A local coding or document project", ""),
                Field::text(
                    "Project root",
                    "Existing absolute directory; no files are changed",
                    "",
                ),
                Field::select("Related goal", optional(goals), goal_id),
                Field::select("Owner", optional(agents), owner),
                Field::text("Completion criteria", "One criterion per line", ""),
            ],
            Kind::WorkflowSave { items, previous, .. } => {
                let mut fields=vec![Field::text("Workflow name", "Saving starts no work; F7 adds a step, F8 removes the focused step", previous.as_ref().map(|template|template.label.as_str()).unwrap_or(""))];
                for item in items {fields.extend(Self::step_fields(item,&agents));}
                fields
            },
            Kind::Task | Kind::GoalPlan(_) | Kind::Selection {..} => vec![
                Field::select(
                    "Project",
                    records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Project(p) if !matches!(&kind,Kind::Selection {project_id,..} if project_id!=&p.id) => Some((p.id.clone(), p.name.clone())),
                            _ => None,
                        })
                        .collect(),
                    project.map(|p| p.id.as_str()),
                ),
                Field::select("Assigned agent", agents, owner),
                Field::select("Related goal", optional(goals), goal_id),
                Field::text("Instructions", "What should the agent do?", ""),
                Field::text("Acceptance criteria", "One criterion per line", ""),
                Field::text("Prerequisites", "Optional task IDs, one per line", ""),
            ],
            Kind::SharedLimits { previous, .. } => vec![
                Field::text("Shared token ceiling", "Lifetime reported tokens across this project/run; 1–10000000", &previous.as_ref().map(|limit|limit.max_tokens).unwrap_or(10000).to_string()),
                Field::text("Shared model requests", "Lifetime admitted requests; 1–100000, never reset by controls", &previous.as_ref().map(|limit|limit.max_model_requests).unwrap_or(20).to_string()),
                Field::text("Concurrent executions", "1–4; also constrained by the daemon and each grant", &previous.as_ref().map(|limit|limit.max_active).unwrap_or(2).to_string()),
            ],
            Kind::WorkflowRun { template } => template.items.iter().flat_map(|item| {
                let model=records.iter().find_map(|record|match record {OrganizationRecord::Node(node) if node.id==item.owner_id=>node.model.as_deref(),_=>None}).unwrap_or(model);
                let mut fields=Self::scope_fields(model);
                for field in &mut fields {field.label=format!("{} · {}",item.key,field.label);}
                fields
            }).chain([
                Field::text("Shared workflow token ceiling", "Optional; leave all three shared fields blank to retain only fixed step grants", ""),
                Field::text("Shared workflow request ceiling", "Optional aggregate limit for this invocation", ""),
                Field::text("Shared workflow concurrency", "Optional aggregate active limit, 1–4", ""),
            ]).collect(),
            Kind::Authorize(task) | Kind::Schedule(task) => {
                let model = records
                    .iter()
                    .find_map(|r| match r {
                        OrganizationRecord::Node(n) if n.id == task.owner_id => n.model.as_deref(),
                        _ => None,
                    })
                    .unwrap_or(model);
                Self::scope_fields(model)
            }
        };
        if matches!(&kind, Kind::Schedule(_)) {
            let now = chrono::Utc::now().timestamp();
            fields[13] = Field::text(
                "First run (UTC epoch)",
                "Required future time; missed intervals are skipped",
                &(now + 3600).to_string(),
            );
            fields.extend([
                Field::text(
                    "Schedule label",
                    "Name for this finite series",
                    "Recurring task",
                ),
                Field::text(
                    "Interval (seconds)",
                    "10–2678400; daemon polls about every 5 seconds",
                    "3600",
                ),
                Field::text(
                    "End time (UTC epoch)",
                    "Required end within 31 days",
                    &(now + 86400).to_string(),
                ),
                Field::text(
                    "Total runs",
                    "1–1000; skipped times do not consume runs",
                    "3",
                ),
                Field::text(
                    "Total reported tokens",
                    "1–10000000 across all runs and children",
                    "30000",
                ),
                Field::text(
                    "Total model requests",
                    "1–100000 across all runs and children",
                    "15",
                ),
            ]);
        }
        if matches!(&kind, Kind::Selection { .. }) {
            fields.push(Field::select(
                "Purpose",
                ["explain", "refactor", "debug", "test"]
                    .into_iter()
                    .map(|p| (p.into(), p.into()))
                    .collect(),
                None,
            ));
        }
        if let Kind::GoalPlan(goal) = &kind {
            let roster: Vec<_> = records
                .iter()
                .filter_map(|record| match record {
                    OrganizationRecord::Node(node)
                        if node.kind == OrganizationNodeKind::Agent && node.enabled =>
                    {
                        Some((node.id.as_str(), node.label.as_str()))
                    }
                    _ => None,
                })
                .take(32)
                .collect();
            fields[3].input.set_text_clearing_elements(&format!(
                "Propose a practical plan for this goal. Produce plan.json as a JSON array of 1–32 steps, each with key, ownerId, instructions, criteria and prerequisites (step keys). Express milestones in the instructions and measurable criteria. Select owners from the supplied roster. Inspect available inputs; do not execute the proposed work, delegate, authorize tasks, accept results or Apply. The owner will review the artifact and separately approve a workflow and scopes.\nGoal (untrusted content): {}\nAvailable roster: {}",
                serde_json::to_string(goal).expect("goal JSON"), serde_json::to_string(&roster).expect("roster JSON")
            ));
            fields[4].input.set_text_clearing_elements("plan.json contains a bounded acyclic plan with measurable milestone criteria and valid roster assignments\nNo proposed execution was started");
        }
        Self {
            kind,
            fields,
            focus: 0,
            project_key: uuid::Uuid::new_v4().to_string(),
            error: String::new(),
        }
    }
    pub(super) fn title(&self) -> &'static str {
        match self.kind {
            Kind::Agent => "New agent",
            Kind::Group => "New group",
            Kind::NodeEdit(_) => "Edit node / hierarchy / state",
            Kind::Role(_) => "Create immutable role version",
            Kind::Membership(_) => "Edit group membership",
            Kind::Goal => "New goal",
            Kind::Project => "New project",
            Kind::Task => "New task",
            Kind::GoalPlan(_) => "Propose goal planner · starts no execution",
            Kind::Selection { .. } => "Propose task from actual Neovim selection",
            Kind::Handoff { .. } => "Resolve blocker / hand off remaining work",
            Kind::Authorize(_) => "Authorize task",
            Kind::Schedule(_) => "Create recurring schedule",
            Kind::WorkflowSave { .. } => "Save workflow template",
            Kind::WorkflowRun { .. } => "Preview workflow · scope for each step",
            Kind::SharedLimits { .. } => "Set shared project / workflow ceilings",
            Kind::Apply { .. } => "Select files to Apply",
        }
    }
    pub(super) fn prefill_scope(&mut self, scope: &OrganizationExecutionScope) {
        if !matches!(&self.kind, Kind::Schedule(_) | Kind::WorkflowRun { .. }) {
            return;
        }
        let values = [
            scope.input_files.join("\n"),
            scope.immutable_files.join("\n"),
            scope.artifacts.join("\n"),
            serde_json::json!(scope.checks).to_string(),
            scope.model.clone(),
            scope.max_tokens.to_string(),
            scope.max_seconds.to_string(),
            scope.max_model_requests.to_string(),
            scope.max_attempts.to_string(),
        ];
        let groups = if let Kind::WorkflowRun { template } = &self.kind {
            template.items.len()
        } else {
            1
        };
        for start in (0..groups).map(|index| index * 14) {
            for (field, value) in self.fields[start..].iter_mut().zip(&values) {
                field.input.set_text_clearing_elements(value);
            }
            self.fields[start + 9].choice = usize::from(!scope.require_tool_approval);
            self.fields[start + 10]
                .input
                .set_text_clearing_elements(&scope.delegates.join("\n"));
            self.fields[start + 11]
                .input
                .set_text_clearing_elements(&scope.max_children.to_string());
            self.fields[start + 12].choice = usize::from(scope.auto_accept_children);
        }
    }
    pub(super) fn key(&mut self, key: KeyEvent) {
        if matches!(&self.kind, Kind::WorkflowSave { .. }) {
            if key.code == KeyCode::F(7) {
                if self.fields.len() < 1 + 32 * 5 {
                    let agents = self.fields[2].choices.clone();
                    self.focus = self.fields.len();
                    self.fields.extend(Self::step_fields(
                        &OrganizationPlanItem {
                            key: String::new(),
                            owner_id: String::new(),
                            instructions: String::new(),
                            criteria: vec![],
                            prerequisites: vec![],
                        },
                        &agents,
                    ));
                } else {
                    self.error = "Workflows contain at most 32 steps".into();
                }
                return;
            }
            if key.code == KeyCode::F(8) {
                if self.focus > 0 && self.fields.len() > 6 {
                    let start = 1 + (self.focus - 1) / 5 * 5;
                    self.fields.drain(start..start + 5);
                    self.focus = start.min(self.fields.len() - 1);
                } else {
                    self.error = "Keep at least one step; focus the step to remove".into();
                }
                return;
            }
        }
        match key.code {
            KeyCode::Tab => self.focus = (self.focus + 1) % self.fields.len(),
            KeyCode::BackTab => {
                self.focus = (self.focus + self.fields.len() - 1) % self.fields.len()
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                self.focus = (self.focus + 1) % self.fields.len()
            }
            _ => {
                let field = &mut self.fields[self.focus];
                if !field.choices.is_empty() {
                    match key.code {
                        KeyCode::Left | KeyCode::Up => {
                            field.choice =
                                (field.choice + field.choices.len() - 1) % field.choices.len()
                        }
                        KeyCode::Right | KeyCode::Down | KeyCode::Char(' ') => {
                            field.choice = (field.choice + 1) % field.choices.len()
                        }
                        _ => {}
                    }
                } else if field.input.text().len() < 64 * 1024
                    || matches!(key.code, KeyCode::Backspace | KeyCode::Delete)
                {
                    // Modified Enter inserts a line; ordinary Enter advances the form.
                    field.input.input(if key.code == KeyCode::Enter {
                        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
                    } else {
                        key
                    });
                }
            }
        }
    }
    pub(super) fn paste(&mut self, value: &str) {
        let field = &mut self.fields[self.focus];
        if field.choices.is_empty() {
            let value: String = value
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .collect();
            if field.input.text().len() + value.len() <= 64 * 1024 {
                field.input.insert_str_at(field.input.cursor(), &value);
            }
        }
    }
    fn scope(&self, start: usize) -> anyhow::Result<OrganizationExecutionScope> {
        let value = |i: usize| self.fields[start + i].value().to_string();
        let required = |i: usize| -> anyhow::Result<String> {
            let text = value(i);
            anyhow::ensure!(
                !text.is_empty(),
                "{} is required",
                self.fields[start + i].label
            );
            Ok(text)
        };
        let optional = |i: usize| (!value(i).is_empty()).then(|| value(i));
        let lines = |i: usize| {
            value(i)
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let criteria = |i| -> anyhow::Result<Vec<String>> {
            required(i)?;
            Ok(lines(i))
        };
        let number = |i: usize| -> anyhow::Result<u32> {
            value(i).parse().map_err(|_| {
                anyhow::anyhow!(
                    "{} must be a positive integer",
                    self.fields[start + i].label
                )
            })
        };
        let scope = OrganizationExecutionScope {
            input_files: lines(0),
            immutable_files: lines(1),
            artifacts: criteria(2)?,
            checks: serde_json::from_str(&value(3))
                .map_err(|e| anyhow::anyhow!("Checks must be an array of argv arrays: {e}"))?,
            model: required(4)?,
            max_tokens: i64::from(number(5)?),
            max_seconds: number(6)?,
            max_model_requests: number(7)?,
            max_attempts: number(8)?,
            require_tool_approval: value(9) == "true",
            delegates: lines(10),
            max_children: number(11)?,
            auto_accept_children: value(12) == "true",
            not_before: optional(13).map(|s| s.parse::<i64>()).transpose()?,
        };
        anyhow::ensure!(
            (1..=10_000_000).contains(&scope.max_tokens)
                && (1..=3600).contains(&scope.max_seconds)
                && (1..=100).contains(&scope.max_model_requests)
                && (1..=3).contains(&scope.max_attempts)
                && scope.max_children <= 32,
            "Budget is outside the supported limits"
        );
        for paths in [&scope.input_files, &scope.immutable_files, &scope.artifacts] {
            let mut unique = BTreeSet::new();
            for path in paths {
                anyhow::ensure!(
                    Path::new(path)
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_)))
                        && !path.contains('\0')
                        && unique.insert(path),
                    "File paths must be unique, relative and without traversal"
                );
            }
        }
        anyhow::ensure!(
            scope
                .immutable_files
                .iter()
                .all(|p| scope.input_files.contains(p)),
            "Immutable files must also be inputs"
        );
        anyhow::ensure!(
            !scope.auto_accept_children || !scope.checks.is_empty(),
            "Checked child acceptance requires declared checks"
        );
        Ok(scope)
    }
    pub(super) fn selection(&self) -> Option<&crate::organization_editor::Selection> {
        if let Kind::Selection { selection, .. } = &self.kind {
            Some(selection)
        } else {
            None
        }
    }
    pub(super) fn command(&self) -> anyhow::Result<Command> {
        let value = |i: usize| self.fields[i].value().to_string();
        let required = |i: usize| -> anyhow::Result<String> {
            let text = value(i);
            anyhow::ensure!(!text.is_empty(), "{} is required", self.fields[i].label);
            Ok(text)
        };
        let optional = |i: usize| (!value(i).is_empty()).then(|| value(i));
        let lines = |i: usize| {
            value(i)
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let criteria = |i| -> anyhow::Result<Vec<String>> {
            required(i)?;
            Ok(lines(i))
        };
        Ok(match &self.kind {
            Kind::Handoff {
                task,
                checkpoint,
                receivers,
            } => {
                let owner_id = required(0)?;
                let receiver = receivers
                    .iter()
                    .find(|node| node.id == owner_id)
                    .ok_or_else(|| anyhow::anyhow!("Captured receiver missing"))?;
                let instructions = required(1)?;
                anyhow::ensure!(instructions.len() <= 4096, "Resolution exceeds 4096 bytes");
                Command::Execution(OrganizationExecutionMutation::Handoff {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    checkpoint_id: checkpoint.id.clone(),
                    owner_id,
                    expected_owner_revision: receiver.revision,
                    instructions,
                })
            }
            Kind::Apply { task, result } => {
                anyhow::ensure!(result.accepted, "Result must be accepted before Apply");
                let paths = self
                    .fields
                    .iter()
                    .filter(|field| !field.value().is_empty())
                    .map(|field| field.value().to_string())
                    .collect::<Vec<_>>();
                Command::Execution(OrganizationExecutionMutation::Apply {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    digest: result.digest.clone(),
                    paths: Some(
                        result
                            .application_paths(Some(&paths))
                            .map_err(anyhow::Error::msg)?,
                    ),
                })
            }
            Kind::Role(previous) => Command::Definition(OrganizationMutation::CreateRoleVersion {
                role_id: previous.as_ref().map(|role| role.role_id.clone()),
                expected_version: previous.as_ref().map(|role| role.version),
                name: required(0)?,
                instructions: required(1)?,
            }),
            Kind::Membership(node) => Command::Definition(OrganizationMutation::SetMembership {
                node_id: node.id.clone(),
                group_id: required(0)?,
                present: value(1) == "true",
            }),
            Kind::Group
            | Kind::NodeEdit(OrganizationNode {
                kind: OrganizationNodeKind::Group,
                ..
            }) => {
                let node = match &self.kind {
                    Kind::NodeEdit(node) => Some(node),
                    _ => None,
                };
                Command::Definition(OrganizationMutation::PutNode {
                    id: node.map(|node| node.id.clone()),
                    expected_revision: node.map(|node| node.revision),
                    kind: OrganizationNodeKind::Group,
                    label: required(0)?,
                    parent_id: optional(1),
                    role_version_id: None,
                    model: None,
                    enabled: value(2) == "true",
                })
            }
            Kind::Agent | Kind::NodeEdit(_) => {
                let node = match &self.kind {
                    Kind::NodeEdit(node) => Some(node),
                    _ => None,
                };
                Command::Definition(OrganizationMutation::PutNode {
                    id: node.map(|node| node.id.clone()),
                    expected_revision: node.map(|node| node.revision),
                    parent_id: optional(1),
                    kind: OrganizationNodeKind::Agent,
                    label: required(0)?,
                    role_version_id: optional(2),
                    model: optional(3),
                    enabled: value(4) == "true",
                })
            }
            Kind::Goal => Command::Definition(OrganizationMutation::PutGoal {
                id: None,
                expected_revision: None,
                parent_id: optional(2),
                objective: required(0)?,
                criteria: criteria(1)?,
            }),
            Kind::Project => {
                let root = required(1)?;
                anyhow::ensure!(
                    Path::new(&root).is_absolute(),
                    "Project root must be absolute"
                );
                let root = std::fs::canonicalize(root)?;
                anyhow::ensure!(root.is_dir(), "Project root must be an existing directory");
                Command::Project {
                    params: ProjectCreateParams {
                        name: required(0)?,
                        roots: vec![ProjectRoot {
                            path: codex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(
                                root,
                            )?,
                        }],
                        metadata: None,
                        idempotency_key: self.project_key.clone(),
                    },
                    goal_id: optional(2),
                    owner_id: optional(3),
                    criteria: criteria(4)?,
                }
            }
            Kind::WorkflowSave {
                project_id,
                previous,
                ..
            } => {
                let items = (1..self.fields.len())
                    .step_by(5)
                    .map(|start| {
                        Ok(OrganizationPlanItem {
                            key: required(start)?,
                            owner_id: required(start + 1)?,
                            instructions: required(start + 2)?,
                            criteria: criteria(start + 3)?,
                            prerequisites: lines(start + 4),
                        })
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Command::Workflow(OrganizationWorkflowMutation::Put {
                    id: previous
                        .as_ref()
                        .map(|template| template.id.clone())
                        .unwrap_or_else(|| self.project_key.clone()),
                    expected_revision: previous.as_ref().map(|template| template.revision),
                    project_id: project_id.clone(),
                    label: required(0)?,
                    items,
                })
            }
            Kind::SharedLimits {
                project_id,
                run_id,
                previous,
            } => Command::Workflow(OrganizationWorkflowMutation::SetLimits {
                project_id: project_id.clone(),
                run_id: run_id.clone(),
                expected_revision: previous.as_ref().map(|limit| limit.revision),
                max_tokens: required(0)?.parse()?,
                max_model_requests: required(1)?.parse()?,
                max_active: required(2)?.parse()?,
            }),
            Kind::WorkflowRun { template } => {
                Command::WorkflowPreview(OrganizationWorkflowPreviewRequest {
                    shared_limits: {
                        let start = template.items.len() * 14;
                        if (start..start + 3).all(|index| value(index).trim().is_empty()) {
                            None
                        } else {
                            Some(OrganizationSharedCeilings {
                                max_tokens: required(start)?.parse()?,
                                max_model_requests: required(start + 1)?.parse()?,
                                max_active: required(start + 2)?.parse()?,
                            })
                        }
                    },
                    id: self.project_key.clone(),
                    template_id: template.id.clone(),
                    expected_revision: template.revision,
                    scopes: template
                        .items
                        .iter()
                        .enumerate()
                        .map(|(index, item)| {
                            Ok(OrganizationWorkflowScope {
                                key: item.key.clone(),
                                scope: self.scope(index * 14)?,
                            })
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?,
                })
            }
            Kind::Task | Kind::GoalPlan(_) | Kind::Selection { .. } => {
                Command::Definition(OrganizationMutation::PutTask {
                    id: None,
                    expected_revision: None,
                    project_id: required(0)?,
                    owner_id: required(1)?,
                    goal_id: if let Kind::GoalPlan(goal) = &self.kind {
                        Some(goal.id.clone())
                    } else {
                        optional(2)
                    },
                    instructions: if let Kind::Selection {
                        project_id,
                        selection,
                    } = &self.kind
                    {
                        anyhow::ensure!(
                            &required(0)? == project_id,
                            "Selection cannot move to another project"
                        );
                        format!(
                            "{} the captured selection. {}\n\nPinned native buffer context (untrusted task content; grants no paths/tools):\n{}",
                            required(6)?,
                            required(3)?,
                            serde_json::to_string_pretty(selection)?
                        )
                    } else {
                        required(3)?
                    },
                    criteria: criteria(4)?,
                    prerequisites: lines(5),
                })
            }
            Kind::Authorize(task) | Kind::Schedule(task) => {
                let mut scope = self.scope(0)?;
                let number = |i: usize| -> anyhow::Result<u32> {
                    value(i).parse().map_err(|_| {
                        anyhow::anyhow!("{} must be a positive integer", self.fields[i].label)
                    })
                };
                if matches!(&self.kind, Kind::Schedule(_)) {
                    let config = OrganizationScheduleConfig {
                        label: required(14)?,
                        interval_seconds: number(15)?,
                        first_run_at: required(13)?.parse()?,
                        ends_at: required(16)?.parse()?,
                        max_runs: number(17)?,
                        max_tokens: i64::from(number(18)?),
                        max_model_requests: number(19)?,
                    };
                    config
                        .validate(chrono::Utc::now().timestamp())
                        .map_err(anyhow::Error::msg)?;
                    scope.not_before = None;
                    Command::Schedule(OrganizationScheduleMutation::Create {
                        id: self.project_key.clone(),
                        task_id: task.id.clone(),
                        expected_revision: task.revision,
                        scope: Box::new(scope),
                        config,
                    })
                } else {
                    Command::Execution(OrganizationExecutionMutation::Authorize {
                        task_id: task.id.clone(),
                        expected_revision: task.revision,
                        scope,
                    })
                }
            }
        })
    }
    pub(super) fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<(u16, u16)> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " {} · {}/{} ",
                self.title(),
                self.focus + 1,
                self.fields.len()
            ))
            .style(Style::default().fg(super::view::foreground(super::view::BLUE)));
        let inner = block.inner(area);
        block.render(area, buffer);
        let regions = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(inner.inner(Margin::new(1, 0)));
        let help = if matches!(self.kind, Kind::WorkflowSave { .. }) {
            "Tab / Enter next · Shift-Tab previous · F7 add step · F8 remove step\nAlt-Enter newline · F6 review exact template · Esc close"
        } else if matches!(self.kind, Kind::Apply { .. }) {
            "Tab / Enter next · Shift-Tab previous · Arrows / Space toggle\nF6 review exact selection · Esc close (no changes)"
        } else {
            "Tab / Enter next · Shift-Tab previous · Left / Right choose\nAlt-Enter newline · F6 review · Esc close (no changes)"
        };
        Paragraph::new(help).render(regions[0], buffer);
        let height = 4;
        let visible = usize::from(regions[1].height / height).max(1);
        let start = self.focus.saturating_sub(visible - 1);
        let mut cursor = None;
        for (row, field) in self.fields.iter().skip(start).take(visible).enumerate() {
            let y = regions[1].y + row as u16 * height;
            let area = Rect::new(
                regions[1].x,
                y,
                regions[1].width,
                height.min(regions[1].bottom().saturating_sub(y)),
            );
            let focused = start + row == self.focus;
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " {}{} ",
                    if focused { "› " } else { "" },
                    field.label
                ))
                .border_style(Style::default().fg(if focused {
                    super::view::foreground(super::view::BLUE)
                } else {
                    Color::DarkGray
                }));
            let inner = block.inner(area);
            block.render(area, buffer);
            if let Some((_, label)) = field.choices.get(field.choice) {
                Paragraph::new(clean(&format!("‹ {label} ›"))).render(inner, buffer);
            } else {
                let mut state = TextAreaState::default();
                field.input.render_ref_styled_with_highlights(
                    inner,
                    buffer,
                    &mut state,
                    Style::default().fg(Color::White),
                    &[],
                );
                if focused {
                    cursor = field.input.cursor_pos_with_state(inner, state);
                }
            }
        }
        let hint = if matches!(self.kind, Kind::Apply { .. }) {
            format!(
                "{} / {} files selected · One Apply per accepted result\nFull result checks do not certify the installed subset",
                self.fields.iter().filter(|f| !f.value().is_empty()).count(),
                self.fields.len()
            )
        } else {
            self.fields[self.focus].hint.to_string()
        };
        Paragraph::new(clean(&format!("{hint}\n{}", self.error)))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: false })
            .render(regions[2], buffer);
        cursor
    }
}
