CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    path TEXT NOT NULL UNIQUE CHECK (length(CAST(path AS BLOB)) BETWEEN 1 AND 255),
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    status TEXT NOT NULL CHECK (status IN ('pending', 'ready', 'missing')),
    temporary TEXT,
    created_ms INTEGER NOT NULL CHECK (created_ms >= 0)
);
