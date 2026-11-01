import {
  type Context,
  context,
  isIdentity,
  type RunRequest,
  runRequest,
} from "./workspace-contract.ts";
export type AttachmentIntent =
  | { kind: "open"; context: Context; selection: RunRequest | null }
  | { kind: "close"; id: string };
export interface AttachmentView {
  id: string;
  prefix: string;
  expiresMs: number;
}
export function attachmentIntent(input: unknown): AttachmentIntent {
  if (!input || typeof input !== "object" || Array.isArray(input))
    throw new Error("invalid_command");
  const v = input as Record<string, unknown>;
  if (
    v.kind === "close" &&
    Object.keys(v).sort().join() === "id,kind" &&
    typeof v.id === "string" &&
    isIdentity(v.id)
  )
    return { kind: "close", id: v.id };
  if (
    v.kind !== "open" ||
    Object.keys(v).sort().join() !== "context,kind,selection"
  )
    throw new Error("invalid_command");
  const scope = context(v.context);
  const selection = v.selection === null ? null : runRequest(v.selection);
  if (
    selection &&
    Object.entries(scope).some(
      ([key, value]) => selection.context[key as keyof Context] !== value,
    )
  )
    throw new Error("context_mismatch");
  return { kind: "open", context: scope, selection };
}
