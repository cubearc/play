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
  type RunDetail,
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
    const asset = exploration.stage.blocks.find(
      (block) => block.kind === "asset",
    )?.asset;
    assert.ok(asset);
    const trail = exploration.trails[0];
    assert.ok(trail?.pipeline);
    const invocation = {
      context,
      key: randomUUID(),
      trail_id: trail.id,
      pipeline_id: trail.pipeline.id,
      pipeline_revision: trail.pipeline.revision,
      source_asset_id: trail.source.id,
      source_revision: trail.source.revision,
      operation: "cipher.rot13",
      operation_version: 1,
      retain_output: true,
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
      engine.startRun({ ...invocation, pipeline_revision: 2 }),
      /state_conflict/,
    );
    await assert.rejects(
      engine.startRun({
        ...invocation,
        context: { ...context, workspace_id: randomUUID() },
      }),
      /workspace_mismatch/,
    );
    let saved: RunDetail | null = null;
    if (worker) {
      let result = await engine.startRun(invocation);
      const duplicate = await engine.startRun(invocation);
      assert.equal(duplicate.id, result.id);
      while (result.status === "queued" || result.status === "running") {
        await new Promise((resolve) => setTimeout(resolve, 30));
        result = (await engine.workspace({
          kind: "run_detail",
          context,
          run_id: result.id,
        })) as RunDetail;
      }
      assert.equal(result.status, "completed");
      assert.deepEqual(result.result?.outputs, { output: "The key." });
      assert.equal(result.attempts.length, 1);
      await assert.rejects(
        engine.startRun({ ...invocation, retain_output: false }),
        /idempotency_conflict/,
      );
      assert.equal(
        (await engine.cancelRun({ context, run_id: result.id })).status,
        "completed",
      );
      saved = result;
    } else
      await assert.rejects(
        engine.startRun(invocation),
        /native_policy_unavailable/,
      );
    const order = {
      kind: "reorder_blocks",
      context,
      key: randomUUID(),
      expected_revision: exploration.stage.revision,
      block_ids: exploration.stage.blocks.map((block) => block.id).reverse(),
    };
    const reordered = await engine.workspace(order);
    assert.deepEqual(await engine.workspace(order), reordered);
    const afterOrder = (await engine.workspace(open)) as ExplorationView;
    assert.deepEqual(
      afterOrder.stage.blocks.map((block) => block.id),
      order.block_ids,
    );
    assert.deepEqual(afterOrder.trails, exploration.trails);
    await assert.rejects(
      engine.workspace({ ...order, key: randomUUID() }),
      /state_conflict/,
    );
    const sourceBlock = afterOrder.stage.blocks.find(
      (block) => block.kind === "asset",
    );
    assert.ok(sourceBlock?.kind === "asset");
    const copyIntent = {
      kind: "duplicate_block",
      context,
      key: randomUUID(),
      block_id: sourceBlock.id,
      expected_revision: afterOrder.stage.revision,
      source: {
        asset: { asset_id: asset.id, revision: asset.revision },
        caption: null,
      },
    };
    const copyResult = (await engine.workspace(copyIntent)) as {
      blockId: string;
    };
    assert.deepEqual(await engine.workspace(copyIntent), copyResult);
    const afterCopy = (await engine.workspace(open)) as ExplorationView;
    const copied = afterCopy.stage.blocks.find(
      (block) => block.id === copyResult.blockId,
    );
    assert.ok(copied?.kind === "asset");
    assert.notEqual(copied.asset.id, asset.id);
    assert.deepEqual(copied.asset.origin, {
      assetId: asset.id,
      revision: asset.revision,
      title: asset.title,
    });
    const copiedTrail = afterCopy.trails.find(
      (trail) => trail.source.id === copied.asset.id,
    );
    assert.ok(copiedTrail?.pipeline);
    assert.deepEqual(
      (
        (await engine.workspace({
          kind: "list_runs",
          context,
          trail_id: copiedTrail.id,
          before: null,
        })) as { runs: unknown[] }
      ).runs,
      [],
    );
    let copiedRun: RunDetail | null = null;
    if (worker) {
      copiedRun = await finished(
        engine,
        context,
        await engine.startRun({
          ...invocation,
          key: randomUUID(),
          trail_id: copiedTrail.id,
          pipeline_id: copiedTrail.pipeline.id,
          pipeline_revision: copiedTrail.pipeline.revision,
          source_asset_id: copied.asset.id,
          source_revision: 1,
        }),
      );
      assert.equal(copiedRun.status, "completed");
      assert.deepEqual(copiedRun.result?.outputs, { output: "The key." });
      const input = copiedRun.plan.inputs.input;
      assert.ok(input && "assetId" in input);
      assert.equal(input.assetId, copied.asset.id);
    }
    await engine.close();
    engine = await Engine.connect(binary, worker, root);
    const listing = (await engine.workspace({ kind: "list" })) as {
      lastWorkspaceId: string;
    };
    assert.equal(listing.lastWorkspaceId, workspace.id);
    await engine.workspace({ kind: "open", workspace_id: workspace.id });
    const restored = (await engine.workspace(open)) as ExplorationView;
    assert.deepEqual(restored.stage.blocks, afterCopy.stage.blocks);
    assert.equal(
      restored.stage.blocks.find((block) => block.kind === "asset")?.asset.id,
      asset.id,
    );
    if (saved)
      assert.deepEqual(
        await engine.workspace({
          kind: "run_detail",
          context,
          run_id: saved.id,
        }),
        saved,
      );
    if (copiedRun)
      assert.deepEqual(
        await engine.workspace({
          kind: "run_detail",
          context,
          run_id: copiedRun.id,
        }),
        copiedRun,
      );
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

