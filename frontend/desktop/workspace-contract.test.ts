import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import {
  assetRead,
  cancelRun,
  context,
  imageImport,
  request,
  runRequest,
} from "./workspace-contract.ts";

const owner = {
  workspace_id: randomUUID(),
  exploration_id: randomUUID(),
  stage_id: randomUUID(),
};
test("duplication accepts closed exact source snapshots without path or partial caption authority", () => {
  const intent = {
    kind: "duplicate_block",
    context: owner,
    key: randomUUID(),
    block_id: randomUUID(),
    expected_revision: 1,
    source: { asset: { asset_id: randomUUID(), revision: 2 }, caption: null },
  };
  assert.deepEqual(request(intent), intent);
  const caption = { asset_id: randomUUID(), revision: 1 };
  const parsed = request({ ...intent, source: { ...intent.source, caption } });
  assert.ok(parsed.kind === "duplicate_block");
  assert.deepEqual(parsed.source, { ...intent.source, caption });
  for (const source of [
    null,
    [],
    {},
    { asset: intent.source.asset },
    { ...intent.source, path: "/private" },
    { ...intent.source, asset: { asset_id: "../source", revision: 1 } },
    { ...intent.source, asset: { ...intent.source.asset, revision: 0 } },
    { ...intent.source, caption: { asset_id: randomUUID() } },
    { ...intent.source, caption: { ...caption, revision: Infinity } },
    { ...intent.source, caption: { ...caption, path: "/private" } },
  ])
    assert.throws(() => request({ ...intent, source }));
  assert.throws(() => request({ ...intent, path: "/private" }));
  assert.throws(() => request({ ...intent, expected_revision: 0 }));
});
test("Block ordering accepts only a bounded complete-order intent with unique UUIDs", () => {
  const order = {
    kind: "reorder_blocks",
    context: owner,
    key: randomUUID(),
    expected_revision: 2,
    block_ids: [randomUUID(), randomUUID()],
  };
  assert.deepEqual(request(order), order);
  for (const block_ids of [
    [],
    [order.block_ids[0], order.block_ids[0]],
    ["../asset"],
    Array.from({ length: 9 }, () => randomUUID()),
    null,
  ])
    assert.throws(() => request({ ...order, block_ids }));
  for (const expected_revision of [0, -1, 1.5, Number.MAX_SAFE_INTEGER + 1])
    assert.throws(() => request({ ...order, expected_revision }));
  assert.throws(() => request({ ...order, path: "/private" }));
  assert.throws(() =>
    request({ ...order, context: { ...owner, stage_id: "foreign" } }),
  );
});
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
  const edit = {
    kind: "revise_text",
    context: owner,
    key: randomUUID(),
    block_id: randomUUID(),
    asset_id: randomUUID(),
    expected_revision: 1,
    text: "é".repeat(512),
  };
  assert.deepEqual(request(edit), edit);
  assert.throws(() => request({ ...edit, path: "/private" }));
  assert.throws(() => request({ ...edit, text: `${edit.text}x` }));
  assert.throws(() => request({ ...edit, expected_revision: 0 }));
  const run = {
    context: owner,
    key: randomUUID(),
    trail_id: randomUUID(),
    pipeline_id: randomUUID(),
    pipeline_revision: 1,
    source_asset_id: randomUUID(),
    source_revision: 1,
    retain_output: true,
    operation: "cipher.rot13",
    operation_version: 1,
  };
  assert.deepEqual(runRequest(run), run);
  assert.throws(() => runRequest({ ...run, source_asset_id: "../private" }));
  assert.throws(() => runRequest({ ...run, source_revision: 0 }));
  assert.throws(() => runRequest({ ...run, source_revision: undefined }));
  assert.throws(() => runRequest({ ...run, retain_output: undefined }));
  assert.throws(() => cancelRun({ context: owner, run_id: "../run" }));
  assert.deepEqual(
    request({
      kind: "list_runs",
      context: owner,
      trail_id: run.trail_id,
      before: null,
    }).kind,
    "list_runs",
  );
  assert.throws(() =>
    request({
      kind: "list_runs",
      context: owner,
      trail_id: run.trail_id,
      before: 0,
    }),
  );
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
  assert.throws(() => runRequest({ ...run, operation_version: 2 ** 32 }));
  assert.throws(() => runRequest({ ...run, input: "literal authority" }));
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
