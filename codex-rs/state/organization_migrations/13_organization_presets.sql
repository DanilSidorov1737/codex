CREATE TABLE organization_presets (
    id TEXT PRIMARY KEY,
    revision INTEGER NOT NULL CHECK (revision > 0),
    data TEXT NOT NULL CHECK (json_valid(data))
);
