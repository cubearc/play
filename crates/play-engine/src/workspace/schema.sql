PRAGMA application_id = 1129664590;
PRAGMA user_version = 1;
CREATE TABLE workspace_meta (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=1)
);
CREATE TABLE explorations (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,id)
);
CREATE TABLE stages (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), position INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
CREATE TABLE notes (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,stage_id,id), UNIQUE(workspace_id,exploration_id,stage_id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE note_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, note_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), text TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,note_id,revision), FOREIGN KEY(workspace_id,exploration_id,note_id) REFERENCES notes(workspace_id,exploration_id,id)
);
CREATE TABLE assets (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('text','image')),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,id,kind), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
CREATE TABLE asset_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, asset_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), data BLOB NOT NULL, digest TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,asset_id,revision), FOREIGN KEY(workspace_id,exploration_id,asset_id) REFERENCES assets(workspace_id,exploration_id,id)
);
CREATE TABLE blocks (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, position INTEGER NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('asset','note')), asset_id TEXT, asset_revision INTEGER, caption_id TEXT, caption_revision INTEGER, caption_kind TEXT NOT NULL DEFAULT 'text' CHECK(caption_kind='text'), note_id TEXT,
 PRIMARY KEY(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,caption_id,caption_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,caption_id,caption_kind) REFERENCES assets(workspace_id,exploration_id,id,kind),
 FOREIGN KEY(workspace_id,exploration_id,stage_id,note_id) REFERENCES notes(workspace_id,exploration_id,stage_id,id),
 CHECK((kind='asset' AND asset_id IS NOT NULL AND asset_revision IS NOT NULL AND note_id IS NULL) OR (kind='note' AND note_id IS NOT NULL AND asset_id IS NULL AND caption_id IS NULL)),
 CHECK((caption_id IS NULL AND caption_revision IS NULL) OR (caption_id IS NOT NULL AND caption_revision IS NOT NULL))
);
CREATE INDEX blocks_order ON blocks(workspace_id,exploration_id,stage_id,position,id);
CREATE TABLE pipelines (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 PRIMARY KEY(workspace_id,id)
);
CREATE TABLE pipeline_revisions (
 workspace_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), definition TEXT NOT NULL,
 PRIMARY KEY(workspace_id,pipeline_id,revision), FOREIGN KEY(workspace_id,pipeline_id) REFERENCES pipelines(workspace_id,id)
);
CREATE TABLE trails (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, hypothesis TEXT NOT NULL,
 source_id TEXT NOT NULL, source_revision INTEGER NOT NULL, pipeline_id TEXT, pipeline_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_id,source_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,pipeline_id,pipeline_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK((pipeline_id IS NULL AND pipeline_revision IS NULL) OR (pipeline_id IS NOT NULL AND pipeline_revision IS NOT NULL))
);
CREATE TABLE view_state (
 workspace_id TEXT NOT NULL, slot TEXT NOT NULL CHECK(slot='primary'), exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,slot), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE exploration_view_state (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE receipts (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), command TEXT NOT NULL, key TEXT NOT NULL, digest TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,command,key)
);
CREATE TRIGGER immutable_assets_update BEFORE UPDATE ON asset_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_assets_delete BEFORE DELETE ON asset_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_notes_update BEFORE UPDATE ON note_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_notes_delete BEFORE DELETE ON note_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_pipelines_update BEFORE UPDATE ON pipeline_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_pipelines_delete BEFORE DELETE ON pipeline_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
