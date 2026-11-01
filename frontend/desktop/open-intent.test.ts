import assert from "node:assert/strict";
import test from "node:test";
import { openIntent, workspaceLink } from "./open-intent.ts";

const workspace = "11111111-1111-4111-8111-111111111111";
const exploration = "22222222-2222-4222-8222-222222222222";
const stage = "33333333-3333-4333-8333-333333333333";
test("desktop links carry only canonical local identities and no file or execution authority", () => {
  assert.deepEqual(
    openIntent(
      `cube-play-open://workspace/${workspace}?exploration=${exploration}&stage=${stage}`,
    ),
    { workspace_id: workspace, exploration_id: exploration, stage_id: stage },
  );
  assert.deepEqual(openIntent(`cube-play-open://workspace/${workspace}`), {
    workspace_id: workspace,
    exploration_id: null,
    stage_id: null,
  });
  for (const url of [
    `cube-play-open://workspace/${workspace}?path=/etc/passwd`,
    `cube-play-open://workspace/${workspace}?run=true`,
    `cube-play-open://workspace/${workspace}?stage=${stage}`,
    `cube-play-open://workspace/${workspace}?exploration=${exploration}&exploration=${exploration}`,
    `cube-play-open://user@workspace/${workspace}`,
    `cube-play-open://workspace:9/${workspace}`,
    `cube-play-open://workspace/${workspace}#execute`,
    `cube-play-open://workspace/%31${workspace.slice(1)}`,
    `cube-play-open://workspace/../${workspace}`,
    "file:///etc/passwd",
    "https://example.com",
    "cube-play://app/index.html",
  ])
    assert.throws(() => openIntent(url), /invalid_open_intent/, url);
});

test("a copied local location round trips without adding content or execution", () => {
  const location = {
    workspace_id: workspace,
    exploration_id: exploration,
    stage_id: stage,
  };
  assert.deepEqual(openIntent(workspaceLink(location)), location);
});
