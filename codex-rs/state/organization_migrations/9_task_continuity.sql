CREATE TABLE organization_checkpoints (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    attempt_id TEXT NOT NULL REFERENCES organization_attempts(id),
    call_id TEXT NOT NULL,
    data TEXT NOT NULL CHECK(json_valid(data)),
    UNIQUE(attempt_id, call_id)
);
CREATE INDEX organization_checkpoints_task ON organization_checkpoints(task_id, id);
CREATE TABLE organization_handoffs (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    checkpoint_id TEXT NOT NULL UNIQUE REFERENCES organization_checkpoints(id),
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_handoffs_task ON organization_handoffs(task_id, id);
