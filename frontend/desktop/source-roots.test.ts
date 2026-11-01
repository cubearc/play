import assert from "node:assert/strict";
import {
  chmod,
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { SourceRoots } from "./source-roots.ts";

const id = "11111111-1111-4111-8111-111111111111";
test("opening a source folder reuses its managed identity and never adopts an adjacent store", async () => {
  const temp = await mkdtemp(join(tmpdir(), "play-source-"));
  const profile = join(temp, "private");
  const source = join(temp, "Puzzle");
  await mkdir(profile, { mode: 0o700 });
  await mkdir(source);
  await writeFile(join(source, "workspace.sqlite3"), "external archive");
  let created = 0,
    opened = 0;
  let key = "";
  try {
    const roots = new SourceRoots(profile);
    const host = {
      create: async (k: string, title: string) => {
        created++;
        key = k;
        assert.equal(title, "Puzzle");
        return id;
      },
      open: async (value: string) => {
        assert.equal(value, id);
        opened++;
      },
    };
    assert.equal(await roots.open(source, host), id);
    assert.equal(created, 1);
    assert.equal(await new SourceRoots(profile).open(source, host), id);
    assert.equal(created, 1);
    assert.equal(opened, 1);
    assert.match(key, /^[0-9a-f-]{36}$/);
    assert.equal(
      await readFile(join(source, "workspace.sqlite3"), "utf8"),
      "external archive",
    );
    assert.equal(await roots.source(id), await realpath(source));
    if (process.platform !== "win32") {
      await chmod(join(profile, "sources.json"), 0o666);
      await assert.rejects(roots.source(id), /unsafe_source_registry/);
    }
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
});
test("an interrupted folder creation retries its original intent without manufacturing an identity", async () => {
  const temp = await mkdtemp(join(tmpdir(), "play-source-retry-"));
  const profile = join(temp, "private");
  const source = join(temp, "Puzzle");
  await mkdir(profile, { mode: 0o700 });
  await mkdir(source);
  try {
    const roots = new SourceRoots(profile);
    let first = "";
    await assert.rejects(
      roots.open(source, {
        create: async (key) => {
          first = key;
          throw new Error("transport_closed");
        },
        open: async () => {
          throw new Error("unexpected_open");
        },
      }),
      /transport_closed/,
    );
    assert.equal(
      await roots.open(source, {
        create: async (key) => {
          assert.equal(key, first);
          return id;
        },
        open: async () => {
          throw new Error("unexpected_open");
        },
      }),
      id,
    );
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
});
