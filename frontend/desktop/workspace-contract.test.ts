import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import { singleGraph } from "./graph-fixture.js";
import {
  assetRead,
  cancelRun,
  challengeImport,
  context,
  fileImport,
  imageImport,
  request,
  runRequest,
} from "./workspace-contract.ts";

const owner = {
  workspace_id: randomUUID(),
  exploration_id: randomUUID(),
  stage_id: randomUUID(),
};
test("Page source is bounded plain text and cannot request paths or executable views", () => {
  const page = {
    kind: "add_page",
    context: owner,
    key: randomUUID(),
    title: "Clue",
    text: '<script>window.play.workspace({kind:"create"})</script>',
  };
  assert.deepEqual(request(page), page);
  assert.throws(
    () => request({ ...page, text: "é".repeat(513) }),
    /input_limit/,
  );
  for (const extra of [
    { path: "/tmp/clue.html" },
    { url: "file:///etc/passwd" },
    { webPreferences: { nodeIntegration: true } },
    { execute: true },
  ])
    assert.throws(() => request({ ...page, ...extra }), /invalid_command/);
});
test("annotations use bounded Note text and exact tagged Note lifecycle snapshots", () => {
  const add = {
    kind: "add_annotation",
    context: owner,
    key: randomUUID(),
    title: "Observation",
    text: "á".repeat(1024),
  };
  assert.deepEqual(request(add), add);
  assert.throws(() => request({ ...add, text: `${add.text}x` }), /input_limit/);
  for (const kind of ["duplicate_block", "archive_block", "restore_block"]) {
    const value = {
      kind,
      context: owner,
      key: randomUUID(),
      block_id: randomUUID(),
      expected_revision: 1,
      source: { kind: "note", note_id: randomUUID(), revision: 2 },
    };
    assert.deepEqual(request(value), value);
    for (const source of [
      { ...value.source, revision: 0 },
      { ...value.source, asset: { asset_id: randomUUID(), revision: 1 } },
      { note_id: value.source.note_id, revision: 2 },
      { kind: "asset", asset: { asset_id: randomUUID(), revision: 1 } },
      { asset: { asset_id: randomUUID(), revision: 1 }, caption: null },
    ])
      assert.throws(() => request({ ...value, source }), /invalid_command/);
  }
});
test("saving an output requires a closed Run/attempt/port intent without renderer output data", () => {
  const value = {
    kind: "pin_run_output",
    context: owner,
    key: randomUUID(),
    expected_revision: 2,
    run_id: randomUUID(),
    attempt_id: randomUUID(),
    port: "output",
  };
  assert.deepEqual(request(value), value);
  for (const candidate of [
    { ...value, attempt_id: undefined },
    { ...value, output: "forged" },
    { ...value, base64: "AA==" },
    { ...value, port: "" },
    { ...value, port: "output.value" },
    { ...value, port: "x".repeat(65) },
    { ...value, expected_revision: 0 },
    { ...value, context: { ...owner, exploration_id: "foreign" } },
  ])
    assert.throws(() => request(candidate), /invalid_command/);
});
test("file import grants only a chooser intent, never a renderer path or bytes", () => {
  const value = { context: owner, key: randomUUID() };
  assert.deepEqual(fileImport(value), value);
  for (const candidate of [
    null,
    {},
    { ...value, path: "/private/secret" },
    { ...value, base64: "AA==" },
    { ...value, url: "https://example.com/" },
    { ...value, key: "bad" },
    { ...value, context: { ...owner, stage_id: "bad" } },
  ])
    assert.throws(() => fileImport(candidate), /invalid_command/);
  assert.throws(
    () =>
      request({ kind: "add_file", ...value, title: "file", base64: "AA==" }),
    /invalid_command/,
  );
});
test("archive and restore require the exact closed Block snapshot and structural revision", () => {
  for (const kind of ["archive_block", "restore_block"]) {
    const intent = {
      kind,
      context: owner,
      key: randomUUID(),
      block_id: randomUUID(),
      expected_revision: 2,
      source: {
        kind: "asset",
        asset: { asset_id: randomUUID(), revision: 1 },
        caption: null,
      },
    };
    assert.deepEqual(request(intent), intent);
    for (const invalid of [
      { ...intent, expected_revision: 0 },
      { ...intent, path: "/private" },
      {
        ...intent,
        source: {
          ...intent.source,
          asset: { asset_id: "../other", revision: 1 },
        },
      },
      {
        ...intent,
        source: { ...intent.source, caption: { asset_id: randomUUID() } },
      },
      {
        ...intent,
        source: {
          ...intent.source,
          caption: { asset_id: randomUUID(), revision: -1 },
        },
      },
    ])
      assert.throws(() => request(invalid));
  }
});
test("duplication accepts closed exact source snapshots without path or partial caption authority", () => {
  const intent = {
    kind: "duplicate_block",
    context: owner,
    key: randomUUID(),
    block_id: randomUUID(),
    expected_revision: 1,
    source: {
      kind: "asset",
      asset: { asset_id: randomUUID(), revision: 2 },
      caption: null,
    },
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
    trail_revision: 1,
    target_node_id: randomUUID(),
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

test("Pipeline intents freeze required bounded configuration and cannot carry privileged paths or implicit bindings", () => {
  const pipeline = {
    kind: "create_pipeline",
    workspace_id: owner.workspace_id,
    key: randomUUID(),
    title: "Decode clue",
    definition: singleGraph({
      operation: "cipher.rot13",
      operation_version: 1,
      parameters: {},
      input_type: "text",
      contract: {
        inputs: [{ name: "input", semantic_type: "text", schema: {} }],
        outputs: [],
        parameters: {},
      },
    }),
  };
  assert.deepEqual(request(pipeline), pipeline);
  for (const value of [
    { ...pipeline, path: "/private" },
    { ...pipeline, definition: null },
    {
      ...pipeline,
      definition: {
        ...pipeline.definition,
        nodes: [
          { ...pipeline.definition.nodes[0], operationVersion: 4294967296 },
        ],
      },
    },
    {
      ...pipeline,
      definition: {
        ...pipeline.definition,
        nodes: [
          {
            ...pipeline.definition.nodes[0],
            parameters: { text: "é".repeat(1024) },
          },
        ],
      },
    },
  ])
    assert.throws(() => request(value));
  const trail = {
    kind: "save_trail",
    bindings: {},
    context: owner,
    key: randomUUID(),
    trail_id: randomUUID(),
    expected_revision: 1,
    hypothesis: "Own hypothesis",
    pipeline: { id: randomUUID(), revision: 2 },
  };
  assert.deepEqual(request(trail), trail);
  assert.throws(
    () =>
      request({
        ...trail,
        pipeline: { ...trail.pipeline, assetId: randomUUID() },
      }),
    /invalid_command/,
  );
  assert.throws(
    () => request({ ...trail, hypothesis: "é".repeat(1025) }),
    /input_limit/,
  );
  const { definition: _definition, ...missing } = pipeline;
  assert.throws(() => request(missing), /invalid_command/);
});
test("named input bindings and explicit targets use a closed bounded IPC contract", () => {
  const definition = singleGraph({
    operation: "cipher.rot13",
    operation_version: 1,
    parameters: {},
    input_type: "text",
    contract: {
      inputs: [{ name: "input", semantic_type: "text", schema: {} }],
      outputs: [],
      parameters: {},
    },
  });
  definition.inputs.push({ name: "caption", semanticType: "text" });
  const node = definition.nodes[0];
  assert.ok(node);
  node.inputs.input = { kind: "input", name: "caption" };
  const create = {
    kind: "create_pipeline",
    workspace_id: owner.workspace_id,
    key: randomUUID(),
    title: "Bound input",
    definition,
  };
  assert.deepEqual(request(create), create);
  for (const inputs of [
    [],
    [...definition.inputs, { name: "caption", semanticType: "text" }],
    [
      { name: "source", semanticType: "text" },
      { name: "Caption", semanticType: "text" },
    ],
    Array.from({ length: 5 }, (_, i) => ({
      name: i ? `slot${i}` : "source",
      semanticType: "text",
    })),
  ])
    assert.throws(() =>
      request({ ...create, definition: { ...definition, inputs } }),
    );
  const trail = {
    kind: "save_trail",
    context: owner,
    key: randomUUID(),
    trail_id: randomUUID(),
    expected_revision: 1,
    hypothesis: "",
    pipeline: { id: randomUUID(), revision: 1 },
    bindings: { caption: { asset_id: randomUUID(), revision: 1 } },
  };
  assert.deepEqual(request(trail), trail);
  for (const bindings of [
    { source: trail.bindings.caption },
    { caption: { ...trail.bindings.caption, path: "/private" } },
    { caption: { ...trail.bindings.caption, revision: 0 } },
    Object.fromEntries(
      ["a", "b", "c", "d"].map((s) => [s, trail.bindings.caption]),
    ),
  ])
    assert.throws(() => request({ ...trail, bindings }));
  const run = {
    context: owner,
    key: randomUUID(),
    trail_id: trail.trail_id,
    trail_revision: 1,
    target_node_id: node.id,
    pipeline_id: trail.pipeline.id,
    pipeline_revision: 1,
    source_asset_id: randomUUID(),
    source_revision: 1,
    operation: "cipher.rot13",
    operation_version: 1,
    retain_output: true,
  };
  assert.deepEqual(runRequest(run), run);
  for (const value of [
    { ...run, trail_revision: undefined },
    { ...run, trail_revision: 0 },
    { ...run, target_node_id: undefined },
    { ...run, target_node_id: "/private" },
  ])
    assert.throws(() => runRequest(value));
});

test("Challenge import accepts only chooser intent, never renderer paths or package data", () => {
  const value = { workspace_id: owner.workspace_id, key: randomUUID() };
  assert.deepEqual(challengeImport(value), value);
  for (const extra of [
    { path: "/private/data" },
    { package: "e30=" },
    { url: "https://example.org" },
    { execute: true },
  ]) {
    assert.throws(
      () => challengeImport({ ...value, ...extra }),
      /invalid_command/,
    );
  }
  assert.throws(
    () => challengeImport({ ...value, workspace_id: "bad" }),
    /invalid_command/,
  );
  assert.throws(
    () => request({ kind: "import_challenge", ...value, package: "e30=" }),
    /invalid_command/,
  );
});

test("Candidate selections accept only exact scoped immutable references", () => {
  const value = {
    kind: "select_candidate",
    context: owner,
    key: randomUUID(),
    expected_revision: 0,
    selection: { kind: "asset", asset_id: randomUUID(), revision: 1 },
  };
  assert.deepEqual(request(value), value);
  assert.deepEqual(
    request({ kind: "get_candidate", context: owner, revision: null }),
    { kind: "get_candidate", context: owner, revision: null },
  );
  const output = {
    kind: "run_output",
    run_id: randomUUID(),
    attempt_id: randomUUID(),
    port: "number",
  };
  assert.deepEqual(
    request({ ...value, selection: output }).kind,
    "select_candidate",
  );
  for (const selection of [
    { ...value.selection, revision: 0 },
    { ...value.selection, path: "/tmp/answer" },
    { ...output, port: "../answer" },
    { ...output, value: 42 },
    { kind: "literal", value: "answer" },
  ])
    assert.throws(() => request({ ...value, selection }), /invalid_command/);
  for (const expected_revision of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1])
    assert.throws(
      () => request({ ...value, expected_revision }),
      /invalid_command/,
    );
});
