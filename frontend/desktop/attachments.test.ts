import assert from "node:assert/strict";
import { access } from "node:fs/promises";
import test from "node:test";
import { Attachments } from "./attachments.ts";

const context = {
  workspace_id: "11111111-1111-4111-8111-111111111111",
  exploration_id: "22222222-2222-4222-8222-222222222222",
  stage_id: "33333333-3333-4333-8333-333333333333",
};
const dispatch = async () => null;

test("concurrent issuance reserves eight slots and only the issuing window can release them", {
  skip: process.platform === "win32",
}, async () => {
  const connections = new Attachments();
  try {
    const issued = await Promise.allSettled(
      Array.from({ length: 9 }, () =>
        connections.open(1, context, null, dispatch),
      ),
    );
    const accepted = issued.flatMap((r) =>
      r.status === "fulfilled" ? [r.value] : [],
    );
    assert.equal(accepted.length, 8);
    assert.deepEqual(
      issued.flatMap((r) =>
        r.status === "rejected" ? [r.reason.message] : [],
      ),
      ["attachment_limit"],
    );
    const first = accepted[0];
    assert.ok(first);
    await assert.rejects(
      connections.close(2, first.id),
      /unauthorized_attachment/,
    );
    await access(first.attachment.path);
    await connections.close(1, first.id);
    await assert.rejects(access(first.attachment.path));
    const replacement = await connections.open(2, context, null, dispatch);
    await access(replacement.attachment.path);
    await connections.clear();
    for (const entry of [...accepted, replacement])
      await assert.rejects(access(entry.attachment.path));
  } finally {
    await connections.clear();
  }
});

test("revocation waits for pending credential creation and never publishes the old lease", {
  skip: process.platform === "win32",
}, async () => {
  const connections = new Attachments();
  try {
    const opening = connections.open(1, context, null, dispatch);
    const rejected = assert.rejects(opening, /attachment_revoked/);
    await connections.clear();
    await rejected;
    const fresh = await connections.open(1, context, null, dispatch);
    await access(fresh.attachment.path);
    await connections.clear();
    await assert.rejects(access(fresh.attachment.path));
  } finally {
    await connections.clear();
  }
});
