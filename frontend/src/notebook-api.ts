import { apiFetch } from "./capability";
import { type LabResult, object, parseResult } from "./lab-api";
import type { Store } from "./workspace-api";
export type Note = {
  id: string;
  revision: number;
  title: string;
  body: string;
  target: { store: Store; path: string } | null;
};
export type Saved = {
  entry: { id: string; title: string; revision: number };
  record: Record<string, unknown> | null;
  error: string | null;
};
export async function request(
  path: string,
  signal?: AbortSignal,
  payload?: unknown,
): Promise<unknown> {
  const response = await apiFetch(`/api${path}`, {
    credentials: "omit",
    ...(signal ? { signal } : {}),
    ...(payload === undefined
      ? {}
      : {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        }),
  });
  let value: unknown;
  try {
    value = await response.json();
  } catch {
    throw new Error(
      "The local API returned an incompatible response. Reopen the session or retry a bounded request.",
    );
  }
  if (!response.ok) {
    const error = object(value);
    throw new Error(
      typeof error.message === "string"
        ? error.message
        : typeof error.error === "object"
          ? String(object(error.error).message)
          : "The local record is unavailable. Reload before retrying.",
    );
  }
  return value;
}
export async function reopen(
  id: string,
  signal: AbortSignal,
): Promise<LabResult> {
  return parseResult(object(await request(`/lab/results/${id}`, signal)));
}
export async function notes(signal: AbortSignal): Promise<Note[]> {
  const value = await request("/investigation/notes", signal);
  if (!Array.isArray(value) || value.length > 100)
    throw new Error("Invalid local notes response.");
  return value.map((item: unknown) => {
    const note = object(item);
    if (
      typeof note.id !== "string" ||
      typeof note.title !== "string" ||
      typeof note.body !== "string" ||
      typeof note.revision !== "number"
    )
      throw new Error("Invalid local note.");
    const target = note.target === null ? null : object(note.target);
    if (
      target &&
      (typeof target.path !== "string" ||
        !["source", "imports"].includes(String(target.store)))
    )
      throw new Error("Invalid note association.");
    return {
      id: note.id,
      title: note.title,
      body: note.body,
      revision: note.revision,
      target: target
        ? { store: target.store as Store, path: target.path as string }
        : null,
    };
  });
}
export async function library(signal: AbortSignal): Promise<Saved[]> {
  const value = object(await request("/lab/library", signal));
  if (!Array.isArray(value.entries) || value.entries.length > 64)
    throw new Error("Invalid artifact library.");
  return value.entries.map((item: unknown) => {
    const value = object(item),
      entry = object(value.entry);
    if (
      typeof entry.id !== "string" ||
      typeof entry.title !== "string" ||
      typeof entry.revision !== "number" ||
      (value.error !== null && typeof value.error !== "string")
    )
      throw new Error("Invalid artifact reference.");
    return {
      entry: { id: entry.id, title: entry.title, revision: entry.revision },
      record: value.record === null ? null : object(value.record),
      error: value.error,
    };
  });
}
