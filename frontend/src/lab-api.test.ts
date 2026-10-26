import { afterEach, describe, expect, it, vi } from "vitest";
import {
  channelUrl,
  execute,
  histogram,
  object,
  operations,
  parseResult,
  pixelBytes,
  resultUrl,
  structured,
} from "./lab-api";
import { catalog, counts, id, metadata, wire } from "./test/lab-fixture";

const signal = () => new AbortController().signal;
afterEach(() => vi.unstubAllGlobals());
describe("typed Lab boundary", () => {
  it("uses advertised versions and exact same-origin requests, supports structured references", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(Response.json(catalog()))
      .mockResolvedValueOnce(Response.json(wire()))
      .mockResolvedValueOnce(Response.json(counts));
    vi.stubGlobal("fetch", fetcher);
    const entries = await operations(signal());
    const operation = entries[0];
    if (!operation) throw new Error("Missing fixture operation");
    const result = await execute(
      operation,
      { type: "file", store: "source", path: "a.png", representation: "bytes" },
      [],
      signal(),
    );
    expect(result.metadata).toEqual(metadata);
    expect(JSON.parse(fetcher.mock.calls[1]?.[1].body).version).toBe(1);
    expect(fetcher.mock.calls[1]?.[1].credentials).toBe("same-origin");
    expect(
      await structured({ ...result, representation: "structured" }, signal()),
    ).toEqual(counts);
    expect(
      await structured(
        {
          ...result,
          representation: "structured",
          inline: { type: "structured", data: counts },
        },
        signal(),
      ),
    ).toEqual(counts);
  });
  it("preserves transient bytes and accepts bounded histogram tuples", () => {
    const result = parseResult(
      wire("image.pixels.read", {
        retained: false,
        content_url: null,
        inline: { type: "bytes", base64: "FFA0" },
      }),
    );
    expect(pixelBytes(result)).toEqual([20, 80, 52]);
    expect(histogram(counts)).toEqual(counts);
    expect(channelUrl({ preview_png_base64: "aGVsbG8=" })).toBe(
      "data:image/png;base64,aGVsbG8=",
    );
    for (const value of [null, "https://outside.test/", 2])
      expect(() => channelUrl({ preview_png_base64: value })).toThrow();
    expect(histogram({ ...counts, colors: [] }).colors).toEqual([]);
  });
  it("cancels a disconnected execution without reusing its ID or submitting results", async () => {
    const fetcher = vi
      .fn()
      .mockImplementationOnce(
        (_url, options) =>
          new Promise((_resolve, reject) =>
            options.signal.addEventListener("abort", () =>
              reject(new DOMException("aborted", "AbortError")),
            ),
          ),
      )
      .mockRejectedValueOnce(new Error("closed"));
    vi.stubGlobal("fetch", fetcher);
    const controller = new AbortController();
    const promise = execute(
      { id: "image.compare", version: 1, description: "", parameters: [] },
      { type: "artifact", id },
      [],
      controller.signal,
      false,
      { type: "artifact", id },
    );
    const rejection = expect(promise).rejects.toThrow("aborted");
    controller.abort();
    await rejection;
    expect(fetcher.mock.calls[1]?.[1].method).toBe("DELETE");
    const body = JSON.parse(fetcher.mock.calls[0]?.[1].body);
    expect(body.retain_result).toBe(false);
    expect(body.other).toEqual({ type: "artifact", id });
  });
  it("reports bounded Core errors and failed content fetches", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(
          Response.json(
            {
              schema_version: 1,
              error: { message: "Budget exceeded.", parameter: "max_colors" },
            },
            { status: 413 },
          ),
        )
        .mockResolvedValueOnce(Response.json({}, { status: 404 })),
    );
    await expect(operations(signal())).rejects.toThrow(
      "Budget exceeded. Parameter: max_colors.",
    );
    await expect(
      structured(
        { ...parseResult(wire()), representation: "structured" },
        signal(),
      ),
    ).rejects.toThrow("unavailable");
  });
  it("rejects untrusted URLs, types, oversized lists and incompatible contracts", async () => {
    for (const value of [null, [], false, 4])
      expect(() => object(value)).toThrow();
    for (const value of [
      "../escape",
      "https://outside.test/",
      id.replace("4fb8", "1fb8"),
    ])
      expect(() => resultUrl(value)).toThrow();
    for (const value of [
      wire("", { retained: "yes" }),
      wire("", { content_url: "https://outside.test" }),
      wire("", { retained: false }),
      wire("", { result: { ...wire().result, byte_size: -1 } }),
      wire("", { result: { ...wire().result, id: "wrong" } }),
      wire("", {
        result: { ...wire().result, metadata: { ...metadata, width: 0 } },
      }),
      wire("", {
        result: { ...wire().result, metadata: { ...metadata, height: 20000 } },
      }),
      wire("", {
        result: { ...wire().result, metadata: { ...metadata, channels: 7 } },
      }),
      wire("", { inline: { type: 2 } }),
    ])
      expect(() => parseResult(value)).toThrow();
    const parsed = parseResult(
      wire("", { result: { ...wire().result, metadata: null } }),
    );
    expect(parsed.metadata).toBeNull();
    for (const inline of [
      null,
      { type: "text", data: "hi" },
      { type: "bytes", base64: "a".repeat(12) },
    ])
      expect(() => pixelBytes({ ...parsed, inline })).toThrow();
    for (const value of [
      {},
      { ...counts, channels: Array(5).fill("red") },
      { ...counts, colors: Array(10001).fill({}) },
      { ...counts, colors: [{ values: [256], count: 1 }] },
      { ...counts, colors: [{ values: [1, 2, 3, 4, 5], count: 1 }] },
      { ...counts, pixels: 1.1 },
      { ...counts, channels: [1] },
    ])
      expect(() => histogram(value)).toThrow();
    await expect(structured(parsed, signal())).rejects.toThrow();
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(
          Response.json({ schema_version: 2, operations: [] }),
        )
        .mockResolvedValueOnce(
          Response.json({ schema_version: 1, operations: null }),
        )
        .mockResolvedValueOnce(
          Response.json({
            schema_version: 1,
            operations: [{ core: {}, lab: null }],
          }),
        )
        .mockResolvedValueOnce(
          Response.json({
            schema_version: 1,
            operations: [{ core: { parameters: null }, lab: {} }],
          }),
        )
        .mockResolvedValueOnce(
          Response.json(
            { schema_version: 1, error: { message: "Denied" } },
            { status: 403 },
          ),
        ),
    );
    await expect(operations(signal())).rejects.toThrow();
    await expect(operations(signal())).rejects.toThrow();
    expect(await operations(signal())).toEqual([]);
    await expect(operations(signal())).rejects.toThrow();
    await expect(operations(signal())).rejects.toThrow("Denied");
  });
});
