import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readdir, readFile, rm } from "node:fs/promises";
import { resolve } from "node:path";
import MCR from "monocart-coverage-reports";

const root = process.cwd();
const raw = resolve("coverage/native-raw");
function run(script, env = process.env) {
  const result = spawnSync(process.execPath, script, { stdio: "inherit", env });
  assert.equal(
    result.status,
    0,
    `Coverage command failed: ${script.join(" ")}`,
  );
}
if (!process.argv.includes("--report-only")) {
  await rm("coverage", { recursive: true, force: true });
  run([
    "node_modules/vitest/vitest.mjs",
    "run",
    "--coverage",
    "--maxWorkers=1",
  ]);
  run(["--test", "desktop-dist/desktop.e2e.js"], {
    ...process.env,
    PLAY_COVERAGE: raw,
  });
}
const report = MCR({
  outputDir: "coverage/native",
  reports: ["json"],
  sourceFilter: (path) =>
    path.includes("desktop-renderer-src/") &&
    !path.includes("/test/") &&
    !path.includes(".test."),
  sourcePath: (path) => {
    const index = path.indexOf("desktop-renderer-src/");
    return index < 0 ? path : resolve(root, path.slice(index));
  },
});
const files = await readdir(raw);
assert.ok(files.length > 0, "Missing native Electron coverage");
const bundle = await readFile("desktop-renderer/app.js");
const digest = createHash("sha256").update(bundle).digest("hex");
for (const file of files) {
  const capture = JSON.parse(await readFile(resolve(raw, file), "utf8"));
  assert.equal(
    capture.bundleSha256,
    digest,
    "Native coverage belongs to a different build",
  );
  await report.add(capture.entries);
}
assert.ok(await report.generate(), "Missing native coverage report");
const combined = MCR({
  outputDir: "coverage",
  reports: ["lcovonly", "json", "json-summary", "text-summary"],
});
await combined.add(
  JSON.parse(await readFile("coverage/unit/coverage-final.json", "utf8")),
);
await combined.add(
  JSON.parse(await readFile("coverage/native/coverage-final.json", "utf8")),
);
assert.ok(await combined.generate(), "Missing combined coverage report");
const summary = JSON.parse(
  await readFile("coverage/coverage-summary.json", "utf8"),
);
const sources = (await readdir("desktop-renderer-src")).filter(
  (name) => /\.(ts|tsx)$/.test(name) && !name.includes(".test."),
);
for (const file of sources)
  assert.ok(
    Object.keys(summary).some((path) =>
      path.endsWith(`/desktop-renderer-src/${file}`),
    ),
    `Unmeasured renderer file: ${file}`,
  );
for (const metric of ["lines", "functions", "statements", "branches"]) {
  assert.ok(
    summary.total[metric].total > 0 && summary.total[metric].pct >= 80,
    `${metric}: ${summary.total[metric].pct}%, required 80%`,
  );
}
