CREATE TABLE organization_authorizations (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    expires_at INTEGER NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0 CHECK(revoked IN (0,1)),
    used_tokens INTEGER NOT NULL DEFAULT 0 CHECK(used_tokens >= 0),
    model_requests INTEGER NOT NULL DEFAULT 0 CHECK(model_requests >= 0),
    data TEXT NOT NULL CHECK(json_valid(data))
);
ALTER TABLE organization_tasks ADD COLUMN authorization_id TEXT REFERENCES organization_authorizations(id);
CREATE TABLE organization_attempts (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    authorization_id TEXT NOT NULL REFERENCES organization_authorizations(id),
    generation INTEGER NOT NULL CHECK(generation > 0),
    thread_id TEXT NOT NULL UNIQUE,
    worker_id TEXT NOT NULL,
    lease_until INTEGER NOT NULL,
    state TEXT NOT NULL,
    data TEXT NOT NULL CHECK(json_valid(data)),
    UNIQUE(task_id, generation)
);
CREATE INDEX organization_attempts_active ON organization_attempts(state, lease_until);
CREATE TABLE organization_actions (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES organization_attempts(id),
    call_id TEXT NOT NULL,
    action_hash TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending','dispatched','completed','failed','uncertain','rejected')),
    data TEXT NOT NULL CHECK(json_valid(data)),
    UNIQUE(attempt_id, call_id)
);
CREATE TABLE organization_approvals (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL UNIQUE REFERENCES organization_actions(id),
    state TEXT NOT NULL,
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE TABLE organization_results (
    attempt_id TEXT PRIMARY KEY REFERENCES organization_attempts(id),
    digest TEXT NOT NULL,
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE TABLE organization_outbox (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    reference TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