async function runFixture(engine: Engine, text: string) {
  const workspace = (await engine.workspace({
    kind: "create",
    key: randomUUID(),
    title: "Lifecycle notebook",
  })) as WorkspaceView;
  const created = (await engine.workspace({
    kind: "create_exploration",
    workspace_id: workspace.id,
    key: randomUUID(),
    title: "Lifecycle",
  })) as { explorationId: string; stageId: string };
  const context = {
    workspace_id: workspace.id,
    exploration_id: created.explorationId,
    stage_id: created.stageId,
  };
  await engine.workspace({
    kind: "add_text",
    context,
    key: randomUUID(),
    title: "Program",
    text,
  });
  const exploration = (await engine.workspace({
    kind: "open_exploration",
    workspace_id: workspace.id,
    exploration_id: created.explorationId,
    stage_id: created.stageId,
  })) as ExplorationView;
  const trail = exploration.trails[0];
  assert.ok(trail?.pipeline);
  return {
    context,
    key: randomUUID(),
    trail_id: trail.id,
    pipeline_id: trail.pipeline.id,
    pipeline_revision: trail.pipeline.revision,
    source_asset_id: trail.source.id,
    source_revision: trail.source.revision,
    operation: "esolang.brainfuck.execute",
    operation_version: 2,
    retain_output: true,
  };
}
async function finished(
  engine: Engine,
  context: Parameters<Engine["cancelRun"]>[0]["context"],
  run: RunDetail,
) {
  const deadline = Date.now() + 5000;
  while (run.status === "queued" || run.status === "running") {
    assert.ok(
      Date.now() < deadline,
      "the engine must reach a bounded terminal Run state",
    );
    await new Promise((resolve) => setTimeout(resolve, 30));
    run = (await engine.workspace({
      kind: "run_detail",
      context,
      run_id: run.id,
    })) as RunDetail;
  }
  return run;
}

