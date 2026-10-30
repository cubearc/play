import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const probe = fileURLToPath(
  new URL("../desktop-native/native-boundary-probe", import.meta.url),
);
const linux = process.platform === "linux";

test("Linux workers deny filesystem, network and process access", {
  skip: !linux,
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-linux-boundary-"));
  const source = join(root, "private.txt");
  try {
    await writeFile(source, "unchanged");
    const result = spawnSync(probe, [source, "9"], {
      env: {},
      encoding: "utf8",
      timeout: 3000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), {
      read_denied: true,
      write_denied: true,
      network_denied: true,
      spawn_denied: true,
    });
    assert.equal(await readFile(source, "utf8"), "unchanged");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("Linux workers enforce CPU and memory ceilings", { skip: !linux }, () => {
  const memory = spawnSync(probe, ["memory"], {
    env: {},
    encoding: "utf8",
    timeout: 3000,
  });
  assert.equal(memory.error, undefined);
  assert.equal(memory.status, 0, memory.stderr);
  assert.equal(memory.stdout.trim(), "true");
  const cpu = spawnSync(probe, ["cpu"], {
    env: {},
    encoding: "utf8",
    timeout: 3000,
  });
  assert.equal(cpu.error, undefined);
  assert.equal(cpu.signal, "SIGKILL");
});

test("Linux policy is inherited by threads and cannot affect unrelated processes", {
  skip: !linux,
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-linux-thread-"));
  const source = join(root, "private.txt");
  const unrelated = spawn(
    process.execPath,
    ["-e", "setInterval(() => {}, 1000)"],
    { stdio: "ignore" },
  );
  const stopped = once(unrelated, "exit");
  try {
    await writeFile(source, "unchanged");
    const result = spawnSync(
      fileURLToPath(
        new URL("../desktop-native/linux-boundary-probe", import.meta.url),
      ),
      [source, String(unrelated.pid)],
      { env: {}, encoding: "utf8", timeout: 3000 },
    );
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "isolated");
    assert.equal(await readFile(source, "utf8"), "unchanged");
    assert.equal(unrelated.exitCode, null);
    assert.doesNotThrow(() => process.kill(unrelated.pid as number, 0));
  } finally {
    unrelated.kill("SIGKILL");
    await stopped;
    await rm(root, { recursive: true, force: true });
  }
});

test("Linux workers refuse unavailable mandatory kernel enforcement", {
  skip: !linux,
}, () => {
  for (const mode of ["unavailable", "seccomp-unavailable"]) {
    const result = spawnSync(
      fileURLToPath(
        new URL("../desktop-native/linux-boundary-probe", import.meta.url),
      ),
      [mode],
      { env: {}, encoding: "utf8", timeout: 3000 },
    );
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "refused");
  }
});
