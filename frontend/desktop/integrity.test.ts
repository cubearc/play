import assert from "node:assert/strict";
import {
  appendFile,
  cp,
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";
import { verifyNative } from "./integrity.ts";
import { nativeFiles } from "./native-files.ts";

test("Linux startup verifies every prepared binary and refuses missing worker integrity", {
  skip: process.platform !== "linux",
}, async () => {
  const source = new URL("../desktop-native/", import.meta.url);
  await verifyNative(source);
  const root = await mkdtemp(join(tmpdir(), "play-integrity-"));
  const staged = pathToFileURL(`${root}/`);
  try {
    const files = nativeFiles();
    assert.ok(files.worker);
    assert.ok(files.terminal);
    const manifest = await readFile(new URL("integrity.json", source), "utf8");
    await writeFile(new URL("integrity.json", staged), manifest);
    for (const name of Object.values(files))
      await cp(new URL(name, source), new URL(name, staged));
    await verifyNative(staged);
    for (const name of Object.values(files)) {
      await appendFile(new URL(name, staged), "changed");
      await assert.rejects(verifyNative(staged), /binary_mismatch/);
      await cp(new URL(name, source), new URL(name, staged));
    }
    const missingTerminal = JSON.parse(manifest);
    delete missingTerminal.terminal;
    await writeFile(
      new URL("integrity.json", staged),
      JSON.stringify(missingTerminal),
    );
    await assert.rejects(verifyNative(staged), /binary_mismatch/);
    await writeFile(new URL("integrity.json", staged), manifest);
    await rm(new URL(files.terminal, staged));
    await assert.rejects(verifyNative(staged), /ENOENT/);
    await cp(new URL(files.terminal, source), new URL(files.terminal, staged));
    const incomplete = JSON.parse(manifest);
    delete incomplete.worker;
    await writeFile(
      new URL("integrity.json", staged),
      JSON.stringify(incomplete),
    );
    await assert.rejects(verifyNative(staged), /binary_mismatch/);
    await writeFile(new URL("integrity.json", staged), manifest);
    await rm(new URL(files.worker, staged));
    await assert.rejects(verifyNative(staged), /ENOENT/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
