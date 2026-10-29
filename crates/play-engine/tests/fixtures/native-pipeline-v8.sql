BEGIN TRANSACTION;
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
INSERT INTO "asset_revisions" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','0a8a0cab-0a6a-4b13-8024-48eaa9f4b962',1,X'4775722078726C2E','f1250b1cb21f9ab573a5497161f2bee107fab7eb6efc57e174606b20237ef87e');
INSERT INTO "asset_revisions" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','11160703-d070-43ab-bf52-93fab441ec79',1,X'89504E470D0A1A0A0000000D4948445200000002000000020802000000FDD49A730000001749444154789C631209307A76671F63C5AAA80DBBBE010029C706F6258517870000000049454E44AE426082','ab6db610b7eebaf21b5995bec577920d31c23cb571110baf901e3eef0a0ee2a7');
INSERT INTO "asset_revisions" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','a8f510f6-157b-438d-a5b1-eaeb48707295',1,X'47757220637670676865722E','5ee60ee5ae7e1462f875976eeefb28b5bbace58b08d4fb086f8bdb472953b1f8');
CREATE TABLE "assets" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('text','image','file','page')),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,id,kind), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO "assets" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','0a8a0cab-0a6a-4b13-8024-48eaa9f4b962','Prior clue','text');
INSERT INTO "assets" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','11160703-d070-43ab-bf52-93fab441ec79','Prior image','image');
INSERT INTO "assets" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','a8f510f6-157b-438d-a5b1-eaeb48707295','Image caption','text');
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
INSERT INTO "blocks" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','e85c86a1-923e-43e4-bd78-b8d6063f8878',2,'note',NULL,NULL,NULL,NULL,'text','3df5b14a-d2de-424b-9f83-f3ecd53d0943',NULL);
INSERT INTO "blocks" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','94706220-5139-475a-ad86-708f5eee1d32',0,'asset','0a8a0cab-0a6a-4b13-8024-48eaa9f4b962',1,NULL,NULL,'text',NULL,NULL);
INSERT INTO "blocks" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','61234a47-b592-400a-b14e-c890e5157942',1,'asset','11160703-d070-43ab-bf52-93fab441ec79',1,'a8f510f6-157b-438d-a5b1-eaeb48707295',1,'text',NULL,NULL);
CREATE TABLE exploration_view_state (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
INSERT INTO "exploration_view_state" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c');
CREATE TABLE explorations (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,id)
);
INSERT INTO "explorations" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','Prior evidence',1);
CREATE TABLE node_attempts (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, id TEXT NOT NULL, node_id TEXT NOT NULL,
 attempt INTEGER NOT NULL CHECK(attempt>0), started_ms INTEGER NOT NULL, finished_ms INTEGER,
 status TEXT NOT NULL CHECK(status IN ('running','completed','failed','cancelled','interrupted')),
 PRIMARY KEY(workspace_id,exploration_id,run_id,id), UNIQUE(workspace_id,exploration_id,run_id,node_id,attempt),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 CHECK((status='running' AND finished_ms IS NULL) OR (status<>'running' AND finished_ms IS NOT NULL))
);
INSERT INTO "node_attempts" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','7466f017-7385-4126-b213-66ac3c0ae2a5','e09b82d0-c6f2-4bda-bd4c-b689f07b197c','72519e77-79d8-4596-8e2f-a82727863598',1,1793276772583,1793276772838,'completed');
CREATE TABLE note_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, note_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), text TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,note_id,revision), FOREIGN KEY(workspace_id,exploration_id,note_id) REFERENCES notes(workspace_id,exploration_id,id)
);
INSERT INTO "note_revisions" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','3df5b14a-d2de-424b-9f83-f3ecd53d0943',1,'');
CREATE TABLE "notes" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 kind TEXT NOT NULL CHECK(kind IN ('reasoning','annotation')), title TEXT NOT NULL,
 source_note_id TEXT, source_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_note_id,source_revision) REFERENCES note_revisions(workspace_id,exploration_id,note_id,revision),
 CHECK((source_note_id IS NULL AND source_revision IS NULL) OR (kind='annotation' AND source_note_id IS NOT NULL AND source_revision>0 AND source_note_id<>id))
);
INSERT INTO "notes" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','3df5b14a-d2de-424b-9f83-f3ecd53d0943',1,'reasoning','Reasoning',NULL,NULL);
CREATE TABLE pipeline_revisions (
 workspace_id TEXT NOT NULL, pipeline_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), definition TEXT NOT NULL,
 PRIMARY KEY(workspace_id,pipeline_id,revision), FOREIGN KEY(workspace_id,pipeline_id) REFERENCES pipelines(workspace_id,id)
);
INSERT INTO "pipeline_revisions" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','95a3b5d1-03e9-4c00-bc76-8bf69581fb80',1,'{"formatVersion":1,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":"72519e77-79d8-4596-8e2f-a82727863598","input":"source","operation":"cipher.rot13","operation_version":1}]}');
INSERT INTO "pipeline_revisions" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','beaa094c-8748-40c0-b143-150c57c644f9',1,'{"formatVersion":1,"inputs":[{"name":"source","semanticType":"image.samples"}],"nodes":[{"id":"00200b3f-fa28-4c75-9ea2-8fd97b3698c3","input":"source","operation":"image.pixels.read","operation_version":1}]}');
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
INSERT INTO "pipeline_runs" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','7466f017-7385-4126-b213-66ac3c0ae2a5','b0884474-463e-4820-b7e7-80bfe3dcb6bd','95a3b5d1-03e9-4c00-bc76-8bf69581fb80',1,'d99aca17-5831-4ad2-8d8a-5cbf90a57b33','3788a0a783352babecb2e8816736fe7bf031b35c1a0d12c50a32496a4127858d','desktop','completed',1793276772576,1793276772838,1,'{"adapterVersion":1,"caller":"desktop","contract":{"category":"ciphers","description":"Rotate ASCII letters by 13; preserve other Unicode.","deterministic":true,"errors":{"$defs":{"ErrorCode":{"description":"Machine-readable failure category. New categories may be added compatibly.","oneOf":[{"enum":["invalid_input","invalid_parameter","unsupported_format","resource_limit","io"],"type":"string"},{"const":"cancelled","description":"Cooperative pipeline cancellation at a bounded operation boundary.","type":"string"}]}},"$schema":"https://json-schema.org/draft/2020-12/schema","description":"Operation failure with safe context and an optional original cause.","properties":{"code":{"$ref":"#/$defs/ErrorCode"},"message":{"type":"string"},"offset":{"format":"uint64","minimum":0,"type":["integer","null"]},"operation":{"type":["string","null"]},"parameter":{"type":["string","null"]}},"required":["code","message"],"title":"OperationError","type":"object"},"example":"cube cipher rot13 --text Hello","inputs":[{"name":"input","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"limits":{"max_input_bytes":null,"max_items":null,"max_memory_bytes":null,"max_output_bytes":null,"max_work":null},"operation":"cipher.rot13","operation_version":1,"outputs":[{"name":"output","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"parameters":{"$schema":"https://json-schema.org/draft/2020-12/schema","additionalProperties":false,"properties":{},"required":[],"type":"object"},"pipeline_adapter":1,"policy":{"automatic_capture":false,"grants_filesystem_access":false,"grants_network_access":false,"private_input":false,"private_output":false,"sensitive_parameters":[]},"reversible":true,"schema_version":1},"contractDigest":"0f3f1af1053752a0cdb1bf965795073bea81aa8e3ad25767f4bf92ae0d9f23be","formatVersion":1,"inputs":{"input":{"assetId":"0a8a0cab-0a6a-4b13-8024-48eaa9f4b962","revision":1,"semanticType":"text"}},"limits":{"cpuSeconds":1,"outputBytes":16384,"runMs":2000,"vmGrowthBytes":134217728},"operation":{"name":"ROT13","operation":"cipher.rot13","operationVersion":1},"parameters":{},"pipeline":{"definition":{"formatVersion":1,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":"72519e77-79d8-4596-8e2f-a82727863598","input":"source","operation":"cipher.rot13","operation_version":1}]},"id":"95a3b5d1-03e9-4c00-bc76-8bf69581fb80","revision":1},"reusePolicy":"none","reusedOutputs":[],"targetNodes":["72519e77-79d8-4596-8e2f-a82727863598"]}','{"outputs":{"output":"The key."},"status":"completed"}',X'');
CREATE TABLE pipelines (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 PRIMARY KEY(workspace_id,id)
);
INSERT INTO "pipelines" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','95a3b5d1-03e9-4c00-bc76-8bf69581fb80','Text transform',1);
INSERT INTO "pipelines" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','beaa094c-8748-40c0-b143-150c57c644f9','image pixel',1);
CREATE TABLE receipts (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), command TEXT NOT NULL, key TEXT NOT NULL, digest TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,command,key)
);
INSERT INTO "receipts" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','create_exploration','73b245db-0226-4ff0-92ad-4147faef6bc5','c2b4d558776ff5b420f72b745b5ad0d6a99afc4bdc63b112ca5bb36500f3b04e','{"explorationId":"c266fa82-f481-4560-8b2c-1439f5120a3a","stageId":"538e76e8-bebe-4d2a-8b0b-07529c4ab76c"}');
INSERT INTO "receipts" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','add_text','f4f67ac8-c74a-40e8-affd-b5b1785a0940','0cd76440c6e69b3ea987b7a9855b11498bac9d113723a2b82aad8e6155ccf1aa','{"blockId":"94706220-5139-475a-ad86-708f5eee1d32"}');
INSERT INTO "receipts" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','add_image','b4298027-6ed1-413c-a321-89156f04f503','1c4d9d497f639a55dcc4410dd66bc03ee3216a17e9691e6fb1c2c35c855719d5','{"blockId":"61234a47-b592-400a-b14e-c890e5157942"}');
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
INSERT INTO "run_inputs" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','7466f017-7385-4126-b213-66ac3c0ae2a5','input','0a8a0cab-0a6a-4b13-8024-48eaa9f4b962',1);
CREATE TABLE stages (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), position INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO "stages" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','Stage 1',3,0);
CREATE TABLE trails (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, hypothesis TEXT NOT NULL,
 source_id TEXT NOT NULL, source_revision INTEGER NOT NULL, pipeline_id TEXT, pipeline_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_id,source_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,pipeline_id,pipeline_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK((pipeline_id IS NULL AND pipeline_revision IS NULL) OR (pipeline_id IS NOT NULL AND pipeline_revision IS NOT NULL))
);
INSERT INTO "trails" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','b0884474-463e-4820-b7e7-80bfe3dcb6bd','Explore Prior clue','','0a8a0cab-0a6a-4b13-8024-48eaa9f4b962',1,'95a3b5d1-03e9-4c00-bc76-8bf69581fb80',1);
INSERT INTO "trails" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','94d808e3-445f-4d82-b9a9-896912ecbe93','Explore Prior image','','11160703-d070-43ab-bf52-93fab441ec79',1,'beaa094c-8748-40c0-b143-150c57c644f9',1);
INSERT INTO "trails" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c','e984f856-3340-4d16-adc7-0280a1d049ce','Explore Image caption','','a8f510f6-157b-438d-a5b1-eaeb48707295',1,'95a3b5d1-03e9-4c00-bc76-8bf69581fb80',1);
CREATE TABLE view_state (
 workspace_id TEXT NOT NULL, slot TEXT NOT NULL CHECK(slot='primary'), exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,slot), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
INSERT INTO "view_state" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350','primary','c266fa82-f481-4560-8b2c-1439f5120a3a','538e76e8-bebe-4d2a-8b0b-07529c4ab76c',2);
CREATE TABLE "workspace_meta" (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=8)
);
INSERT INTO "workspace_meta" VALUES('34ef4959-9f89-465f-8058-d6e41ed7b350',2,8);
CREATE TRIGGER immutable_assets_update BEFORE UPDATE ON asset_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_assets_delete BEFORE DELETE ON asset_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_notes_update BEFORE UPDATE ON note_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_notes_delete BEFORE DELETE ON note_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_pipelines_update BEFORE UPDATE ON pipeline_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE TRIGGER immutable_pipelines_delete BEFORE DELETE ON pipeline_revisions BEGIN SELECT RAISE(ABORT,'immutable_revision'); END;
CREATE INDEX pipeline_run_history ON pipeline_runs(workspace_id,exploration_id,trail_id,created_ms DESC,id);
CREATE TRIGGER immutable_runs_update BEFORE UPDATE ON pipeline_runs WHEN OLD.status NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_runs_delete BEFORE DELETE ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_attempts_update BEFORE UPDATE ON node_attempts WHEN OLD.status<>'running' BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_attempts_delete BEFORE DELETE ON node_attempts BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER immutable_run_inputs_update BEFORE UPDATE ON run_inputs BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER immutable_run_inputs_delete BEFORE DELETE ON run_inputs BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER immutable_run_plan BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,created_ms,retain_output,plan ON pipeline_runs BEGIN SELECT RAISE(ABORT,'immutable_plan'); END;
CREATE TRIGGER immutable_attempt_identity BEFORE UPDATE OF workspace_id,exploration_id,run_id,id,node_id,attempt,started_ms ON node_attempts BEGIN SELECT RAISE(ABORT,'immutable_attempt'); END;
CREATE TRIGGER run_state_transition BEFORE UPDATE OF status ON pipeline_runs WHEN NOT ((OLD.status='queued' AND NEW.status IN ('running','failed','cancelled','interrupted')) OR (OLD.status='running' AND NEW.status IN ('completed','failed','cancelled','interrupted'))) BEGIN SELECT RAISE(ABORT,'invalid_run_transition'); END;
CREATE TRIGGER run_inputs_admission BEFORE INSERT ON run_inputs WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id)<>'queued' BEGIN SELECT RAISE(ABORT,'immutable_input'); END;
CREATE TRIGGER attempt_admission BEFORE INSERT ON node_attempts WHEN (SELECT status FROM pipeline_runs WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.run_id) NOT IN ('queued','running') BEGIN SELECT RAISE(ABORT,'immutable_run'); END;
CREATE TRIGGER immutable_asset_origin_update BEFORE UPDATE ON asset_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_asset_origin_delete BEFORE DELETE ON asset_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_origin_update BEFORE UPDATE ON result_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_origin_delete BEFORE DELETE ON result_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_result_revision BEFORE INSERT ON asset_revisions WHEN EXISTS(SELECT 1 FROM result_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER exclusive_result_origin BEFORE INSERT ON result_origins WHEN EXISTS(SELECT 1 FROM asset_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'ambiguous_origin'); END;
CREATE TRIGGER exclusive_asset_origin BEFORE INSERT ON asset_origins WHEN EXISTS(SELECT 1 FROM result_origins WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND asset_id=NEW.asset_id) BEGIN SELECT RAISE(ABORT,'ambiguous_origin'); END;
CREATE UNIQUE INDEX one_reasoning_note ON notes(workspace_id,exploration_id,stage_id) WHERE kind='reasoning';
CREATE TRIGGER immutable_note_identity BEFORE UPDATE OF workspace_id,exploration_id,stage_id,id,kind,title,source_note_id,source_revision ON notes BEGIN SELECT RAISE(ABORT,'immutable_note_identity'); END;
CREATE INDEX blocks_order ON blocks(workspace_id,exploration_id,stage_id,position,id);
CREATE INDEX blocks_archive ON blocks(workspace_id,exploration_id,stage_id,archived_revision DESC,id);
CREATE TRIGGER preserve_blocks BEFORE DELETE ON blocks BEGIN SELECT RAISE(ABORT,'preserved_block'); END;
CREATE TRIGGER reasoning_archive_update BEFORE UPDATE ON blocks WHEN NEW.archived_revision IS NOT NULL AND NEW.kind='note' AND EXISTS(SELECT 1 FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id AND kind='reasoning') BEGIN SELECT RAISE(ABORT,'protected_reasoning'); END;
CREATE TRIGGER reasoning_archive_insert BEFORE INSERT ON blocks WHEN NEW.archived_revision IS NOT NULL AND NEW.kind='note' AND EXISTS(SELECT 1 FROM notes WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND id=NEW.note_id AND kind='reasoning') BEGIN SELECT RAISE(ABORT,'protected_reasoning'); END;
CREATE TRIGGER result_origin_admission BEFORE INSERT ON result_origins WHEN NOT EXISTS(
 SELECT 1 FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id)
 JOIN assets a ON (a.workspace_id,a.exploration_id,a.id)=(NEW.workspace_id,NEW.exploration_id,NEW.asset_id)
 WHERE r.workspace_id=NEW.workspace_id AND r.exploration_id=NEW.exploration_id AND r.id=NEW.run_id AND n.id=NEW.attempt_id
 AND r.status='completed' AND r.retain_output=1 AND n.status='completed'
 AND a.kind=CASE NEW.semantic_type WHEN 'text' THEN 'text' ELSE 'file' END
) BEGIN SELECT RAISE(ABORT,'invalid_result_origin'); END;
COMMIT;
PRAGMA application_id=1129664590;
PRAGMA user_version=8;
