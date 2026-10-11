CREATE TABLE organization_workflow_templates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES organization_projects(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_workflow_templates_project ON organization_workflow_templates(project_id,id);
CREATE TABLE organization_workflow_runs (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL REFERENCES organization_workflow_templates(id),
    project_id TEXT NOT NULL REFERENCES organization_projects(id),
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_workflow_runs_project ON organization_workflow_runs(project_id,id);
CREATE TABLE organization_workflow_tasks (
    run_id TEXT NOT NULL REFERENCES organization_workflow_runs(id),
    step_key TEXT NOT NULL,
    task_id TEXT NOT NULL UNIQUE REFERENCES organization_tasks(id),
    authorization_id TEXT NOT NULL UNIQUE REFERENCES organization_authorizations(id),
    PRIMARY KEY(run_id,step_key)
);
