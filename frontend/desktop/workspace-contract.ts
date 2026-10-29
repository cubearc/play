import limits from "../../desktop/protocol.json" with { type: "json" };

export interface Context {
  workspace_id: string;
  exploration_id: string;
  stage_id: string;
}
export interface Summary {
  id: string;
  title: string;
  revision: number;
}
export interface AssetView {
  id: string;
  explorationId: string;
  revision: number;
  title: string;
  content:
    | { kind: "image" }
    | { kind: "text" | "page"; text: string }
    | FileContent;
  origin?:
    | { kind: "asset"; assetId: string; revision: number; title: string }
    | RunOutputOrigin;
}
export interface RunOutputOrigin {
  kind: "run_output";
  runId: string;
  attemptId: string;
  nodeId: string;
  trailId: string;
  port: string;
  semanticType: "text" | "bytes";
  operationName: string;
  sequence: number;
}
export interface FileContent {
  kind: "file";
  byteLength: number;
  sha256: string;
  preview: { format: "utf8" | "hex"; text: string; truncated: boolean };
}
export interface AssetRevisionRef {
  asset_id: string;
  revision: number;
}
export type BlockSource =
  | { kind: "asset"; asset: AssetRevisionRef; caption: AssetRevisionRef | null }
  | { kind: "note"; note_id: string; revision: number };
export interface EditableAssetView extends AssetView {
  content: { kind: "text" | "page"; text: string };
}
export interface TextAssetView extends EditableAssetView {
  content: { kind: "text"; text: string };
}
export interface AssetBlockData {
  id: string;
  asset: AssetView;
  caption?: TextAssetView;
}
export interface AssetBlockView extends AssetBlockData {
  kind: "asset";
}
export interface ArchivedBlockView {
  block: BlockView;
  archivedRevision: number;
}
export interface NoteView {
  id: string;
  kind: "reasoning" | "annotation";
  title: string;
  revision: number;
  text: string;
  origin?: { noteId: string; revision: number; title: string };
}
export interface NoteBlockView {
  kind: "note";
  id: string;
  note: NoteView;
}
export type BlockView = AssetBlockView | NoteBlockView;
export interface StageView extends Summary {
  introduction: string;
  blocks: BlockView[];
  archivedBlocks: ArchivedBlockView[];
}
export function reasoningOf(stage: StageView): NoteView {
  const block = stage.blocks.find(
    (block) => block.kind === "note" && block.note.kind === "reasoning",
  );
  if (block?.kind !== "note") throw new Error("invalid_stage");
  return block.note;
}
export function blockSource(block: BlockView): BlockSource {
  if (block.kind === "note")
    return {
      kind: "note",
      note_id: block.note.id,
      revision: block.note.revision,
    };
  return {
    kind: "asset",
    asset: { asset_id: block.asset.id, revision: block.asset.revision },
    caption: block.caption
      ? { asset_id: block.caption.id, revision: block.caption.revision }
      : null,
  };
}
export function blockTitle(block: BlockView): string {
  return block.kind === "note" ? block.note.title : block.asset.title;
}
export interface PipelineRef {
  id: string;
  revision: number;
}
export type GraphSource =
  | { kind: "input"; name: "source" }
  | { kind: "output"; nodeId: string; port: string }
  | { kind: "literal"; value: unknown };
export interface GraphNode {
  id: string;
  operation: string;
  operationVersion: number;
  parameters: Record<string, unknown>;
  position: { x: number; y: number };
  inputs: Record<string, GraphSource>;
}
export interface PipelineDefinition {
  formatVersion: 3;
  inputs: [{ name: "source"; semanticType: string }];
  nodes: GraphNode[];
  output: string;
}
export interface PipelineView extends Summary {
  definition: PipelineDefinition;
  outputNodeId: string;
  inputType: string;
  outputOperation: string;
  operationVersion: number;
  revisions: number[];
  origin?: { pipelineId: string; revision: number };
}
export interface TrailView {
  revision: number;
  archived: boolean;
  id: string;
  workspaceId: string;
  explorationId: string;
  stageId: string;
  title: string;
  hypothesis: string;
  source: AssetView;
  pipelineId: string | null;
  pipeline: PipelineView | null;
}
export interface ExplorationView extends Summary {
  workspaceId: string;
  stages: Summary[];
  stage: StageView;
  trails: TrailView[];
}
export interface WorkspaceView {
  id: string;
  title: string;
  generation: number;
  explorations: Summary[];
  pipelines: PipelineView[];
  selectedExplorationId: string | null;
  selectedStageId: string | null;
}
export interface WorkspaceSummary extends Summary {
  available: boolean;
  error?: string;
}
export interface WorkspaceList {
  workspaces: WorkspaceSummary[];
  lastWorkspaceId: string | null;
}

