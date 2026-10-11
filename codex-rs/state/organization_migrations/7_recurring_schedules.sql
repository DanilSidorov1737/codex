CREATE TABLE organization_schedules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES organization_projects(id),
    state TEXT NOT NULL CHECK(state IN ('active','paused','cancelled','completed','blocked')),
    next_run_at INTEGER,
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_schedules_due ON organization_schedules(state,next_run_at,id);
CREATE INDEX organization_schedules_project ON organization_schedules(project_id,id);
CREATE TABLE organization_schedule_occurrences (
    schedule_id TEXT NOT NULL REFERENCES organization_schedules(id),
    due_at INTEGER NOT NULL CHECK(due_at >= 0),
    task_id TEXT NOT NULL UNIQUE REFERENCES organization_tasks(id),
    authorization_id TEXT NOT NULL UNIQUE REFERENCES organization_authorizations(id),
    PRIMARY KEY(schedule_id,due_at)
);
