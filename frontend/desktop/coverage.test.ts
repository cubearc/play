import assert from "node:assert/strict";
import test from "node:test";
import { captureBeforeClose } from "./coverage.ts";

test("capture survives an explicit exit before idempotent final cleanup", async () => {
  let exited = false;
  let captures = 0;
  const application = {
    close: async () => {
      exited = true;
    },
  };
  const flush = captureBeforeClose(application, async () => {
    assert.equal(exited, false, "The page is unavailable after app.exit");
    captures++;
  });
  await flush();
  exited = true;
  await application.close();
  await application.close();
  assert.equal(captures, 1);
});

test("concurrent closes collect and close exactly once", async () => {
  let captures = 0;
  let closes = 0;
  const application = {
    close: async () => {
      closes++;
    },
  };
  captureBeforeClose(application, async () => {
    captures++;
  });
  await Promise.all([application.close(), application.close()]);
  assert.equal(captures, 1);
  assert.equal(closes, 1);
});

test("failed collection remains a failure and still closes its own application", async () => {
  let closes = 0;
  const application = {
    close: async () => {
      closes++;
    },
  };
  const failure = new Error("collection failed");
  captureBeforeClose(application, async () => {
    throw failure;
  });
  await assert.rejects(application.close(), failure);
  await assert.rejects(application.close(), failure);
  assert.equal(closes, 1);
});
