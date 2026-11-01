import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import { exportIntent } from "./export-contract.ts";

test("exports accept exact scoped evidence only and never renderer file authority", () => {
  const context = {
    workspace_id: randomUUID(),
    exploration_id: randomUUID(),
    stage_id: randomUUID(),
  };
  const valid = {
    context,
    source: { kind: "asset", asset_id: randomUUID(), revision: 1 },
    format: "data",
  };
  assert.deepEqual(exportIntent(valid), valid);
  for (const invalid of [
    { ...valid, path: "/tmp/source.txt" },
    { ...valid, bytes: "forged" },
    { ...valid, source: { ...valid.source, path: "/tmp/source.txt" } },
    { ...valid, source: { ...valid.source, revision: 0 } },
    { ...valid, source: { kind: "literal", value: "forged" } },
    {
      ...valid,
      source: {
        kind: "run_output",
        run_id: randomUUID(),
        attempt_id: randomUUID(),
        port: "../output",
      },
    },
    {
      ...valid,
      source: { kind: "solution", solution_id: randomUUID(), revision: 1 },
    },
    { ...valid, format: "html" },
  ])
    assert.throws(() => exportIntent(invalid), /invalid_command/);
});
