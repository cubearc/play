PRAGMA application_id=1129664590;
PRAGMA user_version=16;
CREATE TABLE asset_origins (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, asset_id TEXT NOT NULL,
 source_asset_id TEXT NOT NULL, source_revision INTEGER NOT NULL CHECK(source_revision>0),
 PRIMARY KEY(workspace_id,exploration_id,asset_id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id) REFERENCES assets(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_asset_id,source_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 CHECK(asset_id<>source_asset_id)
);
CREATE TABLE asset_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, asset_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), data BLOB NOT NULL, digest TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,asset_id,revision), FOREIGN KEY(workspace_id,exploration_id,asset_id) REFERENCES assets(workspace_id,exploration_id,id)
);
CREATE TABLE "assets" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('text','image','file','page')),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,id,kind), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
CREATE TABLE "blocks" (
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
CREATE TABLE candidate_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, candidate_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0),
 asset_id TEXT, asset_revision INTEGER, run_id TEXT, attempt_id TEXT, port TEXT,
 PRIMARY KEY(workspace_id,exploration_id,candidate_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,candidate_id) REFERENCES candidates(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id),
 CHECK((asset_id IS NOT NULL AND asset_revision IS NOT NULL AND asset_revision>0 AND run_id IS NULL AND attempt_id IS NULL AND port IS NULL) OR
 (asset_id IS NULL AND asset_revision IS NULL AND run_id IS NOT NULL AND attempt_id IS NOT NULL AND port IS NOT NULL AND length(port) BETWEEN 1 AND 64))
);
CREATE TABLE candidates (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,stage_id), UNIQUE(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE challenge_imports (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, origin TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id),
 FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
CREATE TABLE exploration_view_state (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE explorations (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,id)
);
CREATE TABLE node_attempts (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, id TEXT NOT NULL, node_id TEXT NOT NULL,
 attempt INTEGER NOT NULL CHECK(attempt>0), started_ms INTEGER NOT NULL, finished_ms INTEGER,
 status TEXT NOT NULL CHECK(status IN ('running','completed','failed','cancelled','interrupted')),
 PRIMARY KEY(workspace_id,exploration_id,run_id,id), UNIQUE(workspace_id,exploration_id,run_id,node_id,attempt),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 CHECK((status='running' AND finished_ms IS NULL) OR (status<>'running' AND finished_ms IS NOT NULL))
);
CREATE TABLE node_results (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, attempt_id TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,run_id,attempt_id),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id)
);
CREATE TABLE note_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, note_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), text TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,note_id,revision), FOREIGN KEY(workspace_id,exploration_id,note_id) REFERENCES notes(workspace_id,exploration_id,id)
);
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
CREATE TABLE "notes" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 kind TEXT NOT NULL CHECK(kind IN ('reasoning','annotation')), title TEXT NOT NULL,
 source_note_id TEXT, source_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_note_id,source_revision) REFERENCES note_revisions(workspace_id,exploration_id,note_id,revision),
 CHECK((source_note_id IS NULL AND source_revision IS NULL) OR (kind='annotation' AND source_note_id IS NOT NULL AND source_revision>0 AND source_note_id<>id))
);
CREATE TABLE pipeline_origins (
 workspace_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, source_pipeline_id TEXT NOT NULL, source_revision INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,pipeline_id),
 FOREIGN KEY(workspace_id,pipeline_id) REFERENCES pipelines(workspace_id,id),
 FOREIGN KEY(workspace_id,source_pipeline_id,source_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK(pipeline_id<>source_pipeline_id)
);
CREATE TABLE pipeline_revisions (
 workspace_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), definition TEXT NOT NULL,
 PRIMARY KEY(workspace_id,pipeline_id,revision), FOREIGN KEY(workspace_id,pipeline_id) REFERENCES pipelines(workspace_id,id)
);
CREATE TABLE "pipeline_runs" (
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
CREATE TABLE pipelines (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 PRIMARY KEY(workspace_id,id)
);
CREATE TABLE receipts (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), command TEXT NOT NULL, key TEXT NOT NULL, digest TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,command,key)
);
CREATE TABLE result_origins (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, asset_id TEXT NOT NULL,
 run_id TEXT NOT NULL, attempt_id TEXT NOT NULL, port TEXT NOT NULL CHECK(length(port) BETWEEN 1 AND 64),
 semantic_type TEXT NOT NULL CHECK(semantic_type IN ('text','bytes')),
 PRIMARY KEY(workspace_id,exploration_id,asset_id), UNIQUE(workspace_id,exploration_id,run_id,attempt_id,port),
 FOREIGN KEY(workspace_id,exploration_id,asset_id) REFERENCES assets(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id)
);
CREATE TABLE run_inputs (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, port TEXT NOT NULL,
 asset_id TEXT NOT NULL, asset_revision INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,run_id,port),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision)
);
CREATE TABLE solution_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, solution_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0), candidate_id TEXT NOT NULL, candidate_revision INTEGER NOT NULL,
 title TEXT NOT NULL, explanation TEXT NOT NULL, snapshot TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,solution_id,revision),
 FOREIGN KEY(workspace_id,exploration_id,solution_id) REFERENCES solutions(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,candidate_id,candidate_revision) REFERENCES candidate_revisions(workspace_id,exploration_id,candidate_id,revision)
);
CREATE TABLE solutions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
CREATE TABLE stages (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), position INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
CREATE TABLE trail_input_bindings (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, trail_id TEXT NOT NULL, slot TEXT NOT NULL CHECK(slot<>'source'), asset_id TEXT NOT NULL, asset_revision INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,trail_id,slot),
 FOREIGN KEY(workspace_id,exploration_id,trail_id) REFERENCES trails(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,asset_id,asset_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision)
);
CREATE TABLE trails (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, hypothesis TEXT NOT NULL,
 source_id TEXT NOT NULL, source_revision INTEGER NOT NULL, pipeline_id TEXT, pipeline_revision INTEGER, revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
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
CREATE TABLE "workspace_meta" (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=16));
CREATE INDEX blocks_archive ON blocks(workspace_id,exploration_id,stage_id,archived_revision DESC,id);
CREATE INDEX blocks_order ON blocks(workspace_id,exploration_id,stage_id,position,id);
CREATE UNIQUE INDEX one_reasoning_note ON notes(workspace_id,exploration_id,stage_id) WHERE kind='reasoning';
CREATE INDEX pipeline_run_history ON pipeline_runs(workspace_id,exploration_id,trail_id,created_ms DESC,id);
CREATE TRIGGER attempt_admission BEFORE INSERT ON node_attempts WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id) NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER candidate_revisions_immutable_delete BEFORE DELETE ON candidate_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER candidate_revisions_immutable_update BEFORE UPDATE ON candidate_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER candidates_immutable_delete BEFORE DELETE ON candidates BEGIN SELECT RAISE(ABORT,'immutable_candidate'); END;
CREATE TRIGGER candidates_immutable_update BEFORE UPDATE ON candidates BEGIN SELECT RAISE(ABORT,'immutable_candidate'); END;
CREATE TRIGGER challenge_imports_immutable_delete BEFORE DELETE ON challenge_imports BEGIN SELECT RAISE(ABORT,'immutable_import'); END;
CREATE TRIGGER challenge_imports_immutable_update BEFORE UPDATE ON challenge_imports BEGIN SELECT RAISE(ABORT,'immutable_import'); END;
CREATE TRIGGER exclusive_asset_origin BEFORE INSERT ON asset_origins WHEN EXISTS(SELECT 1 FROM result_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'ambiguous_origin'); END;
CREATE TRIGGER exclusive_result_origin BEFORE INSERT ON result_origins WHEN EXISTS(SELECT 1 FROM asset_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'ambiguous_origin'); END;
CREATE TRIGGER immutable_asset_origin_delete BEFORE DELETE ON asset_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_asset_origin_update BEFORE UPDATE ON asset_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_assets_delete BEFORE DELETE ON asset_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_assets_update BEFORE UPDATE ON asset_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_attempt_identity BEFORE UPDATE OF workspace_id,exploration_id,run_id,id,node_id,attempt,started_ms ON node_attempts BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_attempts_delete BEFORE DELETE ON node_attempts BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_attempts_update BEFORE UPDATE ON node_attempts WHEN OLD.status<>'running' BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_node_result_delete BEFORE DELETE ON node_results BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER immutable_node_result_update BEFORE UPDATE ON node_results BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER immutable_note_identity BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,kind,title,source_note_id,source_revision ON notes BEGIN SELECT RAISE(ABORT,'immutable_note_identity'); END;
CREATE TRIGGER immutable_notes_delete BEFORE DELETE ON note_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_notes_update BEFORE UPDATE ON note_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_pipeline_origin_delete BEFORE DELETE ON pipeline_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_pipeline_origin_update BEFORE UPDATE ON pipeline_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_pipelines_delete BEFORE DELETE ON pipeline_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_pipelines_update BEFORE UPDATE ON pipeline_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_result_origin_delete BEFORE DELETE ON result_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_origin_update BEFORE UPDATE ON result_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_revision BEFORE INSERT ON asset_revisions WHEN EXISTS(SELECT 1 FROM result_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER immutable_run_inputs_delete BEFORE DELETE ON run_inputs BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER immutable_run_inputs_update BEFORE UPDATE ON run_inputs BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER immutable_run_plan BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,created_ms,retain_output,plan ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_plan'); END;
CREATE TRIGGER immutable_runs_delete BEFORE DELETE ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_runs_update BEFORE UPDATE ON pipeline_runs WHEN OLD.status NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER node_result_admission BEFORE INSERT ON node_results WHEN (SELECT status FROM node_attempts WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND run_id=NEW.run_id AND id=NEW.attempt_id)<>'running' BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER note_targets_annotation BEFORE INSERT ON note_targets WHEN (SELECT kind FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id)<>'annotation' BEGIN SELECT RAISE(ABORT,'invalid_note_target'); END;
CREATE TRIGGER note_targets_immutable_delete BEFORE DELETE ON note_targets BEGIN SELECT RAISE(ABORT,'immutable_note_target'); END;
CREATE TRIGGER note_targets_immutable_update BEFORE UPDATE ON note_targets BEGIN SELECT RAISE(ABORT,'immutable_note_target'); END;
CREATE TRIGGER preserve_blocks BEFORE DELETE ON blocks BEGIN SELECT RAISE(ABORT,'preserved_block'); END;
CREATE TRIGGER reasoning_archive_insert BEFORE INSERT ON blocks WHEN NEW.archived_revision IS NOT NULL AND NEW.kind='note' AND EXISTS(SELECT 1 FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id AND kind='reasoning') BEGIN SELECT RAISE(ABORT,'protected_reasoning'); END;
CREATE TRIGGER reasoning_archive_update BEFORE UPDATE ON blocks WHEN NEW.archived_revision IS NOT NULL AND NEW.kind='note' AND EXISTS(SELECT 1 FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id AND kind='reasoning') BEGIN SELECT RAISE(ABORT,'protected_reasoning'); END;
CREATE TRIGGER result_origin_admission BEFORE INSERT ON result_origins WHEN NOT EXISTS(
 SELECT 1 FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id)
 JOIN assets a ON (a.workspace_id,a.exploration_id,a.id)=(NEW.workspace_id,NEW.exploration_id,NEW.asset_id)
 WHERE r.workspace_id=NEW.workspace_id AND r.exploration_id=NEW.exploration_id AND r.id=NEW.run_id AND n.id=NEW.attempt_id
 AND r.status='completed' AND r.retain_output=1 AND n.status='completed'
 AND a.kind=CASE NEW.semantic_type WHEN 'text' THEN 'text' ELSE 'file' END
) BEGIN SELECT RAISE(ABORT,'invalid_result_origin'); END;
CREATE TRIGGER run_inputs_admission BEFORE INSERT ON run_inputs WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id)<>'queued' BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER run_state_transition BEFORE UPDATE OF status ON pipeline_runs WHEN NOT ((OLD.status='queued' AND NEW.status IN ('running','failed','cancelled','interrupted')) OR (OLD.status='running' AND NEW.status IN ('completed','failed','cancelled','interrupted'))) BEGIN SELECT RAISE(ABORT,'invalid_run_transition'); END;
CREATE TRIGGER solution_revisions_immutable_delete BEFORE DELETE ON solution_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER solution_revisions_immutable_update BEFORE UPDATE ON solution_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER solutions_immutable_delete BEFORE DELETE ON solutions BEGIN SELECT RAISE(ABORT,'immutable_solution'); END;
CREATE TRIGGER solutions_immutable_update BEFORE UPDATE ON solutions BEGIN SELECT RAISE(ABORT,'immutable_solution'); END;
CREATE TRIGGER trail_input_replace BEFORE UPDATE ON trail_input_bindings BEGIN SELECT RAISE(ABORT,'replace_binding'); END;
CREATE TRIGGER trail_input_scope BEFORE INSERT ON trail_input_bindings WHEN NOT EXISTS (
 SELECT 1 FROM trails t JOIN blocks b ON (t.workspace_id,t.exploration_id,t.stage_id)=(b.workspace_id,b.exploration_id,b.stage_id)
 WHERE t.workspace_id=NEW.workspace_id AND t.exploration_id=NEW.exploration_id AND t.id=NEW.trail_id AND b.archived_revision IS NULL AND (b.asset_id=NEW.asset_id OR b.caption_id=NEW.asset_id)
) BEGIN SELECT RAISE(ABORT,'invalid_binding'); END;
