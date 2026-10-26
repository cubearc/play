import { afterEach, expect, it, vi } from "vitest";
import { entry, inspection, listing } from "./test/workspace";
import { importFile, inspectFile, listFiles, mediaUrl } from "./workspace-api";

afterEach(() => vi.unstubAllGlobals());
const signal = () => new AbortController().signal;

it("preserves literal Unicode names, private credentials and byte offsets", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(Response.json(listing([entry()])))
    .mockResolvedValueOnce(Response.json(inspection()))
    .mockResolvedValueOnce(
      Response.json({ store: "imports", path: "private 空.txt" }),
    );
  vi.stubGlobal("fetch", fetcher);
  expect(
    (await listFiles("source", "nested 空", 128, signal())).entries[0]?.name,
  ).toBe("notes.txt");
  expect(fetcher.mock.calls[0]?.[0]).toContain(
    "path=nested+%E7%A9%BA&offset=128",
  );
  expect(
    (await inspectFile("source", "notes.txt", 4096, signal())).rows[0]?.hex,
  ).toBe("4869");
  const file = new File(["private"], "private 空.txt");
  expect(await importFile(file, file.name, signal())).toEqual({
    store: "imports",
    path: file.name,
  });
  expect(fetcher.mock.calls[2]?.[1]).toEqual(
    expect.objectContaining({
      credentials: "same-origin",
      method: "POST",
      body: file,
    }),
  );
  expect(mediaUrl("imports", "a&file?.png")).toBe(
    "/api/workspace/media?store=imports&path=a%26file%3F.png&offset=0",
  );
});

it("parses evidence, dimensions, bounded text and structured container facts", async () => {
  const value = inspection("sample.png", {
    entry: entry("sample.png", { category: "image", modified_ms: null }),
    signatures: [{ format: "png", mime: "image/png", rule: "fixed prefix" }],
    extension_mismatch: true,
    preview: { kind: "image", available: true, mime: "image/png" },
    text: null,
    image: { width: 2, height: 2 },
    media: { tracks: [] },
    notice: "A bounded fact",
    next_offset: 4096,
  });
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(value)));
  expect(await inspectFile("imports", "sample.png", 0, signal())).toEqual(
    value,
  );
});

it.each([
  null,
  [],
  { store: "remote" },
  { ...listing(), entries: "wrong" },
  { ...listing(), entries: Array.from({ length: 129 }, () => entry()) },
  { ...listing(), entries: [entry("bad", { size: -1 })] },
  {
    ...listing(),
    entries: [entry("bad", { path: null, kind: "symlink", accessible: false })],
    truncated: "yes",
  },
  { ...listing(), next_offset: Number.MAX_SAFE_INTEGER + 1 },
])("rejects malformed directory contracts", async (value) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(value)));
  await expect(listFiles("source", "", 0, signal())).rejects.toThrow(
    "unexpected response",
  );
});

it.each([
  {
    ...inspection(),
    preview: { kind: "script", available: true, mime: "text/html" },
  },
  {
    ...inspection(),
    entry: entry("x", { accessible: "yes" as unknown as boolean }),
  },
  {
    ...inspection(),
    text: { value: "x".repeat(65537), bytes: 65537, truncated: true },
  },
  { ...inspection(), rows: [{ offset: 0, hex: "f".repeat(33), ascii: "x" }] },
  { ...inspection(), signatures: [{ format: 1, mime: "x", rule: "x" }] },
  { ...inspection(), image: { width: -1, height: 2 } },
])("rejects malformed inspector contracts", async (value) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(value)));
  await expect(inspectFile("source", "x", 0, signal())).rejects.toThrow(
    "unexpected response",
  );
});

it.each([401, 403, 500])(
  "provides readable fallbacks for non-JSON HTTP %s errors",
  async (status) => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response("not JSON", { status })),
    );
    await expect(listFiles("source", "", 0, signal())).rejects.toThrow(
      status === 500 ? "Refresh" : "Access was denied",
    );
  },
);
it("uses bounded server diagnostics and propagates cancellation", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValueOnce(
        Response.json({ message: "File vanished." }, { status: 404 }),
      )
      .mockRejectedValueOnce(new DOMException("Canceled", "AbortError")),
  );
  await expect(listFiles("source", "", 0, signal())).rejects.toThrow(
    "File vanished.",
  );
  await expect(inspectFile("source", "x", 0, signal())).rejects.toMatchObject({
    name: "AbortError",
  });
});
