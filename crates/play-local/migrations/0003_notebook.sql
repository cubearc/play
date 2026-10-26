ALTER TABLE notes ADD COLUMN target_json TEXT CHECK (target_json IS NULL OR length(CAST(target_json AS BLOB)) <= 8192);
CREATE TABLE saved_results (
    id TEXT PRIMARY KEY,
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    title TEXT NOT NULL CHECK (length(CAST(title AS BLOB)) BETWEEN 1 AND 512),
    revision INTEGER NOT NULL CHECK (revision > 0),
    updated_ms INTEGER NOT NULL CHECK (updated_ms >= 0)
);
