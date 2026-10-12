CREATE TABLE organization_workspace_archives (
 attempt_id TEXT PRIMARY KEY REFERENCES organization_attempts(id),
 original_path TEXT NOT NULL,
 archive_path TEXT NOT NULL,
 tree_sha256 TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('planned','completed','uncertain'))
);