export type WorkspaceRequest =
  | {
      kind: "create_pipeline";
      workspace_id: string;
      key: string;
      title: string;
      definition: PipelineDefinition;
    }
  | {
      kind: "revise_pipeline";
      workspace_id: string;
      key: string;
      pipeline_id: string;
      expected_revision: number;
      definition: PipelineDefinition;
    }
  | {
      kind: "fork_pipeline";
      workspace_id: string;
      key: string;
      pipeline_id: string;
      revision: number;
      title: string;
    }
  | {
      kind: "get_pipeline";
      workspace_id: string;
      pipeline_id: string;
      revision: number;
    }
  | {
      kind: "save_trail";
      context: Context;
      key: string;
      trail_id: string;
      expected_revision: number;
      hypothesis: string;
      pipeline: PipelineRef | null;
    }
  | {
      kind: "add_page";
      context: Context;
      key: string;
      title: string;
      text: string;
    }
  | {
      kind: "add_annotation";
      context: Context;
      key: string;
      title: string;
      text: string;
    }
  | {
      kind: "pin_run_output";
      context: Context;
      key: string;
      expected_revision: number;
      run_id: string;
      attempt_id: string;
      port: string;
    }
  | {
      kind: "archive_block" | "restore_block";
      context: Context;
      key: string;
      block_id: string;
      expected_revision: number;
      source: BlockSource;
    }
  | {
      kind: "duplicate_block";
      context: Context;
      key: string;
      block_id: string;
      expected_revision: number;
      source: BlockSource;
    }
  | {
      kind: "reorder_blocks";
      context: Context;
      key: string;
      expected_revision: number;
      block_ids: string[];
    }
  | {
      kind: "list_runs";
      context: Context;
      trail_id: string;
      before: number | null;
    }
  | { kind: "run_detail"; context: Context; run_id: string }
  | { kind: "view"; workspace_id: string }
  | { kind: "list" }
  | { kind: "create"; key: string; title: string }
  | { kind: "open"; workspace_id: string }
  | {
      kind: "create_exploration";
      workspace_id: string;
      key: string;
      title: string;
    }
  | {
      kind: "open_exploration";
      workspace_id: string;
      exploration_id: string;
      stage_id: string | null;
    }
  | {
      kind: "create_stage";
      workspace_id: string;
      exploration_id: string;
      key: string;
      title: string;
    }
  | {
      kind: "save_note";
      context: Context;
      key: string;
      note_id: string;
      expected_revision: number;
      text: string;
    }
  | {
      kind: "revise_text";
      context: Context;
      key: string;
      block_id: string;
      asset_id: string;
      expected_revision: number;
      text: string;
    }
  | {
      kind: "add_text";
      context: Context;
      key: string;
      title: string;
      text: string;
    };
export interface ImageImport {
  context: Context;
  key: string;
  caption: string;
}
export interface FileImport {
  context: Context;
  key: string;
}
export interface AssetRead {
  context: Context;
  asset_id: string;
  revision: number;
}
export interface RunRequest {
  context: Context;
  key: string;
  trail_id: string;
  pipeline_id: string;
  pipeline_revision: number;
  source_asset_id: string;
  source_revision: number;
  operation: string;
  operation_version: number;
  retain_output: boolean;
}
export interface CancelRun {
  context: Context;
  run_id: string;
}
export type RunStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "interrupted";
export interface RunSummary {
  id: string;
  sequence: number;
  trailId: string;
  pipelineId: string;
  pipelineRevision: number;
  status: RunStatus;
  createdMs: number;
  finishedMs: number | null;
  retainOutput: boolean;
  operation: { operation: string; operationVersion: number; name: string };
}
export interface RunDetail extends RunSummary {
  plan: {
    pipeline: { id: string; revision: number; definition: unknown };
    executionOrder?: string[];
    nodes?: { id: string; operation: { name: string } }[];
    contract: {
      operation: string;
      operation_version: number;
      inputs: { name: string; semantic_type: string }[];
      outputs: { name: string; semantic_type: string }[];
    };
    contractDigest: string;
    targetNodes: string[];
    reusePolicy: "none";
    reusedOutputs: unknown[];
    inputs: Record<
      string,
      | { assetId: string; revision: number; semanticType: string }
      | { literal: unknown }
    >;
    parameters: unknown;
    limits: unknown;
  };
  result: {
    status: RunStatus;
    outputs?: Record<string, string | { base64: string }>;
    code?: string;
    replayGap?: "output_not_retained";
  } | null;
  attempts: {
    id: string;
    nodeId: string;
    attempt: number;
    startedMs: number;
    finishedMs: number | null;
    status: RunStatus;
    result?: RunDetail["result"];
  }[];
}
export interface RunPage {
  runs: RunSummary[];
  before: number | null;
}

