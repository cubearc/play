import {
  type CandidateSelection,
  type Context,
  context,
  isIdentity,
} from "./workspace-contract.ts";

export type ExportSource =
  | CandidateSelection
  | { kind: "solution"; solution_id: string; revision: number };
export interface ExportIntent {
  context: Context;
  source: ExportSource;
  format: "data" | "document";
}
export interface ExportReceipt {
  fileName: string;
  size: number;
  sha256: string;
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("invalid_command");
  return value as Record<string, unknown>;
}
function keys(value: Record<string, unknown>, expected: string[]) {
  if (Object.keys(value).sort().join() !== expected.sort().join())
    throw new Error("invalid_command");
}
export function exportIntent(value: unknown): ExportIntent {
  const input = object(value);
  keys(input, ["context", "source", "format"]);
  const owner = context(input.context),
    source = object(input.source);
  if (source.kind === "asset" || source.kind === "solution") {
    const id = source.kind === "asset" ? "asset_id" : "solution_id";
    keys(source, ["kind", id, "revision"]);
    if (
      !isIdentity(source[id]) ||
      !Number.isSafeInteger(source.revision) ||
      Number(source.revision) < 1
    )
      throw new Error("invalid_command");
  } else if (source.kind === "run_output") {
    keys(source, ["kind", "run_id", "attempt_id", "port"]);
    if (
      !isIdentity(source.run_id) ||
      !isIdentity(source.attempt_id) ||
      typeof source.port !== "string" ||
      !/^[a-zA-Z0-9_-]{1,64}$/.test(source.port)
    )
      throw new Error("invalid_command");
  } else throw new Error("invalid_command");
  if (input.format !== "data" && input.format !== "document")
    throw new Error("invalid_command");
  if (source.kind === "solution" && input.format !== "document")
    throw new Error("invalid_command");
  return {
    context: owner,
    source: source as unknown as ExportSource,
    format: input.format,
  };
}
