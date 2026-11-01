import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import { request } from "./workspace-contract.ts";

const context = {
  workspace_id: randomUUID(),
  exploration_id: randomUUID(),
  stage_id: randomUUID(),
};
const command = (value: unknown) => ({
  kind: "add_structured",
  context,
  key: randomUUID(),
  title: "Typed clue",
  semantic_type: "custom.record",
  value,
});

test("structured requests preserve explicit null, scalars and nested inert values", () => {
  for (const value of [
    null,
    18446744073709551615n,
    false,
    0,
    1.5,
    "<script>inert</script>",
    [1, null],
    { score: 7 },
  ]) {
    const input = command(value);
    assert.deepEqual(request(input), input);
  }
});
test("non-JSON, unsafe, excessive and open-ended structured requests are rejected", () => {
  const cycle: unknown[] = [];
  cycle.push(cycle);
  for (const value of [
    undefined,
    18446744073709551616n,
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.MAX_SAFE_INTEGER + 1,
    new Date(),
    new Map(),
    { nested: undefined },
    cycle,
    "x".repeat(16384),
  ]) {
    assert.throws(() => request(command(value)), /invalid_command|input_limit/);
  }
  assert.throws(
    () => request({ ...command({}), semantic_type: "untrusted path/" }),
    /invalid_command/,
  );
  assert.throws(
    () => request({ ...command({}), host_path: "/private/data" }),
    /invalid_command/,
  );
});
