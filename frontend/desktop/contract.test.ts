import assert from "node:assert/strict";
import test from "node:test";
import { contract, invocation } from "./contract.ts";

test("rejects an operation version outside Rust's wire range before forwarding", () => {
  assert.throws(
    () =>
      invocation({
        operation: "cipher.rot13",
        operation_version: 2 ** 32,
        input: "hello",
      }),
    /invalid_command/,
  );
});

test("measures UTF-8 bytes and refuses extra authority fields", () => {
  const value = {
    operation: "cipher.rot13",
    operation_version: 1,
    input: "é".repeat(contract.input_bytes / 2),
  };
  assert.deepEqual(invocation(value), value);
  assert.throws(
    () => invocation({ ...value, input: `${value.input}x` }),
    /invalid_command/,
  );
  assert.throws(
    () => invocation({ ...value, path: "/private" }),
    /invalid_command/,
  );
});
