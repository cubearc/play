import assert from "node:assert/strict";
import { createHash, randomUUID } from "node:crypto";
import { mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import type { ExportIntent } from "./export-contract.ts";
import { NativeExports } from "./exports.ts";

test("export save dialogs hold no grant; cancellation, changes and context loss publish nothing", async () => {
  const root = await mkdtemp(join(tmpdir(), "export-service-"));
  const bytes = Buffer.from("exact clue");
  const grants = new Set<string>();
  let changed = false;
  let current = true;
  const intent: ExportIntent = {
    context: {
      workspace_id: randomUUID(),
      exploration_id: randomUUID(),
      stage_id: randomUUID(),
    },
    source: { kind: "asset", asset_id: randomUUID(), revision: 1 },
    format: "data",
  };
  const peer = {
    async workspace(value: object): Promise<unknown> {
      const c = value as { kind: string; export_id: string; offset: number };
      if (c.kind === "prepare_export") {
        const exportId = randomUUID();
        grants.add(exportId);
        return {
          exportId,
          fileName: "clue.txt",
          mime: "text/plain",
          size: bytes.length,
          sha256: changed
            ? "0".repeat(64)
            : createHash("sha256").update(bytes).digest("hex"),
        };
      }
      if (c.kind === "release_export") {
        grants.delete(c.export_id);
        return { released: true };
      }
      assert.equal(c.kind, "read_export");
      assert.ok(grants.has(c.export_id));
      return { offset: c.offset, base64: bytes.toString("base64"), done: true };
    },
  };
  const exports = new NativeExports(peer, []);
  try {
    assert.equal(
      await exports.save(
        intent,
        async () => {
          assert.equal(grants.size, 0);
          return null;
        },
        () => current,
      ),
      null,
    );
    await assert.rejects(
      exports.save(
        intent,
        async () => {
          current = false;
          return join(root, "stale.txt");
        },
        () => current,
      ),
      /stale_context/,
    );
    current = true;
    await assert.rejects(
      exports.save(
        intent,
        async () => {
          changed = true;
          return join(root, "changed.txt");
        },
        () => current,
      ),
      /export_source_changed/,
    );
    changed = false;
    const receipt = await exports.save(
      intent,
      async () => {
        assert.equal(grants.size, 0);
        return join(root, "chosen.txt");
      },
      () => current,
    );
    assert.equal(receipt?.fileName, "chosen.txt");
    assert.deepEqual(await readFile(join(root, "chosen.txt")), bytes);
    assert.equal(grants.size, 0);
    assert.deepEqual(await readdir(root), ["chosen.txt"]);
    await exports.settle();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
