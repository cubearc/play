import assert from "node:assert/strict";
import test from "node:test";
import { nativeFiles } from "./native-files.ts";

test("Linux layout includes an independently verified terminal helper", () => {
  const descriptor = Object.getOwnPropertyDescriptor(process, "platform");
  assert.ok(descriptor);
  try {
    Object.defineProperty(process, "platform", { value: "linux" });
    const files = nativeFiles();
    assert.equal(files.terminal, "play-terminal");
    assert.notEqual(files.terminal, files.worker);
  } finally {
    Object.defineProperty(process, "platform", descriptor);
  }
});