export interface MutationResult {
  explorationId?: string;
  stageId?: string;
  noteId?: string;
  revision?: number;
  blockId?: string;
}
export interface Responses {
  create_pipeline: { pipelineId: string; revision: number };
  revise_pipeline: { pipelineId: string; revision: number };
  fork_pipeline: { pipelineId: string; revision: number };
  get_pipeline: PipelineView;
  save_trail: { trailId: string; revision: number };
  add_annotation: { blockId: string };
  add_page: { blockId: string };
  pin_run_output: { blockId: string; stageId: string; revision: number };
  list_runs: RunPage;
  run_detail: RunDetail;
  list: WorkspaceList;
  view: WorkspaceView;
  create: WorkspaceView;
  open: WorkspaceView;
  create_exploration: { explorationId: string; stageId: string };
  open_exploration: ExplorationView;
  create_stage: { explorationId: string; stageId: string };
  save_note: { noteId: string; revision: number };
  add_text: { blockId: string };
  revise_text: { assetId: string; revision: number };
  reorder_blocks: { stageId: string; revision: number };
  duplicate_block: { blockId: string; stageId: string; revision: number };
  archive_block: { blockId: string; revision: number; archived: boolean };
  restore_block: { blockId: string; revision: number; archived: boolean };
}

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const bytes = (value: unknown, maximum: number): value is string =>
  typeof value === "string" &&
  new TextEncoder().encode(value).byteLength <= maximum;
const id = (value: unknown): value is string =>
  typeof value === "string" && uuid.test(value);
