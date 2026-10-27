CREATE TABLE explorations (
    id TEXT PRIMARY KEY,
    investigation_id TEXT NOT NULL REFERENCES investigation(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    created_ms INTEGER NOT NULL,
    updated_ms INTEGER NOT NULL
);
CREATE TABLE exploration_configurations (
    exploration_id TEXT NOT NULL REFERENCES explorations(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    definition_json TEXT NOT NULL CHECK(length(CAST(definition_json AS BLOB)) <= 65536),
    created_ms INTEGER NOT NULL,
    PRIMARY KEY(exploration_id, revision)
);
CREATE TABLE exploration_executions (
    id TEXT PRIMARY KEY,
    exploration_id TEXT NOT NULL,
    configuration_revision INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','succeeded','failed','cancelled','interrupted')),
    evidence_json TEXT NOT NULL DEFAULT '{}' CHECK(length(CAST(evidence_json AS BLOB)) <= 32768),
    started_ms INTEGER NOT NULL,
    ended_ms INTEGER,
    FOREIGN KEY(exploration_id, configuration_revision) REFERENCES exploration_configurations(exploration_id, revision),
    CHECK((status='running' AND ended_ms IS NULL) OR (status!='running' AND ended_ms IS NOT NULL))
);
CREATE TRIGGER immutable_exploration_configuration
BEFORE UPDATE ON exploration_configurations BEGIN
    SELECT RAISE(ABORT,'exploration configurations are immutable');
END;
CREATE TRIGGER immutable_finished_exploration_execution
BEFORE UPDATE ON exploration_executions WHEN OLD.status!='running' BEGIN
    SELECT RAISE(ABORT,'finished exploration executions are immutable');
END;
CREATE INDEX exploration_execution_lineage ON exploration_executions(exploration_id, configuration_revision, step_id, started_ms);
