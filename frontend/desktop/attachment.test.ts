import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";
import { connect } from "node:net";
import { createInterface } from "node:readline";
import test from "node:test";
import { Attachment } from "./attachment.ts";

async function nextLine(lines: AsyncIterableIterator<string>): Promise<string> {
  const line = await lines.next();
  assert.equal(line.done, false);
  assert.ok(line.value);
  return line.value;
}
const context = {
  workspace_id: "11111111-1111-4111-8111-111111111111",
  exploration_id: "22222222-2222-4222-8222-222222222222",
  stage_id: "33333333-3333-4333-8333-333333333333",
};
test("external attachment authenticates one scoped command and revokes its private endpoint", {
  skip: process.platform === "win32",
}, async () => {
  const calls: unknown[] = [];
  const a = await Attachment.create(context, null, async (value) => {
    calls.push(value);
    return { id: "scoped-asset" };
  });
  const credential = JSON.parse(await readFile(a.path, "utf8"));
  assert.equal((await stat(a.path)).mode & 0o777, 0o600);
  const socket = connect(credential.socket);
  const lines = createInterface({ input: socket })[Symbol.asyncIterator]();
  socket.write(
    `${JSON.stringify({ type: "attach", protocol: 1, token: credential.token })}\n`,
  );
  const bootstrap = JSON.parse(await nextLine(lines));
  assert.deepEqual(bootstrap.context, { ...context, generation: 2 });
  socket.write(
    `${JSON.stringify({ type: "request", protocol: 1, nonce: bootstrap.nonce, context: bootstrap.context, command: { kind: "asset", asset_id: context.stage_id, revision: 1 } })}\n`,
  );
  const reply = JSON.parse(await nextLine(lines));
  assert.equal(reply.result.data.id, "scoped-asset");
  assert.deepEqual(calls, [
    { kind: "asset", context, asset_id: context.stage_id, revision: 1 },
  ]);
  socket.destroy();
  await a.close();
  await assert.rejects(stat(a.path), /ENOENT/);
});
test("foreign tokens and forged contexts never reach the engine", {
  skip: process.platform === "win32",
}, async () => {
  let called = 0;
  const a = await Attachment.create(context, null, async () => {
    called++;
    return {};
  });
  try {
    const credential = JSON.parse(await readFile(a.path, "utf8"));
    for (const forged of [true, false]) {
      const socket = connect(credential.socket);
      const lines = createInterface({ input: socket })[Symbol.asyncIterator]();
      socket.write(
        `${JSON.stringify({ type: "attach", protocol: 1, token: forged ? "0".repeat(64) : credential.token })}\n`,
      );
      if (!forged) {
        const bootstrap = JSON.parse(await nextLine(lines));
        bootstrap.context.exploration_id = context.stage_id;
        socket.write(
          `${JSON.stringify({ type: "request", protocol: 1, nonce: bootstrap.nonce, context: bootstrap.context, command: { kind: "status", run_id: context.stage_id } })}\n`,
        );
      }
      assert.equal((await lines.next()).done, true);
      socket.destroy();
    }
    assert.equal(called, 0);
  } finally {
    await a.close();
  }
});
