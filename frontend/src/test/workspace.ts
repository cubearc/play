import type { FileEntry, Inspection, Listing } from "../workspace-api";

export function entry(
  name = "notes.txt",
  changes: Partial<FileEntry> = {},
): FileEntry {
  return {
    name,
    path: name,
    kind: "file",
    category: "text",
    size: 20,
    modified_ms: 1000,
    accessible: true,
    ...changes,
  };
}
export function listing(
  entries: FileEntry[] = [],
  changes: Partial<Listing> = {},
): Listing {
  return {
    store: "source",
    path: "",
    entries,
    next_offset: null,
    truncated: false,
    imports_available: true,
    page_size: 128,
    entry_limit: 4096,
    ...changes,
  };
}
export function inspection(
  name = "notes.txt",
  changes: Partial<Inspection> = {},
): Inspection {
  return {
    store: "source",
    path: name,
    entry: entry(name),
    signatures: [],
    extension_mismatch: false,
    preview: { kind: "text", available: true, mime: "text/plain" },
    text: { value: "Private fixture text.", bytes: 21, truncated: false },
    rows: [{ offset: 0, hex: "4869", ascii: "Hi" }],
    offset: 0,
    next_offset: null,
    notice: null,
    image: null,
    media: null,
    ...changes,
  };
}
