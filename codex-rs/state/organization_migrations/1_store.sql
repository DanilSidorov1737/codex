-- Canonical business state is never rebuilt from transcripts.
CREATE TABLE organization_store (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    database_id TEXT NOT NULL UNIQUE
);
INSERT INTO organization_store VALUES (1, lower(hex(randomblob(16))));
