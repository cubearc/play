import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cancelExecution,
  type Execution,
  executeStep,
  executions,
  explorations,
  json,
  openExecution,
  parseCatalog,
  parseExploration,
  type Step,
  saveExploration,
} from "./exploration-api";
import { id, outputPort, typedWire } from "./test/lab-fixture";

afterEach(() => vi.unstubAllGlobals());

describe("canonical operation contracts", () => {
  const document: { operations: unknown[] } = JSON.parse(
    readFileSync(
      resolve(process.cwd(), "../../cube/contracts/v1/catalog.json"),
      "utf8",
    ),
  );
  function catalog() {
    return {
      schema_version: 2,
      contract_schema_version: 1,
      operations: document.operations.map((contract) => ({
        contract,
        execution: { available: true, reason: null, adapter_version: 2 },
      })),
    };
  }
  it("retains every real Core schema, output and exact integer annotation", () => {
    const entries = parseCatalog(catalog());
    expect(entries).toHaveLength(document.operations.length);
    const gcd = entries.find((e) => e.contract.operation === "number.gcd");
    expect(gcd?.contract.inputs[0]?.schema["x-cube-integer"]).toBeDefined();
    expect(
      entries.find((e) => e.contract.operation === "image.histogram")?.contract
        .outputs[0]?.semantic_type,
    ).toBe("image.histogram");
  });
  it("rejects incompatible versions, duplicates and remote schema references", () => {
    expect(() => parseCatalog({ ...catalog(), schema_version: 9 })).toThrow();
    const input = catalog();
    input.operations.push(
      input.operations[0] as (typeof input.operations)[number],
    );
    expect(() => parseCatalog(input)).toThrow();
    const bad = JSON.parse(JSON.stringify(catalog()));
    bad.operations[0].contract.parameters = {
      $ref: "https://invalid.example/schema",
    };
    expect(() => parseCatalog(bad)).toThrow();
  });
  it("never accepts rounded large data values as exact inputs", () => {
    expect(() => json(9007199254740992)).toThrow();
    expect(
      json({ value: "18446744073709551615", bytes: { base64: "AP8=" } }),
    ).toEqual({ value: "18446744073709551615", bytes: { base64: "AP8=" } });
  });
  it("preserves a positive advertised operation version without treating it as the schema version", () => {
    const input = JSON.parse(JSON.stringify(catalog()));
    input.operations[0].contract.operation_version = 3;
    expect(parseCatalog(input)[0]?.contract.operation_version).toBe(3);
    input.operations[0].contract.operation_version = 0;
    expect(() => parseCatalog(input)).toThrow();
  });
  it("preserves separate graph inputs, versions and stage scope", () => {
    const value = {
      schema_version: 1,
      id: "id",
      revision: 3,
      title: "Hypothesis",
      hypothesis: "Investigate",
      scope: { challenge: "local", stage: "first" },
      steps: [
        {
          id: "step",
          operation: "cipher.rot5",
          operation_version: 1,
          inputs: { text: { type: "literal", value: "A1" } },
          parameters: {},
          private_parameters: [],
          position: [0, 0],
        },
      ],
    };
    expect(parseExploration(value)).toEqual(value);
    expect(() => parseExploration({ ...value, schema_version: 2 })).toThrow();
  });
});

