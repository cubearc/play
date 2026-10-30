import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { contract } from "./contract.ts";
import { Engine } from "./engine.ts";
import { singleGraph } from "./graph-fixture.ts";
import { verifyNative } from "./integrity.ts";

const directory = new URL("../desktop-native/", import.meta.url);
const binary = fileURLToPath(
  new URL(
    process.platform === "win32" ? "play-engine.exe" : "play-engine",
    directory,
  ),
);
const worker = fileURLToPath(
  new URL("Worker.app/Contents/MacOS/play-engine", directory),
);
const probe = fileURLToPath(
  new URL("Probe.app/Contents/MacOS/play-engine", directory),
);
const mac = process.platform === "darwin";

test("the production owner guard stops held native work on EOF or extra input", {
  skip: !mac,
  timeout: 10000,
}, async () => {
  const engine = await Engine.connect(binary);
  const catalog = await engine.catalog();
  await engine.close();
  const rot13 = catalog.find(
    (operation) => operation.operation === "cipher.rot13",
  );
  assert.ok(rot13);
  const heldWorker = fileURLToPath(
    new URL("SupervisionProbe.app/Contents/MacOS/play-engine", directory),
  );
  for (const disconnect of [true, false]) {
    const child = spawn(heldWorker, ["--worker"], { env: {}, stdio: "pipe" });
    const closed = once(child, "close");
    const lines = createInterface({ input: child.stdout });
    const output: string[] = [];
    lines.on("line", (line) => output.push(line));
    try {
      const started = once(lines, "line", {
        signal: AbortSignal.timeout(1500),
      });
      child.stdin.write(
        `${JSON.stringify({
          kind: "pipeline",
          invocation: {
            definition: singleGraph(rot13),
            inputs: { source: { kind: "text", text: "Hello" } },
          },
        })}\n`,
      );
      assert.equal(
        JSON.parse((await started)[0] as string).type,
        "node_started",
      );
      const stopped = once(child, "close", {
        signal: AbortSignal.timeout(1500),
      });
      if (disconnect) child.stdin.end();
      else child.stdin.write("x");
      assert.deepEqual(await stopped, [1, null]);
      assert.equal(
        output.length,
        1,
        "owner loss cannot fabricate a completed result",
      );
      assert.throws(() => process.kill(child.pid as number, 0), {
        code: "ESRCH",
      });
    } finally {
      if (child.exitCode === null && child.signalCode === null)
        child.kill("SIGKILL");
      await closed;
      lines.close();
    }
  }
});

test("the signed worker refuses malformed or multiple task frames", {
  skip: !mac,
}, () => {
  for (const input of [
    "{}",
    "x".repeat(contract.frame_bytes),
    '{"kind":"probe","kind":"pipeline"}\n',
    `${JSON.stringify({ kind: "probe", invocation: { operation: "cipher.rot13", operation_version: 1, input: "Hello", parameters: {} } })}\n{}\n`,
    Buffer.from([0xff, 10]),
  ]) {
    const result = spawnSync(worker, ["--worker"], {
      env: {},
      input,
      encoding: "utf8",
      timeout: 3000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 1);
    assert.equal(result.stdout, "");
  }
});

test("a signed worker completes one task while its owner keeps the lifetime pipe open", {
  skip: !mac,
  timeout: 5000,
}, async () => {
  const child = spawn(worker, ["--worker"], { env: {}, stdio: "pipe" });
  const closed = once(child, "close");
  const lines = createInterface({ input: child.stdout });
  try {
    const reply = once(lines, "line", { signal: AbortSignal.timeout(1500) });
    child.stdin.write(
      `${JSON.stringify({
        kind: "probe",
        invocation: {
          operation: "cipher.rot13",
          operation_version: 1,
          input: "Hello",
          parameters: {},
        },
      })}\n`,
    );
    assert.deepEqual(JSON.parse((await reply)[0] as string), {
      status: "completed",
      output: { text: "Uryyb" },
    });
    assert.deepEqual(await closed, [0, null]);
    assert.equal(child.stdin.writableEnded, false);
  } finally {
    if (child.exitCode === null && child.signalCode === null)
      child.kill("SIGKILL");
    await closed;
    lines.close();
  }
});

test("negotiates the real Rust binary and obtains Core names and versions", async () => {
  await verifyNative(directory);
  const engine = await Engine.connect(binary);
  try {
    const catalog = await engine.catalog();
    const rot13 = catalog.find((entry) => entry.operation === "cipher.rot13");
    assert.ok(rot13);
    assert.equal(rot13.name, "ROT13");
    assert.equal(
      catalog.find((entry) => entry.operation === "esolang.brainfuck.execute")
        ?.operation_version,
      2,
    );
    await assert.rejects(
      engine.run({
        operation: "cipher.rot13",
        operation_version: 1,
        input: "Hello",
      }),
    );
  } finally {
    await engine.close();
  }
});

test("rejects a mismatched binary and malformed first frames", () => {
  for (const input of [
    '{"type":"hello","id":1,"protocol":999,"engine":"0.2.0","nonce":"0123456789abcdef0123456789abcdef"}\n',
    '{"type":"hello","id":1,"id":2}\n',
    '{"type":"cancel","id":1,"run_id":2,"path":"private"}\n',
    "x".repeat(65537),
  ]) {
    const result = spawnSync(binary, ["--ipc"], {
      input,
      encoding: "utf8",
      timeout: 4000,
    });
    assert.notEqual(result.status, 0);
    assert.equal(result.error, undefined);
  }
});

test("invokes Core through a signed sandboxed worker", {
  skip: !mac,
}, async () => {
  const engine = await Engine.connect(binary, worker);
  try {
    assert.deepEqual(
      await engine.run({
        operation: "cipher.rot13",
        operation_version: 1,
        input: "Hello, ação!",
      }),
      {
        status: "completed",
        type: "finished",
        id: 3,
        output: { text: "Uryyb, nçãb!" },
      },
    );
    await assert.rejects(
      engine.run({
        operation: "cipher.rot13",
        operation_version: 99,
        input: "Hello",
      }),
    );
    const result = await engine.run({
      operation: "esolang.brainfuck.execute",
      operation_version: 2,
      input: "++++++++[>++++++++<-]>+.",
    });
    assert.equal(result.status, "completed");
    if (result.status === "completed")
      assert.deepEqual(result.output, { bytes: [65] });
  } finally {
    await engine.close();
  }
});

test("cancels and reaps a real worker before accepting another invocation", {
  skip: !mac,
}, async () => {
  const engine = await Engine.connect(binary, worker);
  try {
    const run = engine.run({
      operation: "esolang.brainfuck.execute",
      operation_version: 2,
      input: "+[]",
    });
    await assert.rejects(
      engine.run({
        operation: "cipher.rot13",
        operation_version: 1,
        input: "busy",
      }),
      /busy/,
    );
    await engine.cancel();
    assert.equal((await run).status, "cancelled");
    assert.equal(
      (
        await engine.run({
          operation: "cipher.rot13",
          operation_version: 1,
          input: "Next",
        })
      ).status,
      "completed",
    );
  } finally {
    await engine.close();
  }
});

test("App Sandbox denies private files, source mutation, networking and children", {
  skip: !mac,
}, async () => {
  const scratch = await mkdtemp(join(tmpdir(), "play-native-"));
  const secret = join(scratch, "private-source");
  const server = createServer();
  try {
    await writeFile(secret, "private-source-unchanged");
    await new Promise<void>((resolve) =>
      server.listen(0, "127.0.0.1", resolve),
    );
    const address = server.address();
    assert.ok(address && typeof address !== "string");
    const result = spawnSync(probe, [secret, String(address.port)], {
      encoding: "utf8",
      timeout: 4000,
    });
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), {
      read_denied: true,
      write_denied: true,
      network_denied: true,
      spawn_denied: true,
    });
    assert.equal(await readFile(secret, "utf8"), "private-source-unchanged");
    assert.equal(
      spawnSync("/usr/bin/true").status,
      0,
      "the worker's process limit must not affect its parent",
    );
  } finally {
    server.close();
    await rm(scratch, { recursive: true, force: true });
  }
});

