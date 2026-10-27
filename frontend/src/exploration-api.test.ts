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
  parseCatalog,
  parseExploration,
  type Step,
  saveExploration,
} from "./exploration-api";

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
    const fetch = vi.fn().mockResolvedValue(
      Response.json({
        schema_version: 1,
        result: { id: "result" },
        inline: { type: "structured", data: { text: "A6" } },
      }),
    );
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
    ).toEqual({ id: "result", output: { text: "A6" } });
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
      .mockResolvedValueOnce(
        Response.json({
          schema_version: 1,
          result: { id: "large-result" },
          inline: null,
        }),
      )
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
    ).toEqual({ id: "large-result", output: null });
    await cancelExecution("run");
    await expect(cancelExecution("run")).rejects.toThrow(
      "could not be cancelled",
    );
  });
});
