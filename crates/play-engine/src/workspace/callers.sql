DROP TRIGGER result_origin_admission;
DROP TRIGGER run_inputs_admission;
DROP TRIGGER attempt_admission;
CREATE TABLE pipeline_runs_next (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL,
 trail_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, pipeline_revision INTEGER NOT NULL,
 intent_key TEXT NOT NULL, intent_digest TEXT NOT NULL, caller TEXT NOT NULL CHECK(caller IN ('desktop','cli')),
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed','cancelled','interrupted')),
 created_ms INTEGER NOT NULL, finished_ms INTEGER, retain_output INTEGER NOT NULL CHECK(retain_output IN (0,1)),
 plan TEXT NOT NULL, result TEXT, reservation BLOB NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,intent_key),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,trail_id) REFERENCES trails(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,pipeline_id,pipeline_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK((status IN ('queued','running') AND finished_ms IS NULL AND result IS NULL) OR (status NOT IN ('queued','running') AND finished_ms IS NOT NULL AND result IS NOT NULL AND length(reservation)=0))
);
INSERT INTO pipeline_runs_next(rowid,workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,status,created_ms,finished_ms,retain_output,plan,result,reservation) SELECT rowid,workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,status,created_ms,finished_ms,retain_output,plan,result,reservation FROM pipeline_runs;
DROP TABLE pipeline_runs;
ALTER TABLE pipeline_runs_next RENAME TO pipeline_runs;
CREATE INDEX pipeline_run_history ON pipeline_runs(workspace_id,exploration_id,trail_id,created_ms DESC,id);
CREATE TRIGGER immutable_runs_update BEFORE UPDATE ON pipeline_runs WHEN OLD.status NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_runs_delete BEFORE DELETE ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_run_plan BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,created_ms,retain_output,plan ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_plan'); END;
CREATE TRIGGER run_state_transition BEFORE UPDATE OF status ON pipeline_runs WHEN NOT ((OLD.status='queued' AND NEW.status IN ('running','failed','cancelled','interrupted')) OR (OLD.status='running' AND NEW.status IN ('completed','failed','cancelled','interrupted'))) BEGIN SELECT RAISE(ABORT,'invalid_run_transition'); END;
CREATE TRIGGER run_inputs_admission BEFORE INSERT ON run_inputs WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id)<>'queued' BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER attempt_admission BEFORE INSERT ON node_attempts WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id) NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER result_origin_admission BEFORE INSERT ON result_origins WHEN NOT EXISTS(
 SELECT 1 FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id)
 JOIN assets a ON (a.workspace_id,a.exploration_id,a.id)=(NEW.workspace_id,NEW.exploration_id,NEW.asset_id)
 WHERE r.workspace_id=NEW.workspace_id AND r.exploration_id=NEW.exploration_id AND r.id=NEW.run_id AND n.id=NEW.attempt_id
 AND r.status='completed' AND r.retain_output=1 AND n.status='completed'
 AND a.kind=CASE NEW.semantic_type WHEN 'text' THEN 'text' ELSE 'file' END
) BEGIN SELECT RAISE(ABORT,'invalid_result_origin'); END;
CREATE TABLE workspace_meta_next (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=12));
INSERT INTO workspace_meta_next SELECT id,generation,12 FROM workspace_meta;
DROP TABLE workspace_meta;
ALTER TABLE workspace_meta_next RENAME TO workspace_meta;
PRAGMA user_version=12;
