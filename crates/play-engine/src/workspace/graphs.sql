CREATE TABLE node_results (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, attempt_id TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,run_id,attempt_id),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id)
);
CREATE TRIGGER immutable_node_result_update BEFORE UPDATE ON node_results BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER immutable_node_result_delete BEFORE DELETE ON node_results BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER node_result_admission BEFORE INSERT ON node_results WHEN (SELECT status FROM node_attempts WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND run_id=NEW.run_id AND id=NEW.attempt_id)<>'running' BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TABLE workspace_meta_next (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=10));
INSERT INTO workspace_meta_next SELECT id,generation,10 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=10;
