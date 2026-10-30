import {
  type Context,
  cancelRun,
  context,
  type RunRequest,
  runRequest,
} from "./workspace-contract.ts";
export type TerminalIntent =
  | {
      kind: "open";
      context: Context;
      selection: RunRequest | null;
      columns: number;
      rows: number;
    }
  | { kind: "read" | "close"; context: Context; session_id: string }
  | { kind: "write"; context: Context; session_id: string; input: string }
  | {
      kind: "resize";
      context: Context;
      session_id: string;
      columns: number;
      rows: number;
    };
export interface TerminalView {
  id: string;
  state: "open" | "closed";
  code: string | null;
  output: string;
  run_id: string | null;
  trail_id: string | null;
}
export function terminalIntent(input: unknown): TerminalIntent {
  if (!input || typeof input !== "object" || Array.isArray(input))
    throw new Error("invalid_command");
  const value = input as Record<string, unknown>;
  const fields: Record<string, string> = {
    open: "columns,context,kind,rows,selection",
    read: "context,kind,session_id",
    close: "context,kind,session_id",
    write: "context,input,kind,session_id",
    resize: "columns,context,kind,rows,session_id",
  };
  if (
    typeof value.kind !== "string" ||
    fields[value.kind] !== Object.keys(value).sort().join()
  )
    throw new Error("invalid_command");
  const scope = context(value.context);
  if (value.kind === "open" || value.kind === "resize") {
    if (
      !Number.isSafeInteger(value.columns) ||
      !Number.isSafeInteger(value.rows) ||
      (value.columns as number) < 20 ||
      (value.columns as number) > 300 ||
      (value.rows as number) < 4 ||
      (value.rows as number) > 100
    )
      throw new Error("invalid_terminal_size");
  }
  if (value.kind === "open") {
    const selection =
      value.selection === null ? null : runRequest(value.selection);
    if (
      selection &&
      JSON.stringify(selection.context) !== JSON.stringify(scope)
    )
      throw new Error("context_mismatch");
    return {
      kind: "open",
      context: scope,
      selection,
      columns: value.columns as number,
      rows: value.rows as number,
    };
  }
  const session = cancelRun({
    context: scope,
    run_id: value.session_id,
  }).run_id;
  if (value.kind === "write") {
    if (
      typeof value.input !== "string" ||
      !value.input.length ||
      new TextEncoder().encode(value.input).length > 1024
    )
      throw new Error("input_limit");
    return {
      kind: "write",
      context: scope,
      session_id: session,
      input: value.input,
    };
  }
  if (value.kind === "resize")
    return {
      kind: "resize",
      context: scope,
      session_id: session,
      columns: value.columns as number,
      rows: value.rows as number,
    };
  return {
    kind: value.kind as "read" | "close",
    context: scope,
    session_id: session,
  };
}
