CREATE TABLE candidates (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,stage_id), UNIQUE(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE candidate_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, candidate_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0),
 asset_id TEXT, asset_revision INTEGER, run_id TEXT, attempt_id TEXT, port TEXT,
 PRIMARY KEY(workspace_id,exploration_id,candidate_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,candidate_id) REFERENCES candidates(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id),
 CHECK((asset_id IS NOT NULL AND asset_revision IS NOT NULL AND asset_revision>0 AND run_id IS NULL AND attempt_id IS NULL AND port IS NULL) OR
 (asset_id IS NULL AND asset_revision IS NULL AND run_id IS NOT NULL AND attempt_id IS NOT NULL AND port IS NOT NULL AND length(port) BETWEEN 1 AND 64))
);
CREATE TRIGGER candidates_immutable_update BEFORE UPDATE ON candidates BEGIN SELECT RAISE(ABORT,'immutable_candidate'); END;
CREATE TRIGGER candidates_immutable_delete BEFORE DELETE ON candidates BEGIN SELECT RAISE(ABORT,'immutable_candidate'); END;
CREATE TRIGGER candidate_revisions_immutable_update BEFORE UPDATE ON candidate_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER candidate_revisions_immutable_delete BEFORE DELETE ON candidate_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=14)
);
INSERT INTO workspace_meta_next SELECT id,generation,14 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=14;