test("archive keeps admitted native work supervised and refuses new work until restore", {
  timeout: 15000,
}, async () => {
  const temporary = await mkdtemp(join(tmpdir(), "play-native-archive-"));
  const engine = await Engine.connect(
    binary,
    worker,
    join(temporary, "private"),
  );
  try {
    const request = await runFixture(engine, "+[]");
    const open = {
      kind: "open_exploration",
      workspace_id: request.context.workspace_id,
      exploration_id: request.context.exploration_id,
      stage_id: request.context.stage_id,
    };
    const before = (await engine.workspace(open)) as ExplorationView;
    const block = before.stage.blocks.find((b) => b.kind === "asset");
    assert.ok(block?.kind === "asset");
    const admitted = worker ? await engine.startRun(request) : null;
    const mutation = {
      kind: "archive_block",
      context: request.context,
      key: randomUUID(),
      block_id: block.id,
      expected_revision: before.stage.revision,
      source: {
        asset: { asset_id: block.asset.id, revision: block.asset.revision },
        caption: null,
      },
    };
    const accepted = await engine.workspace(mutation);
    assert.deepEqual(await engine.workspace(mutation), accepted);
    const archived = (await engine.workspace(open)) as ExplorationView;
    assert.equal(archived.stage.blocks.length, 1);
    assert.equal(archived.trails[0]?.archived, true);
    if (admitted) {
      const still = (await engine.workspace({
        kind: "run_detail",
        context: request.context,
        run_id: admitted.id,
      })) as RunDetail;
      assert.notEqual(still.status, "cancelled");
      assert.deepEqual(still.plan, admitted.plan);
      const cancelled = await engine.cancelRun({
        context: request.context,
        run_id: admitted.id,
      });
      assert.ok(
        cancelled.status === "cancelled" || cancelled.status === "failed",
      );
      assert.deepEqual(cancelled.plan, admitted.plan);
      await assert.rejects(
        engine.startRun({ ...request, key: randomUUID() }),
        /block_archived/,
      );
      assert.equal((await engine.startRun(request)).id, admitted.id);
    } else
      await assert.rejects(
        engine.startRun({ ...request, key: randomUUID() }),
        /native_policy_unavailable/,
      );
    await engine.workspace({
      ...mutation,
      kind: "restore_block",
      key: randomUUID(),
      expected_revision: archived.stage.revision,
    });
    const restored = (await engine.workspace(open)) as ExplorationView;
    assert.deepEqual(restored.stage.blocks, before.stage.blocks);
    assert.equal(restored.trails[0]?.archived, false);
    if (worker) {
      const completed = await finished(
        engine,
        request.context,
        await engine.startRun({
          ...request,
          key: randomUUID(),
          operation: "cipher.rot13",
          operation_version: 1,
        }),
      );
      assert.equal(completed.status, "completed");
      assert.deepEqual(completed.result?.outputs, { output: "+[]" });
    }
  } finally {
    await engine.close();
    await rm(temporary, { recursive: true, force: true });
  }
});

test("durable Runs distinguish real worker failure, cancellation and graceful app shutdown", {
  skip: !worker,
  timeout: 15000,
}, async () => {
  const temporary = await mkdtemp(join(tmpdir(), "play-run-lifecycle-"));
  const root = join(temporary, "private");
  let engine = await Engine.connect(binary, worker, root);
  try {
    const bytesRequest = await runFixture(engine, "++++++++[>++++++++<-]>+.");
    const bytesRun = await finished(
      engine,
      bytesRequest.context,
      await engine.startRun(bytesRequest),
    );
    assert.equal(bytesRun.status, "completed");
    assert.deepEqual(bytesRun.result?.outputs, { output: { base64: "QQ==" } });
    assert.equal(bytesRun.plan.contract.outputs[0]?.semantic_type, "bytes");
    const invalid = await runFixture(engine, "[");
    const failed = await finished(
      engine,
      invalid.context,
      await engine.startRun(invalid),
    );
    assert.equal(failed.status, "failed");
    assert.equal(failed.result?.code, "operation_failed");
    assert.equal(failed.attempts[0]?.status, "failed");
    // Keep the control cell unchanged so Quit is tested against active work.
    const loop = await runFixture(engine, "+[]");
    const running = await engine.startRun(loop);
    assert.equal(running.status, "running");
    const cancelled = await engine.cancelRun({
      context: loop.context,
      run_id: running.id,
    });
    assert.equal(cancelled.status, "cancelled");
    assert.equal(cancelled.attempts[0]?.status, "cancelled");
    assert.deepEqual(await engine.startRun(loop), cancelled);
    const shutdown = await engine.startRun({ ...loop, key: randomUUID() });
    assert.equal(shutdown.status, "running");
    await engine.close();
    engine = await Engine.connect(binary, worker, root);
    await engine.workspace({
      kind: "open",
      workspace_id: loop.context.workspace_id,
    });
    const restored = (await engine.workspace({
      kind: "run_detail",
      context: loop.context,
      run_id: shutdown.id,
    })) as RunDetail;
    assert.equal(restored.status, "cancelled");
    assert.equal(restored.result?.code, "session_closed");
    assert.deepEqual(
      await engine.workspace({
        kind: "run_detail",
        context: loop.context,
        run_id: cancelled.id,
      }),
      cancelled,
    );
  } finally {
    await engine.close();
    await rm(temporary, { recursive: true, force: true });
  }
});

