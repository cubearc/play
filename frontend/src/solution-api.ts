import { apiFetch } from "./capability";
import type { Json } from "./exploration-api";
import { object } from "./lab-api";

export type FinalSelection = {
  result_id: string;
  port: string | null;
  field: { pointer: string; conversion: "text" | "base64_bytes" } | null;
};
export type CandidateRef = { key: string; stage: string; revision: number };
export type SolutionRequest = {
  id: string;
  revision: number;
  title: string;
  explanation: string;
  local_status: "finding" | "resolved";
  source:
    | {
        type: "result";
        selection: FinalSelection;
        candidate: CandidateRef | null;
      }
    | { type: "revision"; revision: number };
};
export type SolutionHeader = {
  id: string;
  revision: number;
  title: string;
  local_status: "finding" | "resolved";
  updated_ms: number;
};
export type EvidenceRecord = {
  id: string;
  sha256: string;
  metadata: Record<string, unknown> | null;
  provenance: {
    operation: string;
    core_version: string;
    parameters: Json[];
    inputs: Json[];
    created_ms: number;
  };
};
export type SolutionSnapshot = {
  schema_version: 1;
  id: string;
  revision: number;
  title: string;
  explanation: string;
  local_status: "finding" | "resolved";
  privacy: "private";
  verification: "unverified";
  final_output: FinalSelection;
  final_metadata: Record<string, unknown>;
  evidence_path: EvidenceRecord[];
  configuration_scope: "executed_path";
  configurations: Json[];
  executions: Json[];
  candidate: Json | null;
  replay_at_save: Json;
  derived_from_revision: number | null;
};
export type SolutionState = {
  solution: SolutionSnapshot;
  latest_revision: number;
  history: {
    revision: number;
    title: string;
    local_status: "finding" | "resolved";
    created_ms: number;
  }[];
  replay_current: Record<string, unknown>;
};
export type FinalPreview = {
  revision: number;
  metadata: {
    representation: "text" | "bytes" | "structured" | "image";
    semantic_type: string;
    sha256: string;
    byte_size: number;
  };
  value: Json;
};
const uuid =
  /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
const invalid = () =>
  new Error("The solution API returned incompatible private evidence.");
