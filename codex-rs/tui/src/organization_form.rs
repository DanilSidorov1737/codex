//! Fixed owner forms reuse the existing textarea and typed administration contracts.
use super::*;
use crate::bottom_pane::TextArea;
use crate::bottom_pane::TextAreaState;
use crossterm::event::KeyModifiers;
use ratatui::layout::Margin;

pub(super) enum Kind {
    GoalMetadata {
        goal: OrganizationGoal,
        previous: Option<OrganizationControl>,
        candidates: Vec<OrganizationGoalDependency>,
    },
    Deadline {
        kind: OrganizationDeadlineKind,
        target_id: String,
        target_revision: i64,
        previous: Option<OrganizationControl>,
    },
    Conversation {
        previous: Option<OrganizationControl>,
        agent_id: Option<String>,
    },
    Setup,
    ResourcePressure(Option<OrganizationControl>),
    ExecutionProfile(Option<OrganizationControl>),
    AuthorizeProfile {
        task: OrganizationTask,
        profiles: Vec<OrganizationControl>,
    },
    Reconciliation {
        attempt: OrganizationTaskAttempt,
        fingerprint: String,
    },
    RoleRollback {
        current: OrganizationRoleVersion,
        versions: Vec<OrganizationRoleVersion>,
    },
    WorkflowRollback {
        current: OrganizationWorkflowTemplate,
        runs: Vec<OrganizationWorkflowRun>,
    },
    ArtifactLibrary(String),
    Mission(Option<OrganizationControl>),
    GoalReview {
        goal: OrganizationGoal,
        previous: Option<OrganizationControl>,
        accepted: Vec<OrganizationTaskReport>,
    },
    Escalation {
        task: OrganizationTask,
        previous: Option<OrganizationControl>,
    },
    Lesson {
        task: OrganizationTask,
        digest: String,
    },
    Playbook(OrganizationProjectPlan),
    NamedView(Vec<String>),
    RestoreEditor(Box<crate::organization_editor::Session>),
    HunkApply {
        task: OrganizationTask,
        result: OrganizationTaskResult,
        files: Vec<HunkFile>,
    },
    Agent,
    Group,
    NodeEdit(OrganizationNode),
    Role(Option<OrganizationRoleVersion>),
    Membership(OrganizationNode),
    PresetSave(OrganizationNode),
    PresetClone(Box<OrganizationPreset>),
    GlobalControl(Option<OrganizationControl>),
    SupervisionControl(OrganizationControl),
    Priority {
        task: OrganizationTask,
        previous: Option<OrganizationControl>,
    },
    ProjectControl {
        project: OrganizationProjectPlan,
        previous: Option<OrganizationControl>,
        accepted: Vec<OrganizationTaskReport>,
    },
    Goal,
    GoalEdit(OrganizationGoal),
    Project,
    Task,
    GoalPlan(OrganizationGoal),
    ProjectPlanning(OrganizationProjectPlan),
    Discovery(OrganizationProjectPlan),
    WorkflowImprovement {
        template: OrganizationWorkflowTemplate,
        runs: Vec<OrganizationWorkflowRun>,
    },
    Team {
        expected_nodes: Vec<OrganizationNode>,
        nodes: Vec<OrganizationNode>,
    },
    Recovery {
        task: OrganizationTask,
        authorization_id: String,
        previous: Option<OrganizationControl>,
    },
    ShareNote {
        note: OrganizationNote,
        projects: Vec<OrganizationProjectPlan>,
    },
    Improvement {
        node: OrganizationNode,
        accepted: Vec<OrganizationTaskReport>,
    },
    Discussion,
    Knowledge {
        note_id: String,
        body_sha256: String,
        previous: Option<OrganizationControl>,
    },
    PromoteNote(OrganizationNote),
    Selection {
        project_id: String,
        selection: crate::organization_editor::Selection,
    },
    Handoff {
        task: OrganizationTask,
        checkpoint: OrganizationTaskCheckpoint,
        receivers: Vec<OrganizationNode>,
    },
    Replan {
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
    WorkflowSeries {
        template: OrganizationWorkflowTemplate,
        goal: Option<OrganizationGoal>,
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

pub(super) struct HunkFile {
    pub(super) path: String,
    pub(super) base: String,
    pub(super) artifact: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_planning_request_creates_reviewable_work_without_execution_authority() {
        let project = OrganizationProjectPlan {
            id: "p".into(),
            name: "Handbook".into(),
            roots: vec![],
            goal_id: None,
            owner_id: Some("a".into()),
            criteria: vec![],
            revision: 3,
        };
        let agent:OrganizationRecord=serde_json::from_value(serde_json::json!({"type":"node","data":{"id":"a","parentId":null,"kind":"agent","label":"Planner","roleVersionId":null,"model":"fixture","enabled":true,"revision":1}})).unwrap();
        let records = vec![OrganizationRecord::Project(project.clone()), agent];
        let mut form = Form::new(
            Kind::ProjectPlanning(project),
            &records,
            Some(&records[0]),
            "fixture",
        );
        assert!(form.command().is_err());
        set(
            &mut form,
            3,
            "Create a handbook plan with two reviewed milestones",
        );
        let Command::Definition(OrganizationMutation::PutTask {
            id: None,
            project_id,
            instructions,
            ..
        }) = form.command().unwrap()
        else {
            panic!("proposal only")
        };
        assert_eq!(project_id, "p");
        assert!(
            instructions.contains("plan.json")
                && instructions.contains("handbook plan")
                && instructions.contains("revision 3")
                && instructions.contains("separately authorizes")
        );
    }

    #[test]
    fn local_business_date_roundtrips_offset_and_pins_target_without_a_grant() {
        let mut form = Form::new(
            Kind::Deadline {
                kind: OrganizationDeadlineKind::Task,
                target_id: "task".into(),
                target_revision: 9,
                previous: None,
            },
            &[],
            None,
            "fixture",
        );
        set(&mut form, 0, "2026-10-20T17:00:00-04:00");
        let Command::Definition(OrganizationMutation::SetControl {
            value,
            expected_revision: None,
        }) = form.command().unwrap()
        else {
            panic!("date metadata only")
        };
        assert!(
            matches!(&value,OrganizationControlValue::Deadline {kind:OrganizationDeadlineKind::Task,target_id,target_revision:9,due_at:Some(_),display_offset_minutes:-240} if target_id=="task")
        );
        let previous = OrganizationControl {
            id: "deadline:task:task".into(),
            revision: 1,
            value,
        };
        let restored = Form::new(
            Kind::Deadline {
                kind: OrganizationDeadlineKind::Task,
                target_id: "task".into(),
                target_revision: 10,
                previous: Some(previous),
            },
            &[],
            None,
            "fixture",
        );
        assert_eq!(restored.fields[0].value(), "2026-10-20T17:00:00-04:00");
        set(&mut form, 0, "2026-10-20T17:00:00");
        assert!(form.command().is_err(), "local date needs explicit offset");
        set(&mut form, 0, "");
        assert!(matches!(
            form.command().unwrap(),
            Command::Definition(OrganizationMutation::SetControl {
                value: OrganizationControlValue::Deadline { due_at: None, .. },
                ..
            })
        ));
    }

    #[test]
    fn conversation_form_keeps_stable_identity_and_creates_only_a_draft() {
        let project = OrganizationRecord::Project(OrganizationProjectPlan {
            id: "p".into(),
            name: "Local".into(),
            roots: vec![],
            goal_id: None,
            owner_id: Some("a".into()),
            criteria: vec![],
            revision: 1,
        });
        let agent:OrganizationRecord=serde_json::from_value(serde_json::json!({"type":"node","data":{"id":"a","parentId":null,"kind":"agent","label":"CEO","roleVersionId":null,"model":"fixture","enabled":true,"revision":1}})).unwrap();
        let previous = OrganizationControl {
            id: "conversation:c".into(),
            revision: 4,
            value: OrganizationControlValue::Conversation {
                id: "c".into(),
                project_id: "p".into(),
                agent_id: "a".into(),
                topic: "Planning".into(),
                entries: vec![],
            },
        };
        let mut form = Form::new(
            Kind::Conversation {
                previous: Some(previous),
                agent_id: None,
            },
            &[project, agent],
            None,
            "fixture",
        );
        assert!(form.command().is_err());
        set(&mut form, 3, "Propose the next bounded project milestone");
        assert!(
            matches!(form.command().unwrap(),Command::Definition(OrganizationMutation::ConversationPrompt {id:Some(id),expected_revision:Some(4),project_id,agent_id,topic,..}) if id=="c" && project_id=="p" && agent_id=="a" && topic=="Planning")
        );
    }

    #[test]
    fn setup_requires_explicit_fields_and_only_builds_paused_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let mut form = Form::new(Kind::Setup, &[], None, "fixture");
        assert!(form.command().is_err());
        for (index, value) in [
            (0, "Workspace"),
            (1, directory.path().to_str().unwrap()),
            (2, "My agent"),
            (3, "Review local sources"),
        ] {
            set(&mut form, index, value);
        }
        let Command::Setup {
            params,
            agent,
            model,
            limits,
            ..
        } = form.command().unwrap()
        else {
            panic!("setup must not authorize inference")
        };
        assert_eq!(agent, "My agent");
        assert_eq!(model, "fixture");
        assert_eq!(limits.max_active, 2);
        let Command::Setup {
            params: repeated, ..
        } = form.command().unwrap()
        else {
            unreachable!()
        };
        assert_eq!(params.idempotency_key, repeated.idempotency_key);
        set(&mut form, 7, "5");
        assert!(form.command().is_err());
    }

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
    fn optional_playbooks_use_arbitrary_workers_and_distinct_reviewers_without_grants() {
        let project = OrganizationProjectPlan {
            id: "project".into(),
            name: "Local".into(),
            roots: vec!["/tmp/project".into()],
            goal_id: None,
            owner_id: Some("worker".into()),
            criteria: vec![],
            revision: 1,
        };
        let records: Vec<_> = ["worker", "reviewer"]
            .into_iter()
            .map(|id| {
                OrganizationRecord::Node(OrganizationNode {
                    id: id.into(),
                    label: format!("Arbitrary {id}"),
                    parent_id: None,
                    kind: OrganizationNodeKind::Agent,
                    role_version_id: None,
                    model: None,
                    enabled: true,
                    revision: 1,
                })
            })
            .collect();
        let mut form = Form::new(Kind::Playbook(project), &records, None, "model");
        assert!(form.command().is_err(), "objective required");
        set(&mut form, 2, "Review permitted local facts");
        for mode in 0..4 {
            form.fields[0].choice = mode;
            form.fields[4].choice = 2;
            let Command::Workflow(OrganizationWorkflowMutation::Put {
                items,
                expected_revision,
                ..
            }) = form.command().unwrap()
            else {
                panic!("editable template only")
            };
            assert_eq!(expected_revision, None);
            assert_eq!(items[0].owner_id, "worker");
            assert_eq!(items[1].owner_id, "reviewer");
            assert_eq!(items[1].prerequisites, vec!["produce"]);
            assert!(items[1].instructions.contains("disagreements"));
            form.fields[4].choice = 1;
            assert!(form.command().is_err(), "self review refuses");
        }
        form.fields[0].choice = 2;
        form.fields[4].choice = 0;
        assert!(form.command().is_err(), "audit requires independent review");
        form.fields[0].choice = 0;
        assert!(
            form.command().is_ok(),
            "single-agent coding retains owner review"
        );
    }

    #[test]
    fn mission_and_lesson_forms_pin_owner_versions_and_accepted_sources() {
        let previous = OrganizationControl {
            id: "mission".into(),
            revision: 7,
            value: OrganizationControlValue::Mission {
                purpose: "Prior purpose".into(),
                priorities: vec!["Evidence".into()],
                constraints: vec!["Exact grants".into()],
            },
        };
        let mut mission = Form::new(Kind::Mission(Some(previous)), &[], None, "model");
        set(&mut mission, 0, "New owner purpose");
        let Command::Definition(OrganizationMutation::SetControl {
            expected_revision,
            value: OrganizationControlValue::Mission { purpose, .. },
        }) = mission.command().unwrap()
        else {
            panic!("intent only")
        };
        assert_eq!(expected_revision, Some(7));
        assert_eq!(purpose, "New owner purpose");
        let mut accepted = task();
        accepted.state = "accepted".into();
        let mut lesson = Form::new(
            Kind::Lesson {
                task: accepted,
                digest: "exact-result".into(),
            },
            &[],
            None,
            "model",
        );
        assert!(lesson.command().is_err());
        set(&mut lesson, 0, "Review this lesson before sharing");
        let Command::Definition(OrganizationMutation::ProposeLesson {
            task_id,
            expected_revision,
            result_digest,
            ..
        }) = lesson.command().unwrap()
        else {
            panic!("proposal only")
        };
        assert_eq!(
            (task_id, expected_revision, result_digest),
            ("original-task".into(), 7, "exact-result".into())
        );
    }

    #[test]
    fn discussion_and_knowledge_forms_capture_scope_and_start_no_execution() {
        let agent = OrganizationNode {
            id: "agent".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Arbitrary contributor".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 1,
        };
        let project = OrganizationProjectPlan {
            id: "project".into(),
            name: "Local".into(),
            roots: vec!["/tmp/project".into()],
            goal_id: None,
            owner_id: Some("agent".into()),
            criteria: vec![],
            revision: 1,
        };
        let records = vec![
            OrganizationRecord::Node(agent),
            OrganizationRecord::Project(project),
        ];
        let mut form = Form::new(Kind::Discussion, &records, Some(&records[1]), "model");
        set(&mut form, 1, "Reviewed debate");
        set(&mut form, 2, "Retain dissent");
        let Command::Workflow(OrganizationWorkflowMutation::PutDiscussion { discussion, .. }) =
            form.command().unwrap()
        else {
            panic!("template only")
        };
        assert_eq!(discussion.participants, vec!["agent"]);
        assert_eq!(discussion.plan_items().unwrap().len(), 2);
        form.fields[6].choice = 1;
        assert!(
            form.command().is_err(),
            "duplicate participants refuse before saving"
        );
        let mut knowledge = Form::new(
            Kind::Knowledge {
                note_id: "private-note".into(),
                body_sha256: "a".repeat(64),
                previous: Some(OrganizationControl {
                    id: "knowledge:private-note".into(),
                    revision: 8,
                    value: OrganizationControlValue::Knowledge {
                        note_id: "private-note".into(),
                        body_sha256: "a".repeat(64),
                        retired: true,
                        reason: "prior".into(),
                    },
                }),
            },
            &[],
            None,
            "model",
        );
        assert!(knowledge.command().is_err());
        set(&mut knowledge, 1, "Reviewed correction");
        knowledge.fields[0].choice = 1;
        assert!(
            matches!(knowledge.command().unwrap(),Command::Definition(OrganizationMutation::SetControl {expected_revision:Some(8),value:OrganizationControlValue::Knowledge {note_id,retired:false,..}}) if note_id=="private-note")
        );
    }

    #[test]
    fn hunk_form_pins_exact_result_indices_and_output_without_loading_later_state() {
        let middle = (0..30)
            .map(|i| format!("context {i}\n"))
            .collect::<String>();
        let base = format!("old first\n{middle}old last\n");
        let artifact = format!("new first\n{middle}new last\n");
        let result = OrganizationTaskResult {
            attempt_id: "attempt".into(),
            artifacts: vec![OrganizationArtifactEvidence {
                path: "file.txt".into(),
                sha256: "a".repeat(64),
                bytes: artifact.len() as u64,
            }],
            artifact_bases: vec![],
            checks: vec![],
            digest: "pinned-result".into(),
            accepted: true,
        };
        let mut form = Form::new(
            Kind::HunkApply {
                task: task(),
                result,
                files: vec![HunkFile {
                    path: "file.txt".into(),
                    base: base.clone(),
                    artifact: artifact.clone(),
                }],
            },
            &[],
            None,
            "model",
        );
        assert!(form.command().is_err());
        form.key(KeyCode::Char(' ').into());
        let Command::Execution(OrganizationExecutionMutation::Apply {
            task_id,
            expected_revision: 7,
            digest,
            hunks: Some(hunks),
            ..
        }) = form.command().unwrap()
        else {
            panic!("hunk apply")
        };
        assert_eq!(task_id, "original-task");
        assert_eq!(digest, "pinned-result");
        assert_eq!(hunks[0].indices, vec![0]);
        use sha2::Digest;
        use sha2::Sha256;
        assert_eq!(
            hunks[0].output_sha256,
            format!(
                "{:x}",
                Sha256::digest(format!("new first\n{middle}old last\n").as_bytes())
            )
        );
    }

    #[test]
    fn finite_workflow_form_captures_series_and_needs_exact_source_with_one_run() {
        let template = OrganizationWorkflowTemplate {
            discussion: None,
            id: "template".into(),
            project_id: "project".into(),
            label: "A workflow".into(),
            revision: 4,
            items: vec![OrganizationPlanItem {
                key: "step".into(),
                owner_id: "agent".into(),
                instructions: "Write result".into(),
                criteria: vec!["Check passes".into()],
                prerequisites: vec![],
            }],
        };
        let records = vec![OrganizationRecord::Task(task())];
        let mut form = Form::new(
            Kind::WorkflowSeries {
                template,
                goal: None,
            },
            &records,
            None,
            "model",
        );
        set(&mut form, 0, "result.txt");
        set(&mut form, 2, "result.txt");
        let Command::SeriesPreview {
            request,
            config,
            source_task,
            ..
        } = form.command().unwrap()
        else {
            panic!("series must only preview")
        };
        assert_eq!(request.expected_revision, 4);
        assert_eq!(config.max_runs, 3);
        assert!(source_task.is_none());
        form.fields[24].choice = 1;
        assert!(
            form.command().is_err(),
            "accepted trigger cannot issue three runs"
        );
        set(&mut form, 21, "1");
        assert!(form.command().is_err(), "source is mandatory");
        form.fields[25].choice = 1;
        let Command::SeriesPreview { source_task, .. } = form.command().unwrap() else {
            panic!("source preview")
        };
        assert_eq!(source_task.as_deref(), Some("original-task"));
        set(&mut form, 18, "0");
        assert!(form.command().is_err(), "past first run cannot confirm");
    }

    #[test]
    fn supervision_form_pins_goal_work_type_and_native_control_revision() {
        let goal = OrganizationGoal {
            id: "goal".into(),
            parent_id: None,
            objective: "Owner evidence".into(),
            criteria: vec!["Accepted result".into()],
            revision: 7,
        };
        let template = OrganizationWorkflowTemplate {
            discussion: None,
            id: "template".into(),
            project_id: "project".into(),
            label: "Finite policy".into(),
            revision: 3,
            items: vec![OrganizationPlanItem {
                key: "step".into(),
                owner_id: "agent".into(),
                instructions: "Write evidence".into(),
                criteria: vec!["Check passes".into()],
                prerequisites: vec![],
            }],
        };
        let mut form = Form::new(
            Kind::WorkflowSeries {
                template,
                goal: Some(goal.clone()),
            },
            &[],
            None,
            "model",
        );
        set(&mut form, 0, "input.txt");
        set(&mut form, 2, "report.txt");
        form.fields[26].choice = 1;
        let Command::SeriesPreview {
            supervision: Some((captured, work_type)),
            ..
        } = form.command().unwrap()
        else {
            panic!("policy preview")
        };
        assert_eq!(*captured, goal);
        assert_eq!(work_type, "bounded goal work");
        set(&mut form, 27, "");
        assert!(form.command().is_err());
        let policy = OrganizationControl {
            id: "supervision:series".into(),
            revision: 9,
            value: OrganizationControlValue::Supervision {
                series_id: "series".into(),
                goal: Box::new(goal),
                work_type: "Evidence".into(),
                state: OrganizationWorkflowState::Active,
            },
        };
        let mut control = Form::new(Kind::SupervisionControl(policy), &[], None, "model");
        control.fields[0].choice = 2;
        assert!(
            matches!(control.command().unwrap(),Command::Workflow(OrganizationWorkflowMutation::ControlSupervision {id, expected_revision:9,action:OrganizationWorkflowControl::Cancel}) if id=="series")
        );
    }

    #[test]
    fn goal_edit_preserves_identity_revision_and_explicit_parent() {
        let goal = OrganizationGoal {
            id: "goal".into(),
            parent_id: None,
            objective: "Old objective".into(),
            criteria: vec!["Measured evidence".into()],
            revision: 8,
        };
        let mut form = Form::new(Kind::GoalEdit(goal), &[], None, "model");
        set(&mut form, 0, "Reviewed objective");
        let Command::Definition(OrganizationMutation::PutGoal {
            id,
            expected_revision,
            parent_id,
            objective,
            ..
        }) = form.command().unwrap()
        else {
            panic!("goal edit")
        };
        assert_eq!(
            (id.as_deref(), expected_revision, parent_id),
            (Some("goal"), Some(8), None)
        );
        assert_eq!(objective, "Reviewed objective");
    }

    #[test]
    fn knowledge_promotion_captures_source_and_requires_owner_reviewed_body() {
        let note: OrganizationNote = serde_json::from_value(serde_json::json!({"id":"source-note","projectId":"p","authorId":"a","taskId":"t","attemptId":"attempt","scope":"agent","body":"Unreviewed evidence","createdAt":1})).unwrap();
        let group: OrganizationRecord = serde_json::from_value(serde_json::json!({"type":"node","data":{"id":"team","parentId":null,"kind":"group","label":"Team","roleVersionId":null,"model":null,"enabled":true,"revision":1}})).unwrap();
        let mut form = Form::new(Kind::PromoteNote(note), &[group], None, "model");
        form.fields[0].choice = 1;
        set(&mut form, 1, "Reviewed correction");
        assert!(
            matches!(form.command().unwrap(), Command::Definition(OrganizationMutation::PromoteNote {source_id,group_id,body}) if source_id=="source-note" && group_id.as_deref()==Some("team") && body=="Reviewed correction")
        );
        set(&mut form, 1, "");
        assert!(form.command().is_err());
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
    fn discovery_team_and_recovery_forms_only_send_exact_owner_proposals() {
        let project = OrganizationProjectPlan {
            id: "project".into(),
            name: "Local parser".into(),
            roots: vec!["/owner/project".into()],
            goal_id: None,
            owner_id: Some("agent".into()),
            criteria: vec!["Inspect parser".into()],
            revision: 7,
        };
        let node = OrganizationNode {
            id: "agent".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Parser".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 2,
        };
        let records = vec![
            OrganizationRecord::Project(project.clone()),
            OrganizationRecord::Node(node.clone()),
        ];
        let mut discovery = Form::new(
            Kind::Discovery(project.clone()),
            &records,
            Some(&records[0]),
            "model",
        );
        discovery.planning_context(&records, None);
        let Command::Definition(OrganizationMutation::PutTask {
            instructions,
            project_id,
            ..
        }) = discovery.command().unwrap()
        else {
            panic!("proposed discovery only")
        };
        assert_eq!(project_id, "project");
        assert!(
            instructions.contains("revision 7")
                && instructions.contains("agent")
                && instructions.contains("Do not implement")
                && instructions.contains("[]")
        );
        let mut team = Form::new(
            Kind::Team {
                expected_nodes: vec![node.clone()],
                nodes: vec![node],
            },
            &records,
            None,
            "model",
        );
        set(&mut team, 0, "Owner-reviewed restructure");
        assert!(matches!(
            team.command().unwrap(),
            Command::Definition(OrganizationMutation::SetControl {
                expected_revision: None,
                value: OrganizationControlValue::Team { applied: false, .. }
            })
        ));
        let recovery = Form::new(
            Kind::Recovery {
                task: task(),
                authorization_id: "original-grant".into(),
                previous: None,
            },
            &records,
            None,
            "model",
        );
        assert!(matches!(
            recovery.command().unwrap(),
            Command::Definition(OrganizationMutation::SetControl {
                value: OrganizationControlValue::SafeRecovery {
                    enabled: false,
                    max_retries: 1,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn guided_team_creation_and_retirement_pin_bases_without_json_or_authority() {
        let node = OrganizationNode {
            id: "existing".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Group,
            label: "Custom group".into(),
            role_version_id: Some("not-in-loaded-role-list".into()),
            model: None,
            enabled: true,
            revision: 7,
        };
        let mut form = Form::new(
            Kind::Team {
                expected_nodes: vec![node.clone()],
                nodes: vec![node.clone()],
            },
            &[OrganizationRecord::Node(node.clone())],
            None,
            "model",
        );
        set(&mut form, 0, "Approved optional worker");
        form.fields[1].choice = 1;
        set(&mut form, 3, "Custom worker");
        let Command::Definition(OrganizationMutation::SetControl {
            value:
                OrganizationControlValue::Team {
                    expected_nodes,
                    nodes,
                    applied,
                    ..
                },
            ..
        }) = form.command().unwrap()
        else {
            panic!("team proposal")
        };
        assert!(!applied);
        assert_eq!(expected_nodes, vec![node.clone()]);
        assert_eq!(nodes[0].revision, 0);
        assert_eq!(nodes[0].kind, OrganizationNodeKind::Agent);
        assert_eq!(nodes[0].parent_id.as_deref(), Some("existing"));
        assert!(uuid::Uuid::parse_str(&nodes[0].id).is_ok());
        assert_ne!(nodes[0].id, node.id);
        form.fields[1].choice = 3;
        let Command::Definition(OrganizationMutation::SetControl {
            value: OrganizationControlValue::Team { nodes, .. },
            ..
        }) = form.command().unwrap()
        else {
            panic!("team proposal")
        };
        assert_eq!(nodes[0].id, node.id);
        assert_eq!(nodes[0].revision, 7);
        assert!(!nodes[0].enabled);
        assert_eq!(
            nodes[0].role_version_id, node.role_version_id,
            "retirement preserves a captured role absent from the loaded selector"
        );
        assert_eq!(nodes[0].parent_id, None);
    }

    #[test]
    fn rolling_plan_advice_keeps_owner_choice_and_requires_review_of_remaining_work() {
        let goal = OrganizationGoal {
            id: "goal".into(),
            parent_id: None,
            objective: "Test Rust parser".into(),
            criteria: vec!["Parser passes oracle".into()],
            revision: 4,
        };
        let node = |id: &str, label: &str, enabled: bool| {
            OrganizationRecord::Node(OrganizationNode {
                id: id.into(),
                parent_id: None,
                kind: OrganizationNodeKind::Agent,
                label: label.into(),
                role_version_id: None,
                model: None,
                enabled,
                revision: 1,
            })
        };
        let mut done = task();
        done.id = "done".into();
        done.owner_id = "rust".into();
        done.goal_id = Some("goal".into());
        done.state = "accepted".into();
        let project = OrganizationRecord::Project(OrganizationProjectPlan {
            id: "project".into(),
            name: "Parser".into(),
            roots: vec![],
            goal_id: Some("goal".into()),
            owner_id: Some("writer".into()),
            criteria: vec![],
            revision: 2,
        });
        let records = vec![
            project.clone(),
            OrganizationRecord::Goal(goal.clone()),
            node("writer", "Writer", true),
            node("rust", "Rust parser", true),
            node("disabled", "Rust specialist", false),
            OrganizationRecord::Task(done.clone()),
        ];
        let mut report = super::super::tests::empty_report();
        let evidence:OrganizationTaskReport=serde_json::from_value(serde_json::json!({"taskId":"done","projectId":"project","ownerId":"rust","title":"accepted parser","state":"accepted","revision":7,"resultDigest":"abc","pendingApprovals":0,"checksPassed":1,"checksFailed":0,"reportedTokens":10,"modelRequests":1,"reportedModelRequests":1})).unwrap();
        report.tasks.push(evidence);
        report.sequence = 42;
        let mut form = Form::new(Kind::GoalPlan(goal), &records, Some(&project), "model");
        form.planning_context(&records, Some(&report));
        assert_eq!(
            form.fields[1].value(),
            "writer",
            "recommendations preserve existing owner selection"
        );
        assert_eq!(form.fields[1].choices[0].0, "rust");
        assert!(
            !form.fields[1]
                .choices
                .iter()
                .any(|(id, _)| id == "disabled")
        );
        assert!(form.fields[1].choices[0].1.contains("outcomes unavailable"));
        set(&mut form, 6, "Owner says parser edge cases remain");
        let Command::Definition(OrganizationMutation::PutTask {
            instructions,
            prerequisites,
            owner_id,
            ..
        }) = form.command().unwrap()
        else {
            panic!("only a proposed task")
        };
        assert_eq!(owner_id, "writer");
        assert_eq!(prerequisites, vec!["done"]);
        assert!(
            instructions.contains("abc")
                && instructions.contains("report 42")
                && instructions.contains("revision 4")
                && instructions.contains("edge cases remain")
        );
        assert_eq!(done.state, "accepted");
    }

    #[test]
    fn shared_limit_form_pins_target_and_revision_without_resetting_usage() {
        let previous = OrganizationSharedLimit {
            id: "run:r".into(),
            project_id: "p".into(),
            run_id: Some("r".into()),
            series_id: None,
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
            discussion: None,
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
            discussion: None,
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
            ..
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
            Kind::GoalMetadata {previous,..} => {
                let (project,responsible,prerequisites)=match previous.as_ref().map(|c|&c.value) {
                    Some(OrganizationControlValue::GoalMetadata {project_id,responsible_node_id,prerequisites,..})=>(Some(project_id.as_str()),Some(responsible_node_id.as_str()),prerequisites.as_slice()),
                    _=>(project.map(|p|p.id.as_str()),owner,&[][..]),
                };
                vec![Field::select("Owning project", records.iter().filter_map(|r| match r {OrganizationRecord::Project(p)=>Some((p.id.clone(),p.name.clone())),_=>None}).collect(),project),Field::select("Responsible agent / team",nodes,responsible),Field::text("Prerequisite goal IDs","One per line; exact current versions captured; bind foreign goals to projects first",&prerequisites.iter().map(|p|p.goal.id.clone()).collect::<Vec<_>>().join("\n")),Field::select("Approve selected cross-project references",vec![("false".into(),"No cross-project approval".into()),("true".into(),"Approve metadata references only; no data or execution authority".into())],Some(if prerequisites.iter().any(|p|p.cross_project_approved){"true"}else{"false"}))]
            }
            Kind::Deadline {previous,..} => {
                let due=match previous.as_ref().map(|c|&c.value) {
                    Some(OrganizationControlValue::Deadline {due_at:Some(due),display_offset_minutes,..})=>chrono::DateTime::from_timestamp(*due,0).zip(chrono::FixedOffset::east_opt(display_offset_minutes*60)).map(|(date,offset)|date.with_timezone(&offset).to_rfc3339()).unwrap_or_default(),
                    _=>String::new(),
                };
                vec![Field::text("Business due date","RFC3339 local date with explicit timezone offset, e.g. 2026-10-20T17:00:00-04:00; blank clears. Does not extend grants",&due)]
            }
            Kind::Conversation {previous,agent_id} => {
                let (project,agent,topic)=match previous.as_ref().map(|c|&c.value) {
                    Some(OrganizationControlValue::Conversation {project_id,agent_id,topic,..})=>(Some(project_id.as_str()),Some(agent_id.as_str()),topic.as_str()),
                    _=>(None,agent_id.as_deref(),""),
                };
                vec![Field::select("Project",records.iter().filter_map(|r|match r {OrganizationRecord::Project(p)=>Some((p.id.clone(),p.name.clone())),_=>None}).collect(),project),Field::select("Selected agent",agents,agent),Field::text("Conversation topic", "Stable topic; previous conversation identity stays fixed",topic),Field::text("Message", "Creates a planning draft only. Authorize its exact model/tool/budget scope separately", "")]
            }
            Kind::RoleRollback {current,versions}=>vec![Field::text("Reason","Creates a new immutable version; E assigns it separately; running captures remain", ""),Field::select("Restore role version",versions.iter().filter(|v|v.role_id==current.role_id && v.version<current.version).map(|v|(v.id.clone(),format!("{} v{} · {}",v.name,v.version,v.id))).collect(),None)],
            Kind::WorkflowRollback {current,runs}=>vec![Field::text("Reason","Restore a version captured in a prior run; exact proposal then % applies; running captures remain", ""),Field::select("Captured earlier workflow",runs.iter().filter(|r|r.preview.template.id==current.id && r.preview.template.revision<current.revision).map(|r|(r.id.clone(),format!("{} v{} · run {}",r.preview.template.label,r.preview.template.revision,r.id))).collect(),None)],
            Kind::ArtifactLibrary(_) => vec![Field::text("Artifact query","Literal substring of captured path/task/project/owner/version/check status; empty browses a bounded catalog", "")],
            Kind::WorkflowImprovement {runs,..} => vec![Field::select("Baseline workflow run",runs.iter().map(|r|(r.id.clone(),format!("{} v{} · {}",r.preview.template.label,r.preview.template.revision,r.id))).collect(),None),Field::select("Candidate workflow run",runs.iter().map(|r|(r.id.clone(),format!("{} v{} · {}",r.preview.template.label,r.preview.template.revision,r.id))).collect(),None),Field::text("Why the checks represent intended work", "Every paired step must have accepted identical protected checks; % separately adopts this tested template", "")],
            Kind::Mission(previous)=> {
                let (purpose,priorities,constraints)=match previous.as_ref().map(|c|&c.value) {Some(OrganizationControlValue::Mission{purpose,priorities,constraints})=>(purpose.clone(),priorities.join("\n"),constraints.join("\n")),_=>(String::new(),"Useful verified outcomes".into(),"Use only exact granted paths, tools, workers and budgets\nFinal acceptance and Apply belong to the owner".into())};
                vec![Field::text("Mission","Owner intent; native permissions/budgets remain enforced",&purpose),Field::text("Priorities","One per line",&priorities),Field::text("Constraints","One per line; agents cannot edit protected controls",&constraints)]
            }
            Kind::GoalReview {goal,previous,accepted}=> {
                let completed=previous.as_ref().is_some_and(|c|matches!(c.value,OrganizationControlValue::GoalReview{completed:true,..}));
                let proofs=match previous.as_ref().map(|c|&c.value) {Some(OrganizationControlValue::GoalReview{evidence,..})=>evidence.as_slice(),_=>&[]};
                let mut fields=vec![Field::select("Goal state",vec![("open".into(),"In progress / record partial evidence".into()),("complete".into(),"Owner verifies every criterion".into())],Some(if completed {"complete"} else {"open"}))];
                for (i,criterion) in goal.criteria.iter().enumerate() {
                    let proof=proofs.iter().find(|p|p.criterion as usize==i);
                    let choices=std::iter::once((String::new(),"No evidence selected".into())).chain(accepted.iter().map(|t|(t.task_id.clone(),t.title.clone()))).collect();
                    fields.push(Field::select(format!("Criterion {}: {criterion}",i+1),choices,proof.map(|p|p.task_id.as_str())));
                    fields.push(Field::text("Why this exact accepted result satisfies it","Actual checks required; task acceptance alone is not goal completion",proof.map(|p|p.explanation.as_str()).unwrap_or("")));
                }
                fields
            }
            Kind::Escalation {task,previous}=> {
                let (assignee,reason,action)=match previous.as_ref().map(|c|&c.value) {Some(OrganizationControlValue::Escalation{assignee_id,reason,requested_action,..})=>(assignee_id.clone(),reason.clone(),requested_action.clone()),_=>(None,format!("{} needs assistance",task.id),String::new())};
                let choices=std::iter::once((String::new(),"Human owner".into())).chain(records.iter().filter_map(|r|match r {OrganizationRecord::Node(n) if n.enabled=>Some((n.id.clone(),n.label.clone())),_=>None})).collect();
                vec![Field::select("Responsible agent / team / human",choices,assignee.as_deref()),Field::text("Reason","Local request; no new execution authority",&reason),Field::text("Requested action","What is needed to unblock this task",&action),Field::select("Status",vec![("open".into(),"Open assistance request".into()),("resolved".into(),"Resolve the exact existing request".into())],None)]
            }
            Kind::Lesson {..}=>vec![Field::text("What to reuse and avoid","Private proposal pinned to the accepted result; O separately reviews/promotes", "")],
            Kind::Playbook(project)=> {
                let workers=records.iter().filter_map(|r|match r {OrganizationRecord::Node(n) if n.enabled && n.kind==OrganizationNodeKind::Agent=>Some((n.id.clone(),n.label.clone())),_=>None}).collect::<Vec<_>>();
                let reviewers=std::iter::once((String::new(),"No reviewer step (owner reviews result)".into())).chain(workers.clone()).collect();
                vec![Field::select("Playbook",vec![("coding".into(),"Coding".into()),("research".into(),"Local research".into()),("audit".into(),"Independent audit".into()),("knowledge".into(),"Knowledge maintenance".into())],None),Field::text("Workflow name","Editable ordinary template; saving runs nothing",&format!("{} workflow",project.name)),Field::text("Objective","Local inputs, outputs and checks are authorized separately",""),Field::select("Worker",workers,project.owner_id.as_deref()),Field::select("Independent reviewer",reviewers,None)]
            }
            Kind::NamedView(names)=>vec![Field::select("Operation",vec![("save".into(),"Save current view".into()),("restore".into(),"Restore view".into()),("delete".into(),"Delete view".into())],None),Field::text("New view name","Up to eight private named views", ""),Field::select("Saved view",names.iter().map(|name|(name.clone(),name.clone())).collect(),None)],
            Kind::Team {expected_nodes,nodes} => {
                let node = &nodes[0];
                let parents = std::iter::once(("keep".into(),"Keep parent / create under selected node".into()))
                    .chain(std::iter::once(("root".into(),"No parent (root)".into())))
                    .chain(expected_nodes.iter().map(|n|(n.id.clone(),n.label.clone()))).collect();
                let roles = std::iter::once(("keep".into(),"Keep captured role".into())).chain(std::iter::once((String::new(),"No role".into()))).chain(records.iter().filter_map(|r|match r {OrganizationRecord::Role(role)=>Some((role.id.clone(),format!("{} v{}",role.name,role.version))),_=>None})).collect();
                vec![Field::text("Why change the team","Save proposal first; % separately reviews/applies; no grants or memory copied",""),
                    Field::select("Operation",vec![("edit".into(),"Rename / move selected definition".into()),("agent".into(),"Create agent".into()),("group".into(),"Create group".into()),("retire".into(),"Retire selected definition".into())],None),
                    Field::select("Parent",parents,None),Field::text("Name","User-defined; no required role or level",&node.label),
                    Field::select("Role",roles,Some("keep")),Field::text("Model","Blank uses configured default; selection grants no execution",node.model.as_deref().unwrap_or(""))]
            },
            Kind::Recovery {previous,..} => {
                let (enabled,max)=match previous.as_ref().map(|c|&c.value) {Some(OrganizationControlValue::SafeRecovery{enabled,max_retries,..})=>(*enabled,*max_retries),_=>(false,1)};
                vec![Field::select("Known startup recovery",vec![("off".into(),"Disabled".into()),("on".into(),"Explicitly permit pre-runtime retries".into())],Some(if enabled {"on"} else {"off"})),Field::text("Max original-grant retries", "1–2; immutable after first save; original attempts/deadline still apply", &max.to_string())]
            },
            Kind::ShareNote {note, projects} => vec![Field::select("Destination project",projects.iter().filter(|p|p.id!=note.project_id).map(|p|(p.id.clone(),p.name.clone())).collect(),None),Field::text("Reviewed shared knowledge", "Only current owner-reviewed PROJECT sources; independent copy retains provenance", &note.body)],
            Kind::Improvement {node,accepted} => vec![Field::select("Evaluated candidate role version",records.iter().filter_map(|r|match r {OrganizationRecord::Role(role) if node.role_version_id.as_ref()!=Some(&role.id)=>Some((role.id.clone(),format!("{} v{}",role.name,role.version))),_=>None}).collect(),None),Field::select("Accepted baseline evaluation",accepted.iter().map(|t|(t.task_id.clone(),t.title.clone())).collect(),None),Field::select("Accepted candidate evaluation",accepted.iter().map(|t|(t.task_id.clone(),t.title.clone())).collect(),None),Field::text("Why the checks represent intended work", "Same project/model/provider/hash-pinned immutable checks; owner adoption is separate", "")],
            Kind::RestoreEditor(session)=>vec![Field::select("Restore choice",vec![("false".into(),format!("{} · saved files only; retain unsaved checkpoint",session.summary())),("true".into(),"Explicitly recover unsaved text into native buffers; files on disk stay untouched".into())],None)],
            Kind::HunkApply {files,..}=>files.iter().flat_map(|file| {
                let patch=diffy::create_patch(&file.base,&file.artifact);
                patch.hunks().iter().enumerate().map(|(index,hunk)|Field::select(format!("{} · hunk {} · original line {}",file.path,index+1,hunk.old_range().start()),vec![(String::new(),"[ ] Preserve this hunk".into()),("true".into(),format!("[x] Apply hunk {} ({} original / {} new lines)",index+1,hunk.old_range().len(),hunk.new_range().len()))],None)).collect::<Vec<_>>()
            }).collect(),
            Kind::Knowledge { .. }=>vec![
                Field::select("Knowledge state",vec![("true".into(),"Retire from fresh retrieval; retain history".into()),("false".into(),"Restore original visibility; superseded notes stay superseded".into())],None),
                Field::text("Owner review reason","Record stale, conflicting or duplicate evidence; corrections use O promotion; no new visibility is granted",""),
            ],
            Kind::Discussion => {
                let mut fields=vec![
                    Field::select("Project",records.iter().filter_map(|r|match r {OrganizationRecord::Project(p)=>Some((p.id.clone(),p.name.clone())),_=>None}).collect(),project.map(|p|p.id.as_str())),
                    Field::text("Discussion name","Saving creates a finite template; scopes and execution need a separate exact review",""),
                    Field::text("Topic","Up to 2048 bytes; conclusions preserve dissent, evidence and proposed follow-ups",""),
                    Field::text("Rounds","1–3; later rounds wait for prior accepted contributions","1"),
                    Field::select("Conclusions author",agents.clone(),owner),
                ];
                for index in 0..8 {fields.push(Field::select(format!("Participant {}",index+1),optional(agents.clone()),if index==0 {owner} else {None}));}
                fields
            }
            Kind::SupervisionControl(_) => vec![Field::select("Policy action", vec![
                ("pause".into(), "Pause admissions and future invocations".into()),
                ("resume".into(), "Resume original finite authority".into()),
                ("cancel".into(), "Revoke; preserve results and usage".into()),
            ], None)],
            Kind::Reconciliation {..} => vec![Field::text("Observed evidence", "Record exact receipts/provider evidence; unknown usage/effects remain charged and fenced", "")],
            Kind::AuthorizeProfile {profiles,..} => vec![Field::select("Exact execution profile",profiles.iter().filter_map(|p|match &p.value {OrganizationControlValue::ExecutionProfile {label,..}=>Some((p.id.clone(),format!("{label} · revision {}",p.revision))),_=>None}).collect(),None)],
            Kind::ExecutionProfile(previous) => {
                let mut fields=Self::scope_fields(model);
                let label=previous.as_ref().and_then(|p|match &p.value {OrganizationControlValue::ExecutionProfile {label,..}=>Some(label.as_str()),_=>None}).unwrap_or("");
                fields.push(Field::text("Profile name", "Preset only; tools follow the qualified native policy; applying requires separate exact authorization", label));
                fields
            }
            Kind::ResourcePressure(previous) => {
                let (path,disk,memory)=match previous.as_ref().map(|c|&c.value) {
                    Some(OrganizationControlValue::ResourcePressure {path,min_disk_bytes,min_available_memory_bytes})=>(path.clone(),min_disk_bytes.to_string(),min_available_memory_bytes.to_string()),
                    _=>(std::env::var("CODEX_HOME").unwrap_or_default(),"0".into(),"0".into()),
                };
                vec![Field::text("Storage path", "Existing local directory on the filesystem to monitor", &path),Field::text("Minimum available disk bytes", "0 disables; below threshold pauses new admissions", &disk),Field::text("Minimum available memory bytes", "0 disables; host/cgroup available memory; unavailable required sample pauses", &memory)]
            }
            Kind::GlobalControl(previous) => {
                let (paused, limits) = match previous.as_ref().map(|c| &c.value) {
                    Some(OrganizationControlValue::Global { paused, limits }) => (*paused, limits.as_ref()),
                    _ => (false, None),
                };
                vec![
                    Field::select("Organization execution", vec![("false".into(), "Running".into()), ("true".into(), "Paused".into())], Some(if paused { "true" } else { "false" })),
                    Field::text("Lifetime reported-token ceiling", "All roots/history; blank all three removes optional global ceilings", &limits.map(|l| l.max_tokens.to_string()).unwrap_or_default()),
                    Field::text("Lifetime admitted-request ceiling", "Unknown requests stay charged; no counter reset", &limits.map(|l| l.max_model_requests.to_string()).unwrap_or_default()),
                    Field::text("Active ceiling", "1–4; default runtime capacity remains unchanged", &limits.map(|l| l.max_active.to_string()).unwrap_or_default()),
                ]
            }
            Kind::Priority { previous, .. } => {
                let priority = match previous.as_ref().map(|c| &c.value) {
                    Some(OrganizationControlValue::Priority { priority, .. }) => *priority,
                    _ => 0,
                };
                vec![Field::text("Task priority", "-10 to 10; each queued minute from its start time adds one; grant/usage unchanged", &priority.to_string())]
            }
            Kind::ProjectControl { project, accepted, .. } => {
                let mut fields = vec![Field::select("Project lifecycle", vec![
                    ("paused".into(), "Pause".into()), ("active".into(), "Resume / reopen".into()),
                    ("completed".into(), "Complete with accepted evidence".into()), ("archived".into(), "Archive paused/completed project".into()),
                ], None)];
                let candidates = optional(accepted.iter().map(|t| (t.task_id.clone(), t.title.clone())).collect());
                for (index, criterion) in project.criteria.iter().enumerate() {
                    fields.push(Field::select(format!("Criterion {} · {}", index + 1, criterion), candidates.clone(), None));
                    fields.push(Field::text("Why this accepted result satisfies the criterion", "Required only for Complete; owner assessment, exact digest is pinned", ""));
                }
                fields
            }
            Kind::PresetSave(node) => vec![Field::text("Preset name", "Definition subtree only; at most 128 nodes", &node.label)],
            Kind::PresetClone(preset) => vec![
                Field::text("Clone group name", "Fresh identities; grants/tasks/private knowledge stay separate", &format!("{} copy", preset.label)),
                Field::select("Parent group / node", optional(nodes), None),
            ],
            Kind::PromoteNote(note) => vec![Field::select("Visibility",optional(records.iter().filter_map(|record|match record {OrganizationRecord::Node(node) if node.kind==OrganizationNodeKind::Group && node.enabled=>Some((node.id.clone(),node.label.clone())),_=>None}).collect()),note.group_id.as_deref()),Field::text("Owner-reviewed knowledge", "Blank team selects project visibility; source/task/attempt provenance is retained", &note.body)],
            Kind::Handoff {
                task, receivers, ..
            } | Kind::Replan { task, receivers, .. } => vec![
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
            Kind::Goal | Kind::GoalEdit(_) => vec![
                Field::text("Objective", "Describe the desired outcome", if let Kind::GoalEdit(goal) = &kind { &goal.objective } else { "" }),
                Field::text("Success criteria", "One criterion per line", &if let Kind::GoalEdit(goal) = &kind { goal.criteria.join("\n") } else { String::new() }),
                Field::select("Parent goal", optional(goals), if let Kind::GoalEdit(goal) = &kind { goal.parent_id.as_deref() } else { goal_id }),
            ],
            Kind::Setup => vec![
                Field::text("Workspace / project name", "Local project metadata", ""),
                Field::text("Project root", "Existing absolute directory", ""),
                Field::text("First agent name", "Any user-defined name; add more agents/teams afterward", ""),
                Field::text("Agent instructions", "Reusable role instructions; no tool authority", ""),
                Field::text("Model", "Explicit configured provider model", model),
                Field::text("Lifetime token ceiling", "Reported usage; in-flight reservations are retained", "10000"),
                Field::text("Lifetime request ceiling", "Finite; unknown requests stay charged", "10"),
                Field::text("Active ceiling", "1–4; runtime default remains two", "2"),
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
            Kind::Task | Kind::GoalPlan(_) | Kind::ProjectPlanning(_) | Kind::Discovery(_) | Kind::Selection {..} => vec![
                Field::select(
                    "Project",
                    records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Project(p) if !matches!(&kind,Kind::Selection {project_id,..} if project_id!=&p.id) && !matches!(&kind,Kind::Discovery(project)|Kind::ProjectPlanning(project) if project.id!=p.id) => Some((p.id.clone(), p.name.clone())),
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
            Kind::WorkflowRun { template } | Kind::WorkflowSeries { template, .. } => template.items.iter().flat_map(|item| {
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
        if let Kind::WorkflowSeries { template, goal } = &kind {
            let now = chrono::Utc::now().timestamp();
            let sources = records
                .iter()
                .filter_map(|record| match record {
                    OrganizationRecord::Task(task) if task.project_id == template.project_id => {
                        Some((
                            task.id.clone(),
                            format!(
                                "{} · {}",
                                task.instructions.chars().take(60).collect::<String>(),
                                task.state
                            ),
                        ))
                    }
                    _ => None,
                })
                .collect();
            fields.extend([
                Field::text(
                    "Series name",
                    "Finite approved workflow; y confirms exact authority",
                    &format!("{} series", template.label),
                ),
                Field::text(
                    "First run (UTC epoch)",
                    "Future time; accepted triggers wait for acceptance",
                    &(now + 3600).to_string(),
                ),
                Field::text(
                    "Interval (seconds)",
                    "10–2678400; missed and overlapping intervals are skipped",
                    "3600",
                ),
                Field::text(
                    "End time (UTC epoch)",
                    "End within 31 days; expiry is retained across controls",
                    &(now + 86400).to_string(),
                ),
                Field::text(
                    "Total runs",
                    "1–1000; accepted-result trigger requires exactly 1",
                    "3",
                ),
                Field::text(
                    "Total reported tokens",
                    "Lifetime series pool; in-flight usage may overrun",
                    "30000",
                ),
                Field::text(
                    "Total model requests",
                    "Lifetime admitted requests across all steps and descendants",
                    "60",
                ),
                Field::select(
                    "Trigger",
                    vec![
                        ("interval".into(), "Recurring interval".into()),
                        (
                            "accepted".into(),
                            "Exact accepted task result (one-shot)".into(),
                        ),
                    ],
                    None,
                ),
                Field::select("Source task", optional(sources), None),
                Field::select("Goal supervision policy", optional(goal.iter().map(|g| (g.id.clone(), g.objective.clone())).collect()), None),
                Field::text("Permitted work type", "Owner label; exact templates, models, paths and finite budgets enforce authority", "bounded goal work"),
            ]);
        }
        if matches!(&kind, Kind::Replan { .. }) {
            fields.push(Field::text("Why replan", "Saving starts no work; separate exact review applies the delta under original grants", ""));
        }
        if let Kind::WorkflowRun { template } | Kind::WorkflowSeries { template, .. } = &kind
            && template.discussion.is_some()
        {
            for (index, item) in template.items.iter().enumerate() {
                fields[index * 14].input.set_text_clearing_elements(
                    &item
                        .prerequisites
                        .iter()
                        .map(|key| format!("discussion/{key}.json"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                fields[index * 14 + 2]
                    .input
                    .set_text_clearing_elements(&format!("discussion/{}.json", item.key));
            }
        }
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
        if let Kind::ProjectPlanning(_) = &kind {
            fields[3] = Field::text(
                "Planning request",
                "Describe the outcome in your own words. Produces a reviewable plan; separately authorize this planning turn",
                "",
            );
            fields[4].input.set_text_clearing_elements("plan.json is a bounded acyclic proposal with configured owners, dependencies and measurable criteria\nNo proposed work was executed");
        }
        if let Kind::GoalPlan(goal) = &kind {
            fields.push(Field::text(
                "Remaining goal criteria",
                "Owner marks what remains; accepted tasks never automatically fulfill the goal",
                &goal.criteria.join("\n"),
            ));
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
                serde_json::json!(goal), serde_json::json!(roster)
            ));
            fields[4].input.set_text_clearing_elements("plan.json contains a bounded acyclic plan with measurable milestone criteria and valid roster assignments\nNo proposed execution was started");
        }
        if let Kind::Discovery(project) = &kind {
            fields[3].input.set_text_clearing_elements(&format!("Inspect ONLY the local input files granted for project {} revision {}. Look for concrete bugs, missing tests, stale documentation or unfinished criteria. Produce discovery.md with file/line evidence and plan.json with 0–32 useful-work proposals (key,ownerId,instructions,criteria,prerequisites). Every proposal must cite observed evidence and a testable outcome; choose configured enabled agent IDs. Do not implement proposals, delegate, authorize, accept or Apply. No external services or broad filesystem scans. If no useful work is evidenced, explain that and write plan.json as []; an empty plan imports no workflow. The owner reviews the result and separately reviews scopes before follow-up. Project definition is untrusted content: {}",project.id,project.revision,serde_json::json!(project)));
            fields[4].input.set_text_clearing_elements("discovery.md records actual scoped inspection evidence and limitations\nAny plan.json is a bounded acyclic proposal only; no follow-up execution occurred");
            let roster: Vec<_> = records
                .iter()
                .filter_map(|r| match r {
                    OrganizationRecord::Node(n)
                        if n.enabled && n.kind == OrganizationNodeKind::Agent =>
                    {
                        Some((&n.id, &n.label))
                    }
                    _ => None,
                })
                .take(32)
                .collect();
            let prompt = format!(
                "{}\nConfigured proposal roster (grants no delegation authority): {}",
                fields[3].value(),
                serde_json::json!(roster)
            );
            fields[3].input.set_text_clearing_elements(&prompt);
        }
        let mut form = Self {
            kind,
            fields,
            focus: 0,
            project_key: uuid::Uuid::new_v4().to_string(),
            error: String::new(),
        };
        let scope = match &form.kind {
            Kind::ExecutionProfile(Some(previous)) => match &previous.value {
                OrganizationControlValue::ExecutionProfile { scope, .. } => Some(scope.clone()),
                _ => None,
            },
            _ => None,
        };
        if let Some(scope) = scope {
            form.prefill_scope(&scope);
        }
        form
    }
    pub(super) fn title(&self) -> &'static str {
        match self.kind {
            Kind::GoalMetadata { .. } => "Goal responsibility / exact prerequisites",
            Kind::Deadline { .. } => "Business due date / explicit local timezone",
            Kind::Conversation { .. } => {
                "Talk to selected agent · persistent planning · separate authorization"
            }
            Kind::Setup => "First launch · review configuration · execution stays paused",
            Kind::ResourcePressure(_) => "Resource pressure · actual disk / available memory",
            Kind::ExecutionProfile(_) => "Save versioned execution profile · no execution",
            Kind::AuthorizeProfile { .. } => "Review exact profile authorization · separate grant",
            Kind::Reconciliation { .. } => "Inspect uncertainty · retain charges / no replay",
            Kind::Mission(_) => "Workspace mission / priorities / constraints",
            Kind::RoleRollback { .. } => "Review role rollback proposal",
            Kind::WorkflowRollback { .. } => "Review captured workflow rollback proposal",
            Kind::ArtifactLibrary(_) => "Browse / search captured artifact library",
            Kind::GoalReview { .. } => "Goal criteria / exact accepted evidence",
            Kind::Escalation { .. } => "Route local assistance / resolve request",
            Kind::Lesson { .. } => "Capture an accepted-result lesson proposal",
            Kind::Playbook(_) => "Optional editable playbook · no required roles",
            Kind::NamedView(_) => "Named views · save / restore / delete",
            Kind::RestoreEditor(_) => "Restore actual Neovim session · exact owner choice",
            Kind::HunkApply { .. } => "Select exact accepted hunks · one durable Apply",
            Kind::Agent => "New agent",
            Kind::Group => "New group",
            Kind::NodeEdit(_) => "Edit node / hierarchy / state",
            Kind::Role(_) => "Create immutable role version",
            Kind::Membership(_) => "Edit group membership",
            Kind::PresetSave(_) => "Save organization preset · definition subtree",
            Kind::PresetClone(_) => "Clone organization preset · fresh identities",
            Kind::GlobalControl(_) => "Organization pause / lifetime ceilings",
            Kind::SupervisionControl(_) => "Goal supervision policy · pause / resume / revoke",
            Kind::Priority { .. } => "Task priority / fair dispatch",
            Kind::ProjectControl { .. } => "Project pause / complete / archive / reopen",
            Kind::Goal => "New goal",
            Kind::GoalEdit(_) => "Edit goal / parent / success criteria",
            Kind::Project => "New project",
            Kind::Task => "New task",
            Kind::GoalPlan(_) => "Propose goal planner · starts no execution",
            Kind::ProjectPlanning(_) => "Plan a project from an owner request · proposal only",
            Kind::Discovery(_) => "Propose finite local discovery · separate authorization",
            Kind::WorkflowImprovement { .. } => {
                "Compare evaluated workflow versions · owner recommendation"
            }
            Kind::Team { .. } => "Propose atomic team definition changes",
            Kind::Recovery { .. } => "Review bounded known-startup recovery policy",
            Kind::ShareNote { .. } => "Review cross-project knowledge copy",
            Kind::Improvement { .. } => "Compare evaluated instructions · propose owner adoption",
            Kind::Discussion => "Draft finite agent discussion · saving starts no execution",
            Kind::Knowledge { .. } => "Review knowledge retirement / restoration",
            Kind::PromoteNote(_) => "Promote / correct knowledge · explicit owner review",
            Kind::Selection { .. } => "Propose task from actual Neovim selection",
            Kind::Handoff { .. } => "Resolve blocker / hand off remaining work",
            Kind::Replan { .. } => "Propose versioned checkpoint replan · starts no execution",
            Kind::Authorize(_) => "Authorize task",
            Kind::Schedule(_) => "Create recurring schedule",
            Kind::WorkflowSave { .. } => "Save workflow template",
            Kind::WorkflowRun { .. } => "Preview workflow · scope for each step",
            Kind::WorkflowSeries { .. } => "Preview recurring workflow / accepted-result trigger",
            Kind::SharedLimits { .. } => "Set shared project / workflow ceilings",
            Kind::Apply { .. } => "Select files to Apply",
        }
    }
    /// Advice changes draft choices only; grants and historical ownership stay intact.
    pub(super) fn planning_context(
        &mut self,
        records: &[OrganizationRecord],
        report: Option<&OrganizationReport>,
    ) {
        if !matches!(
            self.kind,
            Kind::Task
                | Kind::GoalPlan(_)
                | Kind::ProjectPlanning(_)
                | Kind::Discovery(_)
                | Kind::Selection { .. }
        ) {
            return;
        }
        let project = self.fields[0].value().to_string();
        if let Kind::GoalPlan(goal) = &self.kind {
            let accepted: Vec<_> = records.iter().filter_map(|r| match r {
                OrganizationRecord::Task(t) if t.project_id==project && t.goal_id.as_deref()==Some(&goal.id) && t.state=="accepted" => {
                    let evidence=report?.tasks.iter().find(|e|e.task_id==t.id && e.revision==t.revision && e.state=="accepted" && e.result_digest.is_some())?;
                    Some(serde_json::json!({"taskId":t.id,"revision":t.revision,"digest":evidence.result_digest,"criteria":t.criteria,"checksPassed":evidence.checks_passed,"checksFailed":evidence.checks_failed}))
                }, _=>None,
            }).take(8).collect();
            let mut prompt = self.fields[3].value().to_string();
            prompt.push_str(&format!("\nRolling planning context (report {}; at most eight accepted results, no inferred goal completion): {}. Propose ONLY remaining work. Missing evidence is unavailable. All output requires separate owner workflow/scope review.", report.map(|r|r.sequence).map(|n|n.to_string()).unwrap_or_else(||"unavailable".into()),serde_json::json!(accepted)));
            self.fields[3].input.set_text_clearing_elements(&prompt);
            self.fields[5].input.set_text_clearing_elements(
                &accepted
                    .iter()
                    .filter_map(|v| v["taskId"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        let selected = self.fields[1].value().to_string();
        let query = self.fields[3].value().to_lowercase();
        let terms: BTreeSet<_> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 3)
            .collect();
        // ponytail: transparent lexical advice, not a learned quality predictor; qualify one before replacing it.
        let mut ranked:Vec<_>=records.iter().filter_map(|r| match r {
            OrganizationRecord::Node(n) if n.enabled && n.kind==OrganizationNodeKind::Agent => {
                let role=records.iter().find_map(|r|match r {OrganizationRecord::Role(role) if Some(&role.id)==n.role_version_id.as_ref()=>Some(role),_=>None});
                let capabilities=format!("{} {}",n.label,role.map(|r|format!("{} {}",r.name,r.instructions)).unwrap_or_default()).to_lowercase();
                let matches=terms.iter().filter(|t|capabilities.split(|c:char|!c.is_alphanumeric()).any(|s|s==**t)).count();
                let load=records.iter().filter(|r|matches!(r,OrganizationRecord::Task(t) if t.owner_id==n.id && !matches!(t.state.as_str(),"accepted"|"cancelled"|"failed"))).count();
                let outcomes=report.and_then(|r|r.scorecards.as_ref()).and_then(|s| {
                    let entries:Vec<_>=s.entries.iter().filter(|e|e.project_id==project && e.owner_id.as_deref()==Some(&n.id)).collect();
                    (!entries.is_empty()).then(|| entries.iter().fold((0i64,0i64),|(accepted,failed),e|(accepted+e.accepted_results,failed+e.failed_attempts)))
                });
                let evidence=outcomes.map(|(a,f)|format!("{a} accepted/{f} failed captured attempts")).unwrap_or_else(||"outcomes unavailable".into());
                Some((std::cmp::Reverse(matches),load,n.id.clone(),format!("{} · {matches} capability terms · {load} open tasks · {evidence}",n.label)))
            }, _=>None,
        }).collect();
        ranked.sort_by(|a, b| (&a.0, a.1, &a.2).cmp(&(&b.0, b.1, &b.2)));
        let choices = ranked
            .into_iter()
            .map(|(_, _, id, label)| (id, label))
            .collect();
        self.fields[1] =
            Field::select("Assignment advice · choose owner", choices, Some(&selected));
    }
    pub(super) fn prefill_scope(&mut self, scope: &OrganizationExecutionScope) {
        if !matches!(
            &self.kind,
            Kind::Schedule(_)
                | Kind::WorkflowRun { .. }
                | Kind::WorkflowSeries { .. }
                | Kind::ExecutionProfile(_)
        ) {
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
        let groups = if let Kind::WorkflowRun { template } | Kind::WorkflowSeries { template, .. } =
            &self.kind
        {
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
            Kind::GoalMetadata {
                goal,
                previous,
                candidates,
            } => {
                let project_id = required(0)?;
                let prerequisites = lines(2)
                    .iter()
                    .map(|id| {
                        let mut prerequisite = candidates
                            .iter()
                            .find(|p| p.goal.id == *id)
                            .cloned()
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Prerequisite {id} is not loaded or attributed to a project"
                                )
                            })?;
                        prerequisite.cross_project_approved =
                            prerequisite.project_id != project_id && value(3) == "true";
                        Ok(prerequisite)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::GoalMetadata {
                        goal: Box::new(goal.clone()),
                        project_id,
                        responsible_node_id: required(1)?,
                        prerequisites,
                    },
                })
            }
            Kind::Deadline {
                kind,
                target_id,
                target_revision,
                previous,
            } => {
                let (due_at, display_offset_minutes) = if value(0).is_empty() {
                    (None, 0)
                } else {
                    let date = chrono::DateTime::parse_from_rfc3339(&value(0))?;
                    let seconds = date.offset().local_minus_utc();
                    anyhow::ensure!(
                        seconds % 60 == 0 && seconds.abs() <= 14 * 60 * 60,
                        "Use a timezone offset in whole minutes within ±14 hours"
                    );
                    (Some(date.timestamp()), seconds / 60)
                };
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::Deadline {
                        kind: *kind,
                        target_id: target_id.clone(),
                        target_revision: *target_revision,
                        due_at,
                        display_offset_minutes,
                    },
                })
            }
            Kind::Conversation { previous, .. } => {
                let id = previous.as_ref().and_then(|c| match &c.value {
                    OrganizationControlValue::Conversation { id, .. } => Some(id.clone()),
                    _ => None,
                });
                Command::Definition(OrganizationMutation::ConversationPrompt {
                    id,
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    project_id: required(0)?,
                    agent_id: required(1)?,
                    topic: required(2)?,
                    prompt: required(3)?,
                })
            }
            Kind::Reconciliation {
                attempt,
                fingerprint,
            } => Command::Definition(OrganizationMutation::SetControl {
                expected_revision: None,
                value: OrganizationControlValue::Reconciliation {
                    id: self.project_key.clone(),
                    attempt_id: attempt.id.clone(),
                    request_sequence: attempt.model_requests,
                    fingerprint: fingerprint.clone(),
                    evidence: required(0)?,
                },
            }),
            Kind::AuthorizeProfile { task, profiles } => {
                let profile = profiles
                    .iter()
                    .find(|p| p.id == value(0))
                    .context("select an execution profile")?;
                Command::Execution(OrganizationExecutionMutation::AuthorizeProfile {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    profile: Box::new(profile.clone()),
                })
            }
            Kind::ExecutionProfile(previous) => {
                let id = previous
                    .as_ref()
                    .and_then(|p| match &p.value {
                        OrganizationControlValue::ExecutionProfile { id, .. } => Some(id.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| self.project_key.clone());
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|p| p.revision),
                    value: OrganizationControlValue::ExecutionProfile {
                        id,
                        label: required(14)?,
                        policy_version:
                            codex_protocol::organization_execution::ORGANIZATION_POLICY_VERSION,
                        scope: Box::new(self.scope(0)?),
                    },
                })
            }
            Kind::Setup => {
                let root = required(1)?;
                anyhow::ensure!(
                    Path::new(&root).is_absolute(),
                    "Project root must be absolute"
                );
                let root = std::fs::canonicalize(root)?;
                anyhow::ensure!(root.is_dir(), "Project root must be an existing directory");
                let limits = OrganizationSharedCeilings {
                    max_tokens: required(5)?.parse()?,
                    max_model_requests: required(6)?.parse()?,
                    max_active: required(7)?.parse()?,
                };
                anyhow::ensure!(
                    (1..=10_000_000).contains(&limits.max_tokens)
                        && (1..=100_000).contains(&limits.max_model_requests)
                        && (1..=4).contains(&limits.max_active),
                    "Invalid finite setup ceilings"
                );
                Command::Setup {
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
                    agent: required(2)?,
                    instructions: required(3)?,
                    model: required(4)?,
                    limits,
                }
            }
            Kind::RoleRollback { current, versions } => {
                let source = versions
                    .iter()
                    .find(|v| v.id == value(1))
                    .context("select an earlier captured role version")?;
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: OrganizationControlValue::RoleRollback {
                        id: self.project_key.clone(),
                        reason: required(0)?,
                        current: Box::new(current.clone()),
                        source: Box::new(source.clone()),
                        applied: false,
                    },
                })
            }
            Kind::WorkflowRollback { current, runs } => {
                let run = runs
                    .iter()
                    .find(|r| r.id == value(1))
                    .context("select an earlier captured workflow run")?;
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: OrganizationControlValue::WorkflowRollback {
                        id: self.project_key.clone(),
                        reason: required(0)?,
                        current: Box::new(current.clone()),
                        source: Box::new(run.preview.template.clone()),
                        source_run_id: run.id.clone(),
                        source_run_revision: run.revision,
                        applied: false,
                    },
                })
            }
            Kind::ArtifactLibrary(project_id) => {
                let query = value(0);
                anyhow::ensure!(
                    query.len() <= 256 && !query.chars().any(char::is_control),
                    "artifact query supports 256 bytes without controls"
                );
                Command::ArtifactLibrary {
                    id: self.project_key.clone(),
                    project_id: project_id.clone(),
                    query,
                }
            }
            Kind::Mission(previous) => Command::Definition(OrganizationMutation::SetControl {
                expected_revision: previous.as_ref().map(|c| c.revision),
                value: OrganizationControlValue::Mission {
                    purpose: required(0)?,
                    priorities: lines(1),
                    constraints: lines(2),
                },
            }),
            Kind::GoalReview {
                goal,
                previous,
                accepted,
            } => {
                let mut evidence = Vec::new();
                for i in 0..goal.criteria.len() {
                    let id = value(1 + 2 * i);
                    if id.is_empty() {
                        continue;
                    }
                    let task = accepted
                        .iter()
                        .find(|t| t.task_id == id)
                        .context("select current accepted evidence")?;
                    evidence.push(OrganizationProjectEvidence {
                        criterion: i as u32,
                        task_id: id,
                        result_digest: task
                            .result_digest
                            .clone()
                            .context("result digest unavailable")?,
                        explanation: required(2 + 2 * i)?,
                    });
                }
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::GoalReview {
                        goal: Box::new(goal.clone()),
                        completed: value(0) == "complete",
                        evidence,
                    },
                })
            }
            Kind::Escalation { task, previous } => {
                let value = if value(3) == "resolved" {
                    let mut captured = previous
                        .as_ref()
                        .context("save and review a request first")?
                        .value
                        .clone();
                    let OrganizationControlValue::Escalation { resolved, .. } = &mut captured
                    else {
                        anyhow::bail!("invalid assistance request")
                    };
                    *resolved = true;
                    captured
                } else {
                    OrganizationControlValue::Escalation {
                        task_id: task.id.clone(),
                        task_revision: task.revision,
                        assignee_id: optional(0),
                        reason: required(1)?,
                        requested_action: required(2)?,
                        resolved: false,
                    }
                };
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value,
                })
            }
            Kind::Lesson { task, digest } => {
                Command::Definition(OrganizationMutation::ProposeLesson {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    result_digest: digest.clone(),
                    body: required(0)?,
                })
            }
            Kind::Playbook(project) => {
                let owner = required(3)?;
                let reviewer = optional(4);
                anyhow::ensure!(
                    reviewer.as_ref() != Some(&owner),
                    "independent review needs a distinct configured owner"
                );
                anyhow::ensure!(
                    value(0) != "audit" || reviewer.is_some(),
                    "audit playbook needs an independent reviewer"
                );
                let mut items = vec![OrganizationPlanItem {
                    key: "produce".into(),
                    owner_id: owner,
                    instructions: format!(
                        "{}: {}. Use only permitted local inputs. Produce cited, reviewable draft.md and propose useful lessons; claim only evidenced outcomes.",
                        value(0),
                        required(2)?
                    ),
                    criteria: vec![
                        "Owner-selected native checks pass and evidence is cited".into(),
                    ],
                    prerequisites: vec![],
                }];
                if let Some(owner_id) = reviewer {
                    items.push(OrganizationPlanItem{key:"review".into(),owner_id,instructions:"Independently inspect permitted draft.md and its evidence. Produce review.md with verified findings, counterexamples, disagreements and limitations. Root acceptance and Apply stay with the owner.".into(),criteria:vec!["Declared review checks pass; findings link to source evidence".into()],prerequisites:vec!["produce".into()]});
                }
                Command::Workflow(OrganizationWorkflowMutation::Put {
                    id: self.project_key.clone(),
                    expected_revision: None,
                    project_id: project.id.clone(),
                    label: required(1)?,
                    items,
                })
            }
            Kind::RestoreEditor(session) => {
                Command::RestoreEditor(session.clone(), required(0)?.parse()?)
            }
            Kind::HunkApply {
                task,
                result,
                files,
            } => {
                use sha2::Digest;
                use sha2::Sha256;
                anyhow::ensure!(result.accepted, "Accept the exact result first");
                let mut offset = 0;
                let mut selections = Vec::new();
                for file in files {
                    let count = diffy::create_patch(&file.base, &file.artifact)
                        .hunks()
                        .len();
                    let indices: Vec<_> = (0..count)
                        .filter(|i| value(offset + i) == "true")
                        .map(|i| i as u32)
                        .collect();
                    offset += count;
                    if indices.is_empty() {
                        continue;
                    }
                    let (patch, output) = codex_protocol::organization_execution::selected_hunks(
                        &file.base,
                        &file.artifact,
                        &indices,
                    )
                    .map_err(anyhow::Error::msg)?;
                    selections.push(OrganizationHunkSelection {
                        path: file.path.clone(),
                        indices,
                        patch_sha256: format!("{:x}", Sha256::digest(patch.as_bytes())),
                        output_sha256: format!("{:x}", Sha256::digest(output.as_bytes())),
                    });
                }
                anyhow::ensure!(!selections.is_empty(), "Select at least one hunk");
                Command::Execution(OrganizationExecutionMutation::Apply {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    digest: result.digest.clone(),
                    paths: Some(selections.iter().map(|s| s.path.clone()).collect()),
                    hunks: Some(selections),
                })
            }
            Kind::Knowledge {
                note_id,
                body_sha256,
                previous,
            } => Command::Definition(OrganizationMutation::SetControl {
                expected_revision: previous.as_ref().map(|c| c.revision),
                value: OrganizationControlValue::Knowledge {
                    note_id: note_id.clone(),
                    body_sha256: body_sha256.clone(),
                    retired: required(0)?.parse()?,
                    reason: required(1)?,
                },
            }),
            Kind::Discussion => {
                let discussion = OrganizationDiscussion {
                    topic: required(2)?,
                    rounds: required(3)?.parse()?,
                    conclusion_owner_id: required(4)?,
                    participants: (5..13).filter_map(optional).collect(),
                };
                discussion.plan_items().map_err(anyhow::Error::msg)?;
                Command::Workflow(OrganizationWorkflowMutation::PutDiscussion {
                    id: self.project_key.clone(),
                    expected_revision: None,
                    project_id: required(0)?,
                    label: required(1)?,
                    discussion,
                })
            }
            Kind::SupervisionControl(policy) => {
                let OrganizationControlValue::Supervision { series_id, .. } = &policy.value else {
                    anyhow::bail!("Select a supervision policy")
                };
                Command::Workflow(OrganizationWorkflowMutation::ControlSupervision {
                    id: series_id.clone(),
                    expected_revision: policy.revision,
                    action: match required(0)?.as_str() {
                        "pause" => OrganizationWorkflowControl::Pause,
                        "resume" => OrganizationWorkflowControl::Resume,
                        "cancel" => OrganizationWorkflowControl::Cancel,
                        _ => anyhow::bail!("Invalid policy action"),
                    },
                })
            }
            Kind::ResourcePressure(previous) => {
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::ResourcePressure {
                        path: required(0)?,
                        min_disk_bytes: required(1)?.parse()?,
                        min_available_memory_bytes: required(2)?.parse()?,
                    },
                })
            }
            Kind::GlobalControl(previous) => {
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::Global {
                        paused: required(0)?.parse()?,
                        limits: if (1..4).all(|i| value(i).trim().is_empty()) {
                            None
                        } else {
                            Some(OrganizationSharedCeilings {
                                max_tokens: required(1)?.parse()?,
                                max_model_requests: required(2)?.parse()?,
                                max_active: required(3)?.parse()?,
                            })
                        },
                    },
                })
            }
            Kind::Priority { task, previous } => {
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::Priority {
                        task_id: task.id.clone(),
                        task_revision: task.revision,
                        priority: required(0)?.parse()?,
                    },
                })
            }
            Kind::ProjectControl {
                project,
                previous,
                accepted,
            } => {
                let state = match required(0)?.as_str() {
                    "active" => OrganizationProjectState::Active,
                    "paused" => OrganizationProjectState::Paused,
                    "completed" => OrganizationProjectState::Completed,
                    "archived" => OrganizationProjectState::Archived,
                    _ => anyhow::bail!("invalid project lifecycle"),
                };
                let evidence = if state == OrganizationProjectState::Completed {
                    project
                        .criteria
                        .iter()
                        .enumerate()
                        .map(|(index, _)| {
                            let task_id = required(1 + index * 2)?;
                            let result_digest = accepted
                                .iter()
                                .find(|t| t.task_id == task_id)
                                .and_then(|t| t.result_digest.clone())
                                .ok_or_else(|| {
                                    anyhow::anyhow!("accepted result unavailable; refresh")
                                })?;
                            Ok(OrganizationProjectEvidence {
                                criterion: index as u32,
                                task_id,
                                result_digest,
                                explanation: required(2 + index * 2)?,
                            })
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?
                } else {
                    vec![]
                };
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.as_ref().map(|c| c.revision),
                    value: OrganizationControlValue::Project {
                        project_id: project.id.clone(),
                        project_revision: project.revision,
                        state,
                        evidence,
                    },
                })
            }
            Kind::PresetSave(node) => Command::Definition(OrganizationMutation::SavePreset {
                root_id: node.id.clone(),
                expected_root_revision: node.revision,
                label: required(0)?,
            }),
            Kind::PresetClone(preset) => Command::Definition(OrganizationMutation::ClonePreset {
                id: preset.id.clone(),
                expected_revision: preset.revision,
                label: required(0)?,
                parent_id: optional(1),
            }),
            Kind::PromoteNote(note) => Command::Definition(OrganizationMutation::PromoteNote {
                source_id: note.id.clone(),
                group_id: optional(0),
                body: required(1)?,
            }),
            Kind::Handoff {
                task,
                checkpoint,
                receivers,
            }
            | Kind::Replan {
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
                let item = OrganizationReplanItem {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    checkpoint_id: checkpoint.id.clone(),
                    owner_id,
                    expected_owner_revision: receiver.revision,
                    instructions,
                };
                if matches!(&self.kind, Kind::Replan { .. }) {
                    Command::Definition(OrganizationMutation::SetControl {
                        expected_revision: None,
                        value: OrganizationControlValue::Replan {
                            id: self.project_key.clone(),
                            project_id: task.project_id.clone(),
                            reason: required(2)?,
                            items: vec![item],
                            applied_handoff_ids: vec![],
                        },
                    })
                } else {
                    Command::Execution(item.handoff())
                }
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
                    hunks: None,
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
            Kind::Goal | Kind::GoalEdit(_) => Command::Definition(OrganizationMutation::PutGoal {
                id: if let Kind::GoalEdit(goal) = &self.kind {
                    Some(goal.id.clone())
                } else {
                    None
                },
                expected_revision: if let Kind::GoalEdit(goal) = &self.kind {
                    Some(goal.revision)
                } else {
                    None
                },
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
            Kind::WorkflowRun { template } | Kind::WorkflowSeries { template, .. } => {
                let request = OrganizationWorkflowPreviewRequest {
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
                };
                if matches!(&self.kind, Kind::WorkflowSeries { .. }) {
                    let start = template.items.len() * 14 + 3;
                    let config = OrganizationScheduleConfig {
                        label: required(start)?,
                        first_run_at: required(start + 1)?.parse()?,
                        interval_seconds: required(start + 2)?.parse()?,
                        ends_at: required(start + 3)?.parse()?,
                        max_runs: required(start + 4)?.parse()?,
                        max_tokens: required(start + 5)?.parse()?,
                        max_model_requests: required(start + 6)?.parse()?,
                    };
                    config
                        .validate(chrono::Utc::now().timestamp())
                        .map_err(anyhow::Error::msg)?;
                    let source_task = if required(start + 7)? == "accepted" {
                        anyhow::ensure!(
                            config.max_runs == 1,
                            "accepted-result triggers require exactly one run"
                        );
                        Some(required(start + 8)?)
                    } else {
                        None
                    };
                    Command::SeriesPreview {
                        request,
                        config,
                        source_task,
                        supervision: if !value(start + 9).is_empty() {
                            let Kind::WorkflowSeries {
                                goal: Some(goal), ..
                            } = &self.kind
                            else {
                                anyhow::bail!("Linked goal is unavailable")
                            };
                            Some((Box::new(goal.clone()), required(start + 10)?))
                        } else {
                            None
                        },
                    }
                } else {
                    Command::WorkflowPreview(request)
                }
            }
            Kind::WorkflowImprovement { template, runs } => {
                let run = |i| {
                    runs.iter()
                        .find(|r| r.id == value(i))
                        .context("select exact evaluation runs")
                };
                let baseline = run(0)?;
                let candidate = run(1)?;
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: OrganizationControlValue::WorkflowImprovement {
                        id: self.project_key.clone(),
                        reason: required(2)?,
                        template_id: template.id.clone(),
                        template_revision: template.revision,
                        baseline_run_id: baseline.id.clone(),
                        baseline_run_revision: baseline.revision,
                        candidate_run_id: candidate.id.clone(),
                        candidate_run_revision: candidate.revision,
                        applied: false,
                    },
                })
            }
            Kind::NamedView(_) => Command::NamedView {
                name: required(if value(0) == "save" { 1 } else { 2 })?,
                action: required(0)?,
            },
            Kind::Team {
                expected_nodes,
                nodes,
            } => {
                let original = nodes.first().context("select a definition first")?;
                let creating = matches!(value(1).as_str(), "agent" | "group");
                let mut node = original.clone();
                if creating {
                    node.id = self.project_key.clone();
                    node.revision = 0;
                    node.kind = if value(1) == "agent" {
                        OrganizationNodeKind::Agent
                    } else {
                        OrganizationNodeKind::Group
                    };
                    node.enabled = true;
                } else if value(1) == "retire" {
                    node.enabled = false;
                }
                node.parent_id = match value(2).as_str() {
                    "keep" if creating => Some(original.id.clone()),
                    "keep" => original.parent_id.clone(),
                    "root" => None,
                    id => Some(id.to_owned()),
                };
                node.label = required(3)?;
                node.role_version_id = if value(4) == "keep" {
                    original.role_version_id.clone()
                } else {
                    (!value(4).is_empty()).then(|| value(4))
                };
                node.model = (!value(5).is_empty()).then(|| value(5));
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: OrganizationControlValue::Team {
                        id: self.project_key.clone(),
                        reason: required(0)?,
                        expected_nodes: expected_nodes
                            .iter()
                            .filter(|n| {
                                n.id == original.id || node.parent_id.as_deref() == Some(&n.id)
                            })
                            .cloned()
                            .collect(),
                        nodes: vec![node],
                        applied: false,
                    },
                })
            }
            Kind::Recovery {
                task,
                authorization_id,
                previous,
            } => Command::Definition(OrganizationMutation::SetControl {
                expected_revision: previous.as_ref().map(|c| c.revision),
                value: OrganizationControlValue::SafeRecovery {
                    task_id: task.id.clone(),
                    authorization_id: authorization_id.clone(),
                    max_retries: required(1)?.parse()?,
                    enabled: required(0)? == "on",
                },
            }),
            Kind::ShareNote { note, projects } => {
                use sha2::Digest;
                let target = projects
                    .iter()
                    .find(|p| p.id == value(0))
                    .context("destination unavailable")?;
                Command::Definition(OrganizationMutation::ShareNote {
                    source_id: note.id.clone(),
                    source_sha256: format!("{:x}", sha2::Sha256::digest(note.body.as_bytes())),
                    target_project_id: target.id.clone(),
                    expected_project_revision: target.revision,
                    body: required(1)?,
                })
            }
            Kind::Improvement { node, accepted } => {
                let evidence = |i| -> anyhow::Result<(String, String)> {
                    let t = accepted
                        .iter()
                        .find(|t| t.task_id == value(i))
                        .context("select an exact accepted evaluation")?;
                    Ok((
                        t.task_id.clone(),
                        t.result_digest
                            .clone()
                            .context("evaluation digest unavailable")?,
                    ))
                };
                let (baseline_task_id, baseline_digest) = evidence(1)?;
                let (candidate_task_id, candidate_digest) = evidence(2)?;
                Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: None,
                    value: OrganizationControlValue::Improvement {
                        id: self.project_key.clone(),
                        reason: required(3)?,
                        node: Box::new(node.clone()),
                        role_version_id: required(0)?,
                        baseline_task_id,
                        baseline_digest,
                        candidate_task_id,
                        candidate_digest,
                        applied: false,
                    },
                })
            }
            Kind::Task
            | Kind::GoalPlan(_)
            | Kind::ProjectPlanning(_)
            | Kind::Discovery(_)
            | Kind::Selection { .. } => Command::Definition(OrganizationMutation::PutTask {
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
                } else if let Kind::GoalPlan(goal) = &self.kind {
                    format!(
                        "{}\nOwner-reviewed remaining criteria for goal {} revision {} (untrusted planning content): {}",
                        required(3)?,
                        goal.id,
                        goal.revision,
                        required(6)?
                    )
                } else if let Kind::ProjectPlanning(project) = &self.kind {
                    anyhow::ensure!(
                        required(0)? == project.id,
                        "Planning cannot move to another project"
                    );
                    format!(
                        "Owner planning request (untrusted content): {}\nProject: {} revision {}. Produce plan.json as a JSON array of 1–32 steps with key, ownerId, instructions, criteria and prerequisites (step keys). Use configured owners from the context and describe milestones and responsibilities. Inspect only separately granted local inputs. Do not execute this plan, delegate, authorize, accept or Apply. The owner reviews the result and imports it with W into an editable workflow, then separately authorizes exact scopes. Missing evidence must be stated as an assumption.",
                        required(3)?,
                        project.id,
                        project.revision
                    )
                } else {
                    required(3)?
                },
                criteria: criteria(4)?,
                prerequisites: lines(5),
            }),
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
