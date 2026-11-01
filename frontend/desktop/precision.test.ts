import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { contract } from "./contract.ts";
import { Engine } from "./engine.ts";

// Exercise the real client API with an exact JSON peer, independently of Core algorithms.
test("engine transport preserves exact 64-bit JSON integers in both directions", {
  skip: process.platform === "win32",
}, async () => {
  const root = await mkdtemp(join(tmpdir(), "play-json-peer-"));
  const binary = join(root, "peer.mjs");
  const operation = {
    operation: "synthetic.record",
    operation_version: 1,
    name: "Record",
    description: "",
    input_type: "json",
    parameters: {},
    availability: { state: "available" },
    execution: {
      effects: "pure",
      inputs: { input: "json" },
      outputs: { output: "json" },
    },
    contract: {
      inputs: [{ name: "input", semantic_type: "json", schema: {} }],
      outputs: [{ name: "output", semantic_type: "json", schema: {} }],
      parameters: {},
    },
  };
  await writeFile(
    binary,
    `#!${process.execPath}
import {createInterface} from 'node:readline';
const lines = createInterface({input:process.stdin});
lines.on('line', raw => {
  const message = JSON.parse(raw);
  if (message.type === 'hello') process.stdout.write(JSON.stringify({type:'ready',id:message.id,nonce:message.nonce,protocol:${contract.protocol},engine:${JSON.stringify(contract.engine)}})+'\\n');
  else if (message.type === 'catalog') process.stdout.write(JSON.stringify({type:'catalog',id:message.id,offset:0,total:1,nextOffset:null,operations:[${JSON.stringify(operation)}]})+'\\n');
  else if (message.type === 'workspace') {
    const data = message.command.kind === 'wide' ? '{"value":18446744073709551615,"negative":-9223372036854775808,"safe":5}' : raw;
    process.stdout.write('{"type":"workspace","id":'+message.id+',"data":'+data+'}\\n');
  }
});
lines.on('close', () => process.exit(0));
`,
    { mode: 0o700 },
  );
  let engine: Engine | undefined;
  try {
    engine = await Engine.connect(binary);
    const received = (await engine.workspace({ kind: "wide" })) as {
      value: unknown;
      negative: unknown;
      safe: unknown;
    };
    assert.equal(received.value, 18446744073709551615n);
    assert.equal(received.negative, -9223372036854775808n);
    assert.equal(received.safe, 5);
    const echoed = (await engine.workspace({
      kind: "echo",
      value: received.value,
    })) as { command: { value: unknown } };
    assert.equal(echoed.command.value, 18446744073709551615n);
  } finally {
    await engine?.close();
    await rm(root, { recursive: true, force: true });
  }
});

test("JSON round trips keep exact integers, IEEE numbers and signed zero distinct", async () => {
  const { parseJson, stringifyJson } = await import("./json-codec.ts");
  const values = [
    18446744073709551615n,
    -9223372036854775808n,
    5,
    -0,
    1e20,
    1.5,
    { text: "18446744073709551615" },
  ];
  assert.deepEqual(parseJson(stringifyJson(values)), values);
  for (const value of [
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
  ])
    assert.throws(() => stringifyJson(value), /invalid_json_value/);
  assert.throws(
    () => stringifyJson(18446744073709551616n),
    /integer_out_of_range/,
  );
  assert.throws(
    () => parseJson("18446744073709551616"),
    /integer_out_of_range/,
  );
});
