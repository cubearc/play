import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { readImport } from "./imports.ts";

test("native import reads only bounded regular source bytes without modifying them", async () => {
  const directory = await mkdtemp(join(tmpdir(), "play-import-"));
  try {
    const path = join(directory, "source.bin");
    const bytes = Buffer.from([0, 255, 1, 128]);
    await writeFile(path, bytes);
    assert.deepEqual(await readImport(path, bytes.length, "file_limit"), bytes);
    await assert.rejects(
      readImport(path, bytes.length - 1, "file_limit"),
      /file_limit/,
    );
    assert.deepEqual(await readFile(path), bytes);
    await writeFile(path, Buffer.alloc(0));
    assert.deepEqual(await readImport(path, 0, "file_limit"), Buffer.alloc(0));
    await assert.rejects(
      readImport(directory, 1024, "file_limit"),
      /invalid_file/,
    );
    if (process.platform !== "win32") {
      const fifo = join(directory, "named-pipe");
      execFileSync("mkfifo", [fifo]);
      await assert.rejects(
        readImport(fifo, 1024, "file_limit"),
        /invalid_file/,
      );
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
