CREATE TABLE challenges (
    key TEXT PRIMARY KEY CHECK(length(key)=64),
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    definition_json TEXT NOT NULL CHECK(length(CAST(definition_json AS BLOB)) <= 524288),
    stage_id TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
    created_ms INTEGER NOT NULL
);
ALTER TABLE investigation ADD COLUMN active_challenge TEXT REFERENCES challenges(key);
