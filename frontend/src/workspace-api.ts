export type Store = "source" | "imports";
export type Category =
  | "image"
  | "video"
  | "audio"
  | "text"
  | "binary"
  | "unknown";
export type FileEntry = {
  name: string;
  path: string | null;
  kind: "file" | "directory" | "symlink" | "special" | "unavailable";
  category: Category;
  size: number | null;
  modified_ms: number | null;
  accessible: boolean;
};
export type Listing = {
  store: Store;
  path: string;
  entries: FileEntry[];
  next_offset: number | null;
  truncated: boolean;
  imports_available: boolean;
  page_size: number;
  entry_limit: number;
};
export type Inspection = {
  store: Store;
  path: string;
  entry: FileEntry;
  signatures: { format: string; mime: string; rule: string }[];
  extension_mismatch: boolean;
  preview: { kind: Category; available: boolean; mime: string };
  text: { value: string; bytes: number; truncated: boolean } | null;
  rows: { offset: number; hex: string; ascii: string }[];
  offset: number;
  next_offset: number | null;
  notice: string | null;
  image: { width: number; height: number } | null;
  media: Record<string, unknown> | null;
};

const invalid = () =>
  new Error("The local file API returned an unexpected response.");
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw invalid();
  return value as Record<string, unknown>;
}
function text(value: unknown, limit = 4096): string {
  if (typeof value !== "string" || value.length > limit) throw invalid();
  return value;
}
function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw invalid();
  return value;
}
function boolean(value: unknown): boolean {
  if (typeof value !== "boolean") throw invalid();
  return value;
}
function optionalNumber(value: unknown): number | null {
  return value === null ? null : number(value);
}
function choice<T extends string>(value: unknown, choices: readonly T[]): T {
  if (typeof value !== "string" || !choices.some((item) => item === value))
    throw invalid();
  return value as T;
}
const categories = [
  "image",
  "video",
  "audio",
  "text",
  "binary",
  "unknown",
] as const;
const stores = ["source", "imports"] as const;
function array<T>(
  value: unknown,
  limit: number,
  parse: (value: unknown) => T,
): T[] {
  if (!Array.isArray(value) || value.length > limit) throw invalid();
  return value.map(parse);
}
function entry(value: unknown): FileEntry {
  const item = object(value);
  return {
    name: text(item.name),
    path: item.path === null ? null : text(item.path),
    kind: choice(item.kind, [
      "file",
      "directory",
      "symlink",
      "special",
      "unavailable",
    ]),
    category: choice(item.category, categories),
    size: optionalNumber(item.size),
    modified_ms: optionalNumber(item.modified_ms),
    accessible: boolean(item.accessible),
  };
}

async function response(request: Promise<Response>): Promise<unknown> {
  const result = await request;
  if (!result.ok) {
    let message =
      result.status === 401 || result.status === 403
        ? "Access was denied. Reopen your Cube Play session link or select an authorized file."
        : "The file request could not finish. Refresh and try again.";
    try {
      const error = object(await result.json());
      message = text(error.message, 512);
    } catch {
      /* Keep a bounded, readable fallback for non-JSON errors. */
    }
    throw new Error(message);
  }
  return result.json();
}
function url(endpoint: string, store: Store, path: string, offset = 0) {
  return `/api/workspace/${endpoint}?${new URLSearchParams({ store, path, offset: String(offset) })}`;
}
export function mediaUrl(store: Store, path: string) {
  return url("media", store, path);
}

export async function listFiles(
  store: Store,
  path: string,
  offset: number,
  signal: AbortSignal,
): Promise<Listing> {
  const value = object(
    await response(
      fetch(url("files", store, path, offset), {
        signal,
        credentials: "same-origin",
      }),
    ),
  );
  return {
    store: choice(value.store, stores),
    path: text(value.path),
    entries: array(value.entries, 128, entry),
    next_offset: optionalNumber(value.next_offset),
    truncated: boolean(value.truncated),
    imports_available: boolean(value.imports_available),
    page_size: number(value.page_size),
    entry_limit: number(value.entry_limit),
  };
}
export async function inspectFile(
  store: Store,
  path: string,
  offset: number,
  signal: AbortSignal,
): Promise<Inspection> {
  const value = object(
    await response(
      fetch(url("inspect", store, path, offset), {
        signal,
        credentials: "same-origin",
      }),
    ),
  );
  const preview = object(value.preview);
  const body = value.text === null ? null : object(value.text);
  const dimensions = value.image === null ? null : object(value.image);
  return {
    store: choice(value.store, stores),
    path: text(value.path),
    entry: entry(value.entry),
    signatures: array(value.signatures, 32, (value) => {
      const signature = object(value);
      return {
        format: text(signature.format),
        mime: text(signature.mime),
        rule: text(signature.rule),
      };
    }),
    extension_mismatch: boolean(value.extension_mismatch),
    preview: {
      kind: choice(preview.kind, categories),
      available: boolean(preview.available),
      mime: text(preview.mime, 128),
    },
    text: body
      ? {
          value: text(body.value, 65536),
          bytes: number(body.bytes),
          truncated: boolean(body.truncated),
        }
      : null,
    rows: array(value.rows, 256, (value) => {
      const row = object(value);
      return {
        offset: number(row.offset),
        hex: text(row.hex, 32),
        ascii: text(row.ascii, 16),
      };
    }),
    offset: number(value.offset),
    next_offset: optionalNumber(value.next_offset),
    notice: value.notice === null ? null : text(value.notice, 512),
    image: dimensions
      ? { width: number(dimensions.width), height: number(dimensions.height) }
      : null,
    media: value.media === null ? null : object(value.media),
  };
}
export async function importFile(
  file: File,
  name: string,
  signal: AbortSignal,
): Promise<{ store: Store; path: string }> {
  const value = object(
    await response(
      fetch(`/api/workspace/import?${new URLSearchParams({ name })}`, {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/octet-stream" },
        body: file,
        signal,
      }),
    ),
  );
  return { store: choice(value.store, stores), path: text(value.path) };
}
