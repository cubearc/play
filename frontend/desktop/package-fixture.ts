//! Test-only clone: exercise the real queue UI with a controlled sandboxed worker.
import assert from "node:assert/strict";
import { cp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { basename, dirname, join, relative } from "node:path";
import { nativeFiles } from "./native-files.ts";

export async function packagedWorkerFixture(
  executable: string,
  temporary: string,
  controlledNative: string,
): Promise<string> {
  assert.ok(["darwin", "linux"].includes(process.platform));
  const root =
    process.platform === "darwin"
      ? dirname(dirname(dirname(executable)))
      : dirname(executable);
  const cloned = join(temporary, basename(root));
  await cp(root, cloned, { recursive: true, verbatimSymlinks: true });
  if (process.platform === "linux") {
    // A user-owned copy cannot inherit the OS helper's root ownership.
    // Use the original configured helper without disabling Chromium's sandbox.
    await rm(join(cloned, "chrome-sandbox"));
    await symlink(join(root, "chrome-sandbox"), join(cloned, "chrome-sandbox"));
  }
  const app =
    process.platform === "darwin"
      ? join(cloned, "Contents/Resources/app")
      : join(cloned, "resources/app");
  const native = join(app, "frontend/desktop-native");
  const worker = nativeFiles().worker?.split("/")[0];
  assert.ok(worker);
  await rm(join(native, worker), { recursive: true });
  await cp(join(controlledNative, worker), join(native, worker), {
    recursive: true,
  });
  const integrity = await readFile(
    join(controlledNative, "integrity.json"),
    "utf8",
  );
  await writeFile(join(native, "integrity.json"), integrity);
  const proof = JSON.parse(
    await readFile(join(app, "package-proof.json"), "utf8"),
  );
  await writeFile(
    join(app, "package-proof.json"),
    JSON.stringify(
      {
        ...proof,
        qualificationFixture: "owned-worker-supervision",
        native: JSON.parse(integrity),
      },
      null,
      2,
    ),
  );
  return join(cloned, relative(root, executable));
}
