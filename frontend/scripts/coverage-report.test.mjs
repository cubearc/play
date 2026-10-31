import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { Session } from "node:inspector/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { createContext, Script } from "node:vm";
import MCR from "monocart-coverage-reports";
import { mergeV8Coverage } from "./coverage-report.mjs";

test("unit and native profiles cover complementary branches without duplicate counters", async () => {
  const directory = await mkdtemp(join(tmpdir(), "play-v8-merge-"));
  const source = '(function choose(flag) { return flag ? "yes" : "no"; })';
  const filename = `file://${join(directory, "choice.js")}`;
  const session = new Session();
  session.connect();
  try {
    await session.post("Profiler.enable");
    await session.post("Profiler.startPreciseCoverage", {
      callCount: true,
      detailed: true,
    });
    const choose = new Script(source, { filename }).runInContext(
      createContext(),
    );
    assert.equal(choose(true), "yes");
    const first = await session.post("Profiler.takePreciseCoverage");
    assert.equal(choose(false), "no");
    const second = await session.post("Profiler.takePreciseCoverage");
    const entry = (data) => {
      const value = data.result.find((item) => item.url === filename);
      assert.ok(value);
      return { ...value, source };
    };
    const unit = MCR({ outputDir: join(directory, "unit"), reports: ["raw"] });
    await unit.add([entry(first)]);
    const alone = await unit.generate();
    assert.ok(alone.summary.branches.covered < alone.summary.branches.total);
    const merged = await mergeV8Coverage(
      join(directory, "unit/raw"),
      [[entry(second)]],
      { outputDir: join(directory, "merged"), reports: ["json-summary"] },
    );
    assert.equal(merged.summary.functions.total, alone.summary.functions.total);
    assert.equal(merged.summary.branches.total, alone.summary.branches.total);
    assert.equal(
      merged.summary.branches.covered,
      merged.summary.branches.total,
    );
  } finally {
    await session.post("Profiler.stopPreciseCoverage");
    await session.post("Profiler.disable");
    session.disconnect();
    await rm(directory, { recursive: true, force: true });
  }
});
