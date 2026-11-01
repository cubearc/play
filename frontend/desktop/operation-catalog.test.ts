import assert from "node:assert/strict";
import { test } from "node:test";
import { loadCatalog } from "./operation-catalog.ts";

const entry = (operation: string, unavailable = false) => ({
  operation,
  operation_version: 1,
  availability: unavailable
    ? { state: "unavailable", reason: "retention_policy_unavailable" }
    : { state: "available" },
});
const page = (
  offset: number,
  nextOffset: number | null,
  operations: unknown[],
  total = 2,
) => ({ type: "catalog", offset, nextOffset, total, operations });

test("catalog pages preserve exact identities and explicit unavailable capabilities", async () => {
  const offsets: number[] = [];
  const result = await loadCatalog(async (offset) => {
    offsets.push(offset);
    return offset === 0
      ? page(0, 1, [entry("text.reverse")])
      : page(1, null, [entry("cipher.aes.encrypt_blocks", true)]);
  });
  assert.deepEqual(offsets, [0, 1]);
  assert.deepEqual(
    result.map((op) => op.operation),
    ["text.reverse", "cipher.aes.encrypt_blocks"],
  );
  assert.equal(result[1]?.availability.state, "unavailable");
});

test("catalog collection rejects partial, looping, duplicate, changing and oversized metadata", async () => {
  for (const invalid of [
    page(0, null, [entry("one")]),
    page(0, 0, [entry("one")]),
    page(1, null, [entry("one")]),
    page(0, 1, []),
    page(0, null, [entry("one"), entry("one")]),
    page(0, null, [], 513),
    page(0, null, [entry("one")], 0),
    page(0, null, [entry("one")], -1),
    page(0, null, [{ operation: "one", operation_version: 1 }], 1),
  ])
    await assert.rejects(
      loadCatalog(async () => invalid),
      /invalid_catalog/,
    );
  await assert.rejects(
    loadCatalog(async (offset) =>
      offset === 0
        ? page(0, 1, [entry("one")])
        : page(1, null, [entry("two")], 3),
    ),
    /invalid_catalog/,
  );
});
