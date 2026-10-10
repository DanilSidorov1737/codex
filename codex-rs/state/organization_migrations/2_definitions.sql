CREATE TABLE organization_role_versions (
    id TEXT PRIMARY KEY,
    role_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version > 0),
    data TEXT NOT NULL CHECK (json_valid(data)),
    UNIQUE (role_id, version)
);
CREATE TABLE organization_nodes (
    id TEXT PRIMARY KEY,
    parent_id TEXT REFERENCES organization_nodes(id),
    kind TEXT NOT NULL CHECK (kind IN ('agent', 'group')),
    role_version_id TEXT REFERENCES organization_role_versions(id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    data TEXT NOT NULL CHECK (json_valid(data)),
    CHECK (parent_id IS NULL OR parent_id != id),
    CHECK (kind = 'agent' OR role_version_id IS NULL)
);
CREATE INDEX organization_node_children ON organization_nodes(parent_id, id);
CREATE TABLE organization_goals (
    id TEXT PRIMARY KEY,
    parent_id TEXT REFERENCES organization_goals(id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    data TEXT NOT NULL CHECK (json_valid(data)),
    CHECK (parent_id IS NULL OR parent_id != id)
);
CREATE INDEX organization_goal_children ON organization_goals(parent_id, id);
CREATE TABLE organization_projects (
    id TEXT PRIMARY KEY,
    goal_id TEXT REFERENCES organization_goals(id),
    owner_id TEXT REFERENCES organization_nodes(id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    data TEXT NOT NULL CHECK (json_valid(data))
);
CREATE TABLE organization_tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES organization_projects(id),
    goal_id TEXT REFERENCES organization_goals(id),
    owner_id TEXT NOT NULL REFERENCES organization_nodes(id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    state TEXT NOT NULL,
    data TEXT NOT NULL CHECK (json_valid(data))
);
CREATE INDEX organization_project_tasks ON organization_tasks(project_id, state, id);
CREATE TABLE organization_task_dependencies (
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    prerequisite_id TEXT NOT NULL REFERENCES organization_tasks(id),
    PRIMARY KEY (task_id, prerequisite_id),
    CHECK (task_id != prerequisite_id)
);
CREATE INDEX organization_task_dependents ON organization_task_dependencies(prerequisite_id, task_id);
CREATE TABLE organization_memberships (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES organization_nodes(id),
    group_id TEXT NOT NULL REFERENCES organization_nodes(id),
    data TEXT NOT NULL CHECK (json_valid(data)),
    UNIQUE (node_id, group_id),
    CHECK (node_id != group_id)
);
CREATE TABLE organization_audit (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    actor TEXT NOT NULL,
    operation TEXT NOT NULL,
    payload TEXT NOT NULL CHECK (json_valid(payload)),
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
