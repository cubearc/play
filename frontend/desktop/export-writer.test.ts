import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { writeExport } from "./export-writer.ts";

test("native exports publish exact bytes atomically and never replace an existing source", async () => {
  const root = await mkdtemp(join(tmpdir(), "play-export-"));
  const bytes = Buffer.from([0, 255, 128, 1]);
  const meta = {
    size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
  const destination = join(root, "result.bin");
  const read = async (offset: number) => ({
    offset,
    base64: bytes.subarray(offset).toString("base64"),
    done: true,
  });
  try {
    await writeExport(destination, meta, read, () => true);
    assert.deepEqual(await readFile(destination), bytes);
    await assert.rejects(
      writeExport(destination, meta, read, () => true),
      /export_target_exists/,
    );
    assert.deepEqual(await readFile(destination), bytes);
    const source = join(root, "source.bin"),
      alias = join(root, "alias.bin");
    await writeFile(source, "original");
    if (process.platform !== "win32") {
      await symlink(source, alias);
      await assert.rejects(
        writeExport(alias, meta, read, () => true),
        /export_target_exists/,
      );
      assert.equal(await readFile(source, "utf8"), "original");
    }
    await assert.rejects(
      writeExport(join(root, "stale.bin"), meta, read, () => false),
      /stale_context/,
    );
    await assert.rejects(
      writeExport(
        join(root, "corrupt.bin"),
        { ...meta, sha256: "0".repeat(64) },
        read,
        () => true,
      ),
      /export_integrity_failed/,
    );
    const names = await readdir(root);
    assert.equal(
      names.some(
        (name) =>
          name.includes("partial") ||
          name === "stale.bin" ||
          name === "corrupt.bin",
      ),
      false,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("export destination aliases, malformed chunks and cancelled writes leave no partial output", async () => {
  const root = await mkdtemp(join(tmpdir(), "export-writer-"));
  const bytes = Buffer.from("clue");
  const meta = {
    size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
  const read = async (offset: number) => ({
    offset,
    base64: bytes.toString("base64"),
    done: true,
  });
  try {
    const profile = join(root, "private");
    await mkdir(profile);
    await mkdir(join(profile, "sub"));
    await assert.rejects(
      writeExport(join(profile, "sub", "bad.txt"), meta, read, () => true, [
        profile,
      ]),
      /export_target_protected/,
    );
    if (process.platform !== "win32") {
      const alias = join(root, "alias");
      await symlink(profile, alias, "dir");
      await assert.rejects(
        writeExport(join(alias, "bad.txt"), meta, read, () => true, [profile]),
        /export_target_protected/,
      );
    }
    for (const bad of [
      { offset: 1, base64: bytes.toString("base64"), done: true },
      { offset: 0, base64: "???", done: true },
      { offset: 0, base64: bytes.toString("base64"), done: false },
      { offset: 0, base64: "", done: true },
    ]) {
      await assert.rejects(
        writeExport(
          join(root, "invalid.txt"),
          meta,
          async () => bad,
          () => true,
        ),
        /invalid_response/,
      );
    }
    let active = true;
    await assert.rejects(
      writeExport(
        join(root, "cancelled.txt"),
        meta,
        async (offset) => {
          active = false;
          return read(offset);
        },
        () => active,
      ),
      /stale_context/,
    );
    const empty = join(root, "empty.txt");
    await writeExport(
      empty,
      { size: 0, sha256: createHash("sha256").digest("hex") },
      async () => {
        throw new Error("empty output must not read");
      },
      () => true,
    );
    assert.equal((await readFile(empty)).length, 0);
    assert.equal(
      (await readdir(root)).some(
        (name) =>
          name.endsWith(".partial") ||
          name === "invalid.txt" ||
          name === "cancelled.txt",
      ),
      false,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
