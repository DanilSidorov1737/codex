ALTER TABLE organization_shared_limits ADD COLUMN series_id TEXT REFERENCES organization_workflow_series(id);
CREATE TABLE organization_workflow_series (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES organization_projects(id),
 state TEXT NOT NULL, next_run_at INTEGER, data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_workflow_series_due ON organization_workflow_series(state,next_run_at,id);
CREATE TABLE organization_workflow_occurrences (
 series_id TEXT NOT NULL REFERENCES organization_workflow_series(id),
 due_at INTEGER NOT NULL, run_id TEXT NOT NULL UNIQUE REFERENCES organization_workflow_runs(id),
 PRIMARY KEY(series_id,due_at)
);
DROP VIEW organization_shared_usage;
CREATE VIEW organization_project_run_usage AS
WITH roots AS (
    SELECT a.id,t.project_id,a.used_tokens,a.model_requests
    FROM organization_authorizations a JOIN organization_tasks t ON t.id=a.task_id
    WHERE a.root_id=a.id
), projects AS (
    SELECT project_id,sum(used_tokens) tokens,sum(model_requests) requests
    FROM roots GROUP BY project_id
), runs AS (
    SELECT w.run_id,sum(r.used_tokens) tokens,sum(r.model_requests) requests
    FROM roots r JOIN organization_workflow_tasks w ON w.authorization_id=r.id GROUP BY w.run_id
), active_projects AS (
    SELECT t.project_id,count(*) active FROM organization_attempts p
    JOIN organization_tasks t ON t.id=p.task_id WHERE p.occupied=1 GROUP BY t.project_id
), active_runs AS (
    SELECT w.run_id,count(*) active FROM organization_attempts p
    JOIN organization_authorizations a ON a.id=p.authorization_id
    JOIN organization_workflow_tasks w ON w.authorization_id=a.root_id
    WHERE p.occupied=1 GROUP BY w.run_id
), unresolved AS (
    SELECT p.id,t.project_id,a.root_id,
        json_extract(p.data,'$.modelRequests')-COALESCE(json_extract(p.data,'$.reportedModelRequests'),0) pending
    FROM organization_attempts p JOIN organization_tasks t ON t.id=p.task_id
    JOIN organization_authorizations a ON a.id=p.authorization_id
    WHERE p.state NOT IN ('running','waitingApproval','waitingDependency')
        AND json_extract(p.data,'$.modelRequests')>COALESCE(json_extract(p.data,'$.reportedModelRequests'),0)
), unresolved_projects AS (
    SELECT project_id,sum(pending) pending FROM unresolved GROUP BY project_id
), unresolved_runs AS (
    SELECT w.run_id,sum(u.pending) pending FROM unresolved u
    JOIN organization_workflow_tasks w ON w.authorization_id=u.root_id GROUP BY w.run_id
)
SELECT l.id,l.project_id,l.run_id,
    CASE WHEN l.run_id IS NULL THEN COALESCE(p.tokens,0) ELSE COALESCE(r.tokens,0) END reported_tokens,
    CASE WHEN l.run_id IS NULL THEN COALESCE(p.requests,0) ELSE COALESCE(r.requests,0) END model_requests,
    CASE WHEN l.run_id IS NULL THEN COALESCE(ap.active,0) ELSE COALESCE(ar.active,0) END active,
    CASE WHEN l.run_id IS NULL THEN COALESCE(up.pending,0) ELSE COALESCE(ur.pending,0) END unresolved_requests,
    json_extract(l.data,'$.maxTokens') max_tokens,
    json_extract(l.data,'$.maxModelRequests') max_model_requests,
    json_extract(l.data,'$.maxActive') max_active
FROM organization_shared_limits l
LEFT JOIN projects p ON p.project_id=l.project_id
LEFT JOIN runs r ON r.run_id=l.run_id
LEFT JOIN active_projects ap ON ap.project_id=l.project_id
LEFT JOIN active_runs ar ON ar.run_id=l.run_id
LEFT JOIN unresolved_projects up ON up.project_id=l.project_id
LEFT JOIN unresolved_runs ur ON ur.run_id=l.run_id
WHERE l.series_id IS NULL;
CREATE VIEW organization_shared_usage AS
WITH roots AS (
 SELECT o.series_id,a.id,a.used_tokens,a.model_requests FROM organization_workflow_occurrences o
 JOIN organization_workflow_tasks w ON w.run_id=o.run_id
 JOIN organization_authorizations a ON a.id=w.authorization_id AND a.root_id=a.id
), totals AS (
 SELECT series_id,sum(used_tokens) tokens,sum(model_requests) requests FROM roots GROUP BY series_id
), activity AS (
 SELECT r.series_id,count(*) active FROM roots r JOIN organization_authorizations a ON a.root_id=r.id
 JOIN organization_attempts p ON p.authorization_id=a.id WHERE p.occupied=1 GROUP BY r.series_id
), unresolved AS (
 SELECT r.series_id,sum(json_extract(p.data,'$.modelRequests')-COALESCE(json_extract(p.data,'$.reportedModelRequests'),0)) pending
 FROM roots r JOIN organization_authorizations a ON a.root_id=r.id JOIN organization_attempts p ON p.authorization_id=a.id
 WHERE p.state NOT IN ('running','waitingApproval','waitingDependency')
 AND json_extract(p.data,'$.modelRequests')>COALESCE(json_extract(p.data,'$.reportedModelRequests'),0) GROUP BY r.series_id
)
SELECT *,NULL series_id FROM organization_project_run_usage
UNION ALL
SELECT l.id,l.project_id,l.run_id,COALESCE(t.tokens,0),COALESCE(t.requests,0),COALESCE(a.active,0),COALESCE(u.pending,0),
 json_extract(l.data,'$.maxTokens'),json_extract(l.data,'$.maxModelRequests'),json_extract(l.data,'$.maxActive'),l.series_id
FROM organization_shared_limits l LEFT JOIN totals t ON t.series_id=l.series_id
LEFT JOIN activity a ON a.series_id=l.series_id LEFT JOIN unresolved u ON u.series_id=l.series_id
WHERE l.series_id IS NOT NULL;
