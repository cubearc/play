CREATE TABLE workspace_meta_next (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=2)
);
INSERT INTO workspace_meta_next SELECT id,generation,2 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
CREATE TABLE pipeline_runs (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL,
 trail_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, pipeline_revision INTEGER NOT NULL,
 intent_key TEXT NOT NULL, intent_digest TEXT NOT NULL, caller TEXT NOT NULL CHECK(caller='desktop'),
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed','cancelled','interrupted')),
 created_ms INTEGER NOT NULL, finished_ms INTEGER, retain_output INTEGER NOT NULL CHECK(retain_output IN (0,1)),
 plan TEXT NOT NULL, result TEXT, reservation BLOB NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,intent_key),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,trail_id) REFERENCES trails(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,pipeline_id,pipeline_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK((status IN ('queued','running') AND finished_ms IS NULL AND result IS NULL) OR (status NOT IN ('queued','running') AND finished_ms IS NOT NULL AND result IS NOT NULL AND length(reservation)=0))
);
CREATE INDEX pipeline_run_history ON pipeline_runs(workspace_id,exploration_id,trail_id,created_ms DESC,id);
CREATE TABLE run_inputs (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, port TEXT NOT NULL,
 asset_id TEXT NOT NULL, asset_revision INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,run_id,port),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision)
);
CREATE TABLE node_attempts (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, id TEXT NOT NULL, node_id TEXT NOT NULL,
 attempt INTEGER NOT NULL CHECK(attempt>0), started_ms INTEGER NOT NULL, finished_ms INTEGER,
 status TEXT NOT NULL CHECK(status IN ('running','completed','failed','cancelled','interrupted')),
 PRIMARY KEY(workspace_id,exploration_id,run_id,id), UNIQUE(workspace_id,exploration_id,run_id,node_id,attempt),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 CHECK((status='running' AND finished_ms IS NULL) OR (status<>'running' AND finished_ms IS NOT NULL))
);
CREATE TRIGGER immutable_runs_update BEFORE UPDATE ON pipeline_runs WHEN OLD.status NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_runs_delete BEFORE DELETE ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_attempts_update BEFORE UPDATE ON node_attempts WHEN OLD.status<>'running' BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_attempts_delete BEFORE DELETE ON node_attempts BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_run_inputs_update BEFORE UPDATE ON run_inputs BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER immutable_run_inputs_delete BEFORE DELETE ON run_inputs BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
PRAGMA user_version=2;
CREATE TRIGGER immutable_run_plan BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,created_ms,retain_output,plan ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_plan'); END;
CREATE TRIGGER immutable_attempt_identity BEFORE UPDATE OF workspace_id,exploration_id,run_id,id,node_id,attempt,started_ms ON node_attempts BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER run_state_transition BEFORE UPDATE OF status ON pipeline_runs WHEN NOT ((OLD.status='queued' AND NEW.status IN ('running','failed','cancelled','interrupted')) OR (OLD.status='running' AND NEW.status IN ('completed','failed','cancelled','interrupted'))) BEGIN SELECT RAISE(ABORT,'invalid_run_transition'); END;
CREATE TRIGGER run_inputs_admission BEFORE INSERT ON run_inputs WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id)<>'queued' BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER attempt_admission BEFORE INSERT ON node_attempts WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id) NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
