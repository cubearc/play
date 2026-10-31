CREATE TABLE note_targets (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, note_id TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('block','asset','attempt','run_output')), label TEXT NOT NULL,
 block_id TEXT, asset_id TEXT, asset_revision INTEGER, run_id TEXT, attempt_id TEXT, port TEXT,
 PRIMARY KEY(workspace_id,exploration_id,note_id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id,note_id) REFERENCES notes(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id,block_id) REFERENCES blocks(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id),
 CHECK((kind='block' AND block_id IS NOT NULL AND asset_id IS NULL AND asset_revision IS NULL AND run_id IS NULL AND attempt_id IS NULL AND port IS NULL) OR
 (kind='asset' AND block_id IS NULL AND asset_id IS NOT NULL AND asset_revision IS NOT NULL AND asset_revision>0 AND run_id IS NULL AND attempt_id IS NULL AND port IS NULL) OR
 (kind IN ('attempt','run_output') AND block_id IS NULL AND asset_id IS NULL AND asset_revision IS NULL AND run_id IS NOT NULL AND attempt_id IS NOT NULL AND ((kind='attempt' AND port IS NULL) OR (kind='run_output' AND port IS NOT NULL AND length(port) BETWEEN 1 AND 64))))
);
CREATE TRIGGER note_targets_annotation BEFORE INSERT ON note_targets WHEN (SELECT kind FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id)<>'annotation' BEGIN SELECT RAISE(ABORT,'invalid_note_target'); END;
CREATE TRIGGER note_targets_immutable_update BEFORE UPDATE ON note_targets BEGIN SELECT RAISE(ABORT,'immutable_note_target'); END;
CREATE TRIGGER note_targets_immutable_delete BEFORE DELETE ON note_targets BEGIN SELECT RAISE(ABORT,'immutable_note_target'); END;
CREATE TABLE workspace_meta_next (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=16));
INSERT INTO workspace_meta_next SELECT id,generation,16 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=16;
