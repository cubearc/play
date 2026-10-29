CREATE TABLE notes_next (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 kind TEXT NOT NULL CHECK(kind IN ('reasoning','annotation')), title TEXT NOT NULL,
 source_note_id TEXT, source_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_note_id,source_revision) REFERENCES note_revisions(workspace_id,exploration_id,note_id,revision),
 CHECK((source_note_id IS NULL AND source_revision IS NULL) OR (kind='annotation' AND source_note_id IS NOT NULL AND source_revision>0 AND source_note_id<>id))
);
INSERT INTO notes_next SELECT workspace_id,exploration_id,stage_id,id,current_revision,'reasoning','Reasoning',NULL,NULL FROM notes;
DROP TABLE notes;
ALTER TABLE notes_next RENAME TO notes;
CREATE UNIQUE INDEX one_reasoning_note ON notes(workspace_id,exploration_id,stage_id) WHERE kind='reasoning';
CREATE TRIGGER immutable_note_identity BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,kind,title,source_note_id,source_revision ON notes BEGIN SELECT RAISE(ABORT,'immutable_note_identity'); END;
CREATE TABLE blocks_next (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, position INTEGER NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('asset','note')), asset_id TEXT, asset_revision INTEGER, caption_id TEXT, caption_revision INTEGER, caption_kind TEXT NOT NULL DEFAULT 'text' CHECK(caption_kind='text'), note_id TEXT,
 archived_revision INTEGER CHECK(archived_revision IS NULL OR archived_revision>0),
 PRIMARY KEY(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,caption_id,caption_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,caption_id,caption_kind) REFERENCES assets(workspace_id,exploration_id,id,kind),
 FOREIGN KEY(workspace_id,exploration_id,stage_id,note_id) REFERENCES notes(workspace_id,exploration_id,stage_id,id),
 CHECK((kind='asset' AND asset_id IS NOT NULL AND asset_revision IS NOT NULL AND note_id IS NULL) OR (kind='note' AND note_id IS NOT NULL AND asset_id IS NULL AND caption_id IS NULL)),
 CHECK((caption_id IS NULL AND caption_revision IS NULL) OR (caption_id IS NOT NULL AND caption_revision IS NOT NULL))
);
INSERT INTO blocks_next SELECT * FROM blocks;
DROP TABLE blocks;
ALTER TABLE blocks_next RENAME TO blocks;
CREATE INDEX blocks_order ON blocks(workspace_id,exploration_id,stage_id,position,id);
CREATE INDEX blocks_archive ON blocks(workspace_id,exploration_id,stage_id,archived_revision DESC,id);
CREATE TRIGGER preserve_blocks BEFORE DELETE ON blocks BEGIN SELECT RAISE(ABORT,'preserved_block'); END;
CREATE TRIGGER reasoning_archive_update BEFORE UPDATE ON blocks WHEN NEW.archived_revision IS NOT NULL AND NEW.kind='note' AND EXISTS(SELECT 1 FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id AND kind='reasoning') BEGIN SELECT RAISE(ABORT,'protected_reasoning'); END;
CREATE TRIGGER reasoning_archive_insert BEFORE INSERT ON blocks WHEN NEW.archived_revision IS NOT NULL AND NEW.kind='note' AND EXISTS(SELECT 1 FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id AND kind='reasoning') BEGIN SELECT RAISE(ABORT,'protected_reasoning'); END;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=7)
);
INSERT INTO workspace_meta_next SELECT id,generation,7 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=7;