describe("exploration transport and replay boundaries", () => {
  const value = {
    schema_version: 1 as const,
    id: "case",
    revision: 1,
    title: "Case",
    hypothesis: "",
    scope: null,
    steps: [],
  };
  const retainedRun: Execution = {
    id: "execution",
    configuration_revision: 1,
    step: "step",
    status: "succeeded",
    stale: true,
    started_ms: 1,
    ended_ms: 2,
    evidence: {
      result_id: id,
      sha256: "0".repeat(64),
      // JSON object order must not affect immutable port matching.
      ports: [
        {
          schema_sha256: outputPort.schema_sha256,
          operation_version: 1,
          schema_version: 1,
          semantic_type: "text",
          name: "text",
        },
      ],
    },
  };
  it("reopens exactly the retained execution without rerunning or rewriting its old configuration", async () => {
    const fetcher = vi
      .fn()
      .mockImplementation(() => Promise.resolve(Response.json(typedWire())));
    vi.stubGlobal("fetch", fetcher);
    expect(
      await openExecution(retainedRun, new AbortController().signal),
    ).toMatchObject({
      id,
      output: { text: "A6" },
      ports: [outputPort],
    });
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher.mock.calls[0]?.[0]).toBe(`/api/lab/results/${id}`);
    expect(fetcher.mock.calls[0]?.[1].method).toBe("GET");
    await expect(
      openExecution(
        { ...retainedRun, evidence: {} },
        new AbortController().signal,
      ),
    ).rejects.toThrow("no retained output");
    await expect(
      openExecution(
        {
          ...retainedRun,
          evidence: { ...retainedRun.evidence, sha256: "wrong" },
        },
        new AbortController().signal,
      ),
    ).rejects.toThrow("immutable evidence");
  });
  it("rejects mismatched identities, unknown ports, malformed metadata and oversized retained data", async () => {
    const badWires = [
      {
        ...typedWire(),
        result: {
          ...typedWire().result,
          id: "6d434f94-89f3-4fb8-a7d8-c345a0000002",
        },
        content_url:
          "/api/lab/results/6d434f94-89f3-4fb8-a7d8-c345a0000002/content",
      },
      typedWire({ undeclared: "not a named output" }),
      {
        ...typedWire(),
        result: {
          ...typedWire().result,
          metadata: { schema_version: 3, ports: [] },
        },
      },
      { ...typedWire(), result: { ...typedWire().result, byte_size: 4194305 } },
      {
        ...typedWire(),
        result: {
          ...typedWire().result,
          metadata: { schema_version: 2, ports: [outputPort, outputPort] },
        },
      },
      {
        ...typedWire(),
        result: {
          ...typedWire().result,
          metadata: {
            schema_version: 2,
            ports: [{ ...outputPort, schema_sha256: "bad" }],
          },
        },
      },
    ];
    for (const response of badWires) {
      vi.stubGlobal(
        "fetch",
        vi.fn().mockResolvedValue(Response.json(response)),
      );
      await expect(
        openExecution(retainedRun, new AbortController().signal),
      ).rejects.toThrow();
    }
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(Response.json({ ...typedWire(), inline: null }))
        .mockResolvedValueOnce(new Response("x".repeat(4194305))),
    );
    await expect(
      openExecution(retainedRun, new AbortController().signal),
    ).rejects.toThrow("budget");
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(Response.json({ ...typedWire(), inline: null }))
        .mockResolvedValueOnce(new Response(null, { status: 404 })),
    );
    await expect(
      openExecution(retainedRun, new AbortController().signal),
    ).rejects.toThrow("history was preserved");
  });
  it("preserves typed file, artifact and branch sources and rejects malformed navigation", () => {
    const input = {
      ...value,
      steps: [
        {
          id: "node",
          operation: "cipher.rot5",
          operation_version: 1,
          inputs: {
            first: {
              type: "file",
              store: "source",
              path: "private.txt",
              representation: "text",
            },
            second: { type: "artifact", id: "result", port: "text" },
            third: { type: "step", step: "upstream", port: "text" },
          },
          parameters: {},
          position: [0, 0],
        },
      ],
    };
    expect(parseExploration(input).steps[0]?.inputs).toEqual(
      input.steps[0]?.inputs,
    );
    for (const source of [
      { type: "unknown" },
      { type: "file", store: "host", path: "/private", representation: "text" },
    ])
      expect(() =>
        parseExploration({
          ...input,
          steps: [{ ...input.steps[0], inputs: { first: source } }],
        }),
      ).toThrow();
    expect(() =>
      parseExploration({
        ...input,
        steps: [{ ...input.steps[0], position: ["bad", 0] }],
      }),
    ).toThrow();
    expect(() => json(Infinity)).toThrow();
    expect(() => json(undefined)).toThrow();
    let nested: unknown = null;
    for (let i = 0; i < 66; i++) nested = [nested];
    expect(() => json(nested)).toThrow();
  });
  it("loads and saves via the private local API and preserves server failures", async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(Response.json({ explorations: [value] }))
      .mockResolvedValueOnce(Response.json({ ...value, revision: 2 }))
      .mockResolvedValueOnce(
        Response.json(
          { error: { message: "Local storage full" } },
          { status: 503 },
        ),
      );
    vi.stubGlobal("fetch", fetch);
    expect(await explorations(new AbortController().signal)).toEqual([value]);
    expect((await saveExploration(value)).revision).toBe(2);
    expect(fetch.mock.calls[1]?.[1]).toMatchObject({
      method: "PUT",
      credentials: "same-origin",
    });
    await expect(saveExploration(value)).rejects.toThrow("Local storage full");
  });
  it("requires a successful current upstream execution before sending a named artifact", async () => {
    const step: Step = {
      id: "step",
      operation: "cipher.rot5",
      operation_version: 1,
      inputs: {
        text: { type: "step", step: "upstream", port: "text" },
        other: { type: "literal", value: "private" },
      },
      parameters: {},
      private_parameters: ["key"],
      position: [0, 0],
    };
    const fetch = vi.fn().mockResolvedValue(Response.json(typedWire()));
    vi.stubGlobal("fetch", fetch);
    const signal = new AbortController().signal;
    await expect(executeStep(value, step, [], {}, signal)).rejects.toThrow(
      "Run the connected upstream",
    );
    expect(fetch).not.toHaveBeenCalled();
    const run: Execution = {
      id: "upstream-run",
      configuration_revision: 1,
      step: "upstream",
      status: "succeeded",
      stale: false,
      evidence: { result_id: "previous-result" },
      started_ms: 1,
      ended_ms: 2,
    };
    expect(
      await executeStep(value, step, [run], { key: "ephemeral" }, signal),
    ).toMatchObject({ id, output: { text: "A6" }, ports: [outputPort] });
    const request = JSON.parse(fetch.mock.calls[0]?.[1].body ?? "{}");
    expect(request.inputs.text).toEqual({
      type: "artifact",
      id: "previous-result",
      port: "text",
    });
    expect(request.parameters).toEqual({ key: "ephemeral" });
    expect(step.parameters).toEqual({});
  });
  it("validates run status, unavailable inline output and cancellation outcomes", async () => {
    const run = {
      id: "run",
      configuration_revision: 1,
      step: "step",
      status: "interrupted",
      stale: false,
      evidence: {},
      started_ms: 1,
      ended_ms: null,
    };
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(Response.json([run]))
      .mockResolvedValueOnce(Response.json([{ ...run, status: "verified" }]))
      .mockResolvedValueOnce(Response.json({ ...typedWire(), inline: null }))
      .mockResolvedValueOnce(Response.json({ text: "large retained text" }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(new Response(null, { status: 409 }));
    vi.stubGlobal("fetch", fetch);
    expect((await executions("case"))[0]?.status).toBe("interrupted");
    await expect(executions("case")).rejects.toThrow();
    const step: Step = {
      id: "step",
      operation: "cipher.rot5",
      operation_version: 1,
      inputs: {},
      parameters: {},
      private_parameters: [],
      position: [0, 0],
    };
    expect(
      await executeStep(value, step, [], {}, new AbortController().signal),
    ).toMatchObject({ id, output: { text: "large retained text" } });
    await cancelExecution("run");
    await expect(cancelExecution("run")).rejects.toThrow(
      "could not be cancelled",
    );
  });
});
