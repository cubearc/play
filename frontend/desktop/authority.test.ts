import assert from "node:assert/strict";
import test from "node:test";
import { APP_URL, ownsFrame, resourceName } from "./authority.ts";

test("opens only a named packaged resource", () => {
  assert.equal(resourceName("cube-play://app/index.html"), "index.html");
});

test("requires both the registered window and its current top frame", () => {
  const frame = { url: APP_URL };
  const trusted = { mainFrame: frame };
  assert.equal(
    ownsFrame({ sender: trusted, senderFrame: frame }, trusted),
    true,
  );
  for (const event of [
    { sender: { mainFrame: frame }, senderFrame: frame },
    { sender: trusted, senderFrame: { url: APP_URL } },
    { sender: trusted, senderFrame: null },
  ]) {
    assert.equal(ownsFrame(event, trusted), false);
  }
  frame.url = "https://cloud.example/challenge";
  assert.equal(
    ownsFrame({ sender: trusted, senderFrame: frame }, trusted),
    false,
  );
});

test("rejects paths, remote hosts, credentials, queries and encoded traversal", () => {
  for (const url of [
    "file:///etc/passwd",
    "https://app/index.html",
    "cube-play://remote/index.html",
    "cube-play://user@app/index.html",
    "cube-play://app/index.html?path=/etc/passwd",
    "cube-play://app/%2e%2e/private",
    "cube-play://app/../index.html",
    "cube-play://app/%69ndex.html",
    "cube-play://app/index.html#callback",
    "cube-play://app/",
  ]) {
    assert.equal(resourceName(url), null, url);
  }
});