function text(value: unknown, limit = 8192): string {
  if (typeof value !== "string" || value.length > limit) throw invalid();
  return value;
}
function integer(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw invalid();
  return value;
}
function id(value: unknown): string {
  const s = text(value, 36);
  if (!uuid.test(s)) throw invalid();
  return s;
}
function status(value: unknown): "finding" | "resolved" {
  if (value !== "finding" && value !== "resolved") throw invalid();
  return value;
}
function array(value: unknown, limit: number): unknown[] {
  if (!Array.isArray(value) || value.length > limit) throw invalid();
  return value;
}
async function request(
  path: string,
  signal: AbortSignal,
  payload?: SolutionRequest,
  budget = 768 * 1024,
): Promise<Record<string, unknown>> {
  const response = await apiFetch(`/api${path}`, {
    signal,
    credentials: "omit",
    ...(payload
      ? {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        }
      : {}),
  });
  if (!response.body) throw invalid();
  const reader = response.body.getReader();
  const decoder = new TextDecoder("utf-8", { fatal: true });
  let size = 0,
    body = "";
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > budget)
        throw new Error(
          "The solution response exceeds the bounded viewer budget.",
        );
      body += decoder.decode(value, { stream: true });
    }
    body += decoder.decode();
  } finally {
    await reader.cancel();
    reader.releaseLock();
  }
  const value = object(JSON.parse(body));
  if (!response.ok) {
    const error = object(value.error);
    throw new Error(text(error.message, 2048));
  }
  if (value.schema_version !== 1) throw invalid();
  return value;
}
function selection(value: unknown): FinalSelection {
  const v = object(value);
  const port = v.port === null ? null : text(v.port, 128);
  let field: FinalSelection["field"] = null;
  if (v.field !== null) {
    const f = object(v.field);
    if (f.conversion !== "text" && f.conversion !== "base64_bytes")
      throw invalid();
    field = { pointer: text(f.pointer, 512), conversion: f.conversion };
  }
  return { result_id: id(v.result_id), port, field };
}
export function parseSolution(value: Record<string, unknown>): SolutionState {
  const s = object(value.solution);
  if (
    s.schema_version !== 1 ||
    s.privacy !== "private" ||
    s.verification !== "unverified" ||
    s.configuration_scope !== "executed_path"
  )
    throw invalid();
  const evidence = array(s.evidence_path, 64).map((raw) => {
    const r = object(raw),
      p = object(r.provenance);
    return {
      ...r,
      id: id(r.id),
      sha256: text(r.sha256, 64),
      metadata: r.metadata === null ? null : object(r.metadata),
      provenance: {
        operation: text(p.operation, 128),
        core_version: text(p.core_version, 128),
        parameters: array(p.parameters, 32) as Json[],
        inputs: array(p.inputs, 32) as Json[],
        created_ms: integer(p.created_ms),
      },
    } as EvidenceRecord;
  });
  const selected = selection(s.final_output);
  if (!evidence.some((r) => r.id === selected.result_id)) throw invalid();
  const snapshot: SolutionSnapshot = {
    schema_version: 1,
    id: id(s.id),
    revision: integer(s.revision),
    title: text(s.title, 512),
    explanation: text(s.explanation),
    local_status: status(s.local_status),
    privacy: "private",
    verification: "unverified",
    final_output: selected,
    final_metadata: object(s.final_metadata),
    evidence_path: evidence,
    configuration_scope: "executed_path",
    configurations: array(s.configurations, 64) as Json[],
    executions: array(s.executions, 64) as Json[],
    candidate: s.candidate as Json | null,
    replay_at_save: s.replay_at_save as Json,
    derived_from_revision:
      s.derived_from_revision === null
        ? null
        : integer(s.derived_from_revision),
  };
  const latest = integer(value.latest_revision);
  if (snapshot.revision < 1 || latest < snapshot.revision) throw invalid();
  const history = array(value.history, 128).map((raw) => {
    const h = object(raw);
    return {
      revision: integer(h.revision),
      title: text(h.title, 512),
      local_status: status(h.local_status),
      created_ms: integer(h.created_ms),
    };
  });
  return {
    solution: snapshot,
    latest_revision: latest,
    history,
    replay_current: object(value.replay_current),
  };
}
export async function solutions(
  signal: AbortSignal,
): Promise<SolutionHeader[]> {
  const value = await request("/solutions", signal);
  return array(value.solutions, 32).map((raw) => {
    const r = object(raw);
    if (r.privacy !== "private" || r.verification !== "unverified")
      throw invalid();
    return {
      id: id(r.id),
      revision: integer(r.revision),
      title: text(r.title, 512),
      local_status: status(r.local_status),
      updated_ms: integer(r.updated_ms),
    };
  });
}
export async function solution(
  key: string,
  signal: AbortSignal,
  revision?: number,
): Promise<SolutionState> {
  return parseSolution(
    await request(
      `/solutions/${encodeURIComponent(key)}${revision === undefined ? "" : `?revision=${revision}`}`,
      signal,
    ),
  );
}
export async function saveSolution(
  payload: SolutionRequest,
  signal: AbortSignal,
): Promise<SolutionState> {
  return parseSolution(await request("/solutions", signal, payload));
}
export async function finalPreview(
  key: string,
  revision: number,
  signal: AbortSignal,
): Promise<FinalPreview> {
  const value = await request(
      `/solutions/${encodeURIComponent(key)}/final?revision=${revision}`,
      signal,
      undefined,
      8 * 1024 * 1024,
    ),
    m = object(value.metadata);
  if (
    value.solution_id !== key ||
    value.revision !== revision ||
    !["text", "bytes", "structured", "image"].includes(String(m.representation))
  )
    throw invalid();
  if (
    value.value === undefined ||
    (m.representation === "text" && typeof value.value !== "string")
  )
    throw invalid();
  if (m.representation === "bytes" || m.representation === "image") {
    const bytes = object(value.value);
    if (
      typeof bytes.base64 !== "string" ||
      bytes.base64.length % 4 !== 0 ||
      !/^[A-Za-z0-9+/]*={0,2}$/.test(bytes.base64)
    )
      throw invalid();
  }
  return {
    revision,
    metadata: {
      representation:
        m.representation as FinalPreview["metadata"]["representation"],
      semantic_type: text(m.semantic_type, 128),
      sha256: text(m.sha256, 64),
      byte_size: integer(m.byte_size),
    },
    value: value.value as Json,
  };
}
export type RetainedResult = {
  id: string;
  representation: string;
  semantic_type: string;
  metadata: Record<string, unknown>;
  provenance: { operation: string; created_ms: number };
};
export async function retainedResults(
  signal: AbortSignal,
): Promise<RetainedResult[]> {
  const value = await request(
    "/lab/history",
    signal,
    undefined,
    9 * 1024 * 1024,
  );
  return array(value.records, 64).map(parseRetainedResult);
}
function parseRetainedResult(raw: unknown): RetainedResult {
  const r = object(raw),
    p = object(r.provenance);
  return {
    id: id(r.id),
    representation: text(r.representation, 64),
    semantic_type: text(r.semantic_type, 128),
    metadata: r.metadata === null ? {} : object(r.metadata),
    provenance: {
      operation: text(p.operation, 128),
      created_ms: integer(p.created_ms),
    },
  };
}
export async function retainedResult(
  key: string,
  signal: AbortSignal,
): Promise<RetainedResult> {
  const value = await request(
    `/lab/results/${encodeURIComponent(key)}`,
    signal,
  );
  if (value.retained !== true) throw invalid();
  const result = parseRetainedResult(value.result);
  if (result.id !== key) throw invalid();
  return result;
}
export function ports(result: RetainedResult): string[] {
  return result.semantic_type === "operation.outputs" &&
    result.metadata.schema_version === 2
    ? array(result.metadata.ports, 32).map((p) => text(object(p).name, 128))
    : [];
}
export function candidateSelection(
  result: RetainedResult,
  selected: Record<string, unknown>,
): FinalSelection {
  if (selected.kind === "whole") {
    if (ports(result).length) throw invalid();
    return { result_id: result.id, port: null, field: null };
  }
  if (
    selected.kind !== "field" ||
    (selected.conversion !== "text" && selected.conversion !== "base64_bytes")
  )
    throw invalid();
  const pointer = text(selected.pointer, 512);
  if (!pointer.startsWith("/")) throw invalid();
  const names = ports(result);
  if (!names.length)
    return {
      result_id: result.id,
      port: null,
      field: { pointer, conversion: selected.conversion },
    };
  const end = pointer.indexOf("/", 1),
    first = pointer
      .slice(1, end < 0 ? undefined : end)
      .replaceAll("~1", "/")
      .replaceAll("~0", "~");
  if (!names.includes(first)) throw invalid();
  return {
    result_id: result.id,
    port: first,
    field: {
      pointer: end < 0 ? "" : pointer.slice(end),
      conversion: selected.conversion,
    },
  };
}
