CREATE TABLE organization_controls (
 id TEXT PRIMARY KEY, revision INTEGER NOT NULL CHECK(revision>0),
 data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE TABLE organization_control_history (
 id TEXT PRIMARY KEY, control_id TEXT NOT NULL REFERENCES organization_controls(id),
 revision INTEGER NOT NULL CHECK(revision>0), data TEXT NOT NULL CHECK(json_valid(data)),
 UNIQUE(control_id,revision)
);
ALTER TABLE organization_tasks ADD COLUMN queue_since INTEGER;
-- Use existing owner admission timestamps for historical queued roots.
UPDATE organization_tasks SET queue_since=(
 SELECT min(e.created_at) FROM organization_audit e
 JOIN organization_authorizations a ON a.id=json_extract(e.payload,'$.id')
 WHERE a.task_id=organization_tasks.id AND e.operation='task/authorize'
) WHERE authorization_id IS NOT NULL;
CREATE VIEW organization_global_usage AS
SELECT
 (SELECT COALESCE(sum(used_tokens),0) FROM organization_authorizations WHERE root_id=id) reported_tokens,
 (SELECT COALESCE(sum(model_requests),0) FROM organization_authorizations WHERE root_id=id) model_requests,
 (SELECT count(*) FROM organization_attempts WHERE occupied=1) active,
 (SELECT COALESCE(sum(json_extract(data,'$.modelRequests')-COALESCE(json_extract(data,'$.reportedModelRequests'),0)),0)
  FROM organization_attempts WHERE state NOT IN ('running','waitingApproval','waitingDependency')
  AND json_extract(data,'$.modelRequests')>COALESCE(json_extract(data,'$.reportedModelRequests'),0)) unresolved_requests;
