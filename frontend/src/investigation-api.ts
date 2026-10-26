import type { Store } from "./workspace-api";

export type View = {
  store: Store;
  path: string;
  offset: number;
  selected: { store: Store; path: string } | null;
};
export type Investigation = {
  id: string;
  title: string;
  revision: number;
  view: View;
  view_revision: number;
  interrupted_sessions: number;
  warnings: string[];
};
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("The investigation API returned an invalid response.");
  return value as Record<string, unknown>;
}
function text(value: unknown): string {
  if (typeof value !== "string" || value.length > 4096)
    throw new Error("The investigation API returned invalid text.");
  return value;
}
function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw new Error("The investigation API returned an invalid revision.");
  return value;
}
function store(value: unknown): Store {
  if (value !== "source" && value !== "imports")
    throw new Error("The investigation API returned an invalid file store.");
  return value;
}
async function response(result: Response): Promise<Record<string, unknown>> {
  if (!result.ok) {
    let message =
      result.status === 409
        ? "This investigation changed in another tab. Reload before saving again."
        : "Local state could not be saved or restored. Check storage and reopen the workspace.";
    try {
      message = text(object(await result.json()).message);
    } catch {
      /* Keep the readable fallback. */
    }
    throw new Error(message);
  }
  return object(await result.json());
}
export async function loadInvestigation(
  signal: AbortSignal,
): Promise<Investigation> {
  const value = await response(
    await fetch("/api/investigation", { signal, credentials: "same-origin" }),
  );
  const view = object(value.view);
  const selection = view.selected === null ? null : object(view.selected);
  if (!Array.isArray(value.warnings) || value.warnings.length > 16)
    throw new Error("The investigation API returned invalid recovery notices.");
  return {
    id: text(value.id),
    title: text(value.title),
    revision: number(value.revision),
    view_revision: number(value.view_revision),
    interrupted_sessions: number(value.interrupted_sessions),
    warnings: value.warnings.map(text),
    view: {
      store: store(view.store),
      path: text(view.path),
      offset: number(view.offset),
      selected: selection
        ? { store: store(selection.store), path: text(selection.path) }
        : null,
    },
  };
}
export async function saveInvestigation(
  endpoint: "" | "/view",
  payload: unknown,
): Promise<number> {
  const result = await response(
    await fetch(`/api/investigation${endpoint}`, {
      method: "PUT",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    }),
  );
  return number(result.revision);
}
