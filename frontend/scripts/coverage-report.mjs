import assert from "node:assert/strict";
import { readdir } from "node:fs/promises";
import MCR from "monocart-coverage-reports";

export async function mergeV8Coverage(unitRaw, nativeEntries, options) {
  assert.ok(
    (await readdir(unitRaw)).some(
      (name) => name.startsWith("coverage-") && name.endsWith(".json"),
    ),
    "Missing unit V8 profiles",
  );
  const report = MCR({
    ...options,
    inputDir: unitRaw,
    clean: false,
    cleanCache: true,
    reports: options.reports ?? [
      "lcovonly",
      "json-summary",
      "v8-json",
      "text-summary",
    ],
  });
  let count = 0;
  for await (const entries of nativeEntries) {
    await report.add(entries);
    count++;
  }
  assert.ok(count > 0, "Missing actual Electron V8 profiles");
  const result = await report.generate();
  assert.ok(result, "Missing combined V8 coverage report");
  return result;
}
