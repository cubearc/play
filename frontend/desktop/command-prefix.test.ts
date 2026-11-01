import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { contextCommand } from "./cli.ts";

test("external commands preserve exact bundled paths and arguments without an ambient cube", {
  skip: process.platform === "win32",
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-command-prefix-"));
  try {
    const folder = join(root, "Cube's $HOME `echo no`");
    await mkdir(folder);
    const executable = join(folder, "cube");
    await writeFile(executable, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n", {
      mode: 0o700,
    });
    const credential = join(root, "scope's $(printf leaked) file.json");
    const output = execFileSync(
      "/bin/sh",
      ["-c", contextCommand(executable, credential)],
      {
        encoding: "utf8",
        env: { PATH: "/usr/bin:/bin" },
        timeout: 3000,
      },
    );
    assert.deepEqual(output.trimEnd().split("\n"), [
      "play",
      "--context",
      credential,
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
