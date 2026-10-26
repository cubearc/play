PRAGMA application_id = 1129335897;

CREATE TABLE workspace (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    id TEXT NOT NULL UNIQUE,
    created_ms INTEGER NOT NULL CHECK (created_ms >= 0)
);
CREATE TABLE investigation (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    id TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL REFERENCES workspace(id),
    title TEXT NOT NULL CHECK (length(CAST(title AS BLOB)) BETWEEN 1 AND 512),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    view_json TEXT NOT NULL DEFAULT '{"store":"source","path":"","offset":0,"selected":null}',
    view_revision INTEGER NOT NULL DEFAULT 0 CHECK (view_revision >= 0),
    created_ms INTEGER NOT NULL CHECK (created_ms >= 0),
    updated_ms INTEGER NOT NULL CHECK (updated_ms >= 0),
    CHECK (length(CAST(view_json AS BLOB)) <= 12288)
);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    started_ms INTEGER NOT NULL CHECK (started_ms >= 0),
    ended_ms INTEGER,
    status TEXT NOT NULL CHECK (status IN ('running', 'closed', 'interrupted'))
);
CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    title TEXT NOT NULL CHECK (length(CAST(title AS BLOB)) BETWEEN 1 AND 512),
    body TEXT NOT NULL CHECK (length(CAST(body AS BLOB)) <= 65536),
    revision INTEGER NOT NULL CHECK (revision > 0),
    updated_ms INTEGER NOT NULL CHECK (updated_ms >= 0)
);
CREATE TABLE progress (
    challenge_key TEXT NOT NULL CHECK (length(CAST(challenge_key AS BLOB)) BETWEEN 1 AND 128),
    stage_key TEXT NOT NULL CHECK (length(CAST(stage_key AS BLOB)) BETWEEN 1 AND 128),
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    local_status TEXT NOT NULL CHECK (local_status IN ('unstarted', 'in_progress', 'completed')),
    revision INTEGER NOT NULL CHECK (revision > 0),
    updated_ms INTEGER NOT NULL CHECK (updated_ms >= 0),
    PRIMARY KEY (challenge_key, stage_key)
);
