import { expect, it } from "vitest";
import type { RunDetail } from "../desktop/workspace-contract";
import { attemptOutputType, formatResultValue } from "./ResultValue";

it("renders bytes explicitly while preserving text and structured JSON values", () => {
  expect(formatResultValue({ base64: "AP8=" }, "bytes")).toBe("Base64 AP8=");
  expect(formatResultValue("<script>literal text</script>", "text")).toBe(
    "<script>literal text</script>",
  );
  expect(formatResultValue({ base64: "AP8=" }, "json")).toBe(
    '{\n  "base64": "AP8="\n}',
  );
  for (const value of [null, false, 0, [1, 2]])
    expect(formatResultValue(value, "json")).toBe(
      JSON.stringify(value, null, 2),
    );
});

it("uses each contributing node's declared output type and never guesses an unknown port", () => {
  const run: RunDetail = {
    id: "run",
    sequence: 1,
    caller: "desktop",
    trailId: "trail",
    pipelineId: "pipeline",
    pipelineRevision: 1,
    status: "completed",
    createdMs: 1,
    finishedMs: 2,
    retainOutput: true,
    operation: {
      operation: "test.target",
      operationVersion: 1,
      name: "Target",
    },
    plan: {
      pipeline: { id: "pipeline", revision: 1, definition: {} },
      contract: {
        operation: "test.target",
        operation_version: 1,
        inputs: [],
        outputs: [{ name: "output", semantic_type: "text" }],
      },
      contractDigest: "target-contract",
      targetNodes: ["target"],
      nodes: [
        {
          id: "upstream",
          operation: { name: "Upstream" },
          contractDigest: "upstream-contract",
        },
      ],
      contracts: {
        "upstream-contract": {
          outputs: [{ name: "output", semantic_type: "json" }],
        },
      },
      reusePolicy: "none",
      reusedOutputs: [],
      inputs: {},
      parameters: {},
      limits: {},
    },
    result: { status: "completed", outputs: { output: "done" } },
    attempts: [],
  };
  expect(attemptOutputType(run, "target", "output")).toBe("text");
  expect(attemptOutputType(run, "upstream", "output")).toBe("json");
  expect(attemptOutputType(run, "upstream", "missing")).toBeUndefined();
  expect(attemptOutputType(run, "target", "missing")).toBeUndefined();
  expect(attemptOutputType(run, "unknown", "output")).toBeUndefined();
});
