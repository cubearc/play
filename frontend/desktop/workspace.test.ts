import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { contract } from "./contract.ts";
import { Engine } from "./engine.ts";
import {
  contextOf,
  type ExplorationView,
  type WorkspaceView,
} from "./workspace-contract.ts";

const binary = fileURLToPath(
  new URL(
    process.platform === "win32"
      ? "../desktop-native/play-engine.exe"
      : "../desktop-native/play-engine",
    import.meta.url,
  ),
);
const worker =
  process.platform === "darwin"
    ? fileURLToPath(
        new URL(
          "../desktop-native/Worker.app/Contents/MacOS/play-engine",
          import.meta.url,
        ),
      )
    : undefined;

test("private IPC resolves exact owned asset revisions and refuses unscoped managed execution", {
  timeout: 15000,
}, async () => {
  const temporary = await mkdtemp(join(tmpdir(), "play-managed-ipc-"));
  const root = join(temporary, "private");
  let engine = await Engine.connect(binary, worker, root);
  try {
    const workspace = (await engine.workspace({
      kind: "create",
      key: randomUUID(),
      title: "IPC notebook",
    })) as WorkspaceView;
    const created = (await engine.workspace({
      kind: "create_exploration",
      workspace_id: workspace.id,
      key: randomUUID(),
      title: "Signal",
    })) as { explorationId: string; stageId: string };
    const open = {
      kind: "open_exploration",
      workspace_id: workspace.id,
      exploration_id: created.explorationId,
      stage_id: created.stageId,
    };
    let exploration = (await engine.workspace(open)) as ExplorationView;
    const context = contextOf(exploration);
    await engine.workspace({
      kind: "add_text",
      context,
      key: randomUUID(),
      title: "Clue",
      text: "Gur xrl.",
    });
    exploration = (await engine.workspace(open)) as ExplorationView;
    const asset = exploration.stage.blocks[0]?.asset;
    assert.ok(asset);
    const invocation = {
      context,
      asset_id: asset.id,
      revision: 1,
      operation: "cipher.rot13",
      operation_version: 1,
    };
    await assert.rejects(
      engine.run({
        operation: "cipher.rot13",
        operation_version: 1,
        input: "literal",
      }),
      /scoped_invocation_required/,
    );
    await assert.rejects(
      engine.runAsset({ ...invocation, revision: 2 }),
      /not_found/,
    );
    await assert.rejects(
      engine.runAsset({
        ...invocation,
        context: { ...context, workspace_id: randomUUID() },
      }),
      /workspace_mismatch/,
    );
    if (worker) {
      const result = await engine.runAsset(invocation);
      assert.equal(result.status, "completed");
      if (result.status === "completed")
        assert.deepEqual(result.output, { text: "The key." });
    } else
      await assert.rejects(
        engine.runAsset(invocation),
        /native_policy_unavailable/,
      );
    await engine.close();
    engine = await Engine.connect(binary, worker, root);
    const listing = (await engine.workspace({ kind: "list" })) as {
      lastWorkspaceId: string;
    };
    assert.equal(listing.lastWorkspaceId, workspace.id);
    await engine.workspace({ kind: "open", workspace_id: workspace.id });
    const restored = (await engine.workspace(open)) as ExplorationView;
    assert.equal(restored.stage.blocks[0]?.asset.id, asset.id);
  } finally {
    await engine.close();
    await rm(temporary, { recursive: true, force: true });
  }
});

test("another process cannot own a live workspace and a killed owner releases its OS lock", {
  timeout: 15000,
}, async () => {
  const temporary = await mkdtemp(join(tmpdir(), "play-owner-ipc-"));
  const root = join(temporary, "private");
  const first = await Engine.connect(binary, undefined, root);
  const workspace = (await first.workspace({
    kind: "create",
    key: randomUUID(),
    title: "Lock proof",
  })) as WorkspaceView;
  await first.close();
  const owner = spawn(binary, ["--ipc", "--data-root", root], {
    stdio: ["pipe", "pipe", "ignore"],
  });
  const lines = createInterface({ input: owner.stdout });
  async function request(value: object) {
    const reply = once(lines, "line", { signal: AbortSignal.timeout(3000) });
    owner.stdin.write(`${JSON.stringify(value)}\n`);
    return JSON.parse((await reply)[0] as string) as {
      type: string;
      data?: WorkspaceView;
    };
  }
  let contender: Engine | undefined;
  try {
    assert.equal(
      (
        await request({
          type: "hello",
          id: 1,
          protocol: contract.protocol,
          engine: contract.engine,
          nonce: "0123456789abcdef0123456789abcdef",
        })
      ).type,
      "ready",
    );
    assert.equal(
      (
        await request({
          type: "workspace",
          id: 2,
          command: { kind: "open", workspace_id: workspace.id },
        })
      ).data?.id,
      workspace.id,
    );
    contender = await Engine.connect(binary, undefined, root);
    await assert.rejects(
      contender.workspace({ kind: "open", workspace_id: workspace.id }),
      /workspace_busy/,
    );
    const exit = once(owner, "exit");
    owner.kill("SIGKILL");
    await exit;
    const opened = (await contender.workspace({
      kind: "open",
      workspace_id: workspace.id,
    })) as WorkspaceView;
    assert.equal(opened.id, workspace.id);
  } finally {
    if (owner.exitCode === null && owner.signalCode === null) {
      const exit = once(owner, "exit");
      owner.kill();
      await exit;
    }
    lines.close();
    await contender?.close();
    await rm(temporary, { recursive: true, force: true });
  }
});
