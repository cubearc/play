import { readFileSync } from "node:fs";

export const contract = JSON.parse(
  readFileSync(new URL("../../desktop/protocol.json", import.meta.url), "utf8"),
) as typeof import("../../desktop/protocol.json");

export interface Invocation {
  operation: string;
  operation_version: number;
  input: string;
}

export interface Operation {
  contract: {
    inputs: { name: string; semantic_type: string; schema: unknown }[];
    outputs: { name: string; semantic_type: string; schema: unknown }[];
    parameters: unknown;
  };
  operation: string;
  name: string;
  operation_version: number;
  description: string;
  input_type: string;
  parameters: Record<string, unknown>;
}

export type RunResult =
  | {
      status: "completed";
      output:
        | { text: string }
        | { bytes: number[] }
        | { value: unknown }
        | { ports: Record<string, unknown> };
    }
  | { status: "cancelled" }
  | { status: "failed"; code: string };

export function invocation(value: unknown): Invocation {
  if (
    value === null ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).sort().join() !== "input,operation,operation_version"
  ) {
    throw new Error("invalid_command");
  }
  const request = value as Record<string, unknown>;
  if (
    typeof request.input !== "string" ||
    Buffer.byteLength(request.input) > contract.input_bytes ||
    typeof request.operation !== "string" ||
    request.operation.length > 128 ||
    typeof request.operation_version !== "number" ||
    !Number.isSafeInteger(request.operation_version) ||
    request.operation_version < 1 ||
    request.operation_version > 0xffffffff
  ) {
    throw new Error("invalid_command");
  }
  return request as unknown as Invocation;
}
