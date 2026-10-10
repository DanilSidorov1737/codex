-- Child grants inherit scope and debit the root grant's protected budget.
ALTER TABLE organization_authorizations ADD COLUMN parent_attempt_id TEXT REFERENCES organization_attempts(id);
ALTER TABLE organization_authorizations ADD COLUMN root_id TEXT REFERENCES organization_authorizations(id);
UPDATE organization_authorizations SET root_id=id;
CREATE INDEX organization_authorizations_root ON organization_authorizations(root_id);
ALTER TABLE organization_tasks ADD COLUMN proposed_root_id TEXT REFERENCES organization_authorizations(id);
CREATE TABLE organization_coordination_calls (
    attempt_id TEXT NOT NULL REFERENCES organization_attempts(id),
    call_id TEXT NOT NULL,
    arguments TEXT NOT NULL,
    result TEXT NOT NULL CHECK(json_valid(result)),
    PRIMARY KEY(attempt_id,call_id)
);
CREATE TABLE organization_notes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES organization_projects(id),
    author_id TEXT NOT NULL REFERENCES organization_nodes(id),
    scope TEXT NOT NULL CHECK(scope IN ('project','agent')),
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_notes_scope ON organization_notes(project_id,scope,author_id);
CREATE TABLE organization_messages (
    id TEXT PRIMARY KEY,
    root_id TEXT NOT NULL REFERENCES organization_authorizations(id),
    recipient_task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(acknowledged IN (0,1)),
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_messages_inbox ON organization_messages(recipient_task_id,acknowledged);
CREATE TABLE organization_waits (
    attempt_id TEXT NOT NULL REFERENCES organization_attempts(id),
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    PRIMARY KEY(attempt_id,task_id)
);