test("killing the real engine recovers a running node as interrupted without inventing output", {
  skip: !worker,
  timeout: 15000,
}, async () => {
  const temporary = await mkdtemp(join(tmpdir(), "play-run-crash-"));
  const root = join(temporary, "private");
  const setup = await Engine.connect(binary, worker, root);
  const request = await runFixture(setup, "+[]");
  await setup.close();
  const owner = spawn(
    binary,
    ["--ipc", "--worker", worker ?? "", "--data-root", root],
    { stdio: ["pipe", "pipe", "ignore"] },
  );
  const lines = createInterface({ input: owner.stdout });
  async function send(value: object) {
    const reply = once(lines, "line", { signal: AbortSignal.timeout(3000) });
    owner.stdin.write(`${JSON.stringify(value)}\n`);
    return JSON.parse((await reply)[0] as string) as {
      type: string;
      data: RunDetail;
    };
  }
  let recovered: Engine | undefined;
  try {
    assert.equal(
      (
        await send({
          type: "hello",
          id: 1,
          protocol: contract.protocol,
          engine: contract.engine,
          nonce: "0123456789abcdef0123456789abcdef",
        })
      ).type,
      "ready",
    );
    await send({
      type: "workspace",
      id: 2,
      command: { kind: "open", workspace_id: request.context.workspace_id },
    });
    const started = await send({ type: "start_run", id: 3, request });
    assert.equal(started.data.status, "running");
    const exit = once(owner, "exit");
    owner.kill("SIGKILL");
    await exit;
    recovered = await Engine.connect(binary, worker, root);
    await recovered.workspace({
      kind: "open",
      workspace_id: request.context.workspace_id,
    });
    const run = (await recovered.workspace({
      kind: "run_detail",
      context: request.context,
      run_id: started.data.id,
    })) as RunDetail;
    assert.equal(run.status, "interrupted");
    assert.equal(run.attempts[0]?.status, "interrupted");
    assert.equal(run.result?.code, "owner_stopped");
    assert.equal(run.result?.outputs, undefined);
    assert.equal((await recovered.startRun(request)).id, run.id);
  } finally {
    if (owner.exitCode === null && owner.signalCode === null) {
      const exit = once(owner, "exit");
      owner.kill();
      await exit;
    }
    lines.close();
    await recovered?.close();
    await rm(temporary, { recursive: true, force: true });
  }
});

test("editing an input during a real worker Run preserves its plan and rejects stale dispatch", {
  skip: !worker,
  timeout: 15000,
}, async () => {
  const temporary = await mkdtemp(join(tmpdir(), "play-live-input-edit-"));
  const engine = await Engine.connect(
    binary,
    worker,
    join(temporary, "private"),
  );
  try {
    const request = await runFixture(engine, "+[]");
    const opened = (await engine.workspace({
      kind: "open_exploration",
      workspace_id: request.context.workspace_id,
      exploration_id: request.context.exploration_id,
      stage_id: request.context.stage_id,
    })) as ExplorationView;
    const block = opened.stage.blocks.find((block) => block.kind === "asset");
    assert.ok(block);
    const running = await engine.startRun(request);
    assert.equal(running.status, "running");
    const mutation = {
      kind: "revise_text",
      context: request.context,
      key: randomUUID(),
      block_id: block.id,
      asset_id: block.asset.id,
      expected_revision: 1,
      text: "nycun",
    };
    const saved = await engine.workspace(mutation);
    assert.deepEqual(await engine.workspace(mutation), saved);
    const stillRunning = (await engine.workspace({
      kind: "run_detail",
      context: request.context,
      run_id: running.id,
    })) as RunDetail;
    assert.equal(stillRunning.status, "running");
    assert.deepEqual(stillRunning.plan, running.plan);
    const cancelled = await engine.cancelRun({
      context: request.context,
      run_id: running.id,
    });
    assert.equal(cancelled.status, "cancelled");
    await assert.rejects(
      engine.startRun({ ...request, key: randomUUID() }),
      /state_conflict/,
    );
    const completed = await finished(
      engine,
      request.context,
      await engine.startRun({
        ...request,
        key: randomUUID(),
        source_revision: 2,
        operation: "cipher.rot13",
        operation_version: 1,
      }),
    );
    assert.equal(completed.status, "completed");
    assert.deepEqual(completed.result?.outputs, { output: "alpha" });
    assert.deepEqual(
      (
        (await engine.workspace({
          kind: "run_detail",
          context: request.context,
          run_id: running.id,
        })) as RunDetail
      ).plan,
      running.plan,
    );
  } finally {
    await engine.close();
    await rm(temporary, { recursive: true, force: true });
  }
});
