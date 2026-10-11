CREATE TABLE organization_shared_limits (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES organization_projects(id),
    run_id TEXT REFERENCES organization_workflow_runs(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    data TEXT NOT NULL CHECK(json_valid(data))
);
CREATE INDEX organization_shared_limits_project ON organization_shared_limits(project_id,run_id);

-- One projection serves reporting and candidate/admission checks. Root ledgers
-- already include descendants; summing attempts or child grants would double count.
CREATE VIEW organization_shared_usage AS
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
LEFT JOIN unresolved_runs ur ON ur.run_id=l.run_id;
