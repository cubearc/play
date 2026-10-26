import { object } from "./lab-api";
import { request } from "./notebook-api";
export type CandidateRecord = {
  revision: number;
  result_id: string;
  representation: "text" | "bytes";
  byte_size: number;
  sha256: string;
  selection: Record<string, unknown>;
  status: "private_unverified";
  artifact_sha256: string;
  artifact_operation: string;
  core_version: string;
};
export type CandidateState = {
  candidate: CandidateRecord | null;
  history: CandidateRecord[];
  error: string | null;
};
export function parseCandidates(value: unknown): CandidateState {
  const state = object(value);
  if (
    state.schema_version !== 1 ||
    !Array.isArray(state.history) ||
    state.history.length > 256 ||
    (state.error !== null && typeof state.error !== "string")
  )
    throw new Error("Incompatible local candidate response.");
  function parse(value: unknown): CandidateRecord {
    const item = object(value);
    if (
      item.status !== "private_unverified" ||
      !Number.isSafeInteger(item.revision) ||
      Number(item.revision) < 1 ||
      typeof item.result_id !== "string" ||
      typeof item.sha256 !== "string" ||
      typeof item.artifact_sha256 !== "string" ||
      typeof item.artifact_operation !== "string" ||
      typeof item.core_version !== "string" ||
      !Number.isSafeInteger(item.byte_size) ||
      Number(item.byte_size) < 0 ||
      Number(item.byte_size) > 65536 ||
      !["text", "bytes"].includes(String(item.representation))
    )
      throw new Error("Invalid private candidate record.");
    return {
      revision: Number(item.revision),
      result_id: item.result_id,
      sha256: item.sha256,
      byte_size: Number(item.byte_size),
      representation: item.representation as "text" | "bytes",
      selection: object(item.selection),
      status: "private_unverified",
      artifact_sha256: item.artifact_sha256,
      artifact_operation: item.artifact_operation,
      core_version: item.core_version,
    };
  }
  return {
    candidate: state.candidate === null ? null : parse(state.candidate),
    history: state.history.map(parse),
    error: state.error as string | null,
  };
}
export async function candidates(
  key: string,
  stage: string,
  signal?: AbortSignal,
  payload?: unknown,
): Promise<CandidateState> {
  const query = new URLSearchParams({ key, stage });
  return parseCandidates(
    await request(
      `/challenges/candidates${payload === undefined ? `?${query}` : ""}`,
      signal,
      payload,
    ),
  );
}
