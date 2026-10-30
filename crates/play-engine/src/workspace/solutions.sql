CREATE TABLE solutions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE solution_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, solution_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0), candidate_id TEXT NOT NULL, candidate_revision INTEGER NOT NULL,
 title TEXT NOT NULL, explanation TEXT NOT NULL, snapshot TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,solution_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,solution_id) REFERENCES solutions(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,candidate_id,candidate_revision) REFERENCES candidate_revisions(workspace_id,exploration_id,candidate_id,revision)
);
CREATE TRIGGER solutions_immutable_update BEFORE UPDATE ON solutions BEGIN SELECT RAISE(ABORT,'immutable_solution'); END;
CREATE TRIGGER solutions_immutable_delete BEFORE DELETE ON solutions BEGIN SELECT RAISE(ABORT,'immutable_solution'); END;
CREATE TRIGGER solution_revisions_immutable_update BEFORE UPDATE ON solution_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER solution_revisions_immutable_delete BEFORE DELETE ON solution_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TABLE workspace_meta_next (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=15));
INSERT INTO workspace_meta_next SELECT id,generation,15 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=15;
