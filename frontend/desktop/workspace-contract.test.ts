import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import {
  assetRead,
  assetRun,
  context,
  imageImport,
  request,
} from "./workspace-contract.ts";

const owner = {
  workspace_id: randomUUID(),
  exploration_id: randomUUID(),
  stage_id: randomUUID(),
};
test("the workspace bridge accepts closed scoped intents without path or generic IPC authority", () => {
  assert.deepEqual(context(owner), owner);
  assert.deepEqual(request({ kind: "list" }), { kind: "list" });
  const add = {
    kind: "add_text",
    context: owner,
    key: randomUUID(),
    title: "Caption",
    text: "é".repeat(512),
  };
  assert.deepEqual(request(add), add);
  const run = {
    context: owner,
    asset_id: randomUUID(),
    revision: 1,
    operation: "cipher.rot13",
    operation_version: 1,
  };
  assert.deepEqual(assetRun(run), run);
  assert.deepEqual(
    imageImport({ context: owner, key: randomUUID(), caption: "Caption" })
      .context,
    owner,
  );
  for (const value of [
    null,
    [],
    { kind: "__proto__" },
    { kind: "toString" },
    { kind: "list", path: "/private" },
    { kind: "open", workspace_id: "../source" },
    { ...add, context: { ...owner, stage_id: "arbitrary" } },
    { ...add, text: `${add.text}x` },
    { kind: "add_image", png: "raw", context: owner },
  ])
    assert.throws(() => request(value));
  assert.throws(() =>
    imageImport({
      context: owner,
      key: randomUUID(),
      caption: "",
      path: "/private",
    }),
  );
  for (const revision of [0, -1, 1.5, Number.MAX_SAFE_INTEGER + 1, Infinity])
    assert.throws(() =>
      assetRead({ context: owner, asset_id: randomUUID(), revision }),
    );
  assert.throws(() => assetRun({ ...run, operation_version: 2 ** 32 }));
  assert.throws(() => assetRun({ ...run, input: "literal authority" }));
  assert.throws(() =>
    request({
      kind: "save_note",
      context: owner,
      key: randomUUID(),
      note_id: randomUUID(),
      expected_revision: 1,
      text: "x".repeat(2049),
    }),
  );
});
