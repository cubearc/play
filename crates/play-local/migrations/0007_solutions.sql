CREATE TABLE solutions (
    id TEXT PRIMARY KEY,
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    updated_ms INTEGER NOT NULL
);
CREATE TABLE solution_revisions (
    solution_id TEXT NOT NULL REFERENCES solutions(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    snapshot_json TEXT NOT NULL CHECK(length(CAST(snapshot_json AS BLOB)) <= 524288),
    created_ms INTEGER NOT NULL,
    PRIMARY KEY(solution_id,revision)
);
CREATE TRIGGER immutable_solution_revision_update
BEFORE UPDATE ON solution_revisions BEGIN
    SELECT RAISE(ABORT,'solution revisions are immutable');
END;
CREATE TRIGGER immutable_solution_revision_delete
BEFORE DELETE ON solution_revisions BEGIN
    SELECT RAISE(ABORT,'solution revisions are immutable');
END;
