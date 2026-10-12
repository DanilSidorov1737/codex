CREATE TABLE organization_token_reservations (
 attempt_id TEXT NOT NULL REFERENCES organization_attempts(id),
 sequence INTEGER NOT NULL CHECK(sequence>0),
 root_id TEXT NOT NULL REFERENCES organization_authorizations(id),
 tokens INTEGER NOT NULL CHECK(tokens>0),
 released INTEGER NOT NULL DEFAULT 0 CHECK(released IN (0,1)),
 PRIMARY KEY(attempt_id,sequence)
);
-- An older host admitted these requests without a reservation. Preserve their
-- unknown effects and conservatively fence their remaining root allowance.
INSERT INTO organization_token_reservations(attempt_id,sequence,root_id,tokens)
SELECT p.id,json_extract(p.data,'$.modelRequests'),a.root_id,
 max(1,json_extract(r.data,'$.scope.maxTokens')-r.used_tokens)
FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id
JOIN organization_authorizations r ON r.id=a.root_id
WHERE json_extract(p.data,'$.modelRequests')>COALESCE(json_extract(p.data,'$.reportedModelRequests'),0);
