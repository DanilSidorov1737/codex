-- Expand the native note discriminator while preserving immutable note history.
ALTER TABLE organization_notes RENAME TO organization_notes_v1;
CREATE TABLE organization_notes (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES organization_projects(id),
 author_id TEXT NOT NULL REFERENCES organization_nodes(id),
 scope TEXT NOT NULL CHECK(scope IN ('project','agent','team')),
 group_id TEXT REFERENCES organization_nodes(id),
 superseded_by TEXT REFERENCES organization_notes(id),
 data TEXT NOT NULL CHECK(json_valid(data)),
 CHECK((scope='team')=(group_id IS NOT NULL))
);
INSERT INTO organization_notes(id,project_id,author_id,scope,data)
 SELECT id,project_id,author_id,scope,data FROM organization_notes_v1;
DROP TABLE organization_notes_v1;
CREATE INDEX organization_notes_scope ON organization_notes(project_id,scope,author_id,group_id);
