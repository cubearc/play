ALTER TABLE blocks ADD COLUMN archived_revision INTEGER CHECK(archived_revision IS NULL OR (archived_revision>0 AND kind='asset'));
CREATE INDEX blocks_archive ON blocks(workspace_id,exploration_id,stage_id,archived_revision DESC,id);
CREATE TRIGGER preserve_blocks BEFORE DELETE ON blocks BEGIN SELECT RAISE(ABORT,'preserved_block'); END;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=4)
);
INSERT INTO workspace_meta_next SELECT id,generation,4 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=4;
