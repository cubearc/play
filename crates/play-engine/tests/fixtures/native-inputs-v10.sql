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
INSERT INTO "asset_revisions" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','f6f98054-1236-49dd-a335-0ed1929933b1',1,X'4775722078726C2E','f1250b1cb21f9ab573a5497161f2bee107fab7eb6efc57e174606b20237ef87e');
INSERT INTO "asset_revisions" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','be9de8eb-a974-4ef7-9e54-e74e9cfbbca5',1,X'89504E470D0A1A0A0000000D4948445200000002000000020802000000FDD49A730000001749444154789C631209307A76671F63C5AAA80DBBBE010029C706F6258517870000000049454E44AE426082','ab6db610b7eebaf21b5995bec577920d31c23cb571110baf901e3eef0a0ee2a7');
INSERT INTO "asset_revisions" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','e936e3ea-e631-44b6-a1df-d0a581636977',1,X'47757220637670676865722E','5ee60ee5ae7e1462f875976eeefb28b5bbace58b08d4fb086f8bdb472953b1f8');
CREATE TABLE "assets" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('text','image','file','page')),
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,id,kind), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO "assets" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','f6f98054-1236-49dd-a335-0ed1929933b1','Prior clue','text');
INSERT INTO "assets" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','be9de8eb-a974-4ef7-9e54-e74e9cfbbca5','Prior image','image');
INSERT INTO "assets" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','e936e3ea-e631-44b6-a1df-d0a581636977','Image caption','text');
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
INSERT INTO "blocks" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','43e4a300-b87f-4e0b-90a2-6452f37a0775',2,'note',NULL,NULL,NULL,NULL,'text','1c210e9b-bfd1-462b-ba7f-9e928f415943',NULL);
INSERT INTO "blocks" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','e720ea48-12f2-4d17-9d0c-920edfc50022',0,'asset','f6f98054-1236-49dd-a335-0ed1929933b1',1,NULL,NULL,'text',NULL,NULL);
INSERT INTO "blocks" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','732d1669-98b6-470a-9933-c6238a7e23dd',1,'asset','be9de8eb-a974-4ef7-9e54-e74e9cfbbca5',1,'e936e3ea-e631-44b6-a1df-d0a581636977',1,'text',NULL,NULL);
CREATE TABLE exploration_view_state (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
INSERT INTO "exploration_view_state" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3');
CREATE TABLE explorations (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,id)
);
INSERT INTO "explorations" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','Prior evidence',1);
CREATE TABLE node_attempts (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, id TEXT NOT NULL, node_id TEXT NOT NULL,
 attempt INTEGER NOT NULL CHECK(attempt>0), started_ms INTEGER NOT NULL, finished_ms INTEGER,
 status TEXT NOT NULL CHECK(status IN ('running','completed','failed','cancelled','interrupted')),
 PRIMARY KEY(workspace_id,exploration_id,run_id,id), UNIQUE(workspace_id,exploration_id,run_id,node_id,attempt),
 FOREIGN KEY(workspace_id,exploration_id,run_id) REFERENCES pipeline_runs(workspace_id,exploration_id,id),
 CHECK((status='running' AND finished_ms IS NULL) OR (status<>'running' AND finished_ms IS NOT NULL))
);
INSERT INTO "node_attempts" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','8070cf35-53df-418d-bce9-41694e365b08','133f8108-2d92-4d13-a330-4816bfb86ea8','5b564f44-9bfc-45e8-88e7-d3e045efd2b0',1,1793293780243,1793293780312,'completed');
CREATE TABLE node_results (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, run_id TEXT NOT NULL, attempt_id TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,run_id,attempt_id),
 FOREIGN KEY(workspace_id,exploration_id,run_id,attempt_id) REFERENCES node_attempts(workspace_id,exploration_id,run_id,id)
);
INSERT INTO "node_results" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','8070cf35-53df-418d-bce9-41694e365b08','133f8108-2d92-4d13-a330-4816bfb86ea8','{"outputs":{"output":"The key."},"status":"completed"}');
CREATE TABLE note_revisions (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, note_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), text TEXT NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,note_id,revision), FOREIGN KEY(workspace_id,exploration_id,note_id) REFERENCES notes(workspace_id,exploration_id,id)
);
INSERT INTO "note_revisions" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','1c210e9b-bfd1-462b-ba7f-9e928f415943',1,'');
CREATE TABLE "notes" (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 kind TEXT NOT NULL CHECK(kind IN ('reasoning','annotation')), title TEXT NOT NULL,
 source_note_id TEXT, source_revision INTEGER,
 PRIMARY KEY(workspace_id,exploration_id,id), UNIQUE(workspace_id,exploration_id,stage_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_note_id,source_revision) REFERENCES note_revisions(workspace_id,exploration_id,note_id,revision),
 CHECK((source_note_id IS NULL AND source_revision IS NULL) OR (kind='annotation' AND source_note_id IS NOT NULL AND source_revision>0 AND source_note_id<>id))
);
INSERT INTO "notes" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','1c210e9b-bfd1-462b-ba7f-9e928f415943',1,'reasoning','Reasoning',NULL,NULL);
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
INSERT INTO "pipeline_revisions" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','dacd7af3-9dc0-4020-b989-2798721049e2',1,'{"formatVersion":3,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":"5b564f44-9bfc-45e8-88e7-d3e045efd2b0","operation":"cipher.rot13","operationVersion":1,"parameters":{},"position":{"x":80,"y":120},"inputs":{"input":{"kind":"input","name":"source"}}}],"output":"5b564f44-9bfc-45e8-88e7-d3e045efd2b0"}');
INSERT INTO "pipeline_revisions" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','62fcce1a-947f-4f56-a146-f38c5c0bc232',1,'{"formatVersion":3,"inputs":[{"name":"source","semanticType":"image.samples"}],"nodes":[{"id":"eea9bd98-dea8-4b35-90c1-97a4771dcc97","operation":"image.pixels.read","operationVersion":1,"parameters":{"expand_palette":false,"x":0,"y":0},"position":{"x":80,"y":120},"inputs":{"input":{"kind":"input","name":"source"}}}],"output":"eea9bd98-dea8-4b35-90c1-97a4771dcc97"}');
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
INSERT INTO "pipeline_runs" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','8070cf35-53df-418d-bce9-41694e365b08','2ce13593-a813-4e81-82ed-65260f38538f','dacd7af3-9dc0-4020-b989-2798721049e2',1,'b63e2e03-fae3-4c31-b9b3-b80117b7d92a','63b16546040d7ab17d5420f059150112b00db283224c7479ccd7a3547c6f6e5d','desktop','completed',1793293779432,1793293780344,1,'{"adapterVersion":2,"caller":"desktop","contract":{"category":"ciphers","description":"Rotate ASCII letters by 13; preserve other Unicode.","deterministic":true,"errors":{"$defs":{"ErrorCode":{"description":"Machine-readable failure category. New categories may be added compatibly.","oneOf":[{"enum":["invalid_input","invalid_parameter","unsupported_format","resource_limit","io"],"type":"string"},{"const":"cancelled","description":"Cooperative pipeline cancellation at a bounded operation boundary.","type":"string"}]}},"$schema":"https://json-schema.org/draft/2020-12/schema","description":"Operation failure with safe context and an optional original cause.","properties":{"code":{"$ref":"#/$defs/ErrorCode"},"message":{"type":"string"},"offset":{"format":"uint64","minimum":0,"type":["integer","null"]},"operation":{"type":["string","null"]},"parameter":{"type":["string","null"]}},"required":["code","message"],"title":"OperationError","type":"object"},"example":"cube cipher rot13 --text Hello","inputs":[{"name":"input","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"limits":{"max_input_bytes":null,"max_items":null,"max_memory_bytes":null,"max_output_bytes":null,"max_work":null},"operation":"cipher.rot13","operation_version":1,"outputs":[{"name":"output","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"parameters":{"$schema":"https://json-schema.org/draft/2020-12/schema","additionalProperties":false,"properties":{},"required":[],"type":"object"},"pipeline_adapter":1,"policy":{"automatic_capture":false,"grants_filesystem_access":false,"grants_network_access":false,"private_input":false,"private_output":false,"sensitive_parameters":[]},"reversible":true,"schema_version":1},"contractDigest":"0f3f1af1053752a0cdb1bf965795073bea81aa8e3ad25767f4bf92ae0d9f23be","executionOrder":["5b564f44-9bfc-45e8-88e7-d3e045efd2b0"],"formatVersion":2,"inputs":{"source":{"assetId":"f6f98054-1236-49dd-a335-0ed1929933b1","revision":1,"semanticType":"text"}},"limits":{"cpuSeconds":1,"outputBytes":16384,"runMs":2000,"vmGrowthBytes":134217728},"nodes":[{"contract":{"category":"ciphers","description":"Rotate ASCII letters by 13; preserve other Unicode.","deterministic":true,"errors":{"$defs":{"ErrorCode":{"description":"Machine-readable failure category. New categories may be added compatibly.","oneOf":[{"enum":["invalid_input","invalid_parameter","unsupported_format","resource_limit","io"],"type":"string"},{"const":"cancelled","description":"Cooperative pipeline cancellation at a bounded operation boundary.","type":"string"}]}},"$schema":"https://json-schema.org/draft/2020-12/schema","description":"Operation failure with safe context and an optional original cause.","properties":{"code":{"$ref":"#/$defs/ErrorCode"},"message":{"type":"string"},"offset":{"format":"uint64","minimum":0,"type":["integer","null"]},"operation":{"type":["string","null"]},"parameter":{"type":["string","null"]}},"required":["code","message"],"title":"OperationError","type":"object"},"example":"cube cipher rot13 --text Hello","inputs":[{"name":"input","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"limits":{"max_input_bytes":null,"max_items":null,"max_memory_bytes":null,"max_output_bytes":null,"max_work":null},"operation":"cipher.rot13","operation_version":1,"outputs":[{"name":"output","required":true,"schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"string","type":"string"},"semantic_type":"text"}],"parameters":{"$schema":"https://json-schema.org/draft/2020-12/schema","additionalProperties":false,"properties":{},"required":[],"type":"object"},"pipeline_adapter":1,"policy":{"automatic_capture":false,"grants_filesystem_access":false,"grants_network_access":false,"private_input":false,"private_output":false,"sensitive_parameters":[]},"reversible":true,"schema_version":1},"contractDigest":"0f3f1af1053752a0cdb1bf965795073bea81aa8e3ad25767f4bf92ae0d9f23be","id":"5b564f44-9bfc-45e8-88e7-d3e045efd2b0","inputs":{"input":{"kind":"input","name":"source"}},"operation":{"name":"ROT13","operation":"cipher.rot13","operationVersion":1},"parameters":{}}],"operation":{"name":"ROT13","operation":"cipher.rot13","operationVersion":1},"parameters":{},"pipeline":{"definition":{"formatVersion":3,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":"5b564f44-9bfc-45e8-88e7-d3e045efd2b0","inputs":{"input":{"kind":"input","name":"source"}},"operation":"cipher.rot13","operationVersion":1,"parameters":{},"position":{"x":80,"y":120}}],"output":"5b564f44-9bfc-45e8-88e7-d3e045efd2b0"},"id":"dacd7af3-9dc0-4020-b989-2798721049e2","revision":1},"reusePolicy":"none","reusedOutputs":[],"targetNodes":["5b564f44-9bfc-45e8-88e7-d3e045efd2b0"],"trail":{"hypothesis":"","id":"2ce13593-a813-4e81-82ed-65260f38538f","revision":1}}','{"outputs":{"output":"The key."},"status":"completed"}',X'');
CREATE TABLE pipelines (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), id TEXT NOT NULL, title TEXT NOT NULL, current_revision INTEGER NOT NULL CHECK(current_revision>0),
 PRIMARY KEY(workspace_id,id)
);
INSERT INTO "pipelines" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','dacd7af3-9dc0-4020-b989-2798721049e2','ROT13',1);
INSERT INTO "pipelines" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','62fcce1a-947f-4f56-a146-f38c5c0bc232','image pixel',1);
CREATE TABLE receipts (
 workspace_id TEXT NOT NULL REFERENCES workspace_meta(id), command TEXT NOT NULL, key TEXT NOT NULL, digest TEXT NOT NULL, result TEXT NOT NULL,
 PRIMARY KEY(workspace_id,command,key)
);
INSERT INTO "receipts" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','create_exploration','904d6235-4890-4652-a68c-0ac0689a026a','b95b96156b61d316312b7e921b58aa43b362415b44d688cbdfed66a1bdf47ce7','{"explorationId":"cf717282-21c1-4672-8341-ad4b6e79ca0d","stageId":"249cfa8a-d1f5-40c3-868d-6cb096ceb4a3"}');
INSERT INTO "receipts" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','add_text','2e23c1a3-a40a-4872-a73b-2adff4e0df72','1eee561c84ea13221865bbb8436e5ce52f0661c18c94df2f8e629c58f5be84df','{"blockId":"e720ea48-12f2-4d17-9d0c-920edfc50022"}');
INSERT INTO "receipts" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','add_image','86f19bf2-0641-4789-ae7c-ddb9161dfc0a','cc5d3d84bef82f0e331790932d3bbbbdfff95e2c9112364164e8b1758670df0c','{"blockId":"732d1669-98b6-470a-9933-c6238a7e23dd"}');
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
INSERT INTO "run_inputs" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','8070cf35-53df-418d-bce9-41694e365b08','source','f6f98054-1236-49dd-a335-0ed1929933b1',1);
CREATE TABLE stages (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), position INTEGER NOT NULL,
 PRIMARY KEY(workspace_id,exploration_id,id), FOREIGN KEY(workspace_id,exploration_id) REFERENCES explorations(workspace_id,id)
);
INSERT INTO "stages" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','Stage 1',3,0);
CREATE TABLE trails (
 workspace_id TEXT NOT NULL, exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, hypothesis TEXT NOT NULL,
 source_id TEXT NOT NULL, source_revision INTEGER NOT NULL, pipeline_id TEXT, pipeline_revision INTEGER, revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
 PRIMARY KEY(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id),
 FOREIGN KEY(workspace_id,exploration_id,source_id,source_revision) REFERENCES asset_revisions(workspace_id,exploration_id,asset_id,revision),
 FOREIGN KEY(workspace_id,pipeline_id,pipeline_revision) REFERENCES pipeline_revisions(workspace_id,pipeline_id,revision),
 CHECK((pipeline_id IS NULL AND pipeline_revision IS NULL) OR (pipeline_id IS NOT NULL AND pipeline_revision IS NOT NULL))
);
INSERT INTO "trails" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','2ce13593-a813-4e81-82ed-65260f38538f','Explore Prior clue','','f6f98054-1236-49dd-a335-0ed1929933b1',1,'dacd7af3-9dc0-4020-b989-2798721049e2',1,1);
INSERT INTO "trails" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','4b85dbe5-45e0-485c-b5fc-ec32910cd7aa','Explore Prior image','','be9de8eb-a974-4ef7-9e54-e74e9cfbbca5',1,'62fcce1a-947f-4f56-a146-f38c5c0bc232',1,1);
INSERT INTO "trails" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3','35e2d443-f58d-43c2-a579-12f0b46d57df','Explore Image caption','','e936e3ea-e631-44b6-a1df-d0a581636977',1,'dacd7af3-9dc0-4020-b989-2798721049e2',1,1);
CREATE TABLE view_state (
 workspace_id TEXT NOT NULL, slot TEXT NOT NULL CHECK(slot='primary'), exploration_id TEXT NOT NULL, stage_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(workspace_id,slot), FOREIGN KEY(workspace_id,exploration_id,stage_id) REFERENCES stages(workspace_id,exploration_id,id)
);
INSERT INTO "view_state" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9','primary','cf717282-21c1-4672-8341-ad4b6e79ca0d','249cfa8a-d1f5-40c3-868d-6cb096ceb4a3',2);
CREATE TABLE "workspace_meta" (id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation=2),schema_version INTEGER NOT NULL CHECK(schema_version=10));
INSERT INTO "workspace_meta" VALUES('ba0164f2-3aec-4807-af64-4f16823b4fd9',2,10);
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
CREATE TRIGGER immutable_node_result_update BEFORE UPDATE ON node_results BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER immutable_node_result_delete BEFORE DELETE ON node_results BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
CREATE TRIGGER node_result_admission BEFORE INSERT ON node_results WHEN (SELECT status FROM node_attempts WHERE workspace_id=NEW.workspace_id AND exploration_id=NEW.exploration_id AND run_id=NEW.run_id AND id=NEW.attempt_id)<>'running' BEGIN SELECT RAISE(ABORT,'immutable_result'); END;
COMMIT;
PRAGMA user_version=10;
PRAGMA application_id=1129664590;
