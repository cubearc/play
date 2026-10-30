import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { Cli } from "./cli.ts";
import { Engine } from "./engine.ts";
import type {
  ExplorationView,
  MutationResult,
  WorkspaceView,
} from "./workspace-contract.ts";

const native = new URL("../desktop-native/", import.meta.url);
const binary = (name: string) => fileURLToPath(new URL(name, native));
test("real Cube CLI reads only its fixed Stage assets and saved Pipelines", async () => {
  const root = await mkdtemp(join(tmpdir(), "play-cli-"));
  const engine = await Engine.connect(binary("play-engine"), undefined, root);
  const cli = new Cli(binary("cube"), engine);
  try {
    const w = (await engine.workspace({
      kind: "create",
      key: randomUUID(),
      title: "CLI workspace",
    })) as WorkspaceView;
    const e = (await engine.workspace({
      kind: "create_exploration",
      workspace_id: w.id,
      key: randomUUID(),
      title: "Private exploration",
    })) as MutationResult;
    assert.ok(e.explorationId && e.stageId);
    const context = {
      workspace_id: w.id,
      exploration_id: e.explorationId,
      stage_id: e.stageId,
    };
    await engine.workspace({
      kind: "add_text",
      context,
      key: randomUUID(),
      title: "Cipher",
      text: "Uryyb",
    });
    const v = (await engine.workspace({
      kind: "open_exploration",
      workspace_id: w.id,
      exploration_id: e.explorationId,
      stage_id: e.stageId,
    })) as ExplorationView;
    const block = v.stage.blocks.find((b) => b.kind === "asset");
    assert.ok(block && block.kind === "asset");
    const intent = {
      kind: "asset" as const,
      context,
      asset_id: block.asset.id,
      revision: block.asset.revision,
    };
    assert.deepEqual(await cli.execute(intent), block.asset);
    await assert.rejects(
      cli.execute({
        ...intent,
        context: { ...context, stage_id: randomUUID() },
      }),
      /not_found/,
    );
    await assert.rejects(
      cli.execute({
        ...intent,
        context: { ...context, workspace_id: randomUUID() },
      }),
      /workspace_mismatch/,
    );
    assert.deepEqual(await cli.execute(intent), block.asset);
  } finally {
    await cli.close();
    await engine.close();
    await rm(root, { recursive: true, force: true });
  }
});

test("attached client rejects forged frames and kills a disconnected peer", {
  timeout: 15000,
}, async () => {
  const { writeFile, chmod } = await import("node:fs/promises");
  const root = await mkdtemp(join(tmpdir(), "play-cli-peer-"));
  let calls = 0;
  const engine = {
    cliRead: async () => {
      calls++;
      throw new Error("unexpected_dispatch");
    },
  } as unknown as Engine;
  const context = {
    workspace_id: randomUUID(),
    exploration_id: randomUUID(),
    stage_id: randomUUID(),
  };
  try {
    for (const [name, script] of [
      [
        "forged",
        "process.stdout.write(JSON.stringify({type:'request',protocol:1,nonce:'wrong'})+'\\n');",
      ],
      ["overflow", "process.stdout.write('x'.repeat(262145));"],
      ["closed", "process.exit(0);"],
    ]) {
      const path = join(root, name as string);
      await writeFile(path, `#!${process.execPath}\n${script}\n`);
      await chmod(path, 0o700);
      const cli = new Cli(path, engine);
      await assert.rejects(
        cli.execute({
          kind: "asset",
          context,
          asset_id: randomUUID(),
          revision: 1,
        }),
        /invalid_response|output_limit|transport_closed/,
      );
      await cli.close();
    }
    const hanging = join(root, "hanging");
    await writeFile(
      hanging,
      `#!${process.execPath}\nsetInterval(()=>{},1000);\n`,
    );
    await chmod(hanging, 0o700);
    const cli = new Cli(hanging, engine);
    const intent = {
      kind: "asset" as const,
      context,
      asset_id: randomUUID(),
      revision: 1,
    };
    const pending = assert.rejects(cli.execute(intent));
    await assert.rejects(cli.execute(intent), /busy/);
    await cli.close();
    await pending;
    await assert.rejects(cli.execute(intent), /cli_stopped/);
    assert.equal(calls, 0);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
