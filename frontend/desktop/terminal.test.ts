import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { Engine } from "./engine.ts";
import { type TerminalIntent, terminalIntent } from "./terminal-contract.ts";
import type {
  ExplorationView,
  MutationResult,
  RunDetail,
  WorkspaceView,
} from "./workspace-contract.ts";

const binary = (name: string) =>
  fileURLToPath(new URL(`../desktop-native/${name}`, import.meta.url));
test("real contextual PTY resizes, resolves Assets and executes the same pinned Pipeline", {
  skip: process.platform !== "darwin",
  timeout: 20000,
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-pty-"));
  const engine = await Engine.connect(
    binary("play-engine"),
    binary("Worker.app/Contents/MacOS/play-engine"),
    root,
    binary("Terminal.app/Contents/MacOS/play-engine"),
  );
  let transcript = "";
  let runId: string | null = null;
  const confirmedRun = (): string => {
    assert.ok(runId);
    return runId;
  };
  const call = async (command: TerminalIntent) => {
    const view = await engine.terminal(command);
    transcript += Buffer.from(view.output, "base64").toString("utf8");
    if (view.run_id) runId = view.run_id;
    return view;
  };
  try {
    const { context, trail, selection } = await fixture(engine);
    const desktop = await engine.startRun(selection);
    const done = async (id: string) => {
      let result: RunDetail;
      const deadline = Date.now() + 3000;
      do {
        result = (await engine.workspace({
          kind: "run_detail",
          context,
          run_id: id,
        })) as RunDetail;
        if (!["queued", "running"].includes(result.status)) return result;
        await delay(15);
      } while (Date.now() < deadline);
      assert.fail("Run did not reach a terminal state");
    };
    const expected = await done(desktop.id);
    assert.equal(expected.status, "completed");
    assert.ok(expected.result?.outputs);
    assert.match(JSON.stringify(expected.result.outputs), /Hello/);
    const opened = await call({
      kind: "open",
      context,
      selection,
      columns: 90,
      rows: 20,
    });
    const session_id = opened.id;
    const until = async (test: () => boolean) => {
      const deadline = Date.now() + 3000;
      while (!test() && Date.now() < deadline) {
        await delay(15);
        const status = await call({ kind: "read", context, session_id });
        assert.equal(status.state, "open", `${status.code}: ${transcript}`);
      }
      assert.ok(test(), transcript);
    };
    await until(() => transcript.includes("cube play> "));
    await call({
      kind: "write",
      context,
      session_id,
      input: `asset show ${trail.source.id} --revision 1\r`,
    });
    await until(() => transcript.includes('"text": "Uryyb"'));
    await call({ kind: "write", context, session_id, input: "size\r" });
    await until(() => transcript.includes("90 columns, 20 rows"));
    await call({ kind: "resize", context, session_id, columns: 112, rows: 28 });
    await call({ kind: "write", context, session_id, input: "size\r" });
    await until(() => transcript.includes("112 columns, 28 rows"));
    await call({ kind: "write", context, session_id, input: "selected\r" });
    await until(() => !!runId);
    const actual = await done(confirmedRun());
    assert.equal(actual.caller, "cli");
    assert.equal(actual.status, "completed");
    assert.deepEqual(actual.result?.outputs, expected.result?.outputs);
    assert.deepEqual(
      actual.plan.pipeline.definition,
      expected.plan.pipeline.definition,
    );
    assert.notEqual(actual.id, expected.id);
    runId = null;
    await call({ kind: "write", context, session_id, input: "selected\r" });
    await until(() => !!runId);
    assert.notEqual(runId, actual.id);
    await done(confirmedRun());
    await call({ kind: "write", context, session_id, input: "$(whoami)\r" });
    await until(() => transcript.includes("Shell expressions are unavailable"));
    await assert.rejects(
      call({
        kind: "read",
        context: { ...context, stage_id: randomUUID() },
        session_id,
      }),
      /context_mismatch/,
    );
    assert.equal(
      (await call({ kind: "close", context, session_id })).state,
      "closed",
    );
    await assert.rejects(
      call({ kind: "write", context, session_id, input: "help\r" }),
      /terminal_closed/,
    );
  } finally {
    await engine.close();
    await rm(root, { recursive: true, force: true });
  }
});

test("terminal boundary rejects widened command shapes and unbounded input", () => {
  const context = {
    workspace_id: randomUUID(),
    exploration_id: randomUUID(),
    stage_id: randomUUID(),
  };
  const valid = {
    kind: "open",
    context,
    selection: null,
    columns: 80,
    rows: 24,
  };
  assert.deepEqual(terminalIntent(valid), valid);
  for (const value of [
    { ...valid, path: "/tmp" },
    { ...valid, columns: 0 },
    { ...valid, rows: 101 },
    { ...valid, selection: {} },
    {
      kind: "write",
      context,
      session_id: randomUUID(),
      input: "x".repeat(1025),
    },
    { kind: "read", context, session_id: "other" },
  ])
    assert.throws(() => terminalIntent(value));
});

