CREATE TABLE assets_next (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('text','image','file')),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,id,kind), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO assets_next SELECT * FROM assets;
DROP TABLE assets;
ALTER TABLE assets_next RENAME TO assets;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=5)
);
INSERT INTO workspace_meta_next SELECT id,generation,5 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=5;
