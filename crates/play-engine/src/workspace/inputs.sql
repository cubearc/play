CREATE TABLE trail_input_bindings (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, trail_id TEXT NOT NULL, slot TEXT NOT NULL CHECK(slot<>'source'), asset_id TEXT NOT NULL, asset_revision INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,trail_id,slot),
 FOREIGN KEY(workspace_id,exploration_id,trail_id) REFERENCES trails(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision)
);
CREATE TRIGGER trail_input_scope BEFORE INSERT ON trail_input_bindings WHEN NOT EXISTS (
 SELECT 1 FROM trails t JOIN blocks b ON (t.workspace_id,t.exploration_id,t.stage_id)=(b.workspace_id,b.exploration_id,b.stage_id)
 WHERE t.workspace_id=NEW.workspace_id AND t.exploration_id=NEW.exploration_id AND t.id=NEW.trail_id AND b.archived_revision IS NULL AND (b.asset_id=NEW.asset_id OR b.caption_id=NEW.asset_id)
) BEGIN SELECT RAISE(ABORT,'invalid_binding'); END;
CREATE TRIGGER trail_input_replace BEFORE UPDATE ON trail_input_bindings BEGIN SELECT RAISE(ABORT,'replace_binding'); END;
CREATE TABLE workspace_meta_next (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=11));
INSERT INTO workspace_meta_next SELECT id,generation,11 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=11;
