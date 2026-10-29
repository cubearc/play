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
  content: { kind: "image" } | { kind: "text"; text: string };
}
export interface TextAssetView extends AssetView {
  content: { kind: "text"; text: string };
}
export interface BlockView {
  id: string;
  asset: AssetView;
  caption?: TextAssetView;
}
export interface NoteView {
  id: string;
  revision: number;
  text: string;
}
export interface StageView extends Summary {
  introduction: string;
  blocks: BlockView[];
  note: NoteView;
}
export interface PipelineView extends Summary {
  input: string;
  initialOperation: string;
  operationVersion: number;
}
export interface TrailView {
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
  };
  if (typeof kind !== "string" || !Object.hasOwn(fields, kind))
    throw new Error("invalid_command");
  const expected = fields[kind];
  if (!expected) throw new Error("invalid_command");
  keys(record, ["kind", ...expected]);
  for (const field of expected) {
    const value = record[field];
    if (field === "context") context(value);
    else if (field === "expected_revision") {
      if (!revision(value)) throw new Error("invalid_command");
    } else if (field === "before") {
      if (value !== null && !revision(value))
        throw new Error("invalid_command");
    } else if (field === "title") {
      if (!bytes(value, 120) || !value.trim())
        throw new Error("invalid_command");
    } else if (field === "text") {
      if (
        !bytes(
          value,
          kind === "save_note" ? limits.note_bytes : limits.text_bytes,
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
export function imageImport(value: unknown): ImageImport {
  const record = object(value);
  keys(record, ["context", "key", "caption"]);
  context(record.context);
  if (!id(record.key) || !bytes(record.caption, limits.text_bytes))
    throw new Error("invalid_command");
  return record as unknown as ImageImport;
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