async function fixture(engine: Engine) {
  const w = (await engine.workspace({
    kind: "create",
    key: randomUUID(),
    title: "PTY workspace",
  })) as WorkspaceView;
  const e = (await engine.workspace({
    kind: "create_exploration",
    workspace_id: w.id,
    key: randomUUID(),
    title: "PTY exploration",
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
    title: "Caption",
    text: "Uryyb",
  });
  const view = (await engine.workspace({
    kind: "open_exploration",
    ...context,
  })) as ExplorationView;
  const trail = view.trails[0];
  assert.ok(trail?.pipeline);
  const selection = {
    context,
    key: randomUUID(),
    trail_id: trail.id,
    trail_revision: trail.revision,
    pipeline_id: trail.pipeline.id,
    pipeline_revision: trail.pipeline.revision,
    target_node_id: trail.pipeline.outputNodeId,
    source_asset_id: trail.source.id,
    source_revision: trail.source.revision,
    operation: trail.pipeline.outputOperation,
    operation_version: trail.pipeline.operationVersion,
    retain_output: true,
  };
  return { context, trail, selection };
}

test("terminal cancellation, lost renderer lease and killed owner reap only owned processes", {
  skip: process.platform !== "darwin",
  timeout: 25000,
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-pty-lifetime-"));
  const engine = await Engine.connect(
    binary("play-engine"),
    binary("SupervisionProbe.app/Contents/MacOS/play-engine"),
    root,
    binary("Terminal.app/Contents/MacOS/play-engine"),
  );
  const unrelated = spawn(
    process.execPath,
    ["-e", "setInterval(() => {}, 1000)"],
    { stdio: "ignore" },
  );
  const unrelatedExit = once(unrelated, "exit");
  const processes = () => {
    const result = spawnSync("/bin/ps", ["-axo", "pid=,ppid=,command="], {
      encoding: "utf8",
    });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout
      .trim()
      .split("\n")
      .map((line) => {
        const [pid, parent, ...args] = line.trim().split(/\s+/);
        return {
          pid: Number(pid),
          parent: Number(parent),
          command: args.join(" "),
        };
      });
  };
  const gone = async (pid: number) => {
    const deadline = Date.now() + 2000;
    while (Date.now() < deadline && processes().some((p) => p.pid === pid))
      await delay(20);
    assert.throws(() => process.kill(pid, 0), { code: "ESRCH" });
  };
  try {
    const { context, selection } = await fixture(engine);
    const open = () =>
      engine.terminal({
        kind: "open",
        context,
        selection,
        columns: 80,
        rows: 24,
      });
    let session = await open();
    const read = () =>
      engine.terminal({ kind: "read", context, session_id: session.id });
    const write = (input: string) =>
      engine.terminal({
        kind: "write",
        context,
        session_id: session.id,
        input,
      });
    await write("selected\r");
    let runId: string | null = null;
    for (let n = 0; n < 100 && !runId; n++) {
      await delay(10);
      runId = (await read()).run_id;
    }
    assert.ok(runId);
    const detail = () =>
      engine.workspace({
        kind: "run_detail",
        context,
        run_id: runId,
      }) as Promise<RunDetail>;
    for (let n = 0; n < 100 && (await detail()).status === "queued"; n++)
      await delay(10);
    assert.equal((await detail()).status, "running");
    await write(`run cancel ${runId}\r`);
    for (let n = 0; n < 100 && (await detail()).status === "running"; n++) {
      await delay(10);
      await read();
    }
    assert.equal((await detail()).status, "cancelled");
    assert.equal((await detail()).result?.outputs, undefined);
    const owner = processes().find((p) =>
      p.command.includes(`--data-root ${root}`),
    );
    assert.ok(owner);
    const child = () => {
      const children = processes().filter((p) => p.parent === owner.pid);
      assert.equal(children.length, 1);
      const pid = children[0]?.pid;
      assert.ok(pid);
      return pid;
    };
    const leased = child();
    // Stop polling to model a destroyed renderer; engine enforces its own lease.
    await delay(10500);
    const timedOut = await read();
    assert.equal(timedOut.state, "closed");
    assert.equal(timedOut.code, "terminal_timeout");
    await gone(leased);
    session = await open();
    await delay(60);
    const killed = child();
    process.kill(owner.pid, "SIGKILL");
    await gone(killed);
    assert.doesNotThrow(() => process.kill(unrelated.pid as number, 0));
  } finally {
    await engine.close();
    unrelated.kill("SIGKILL");
    await unrelatedExit;
    await rm(root, { recursive: true, force: true });
  }
});
