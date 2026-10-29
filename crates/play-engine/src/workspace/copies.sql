CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=3)
);
INSERT INTO workspace_meta_next SELECT id,generation,3 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
CREATE TABLE asset_origins (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, asset_id TEXT NOT NULL,
 source_asset_id TEXT NOT NULL, source_revision INTEGER NOT NULL CHECK(source_revision>0),
 PRIMARY KEY(workspace_id,exploration_id,asset_id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id) REFERENCES assets(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_asset_id,source_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 CHECK(asset_id<>source_asset_id)
);
CREATE TRIGGER immutable_asset_origin_update BEFORE UPDATE ON asset_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_asset_origin_delete BEFORE DELETE ON asset_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
PRAGMA user_version=3;