const revision = (value: unknown) =>
  Number.isSafeInteger(value) && typeof value === "number" && value > 0;
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("invalid_command");
  return value as Record<string, unknown>;
}
function keys(value: Record<string, unknown>, expected: string[]) {
  if (Object.keys(value).sort().join() !== expected.sort().join())
    throw new Error("invalid_command");
}
export function context(value: unknown): Context {
  const record = object(value);
  keys(record, ["workspace_id", "exploration_id", "stage_id"]);
  if (!Object.values(record).every(id)) throw new Error("invalid_command");
  return record as unknown as Context;
}
export function request(value: unknown): WorkspaceRequest {
  const record = object(value);
  const kind = record.kind;
  const fields: Record<string, string[]> = {
    create_pipeline: ["workspace_id", "key", "title", "definition"],
    revise_pipeline: [
      "workspace_id",
      "key",
      "pipeline_id",
      "expected_revision",
      "definition",
    ],
    fork_pipeline: ["workspace_id", "key", "pipeline_id", "revision", "title"],
    get_pipeline: ["workspace_id", "pipeline_id", "revision"],
    save_trail: [
      "context",
      "key",
      "trail_id",
      "expected_revision",
      "hypothesis",
      "pipeline",
    ],
    add_annotation: ["context", "key", "title", "text"],
    add_page: ["context", "key", "title", "text"],
    pin_run_output: [
      "context",
      "key",
      "expected_revision",
      "run_id",
      "attempt_id",
      "port",
    ],
    list_runs: ["context", "trail_id", "before"],
    run_detail: ["context", "run_id"],
    list: [],
    view: ["workspace_id"],
    create: ["key", "title"],
    open: ["workspace_id"],
    create_exploration: ["workspace_id", "key", "title"],
    open_exploration: ["workspace_id", "exploration_id", "stage_id"],
    create_stage: ["workspace_id", "exploration_id", "key", "title"],
    save_note: ["context", "key", "note_id", "expected_revision", "text"],
    add_text: ["context", "key", "title", "text"],
    revise_text: [
      "context",
      "key",
      "block_id",
      "asset_id",
      "expected_revision",
      "text",
    ],
    reorder_blocks: ["context", "key", "expected_revision", "block_ids"],
    duplicate_block: [
      "context",
      "key",
      "block_id",
      "expected_revision",
      "source",
    ],
    archive_block: [
      "context",
      "key",
      "block_id",
      "expected_revision",
      "source",
    ],
    restore_block: [
      "context",
      "key",
      "block_id",
      "expected_revision",
      "source",
    ],
  };
  if (typeof kind !== "string" || !Object.hasOwn(fields, kind))
    throw new Error("invalid_command");
  const expected = fields[kind];
  if (!expected) throw new Error("invalid_command");
  keys(record, ["kind", ...expected]);
  for (const field of expected) {
    const value = record[field];
    if (field === "context") context(value);
    else if (field === "pipeline") {
      if (value !== null) {
        const p = object(value);
        keys(p, ["id", "revision"]);
        if (!id(p.id) || !revision(p.revision))
          throw new Error("invalid_command");
      }
    } else if (field === "definition") {
      graphDefinition(value);
    } else if (field === "parameters") {
      object(value);
      if (!bytes(JSON.stringify(value), limits.pipeline_parameter_bytes))
        throw new Error("input_limit");
    } else if (field === "hypothesis") {
      if (!bytes(value, limits.hypothesis_bytes))
        throw new Error("input_limit");
    } else if (field === "operation") {
      if (!bytes(value, 128) || !value) throw new Error("invalid_command");
    } else if (field === "source") {
      const source = object(value);
      if (source.kind === "asset") {
        keys(source, ["kind", "asset", "caption"]);
        assetRevision(source.asset);
        if (source.caption !== null) assetRevision(source.caption);
      } else if (source.kind === "note") {
        keys(source, ["kind", "note_id", "revision"]);
        if (!id(source.note_id) || !revision(source.revision))
          throw new Error("invalid_command");
      } else throw new Error("invalid_command");
    } else if (field === "block_ids") {
      if (!Array.isArray(value) || value.length > limits.block_limit)
        throw new Error("input_limit");
      if (
        !value.length ||
        !value.every(id) ||
        new Set(value).size !== value.length
      )
        throw new Error("invalid_command");
    } else if (
      field === "expected_revision" ||
      field === "revision" ||
      field === "operation_version"
    ) {
      if (
        !revision(value) ||
        (field === "operation_version" && Number(value) > 0xffffffff)
      )
        throw new Error("invalid_command");
    } else if (field === "before") {
      if (value !== null && !revision(value))
        throw new Error("invalid_command");
    } else if (field === "port") {
      if (typeof value !== "string" || !/^[a-zA-Z0-9_-]{1,64}$/.test(value))
        throw new Error("invalid_command");
    } else if (field === "title") {
      if (!bytes(value, 120) || !value.trim())
        throw new Error("invalid_command");
    } else if (field === "text") {
      if (
        !bytes(
          value,
          kind === "save_note" || kind === "add_annotation"
            ? limits.note_bytes
            : limits.text_bytes,
        )
      )
        throw new Error("input_limit");
    } else if (
      field === "stage_id" &&
      kind === "open_exploration" &&
      value === null
    )
      continue;
    else if (!id(value)) throw new Error("invalid_command");
  }
  return record as unknown as WorkspaceRequest;
}
function assetRevision(value: unknown) {
  const reference = object(value);
  keys(reference, ["asset_id", "revision"]);
  if (!id(reference.asset_id) || !revision(reference.revision))
    throw new Error("invalid_command");
}
export function imageImport(value: unknown): ImageImport {
  const record = object(value);
  keys(record, ["context", "key", "caption"]);
  context(record.context);
  if (!id(record.key) || !bytes(record.caption, limits.text_bytes))
    throw new Error("invalid_command");
  return record as unknown as ImageImport;
}
export function fileImport(value: unknown): FileImport {
  const record = object(value);
  keys(record, ["context", "key"]);
  context(record.context);
  if (!id(record.key)) throw new Error("invalid_command");
  return record as unknown as FileImport;
}
export function assetRead(value: unknown): AssetRead {
  const record = object(value);
  keys(record, ["context", "asset_id", "revision"]);
  context(record.context);
  if (!id(record.asset_id) || !revision(record.revision))
    throw new Error("invalid_command");
  return record as unknown as AssetRead;
}
export function runRequest(value: unknown): RunRequest {
  const record = object(value);
  keys(record, [
    "context",
    "key",
    "trail_id",
    "pipeline_id",
    "pipeline_revision",
    "source_asset_id",
    "source_revision",
    "operation",
    "operation_version",
    "retain_output",
  ]);
  context(record.context);
  if (
    !id(record.key) ||
    !id(record.trail_id) ||
    !id(record.pipeline_id) ||
    !revision(record.pipeline_revision) ||
    !id(record.source_asset_id) ||
    !revision(record.source_revision) ||
    !bytes(record.operation, 128) ||
    !revision(record.operation_version) ||
    Number(record.operation_version) > 0xffffffff ||
    typeof record.retain_output !== "boolean"
  )
    throw new Error("invalid_command");
  return record as unknown as RunRequest;
}
export function cancelRun(value: unknown): CancelRun {
  const record = object(value);
  keys(record, ["context", "run_id"]);
  context(record.context);
  if (!id(record.run_id)) throw new Error("invalid_command");
  return record as unknown as CancelRun;
}
export function contextOf(exploration: ExplorationView): Context {
  return {
    workspace_id: exploration.workspaceId,
    exploration_id: exploration.id,
    stage_id: exploration.stage.id,
  };
}