test("worker ceilings reject excess new VM and stop CPU loops", {
  skip: !mac,
}, () => {
  const memory = spawnSync(probe, ["memory"], {
    encoding: "utf8",
    timeout: 4000,
  });
  assert.equal(memory.status, 0, memory.stderr);
  assert.equal(
    memory.stdout.trim(),
    "true",
    "the requested allocation must exceed the additional VM limit",
  );
  const cpu = spawnSync(probe, ["cpu"], { timeout: 4000 });
  assert.equal(
    cpu.error,
    undefined,
    "the kernel CPU limit must end the worker before the test timeout",
  );
  assert.ok(
    cpu.signal === "SIGKILL" || cpu.signal === "SIGXCPU",
    String(cpu.signal),
  );
});

test("cancellation acknowledgement follows actual worker reaping", {
  skip: !mac,
  timeout: 5000,
}, async () => {
  const child = spawn(binary, ["--ipc", "--worker", worker], {
    env: {},
    stdio: "pipe",
  });
  const lines = createInterface({ input: child.stdout });
  const iterator = lines[Symbol.asyncIterator]();
  const next = async () => {
    const line = await iterator.next();
    assert.equal(line.done, false);
    return JSON.parse(line.value as string) as Record<string, unknown>;
  };
  try {
    const probeEngine = await Engine.connect(binary);
    const brainfuck = (await probeEngine.catalog()).find(
      (o) => o.operation === "esolang.brainfuck.execute",
    );
    assert.ok(brainfuck);
    await probeEngine.close();
    const commands = [
      {
        type: "hello",
        id: 1,
        protocol: contract.protocol,
        engine: "0.2.0",
        nonce: "0123456789abcdef0123456789abcdef",
      },
      {
        type: "invoke",
        id: 2,
        invocation: {
          operation: "esolang.brainfuck.execute",
          operation_version: 2,
          input: "+[]",
          parameters: brainfuck.parameters,
        },
      },
      { type: "cancel", id: 3, run_id: 2 },
    ];
    child.stdin.write(
      `${commands.map((value) => JSON.stringify(value)).join("\n")}\n`,
    );
    assert.equal((await next()).type, "ready");
    const started = await next();
    assert.equal(started.type, "started");
    assert.equal((await next()).status, "cancelled");
    assert.equal((await next()).type, "cancelled");
    const pid = started.worker_pid;
    assert.ok(typeof pid === "number" && pid > 0);
    assert.throws(() => process.kill(pid, 0), { code: "ESRCH" });
    assert.ok(child.pid);
    assert.doesNotThrow(
      () => process.kill(child.pid as number, 0),
      "the engine must survive cancellation",
    );
  } finally {
    child.stdin.end();
    await once(child, "exit");
    lines.close();
  }
});
