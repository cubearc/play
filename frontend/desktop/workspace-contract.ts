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
export interface AssetRun extends AssetRead {
  operation: string;
  operation_version: number;
}
export interface MutationResult {
  explorationId?: string;
  stageId?: string;
  noteId?: string;
  revision?: number;
  blockId?: string;
}
export interface Responses {
  list: WorkspaceList;
  create: WorkspaceView;
  open: WorkspaceView;
  create_exploration: { explorationId: string; stageId: string };
  open_exploration: ExplorationView;
  create_stage: { explorationId: string; stageId: string };
  save_note: { noteId: string; revision: number };
  add_text: { blockId: string };
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
    list: [],
    create: ["key", "title"],
    open: ["workspace_id"],
    create_exploration: ["workspace_id", "key", "title"],
    open_exploration: ["workspace_id", "exploration_id", "stage_id"],
    create_stage: ["workspace_id", "exploration_id", "key", "title"],
    save_note: ["context", "key", "note_id", "expected_revision", "text"],
    add_text: ["context", "key", "title", "text"],
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
export function assetRun(value: unknown): AssetRun {
  const record = object(value);
  keys(record, [
    "context",
    "asset_id",
    "revision",
    "operation",
    "operation_version",
  ]);
  assetRead({
    context: record.context,
    asset_id: record.asset_id,
    revision: record.revision,
  });
  if (
    !bytes(record.operation, 128) ||
    !revision(record.operation_version) ||
    Number(record.operation_version) > 0xffffffff
  )
    throw new Error("invalid_command");
  return record as unknown as AssetRun;
}
export function contextOf(exploration: ExplorationView): Context {
  return {
    workspace_id: exploration.workspaceId,
    exploration_id: exploration.id,
    stage_id: exploration.stage.id,
  };
}
