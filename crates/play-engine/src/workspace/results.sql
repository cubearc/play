CREATE TABLE result_origins (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, asset_id TEXT NOT NULL,
 run_id TEXT NOT NULL, attempt_id TEXT NOT NULL, port TEXT NOT NULL CHECK(length(port) BETWEEN 1 AND 64),
 semantic_type TEXT NOT NULL CHECK(semantic_type IN ('text','bytes')),
 PRIMARY KEY(workspace_id,exploration_id,asset_id), UNIQUE(workspace_id,exploration_id,run_id,attempt_id,port),
 FOREIGN KEY(workspace_id,exploration_id,asset_id) REFERENCES assets(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id)
);
CREATE TRIGGER immutable_result_origin_update BEFORE UPDATE ON result_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_origin_delete BEFORE DELETE ON result_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_revision BEFORE INSERT ON asset_revisions WHEN EXISTS(SELECT 1 FROM result_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER exclusive_result_origin BEFORE INSERT ON result_origins WHEN EXISTS(SELECT 1 FROM asset_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'ambiguous_origin'); END;
CREATE TRIGGER exclusive_asset_origin BEFORE INSERT ON asset_origins WHEN EXISTS(SELECT 1 FROM result_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'ambiguous_origin'); END;
CREATE TRIGGER result_origin_admission BEFORE INSERT ON result_origins WHEN NOT EXISTS(
 SELECT 1 FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id)
 JOIN assets a ON (a.workspace_id,a.exploration_id,a.id)=(NEW.workspace_id,NEW.exploration_id,NEW.asset_id)
 WHERE r.workspace_id=NEW.workspace_id AND r.exploration_id=NEW.exploration_id AND r.id=NEW.run_id AND n.id=NEW.attempt_id
 AND r.status='completed' AND r.retain_output=1 AND n.status='completed'
 AND a.kind=CASE NEW.semantic_type WHEN 'text' THEN 'text' ELSE 'file' END
) BEGIN SELECT RAISE(ABORT,'invalid_result_origin'); END;
CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=6)
);
INSERT INTO workspace_meta_next SELECT id,generation,6 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=6;