export function graphDefinition(value: unknown): PipelineDefinition {
  const d = object(value);
  keys(d, ["formatVersion", "inputs", "nodes", "output"]);
  if (!bytes(JSON.stringify(d), limits.pipeline_definition_bytes))
    throw new Error("input_limit");
  if (
    d.formatVersion !== 3 ||
    !id(d.output) ||
    !Array.isArray(d.inputs) ||
    d.inputs.length !== 1 ||
    !Array.isArray(d.nodes) ||
    d.nodes.length < 1 ||
    d.nodes.length > limits.pipeline_node_limit
  )
    throw new Error("invalid_command");
  const input = object(d.inputs[0]);
  keys(input, ["name", "semanticType"]);
  if (
    input.name !== "source" ||
    !["text", "image.samples"].includes(String(input.semanticType))
  )
    throw new Error("invalid_command");
  for (const item of d.nodes) {
    const n = object(item);
    keys(n, [
      "id",
      "operation",
      "operationVersion",
      "parameters",
      "position",
      "inputs",
    ]);
    if (
      !id(n.id) ||
      !bytes(n.operation, 128) ||
      !n.operation ||
      !revision(n.operationVersion) ||
      Number(n.operationVersion) > 0xffffffff
    )
      throw new Error("invalid_command");
    object(n.parameters);
    if (!bytes(JSON.stringify(n.parameters), limits.pipeline_parameter_bytes))
      throw new Error("input_limit");
    const p = object(n.position);
    keys(p, ["x", "y"]);
    if (
      !Object.values(p).every(
        (v) =>
          typeof v === "number" &&
          Number.isInteger(v) &&
          v >= 0 &&
          v <= limits.pipeline_canvas_limit,
      )
    )
      throw new Error("invalid_command");
    for (const [port, v] of Object.entries(object(n.inputs))) {
      if (!/^[a-zA-Z0-9_-]{1,64}$/.test(port))
        throw new Error("invalid_command");
      const source = object(v);
      if (source.kind === "input") {
        keys(source, ["kind", "name"]);
        if (source.name !== "source") throw new Error("invalid_command");
      } else if (source.kind === "output") {
        keys(source, ["kind", "nodeId", "port"]);
        if (!id(source.nodeId) || !bytes(source.port, 64) || !source.port)
          throw new Error("invalid_command");
      } else if (source.kind === "literal") {
        keys(source, ["kind", "value"]);
        if (!bytes(JSON.stringify(source.value), limits.pipeline_literal_bytes))
          throw new Error("input_limit");
      } else throw new Error("invalid_command");
    }
  }
  return d as unknown as PipelineDefinition;
}
