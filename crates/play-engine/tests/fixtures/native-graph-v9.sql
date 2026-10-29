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
INSERT INTO "asset_revisions" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','e3fab60f-9bb3-4a2e-80db-e16d39721ac8',1,X'4775722078726C2E','f1250b1cb21f9ab573a5497161f2bee107fab7eb6efc57e174606b20237ef87e');
INSERT INTO "asset_revisions" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','905a67d0-63d8-4368-be5f-e4afb51b6d55',1,X'89504E470D0A1A0A0000000D4948445200000002000000020802000000FDD49A730000001749444154789C631209307A76671F63C5AAA80DBBBE010029C706F6258517870000000049454E44AE426082','ab6db610b7eebaf21b5995bec577920d31c23cb571110baf901e3eef0a0ee2a7');
INSERT INTO "asset_revisions" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','f0377879-a2b8-4f2f-b0f8-f4c8e0f6f566',1,X'47757220637670676865722E','5ee60ee5ae7e1462f875976eeefb28b5bbace58b08d4fb086f8bdb472953b1f8');
CREATE TABLE "assets" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('text','image','file','page')),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,id,kind), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO "assets" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','e3fab60f-9bb3-4a2e-80db-e16d39721ac8','Prior clue','text');
INSERT INTO "assets" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','905a67d0-63d8-4368-be5f-e4afb51b6d55','Prior image','image');
INSERT INTO "assets" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','f0377879-a2b8-4f2f-b0f8-f4c8e0f6f566','Image caption','text');
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
INSERT INTO "blocks" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','965db923-1620-48e3-a7f2-9264a1f1330f',2,'note',NULL,NULL,NULL,NULL,'text','16fa685b-f794-4545-b239-7d4fd72cef0b',NULL);
INSERT INTO "blocks" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','9acd26f2-f31a-4006-acf8-46cee49b0406',0,'asset','e3fab60f-9bb3-4a2e-80db-e16d39721ac8',1,NULL,NULL,'text',NULL,NULL);
INSERT INTO "blocks" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','c0326b36-f792-460e-ae02-ec17771f4288',1,'asset','905a67d0-63d8-4368-be5f-e4afb51b6d55',1,'f0377879-a2b8-4f2f-b0f8-f4c8e0f6f566',1,'text',NULL,NULL);
CREATE TABLE exploration_view_state (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
INSERT INTO "exploration_view_state" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a');
CREATE TABLE explorations (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,id)
);
INSERT INTO "explorations" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','Prior evidence',1);
CREATE TABLE node_attempts (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, id TEXT NOT NULL, node_id TEXT NOT NULL,
 attempt INTEGER NOT NULL CHECK(attempt>0), started_ms INTEGER NOT NULL, finished_ms INTEGER,
 status TEXT NOT NULL CHECK(status IN ('running','completed','failed','cancelled','interrupted')),
 PRIMARY KEY(workspace_id,exploration_id,run_id,id), UNIQUE(workspace_id,exploration_id,run_id,node_id,attempt),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 CHECK((status='running' AND finished_ms IS NULL) OR (status<>'running' AND finished_ms IS NOT NULL))
);
INSERT INTO "node_attempts" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','4c7fc3d1-e23d-4862-811e-3374974624c0','219dd8ad-3081-40b4-b8b8-85c1fc9fc6f4','013b0a8c-7ed5-45c7-bf50-28899717ae0e',1,1793286739891,1793286740108,'completed');
CREATE TABLE note_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, note_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), text TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,note_id,revision), FOREIGN KEY(workspace_id,exploration_id,note_id) REFERENCES notes(workspace_id,exploration_id,id)
);
INSERT INTO "note_revisions" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','16fa685b-f794-4545-b239-7d4fd72cef0b',1,'');
CREATE TABLE "notes" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 kind TEXT NOT NULL CHECK(kind IN ('reasoning','annotation')), title TEXT NOT NULL,
 source_note_id TEXT, source_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_note_id,source_revision) REFERENCES note_revisions(workspace_id,exploration_id,note_id,revision),
 CHECK((source_note_id IS NULL AND source_revision IS NULL) OR (kind='annotation' AND source_note_id IS NOT NULL AND source_revision>0 AND source_note_id<>id))
);
INSERT INTO "notes" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','16fa685b-f794-4545-b239-7d4fd72cef0b',1,'reasoning','Reasoning',NULL,NULL);
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
INSERT INTO "pipeline_revisions" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','ae3851da-ab74-4f33-a662-9132cf390154',1,'{"formatVersion":2,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":"013b0a8c-7ed5-45c7-bf50-28899717ae0e","operation":"cipher.rot13","operation_version":1,"input":"source","parameters":{}}]}');
INSERT INTO "pipeline_revisions" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','1a6a6e67-69fd-40c9-adbd-58d5430d88d3',1,'{"formatVersion":2,"inputs":[{"name":"source","semanticType":"image.samples"}],"nodes":[{"id":"fe1230d6-9712-474a-912b-8786ffd0468b","operation":"image.pixels.read","operation_version":1,"input":"source","parameters":{"expand_palette":false,"x":0,"y":0}}]}');
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
INSERT INTO "pipeline_runs" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','4c7fc3d1-e23d-4862-811e-3374974624c0','69bc78e9-d063-47b6-b479-cd1892c35cd1','ae3851da-ab74-4f33-a662-9132cf390154',1,'2c15dff7-1f95-4c7e-944f-8301e0073499','ccc2d0308f0a69776629cfe42e6d023b8160b39467c80337e8602714633bbbe5','desktop','completed',1793286739885,1793286740108,1,'{"adapterVersion":1,"caller":"desktop","contract":{"category":"ciphers","description":"Rotate ASCII letters by 13; preserve other Unicode.","deterministic":true,"errors":{"$defs":{"ErrorCode":{"description":"Machine-readable failure category. New categories may be added compatibly.","oneOf":[{"enum":["invalid_input","invalid_parameter","unsupported_format","resource_limit","io"],"type":"string"},{"const":"cancelled","description":"Cooperative pipeline cancellation at a bounded operation boundary.","type":"string"}]}},"$schema":"https://json-schema.org/draft/2020-12/schema","description":"Operation failure with safe context and an optional original cause.","properties":{"code":{"$ref":"#/$defs/ErrorCode"},"message":{"type":"string"},"offset":{"format":"uint64","minimum":0,"type":["integer","null"]},"operation":{"type":["string","null"]},"parameter":{"type":["string","null"]}},"required":["code","message"],"title":"OperationError","type":"object"},"example":"cube cipher rot13 --text Hello","inputs":[{"name":"input","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"limits":{"max_input_bytes":null,"max_items":null,"max_memory_bytes":null,"max_output_bytes":null,"max_work":null},"operation":"cipher.rot13","operation_version":1,"outputs":[{"name":"output","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"parameters":{"$schema":"https://json-schema.org/draft/2020-12/schema","additionalProperties":false,"properties":{},"required":[],"type":"object"},"pipeline_adapter":1,"policy":{"automatic_capture":false,"grants_filesystem_access":false,"grants_network_access":false,"private_input":false,"private_output":false,"sensitive_parameters":[]},"reversible":true,"schema_version":1},"contractDigest":"0f3f1af1053752a0cdb1bf965795073bea81aa8e3ad25767f4bf92ae0d9f23be","formatVersion":1,"inputs":{"input":{"assetId":"e3fab60f-9bb3-4a2e-80db-e16d39721ac8","revision":1,"semanticType":"text"}},"limits":{"cpuSeconds":1,"outputBytes":16384,"runMs":2000,"vmGrowthBytes":134217728},"operation":{"name":"ROT13","operation":"cipher.rot13","operationVersion":1},"parameters":{},"pipeline":{"definition":{"formatVersion":2,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":"013b0a8c-7ed5-45c7-bf50-28899717ae0e","input":"source","operation":"cipher.rot13","operation_version":1,"parameters":{}}]},"id":"ae3851da-ab74-4f33-a662-9132cf390154","revision":1},"reusePolicy":"none","reusedOutputs":[],"targetNodes":["013b0a8c-7ed5-45c7-bf50-28899717ae0e"],"trail":{"hypothesis":"","id":"69bc78e9-d063-47b6-b479-cd1892c35cd1","revision":1}}','{"outputs":{"output":"The key."},"status":"completed"}',X'');
CREATE TABLE pipelines (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 PRIMARY KEY(workspace_id,id)
);
INSERT INTO "pipelines" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','ae3851da-ab74-4f33-a662-9132cf390154','ROT13',1);
INSERT INTO "pipelines" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','1a6a6e67-69fd-40c9-adbd-58d5430d88d3','image pixel',1);
CREATE TABLE receipts (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), command TEXT NOT NULL, key TEXT NOT NULL, digest TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,command,key)
);
INSERT INTO "receipts" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','create_exploration','98551f75-fb64-4c80-beee-8f94b845eaba','a5f478894e809311296e0b6011d3b2a67f544da5761c436753f3009eba145cf9','{"explorationId":"78b6fbdd-0f59-4461-b731-cedfd4e6ffd5","stageId":"395a2a51-81b8-4a9d-870f-8b4d4c335c4a"}');
INSERT INTO "receipts" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','add_text','edb9ca80-2ab8-443e-9350-910ec549c029','d031f05caac764b5bc0665e7bd318f00c5f5d8a36b6f999ae5fd117814d1becf','{"blockId":"9acd26f2-f31a-4006-acf8-46cee49b0406"}');
INSERT INTO "receipts" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','add_image','b6a68214-a9b6-4a39-8f86-0767d896b245','643cda772fa4c641e37925e45faede8e0c60dc5dba115b23e5614260f319eeb6','{"blockId":"c0326b36-f792-460e-ae02-ec17771f4288"}');
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
INSERT INTO "run_inputs" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','4c7fc3d1-e23d-4862-811e-3374974624c0','input','e3fab60f-9bb3-4a2e-80db-e16d39721ac8',1);
CREATE TABLE stages (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), position INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO "stages" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','Stage 1',3,0);
CREATE TABLE trails (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, hypothesis TEXT NOT NULL,
 source_id TEXT NOT NULL, source_revision INTEGER NOT NULL, pipeline_id TEXT, pipeline_revision INTEGER, revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
 PRIMARY KEY(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_id,source_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,pipeline_id,pipeline_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK((pipeline_id IS NULL AND pipeline_revision IS NULL) OR (pipeline_id IS NOT NULL AND pipeline_revision IS NOT NULL))
);
INSERT INTO "trails" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','69bc78e9-d063-47b6-b479-cd1892c35cd1','Explore Prior clue','','e3fab60f-9bb3-4a2e-80db-e16d39721ac8',1,'ae3851da-ab74-4f33-a662-9132cf390154',1,1);
INSERT INTO "trails" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','8469ff74-1f11-4287-86c0-0463f084daa1','Explore Prior image','','905a67d0-63d8-4368-be5f-e4afb51b6d55',1,'1a6a6e67-69fd-40c9-adbd-58d5430d88d3',1,1);
INSERT INTO "trails" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a','57e7e51c-f73d-49db-8631-17e6d23d949b','Explore Image caption','','f0377879-a2b8-4f2f-b0f8-f4c8e0f6f566',1,'ae3851da-ab74-4f33-a662-9132cf390154',1,1);
CREATE TABLE view_state (
 workspace_id TEXT NOT NULL, slot TEXT NOT NULL CHECK(slot='primary'), exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,slot), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
INSERT INTO "view_state" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc','primary','78b6fbdd-0f59-4461-b731-cedfd4e6ffd5','395a2a51-81b8-4a9d-870f-8b4d4c335c4a',2);
CREATE TABLE "workspace_meta" (
 id TEXT PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation=2), schema_version INTEGER NOT NULL CHECK(schema_version=9)
);
INSERT INTO "workspace_meta" VALUES('0fad1399-b5a6-4ba6-b287-6c0a206b75fc',2,9);
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
CREATE TRIGGER immutable_pipeline_origin_update BEFORE UPDATE ON pipeline_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
CREATE TRIGGER immutable_pipeline_origin_delete BEFORE DELETE ON pipeline_origins BEGIN SELECT RAISE(ABORT,'immutable_origin'); END;
COMMIT;
PRAGMA user_version=9;
PRAGMA application_id=1129664590;
