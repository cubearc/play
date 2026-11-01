import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { contract } from "./contract.ts";
import { Engine } from "./engine.ts";

test("negotiated discovery is loaded once and caller edits cannot alter its snapshot", {
  skip: process.platform === "win32",
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-catalog-peer-"));
  const binary = join(root, "peer.mjs");
  await writeFile(
    binary,
    `#!${process.execPath}
import { createInterface } from 'node:readline';
let reads = 0;
const lines = createInterface({ input: process.stdin });
lines.on('line', raw => {
  const message = JSON.parse(raw);
  let reply;
  if (message.type === 'hello') reply = { type: 'ready', protocol: ${contract.protocol}, engine: ${JSON.stringify(contract.engine)}, nonce: message.nonce };
  else if (message.type === 'catalog') {
    reads++;
    reply = { type: 'catalog', offset: 0, total: 1, nextOffset: null, operations: [{ operation: 'synthetic.snapshot', operation_version: 1, name: 'Snapshot', parameters: { limit: 3 }, availability: { state: 'available' } }] };
  } else if (message.type === 'workspace') reply = { type: 'workspace', data: { reads } };
  process.stdout.write(JSON.stringify({ ...reply, id: message.id }) + '\\n');
});
lines.on('close', () => process.exit(0));
`,
    { mode: 0o700 },
  );
  let engine: Engine | undefined;
  try {
    engine = await Engine.connect(binary);
    const first = await engine.catalog();
    const [second, third] = await Promise.all([
      engine.catalog(),
      engine.catalog(),
    ]);
    assert.deepEqual(second, first);
    assert.deepEqual(third, first);
    assert.deepEqual(await engine.workspace({ kind: "catalog_reads" }), {
      reads: 1,
    });
    assert.ok(first[0]);
    first[0].name = "Caller draft";
    first[0].parameters.limit = 9;
    first.pop();
    const retained = await engine.catalog();
    assert.equal(retained[0]?.name, "Snapshot");
    assert.equal(retained[0]?.parameters.limit, 3);
    await engine.close();
    await assert.rejects(engine.catalog(), /engine_stopped/);
  } finally {
    await engine?.close();
    await rm(root, { recursive: true, force: true });
  }
});
