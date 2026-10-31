//! Build-only preview packaging; no certificate or release channel is selected.
import { execFileSync } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { packager } from "@electron/packager";
import { contract } from "./contract.ts";
import { verifyNative } from "./integrity.ts";
import { nativeFiles } from "./native-files.ts";

const root = fileURLToPath(new URL("../../", import.meta.url));
const frontend = join(root, "frontend");
const native = join(frontend, "desktop-native");
const output = resolve(process.argv[2] ?? join(frontend, "desktop-packages"));
const platform = process.platform;
const arch = process.arch;
if (platform !== "darwin" && platform !== "linux" && platform !== "win32")
  throw new Error("package_target_unavailable");
if (arch !== "x64" && arch !== "arm64")
  throw new Error("package_target_unavailable");
const files = nativeFiles();
if (!files.worker || !files.terminal)
  throw new Error("package_execution_policy_unavailable");
const metadata = JSON.parse(
  await readFile(join(frontend, "package.json"), "utf8"),
) as {
  version: string;
  devDependencies: { electron: string };
};
await verifyNative(pathToFileURL(`${native}/`));
const stage = await mkdtemp(join(tmpdir(), "cube-play-package-"));
try {
  const app = join(stage, "app");
  const target = join(app, "frontend");
  await mkdir(join(target, "desktop-main"), { recursive: true });
  await cp(
    join(frontend, "desktop-main/main.js"),
    join(target, "desktop-main/main.js"),
  );
  await cp(
    join(frontend, "desktop-dist/preload.cjs"),
    join(target, "desktop-main/preload.cjs"),
  );
  await cp(
    join(frontend, "desktop-renderer"),
    join(target, "desktop-renderer"),
    { recursive: true },
  );
  const bundled = join(target, "desktop-native");
  await mkdir(bundled);
  const roots = new Set(Object.values(files).map((name) => name.split("/")[0]));
  for (const name of roots) {
    if (!name) throw new Error("invalid_native_layout");
    await cp(join(native, name), join(bundled, name), { recursive: true });
  }
  await cp(join(native, "integrity.json"), join(bundled, "integrity.json"));
  await verifyNative(pathToFileURL(`${bundled}/`));
  await mkdir(join(app, "desktop"));
  await cp(
    join(root, "desktop/protocol.json"),
    join(app, "desktop/protocol.json"),
  );
  await writeFile(
    join(app, "package.json"),
    JSON.stringify(
      {
        name: "cubearc-play-preview",
        version: metadata.version,
        productName: "Cube Play",
        type: "module",
        main: "frontend/desktop-main/main.js",
        private: true,
      },
      null,
      2,
    ),
  );
  const git = (args: string[]) =>
    execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
  const proof = {
    formatVersion: 1,
    distribution: "local_preview",
    publisherSigning: "absent",
    platform,
    architecture: arch,
    appVersion: metadata.version,
    electronVersion: metadata.devDependencies.electron,
    engineVersion: contract.engine,
    protocol: contract.protocol,
    sourceRevision: git(["rev-parse", "HEAD"]),
    sourceDirty: git(["status", "--porcelain"]) !== "",
    cliVersion: execFileSync(join(native, files.cli), ["--version"], {
      env: {},
      encoding: "utf8",
      timeout: 5000,
    }).trim(),
    native: JSON.parse(await readFile(join(native, "integrity.json"), "utf8")),
  };
  await writeFile(
    join(app, "package-proof.json"),
    JSON.stringify(proof, null, 2),
  );
  await mkdir(output, { recursive: true });
  const paths = await packager({
    dir: app,
    out: output,
    name: "Cube Play",
    executableName: "cube-play",
    appBundleId: "com.cubearc.play.preview",
    appVersion: metadata.version,
    electronVersion: metadata.devDependencies.electron,
    platform,
    arch,
    asar: false,
    prune: false,
    overwrite: false,
  });
  if (paths.length !== 1)
    throw new Error("package_not_created_choose_empty_destination");
  console.log(JSON.stringify({ ...proof, packages: paths }, null, 2));
} finally {
  await rm(stage, { recursive: true, force: true });
}
