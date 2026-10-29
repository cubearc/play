ALTER TABLE trails ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0);
CREATE TABLE pipeline_origins (
 workspace_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, source_pipeline_id TEXT NOT NULL, source_revision INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,pipeline_id),
 FOREIGN KEY(workspace_id,pipeline_id) REFERENCES pipelines(workspace_id,id),
 FOREIGN KEY(workspace_id,source_pipeline_id,source_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK(pipeline_id<>source_pipeline_id)
);
CREATE TRIGGER immutable_pipeline_origin_update BEFORE UPDATE ON pipeline_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_pipeline_origin_delete BEFORE DELETE ON pipeline_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=9)
);
INSERT INTO workspace_meta_next SELECT id,generation,9 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=9;
