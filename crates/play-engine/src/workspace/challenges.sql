CREATE TABLE challenge_imports (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, origin TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id),
 FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
CREATE TRIGGER challenge_imports_immutable_update BEFORE UPDATE ON challenge_imports BEGIN SELECT RAISE(ABORT,'immutable_import'); END;
CREATE TRIGGER challenge_imports_immutable_delete BEFORE DELETE ON challenge_imports BEGIN SELECT RAISE(ABORT,'immutable_import'); END;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=13)
);
INSERT INTO workspace_meta_next SELECT id,generation,13 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=13;
