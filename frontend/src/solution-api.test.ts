import { afterEach, expect, it, vi } from "vitest";
import { setSessionToken } from "./capability";
import {
  candidateSelection,
  finalPreview,
  parseSolution,
  ports,
  retainedResult,
  retainedResults,
  saveSolution,
  solution,
  solutions,
} from "./solution-api";
import { id } from "./test/lab-fixture";
import {
  finalWire,
  retained,
  solutionId,
  solutionWire,
} from "./test/solution-fixture";

afterEach(() => vi.unstubAllGlobals());
it("reads bounded private snapshots and final values and sends only an explicit typed revision request", async () => {
  setSessionToken("local-test-capability");
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({
        schema_version: 1,
        solutions: [
          {
            id: solutionId,
            revision: 1,
            title: "Digit finding",
            local_status: "finding",
            privacy: "private",
            verification: "unverified",
            updated_ms: 1,
          },
        ],
      }),
    )
    .mockResolvedValueOnce(Response.json(solutionWire()))
    .mockResolvedValueOnce(Response.json(solutionWire(2)))
    .mockResolvedValueOnce(Response.json(finalWire(2)));
  vi.stubGlobal("fetch", fetcher);
  const signal = new AbortController().signal;
  expect(await solutions(signal)).toHaveLength(1);
  expect((await solution(solutionId, signal, 1)).solution.privacy).toBe(
    "private",
  );
  const request = {
    id: solutionId,
    revision: 1,
    title: "Changed explanation",
    explanation: "Keep the exact earlier path",
    local_status: "resolved" as const,
    source: { type: "revision" as const, revision: 1 },
  };
  expect((await saveSolution(request, signal)).latest_revision).toBe(2);
  expect((await finalPreview(solutionId, 2, signal)).value).toBe("A6");
  expect(JSON.parse(fetcher.mock.calls[2]?.[1].body)).toEqual(request);
  expect(fetcher.mock.calls[2]?.[1].method).toBe("PUT");
  for (const [, options] of fetcher.mock.calls) {
    expect(options.credentials).toBe("omit");
    expect(options.redirect).toBe("error");
    expect(options.headers.get("Authorization")).toMatch(/^Bearer /);
  }
});
it("preserves legacy associations and resolves explicit candidate pointers using actual stored port names", async () => {
  const legacy = {
    ...retained,
    id,
    representation: "bytes",
    semantic_type: "bytes",
    metadata: {},
  };
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValueOnce(
        Response.json({
          schema_version: 1,
          records: [{ ...legacy, metadata: null }, retained],
        }),
      )
      .mockResolvedValueOnce(
        Response.json({ schema_version: 1, retained: true, result: retained }),
      ),
  );
  const signal = new AbortController().signal;
  const results = await retainedResults(signal);
  const first = results[0];
  if (!first) throw new Error("Expected the retained legacy result");
  expect(ports(first)).toEqual([]);
  expect(ports(await retainedResult(id, signal))).toEqual(["text"]);
  expect(candidateSelection(legacy, { kind: "whole" })).toEqual({
    result_id: id,
    port: null,
    field: null,
  });
  expect(
    candidateSelection(retained, {
      kind: "field",
      pointer: "/text",
      conversion: "text",
    }),
  ).toEqual({
    result_id: id,
    port: "text",
    field: { pointer: "", conversion: "text" },
  });
  expect(
    candidateSelection(
      {
        ...retained,
        metadata: { schema_version: 2, ports: [{ name: "data/part" }] },
      },
      {
        kind: "field",
        pointer: "/data~1part/base64",
        conversion: "base64_bytes",
      },
    ),
  ).toEqual({
    result_id: id,
    port: "data/part",
    field: { pointer: "/base64", conversion: "base64_bytes" },
  });
  expect(
    candidateSelection(
      { ...legacy, representation: "structured" },
      { kind: "field", pointer: "/channels/0", conversion: "text" },
    ).field?.pointer,
  ).toBe("/channels/0");
  expect(() => candidateSelection(retained, { kind: "whole" })).toThrow();
  expect(() =>
    candidateSelection(retained, {
      kind: "field",
      pointer: "/missing",
      conversion: "text",
    }),
  ).toThrow();
  expect(() => candidateSelection(legacy, { kind: "script" })).toThrow();
  const wire = solutionWire();
  const s = wire.solution as Record<string, unknown>;
  const records = s.evidence_path as Record<string, unknown>[];
  const record = records[0];
  if (!record) throw new Error("Expected the frozen evidence record");
  record.metadata = null;
  expect(parseSolution(wire).solution.evidence_path[0]?.metadata).toBeNull();
});
it("rejects false verification claims, missing evidence, malformed content and oversized responses", async () => {
  const wrong = solutionWire();
  (wrong.solution as Record<string, unknown>).verification = "verified";
  expect(() => parseSolution(wrong)).toThrow();
  const absent = solutionWire();
  (absent.solution as Record<string, unknown>).evidence_path = [];
  expect(() => parseSolution(absent)).toThrow();
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValueOnce(
        Response.json(
          {
            schema_version: 1,
            error: { message: "Snapshot budget exhausted" },
          },
          { status: 413 },
        ),
      )
      .mockResolvedValueOnce(new Response("x".repeat(768 * 1024 + 1)))
      .mockResolvedValueOnce(Response.json({ ...finalWire(), value: 7 }))
      .mockResolvedValueOnce(
        Response.json({
          ...finalWire(),
          metadata: { ...finalWire().metadata, representation: "bytes" },
          value: { base64: "not bytes!" },
        }),
      ),
  );
  const signal = new AbortController().signal;
  await expect(solutions(signal)).rejects.toThrow("budget exhausted");
  await expect(solutions(signal)).rejects.toThrow("bounded viewer budget");
  await expect(finalPreview(solutionId, 1, signal)).rejects.toThrow();
  await expect(finalPreview(solutionId, 1, signal)).rejects.toThrow();
});
