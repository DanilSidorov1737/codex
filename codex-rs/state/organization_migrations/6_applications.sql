-- A separate accepted-result application intent, never automatically replayed.
CREATE TABLE organization_applications (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES organization_tasks(id),
    digest TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending','completed','uncertain')),
    data TEXT NOT NULL CHECK(json_valid(data)),
    UNIQUE(task_id,digest)
);
