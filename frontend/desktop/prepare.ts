import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { nativeFiles } from "./native-files.ts";

const root = fileURLToPath(new URL("../../", import.meta.url));
const output = join(root, "frontend/desktop-native");

function command(binary: string, args: string[]): void {
  const result = spawnSync(binary, args, { cwd: root, stdio: "inherit" });
  if (result.error || result.status !== 0)
    throw new Error(`Desktop build failed: ${binary}`);
}

function bundle(
  name: string,
  binary: string,
  identifier: string,
  client?: string,
): void {
  const app = join(output, `${name}.app`);
  mkdirSync(join(app, "Contents/MacOS"), { recursive: true });
  cpSync(binary, join(app, "Contents/MacOS/play-engine"));
  if (client) {
    const target = join(app, "Contents/MacOS/cube");
    cpSync(client, target);
    command("/usr/bin/codesign", [
      "--force",
      "--sign",
      "-",
      "--entitlements",
      join(root, "desktop/macos/terminal-client.entitlements.plist"),
      target,
    ]);
  }
  const info = readFileSync(join(root, "desktop/macos/Info.plist"), "utf8");
  writeFileSync(
    join(app, "Contents/Info.plist"),
    info.replace("com.cubearc.play.boundary-worker", identifier),
  );
  command("/usr/bin/codesign", [
    "--force",
    "--sign",
    "-",
    "--entitlements",
    join(root, "desktop/macos/worker.entitlements.plist"),
    app,
  ]);
  command("/usr/bin/codesign", ["--verify", "--strict", app]);
}

command("cargo", [
  "build",
  "--locked",
  "-p",
  "play-engine",
  "--example",
  "native-boundary-probe",
  "--example",
  "native-supervision-probe",
  ...(process.platform === "linux"
    ? ["--example", "linux-boundary-probe"]
    : []),
  "--bin",
  "play-engine",
]);
command("cargo", [
  "build",
  "--locked",
  "--manifest-path",
  join(root, "../cube/Cargo.toml"),
  "-p",
  "cube-cli",
]);
rmSync(output, { recursive: true, force: true });
mkdirSync(output, { recursive: true });
const files = nativeFiles();
cpSync(join(root, "target/debug", files.engine), join(output, files.engine));
cpSync(join(root, "../cube/target/debug", files.cli), join(output, files.cli));
if (process.platform === "darwin") {
  bundle(
    "Terminal",
    join(root, "target/debug/play-engine"),
    "com.cubearc.play.context-terminal",
    join(output, files.cli),
  );
  bundle(
    "TerminalProbe",
    join(root, "target/debug/play-engine"),
    "com.cubearc.play.terminal-probe",
    join(root, "target/debug/examples/native-boundary-probe"),
  );
  bundle(
    "Worker",
    join(root, "target/debug/play-engine"),
    "com.cubearc.play.boundary-worker",
  );
  bundle(
    "SupervisionProbe",
    join(root, "target/debug/examples/native-supervision-probe"),
    "com.cubearc.play.supervision-probe",
  );
  bundle(
    "Probe",
    join(root, "target/debug/examples/native-boundary-probe"),
    "com.cubearc.play.boundary-probe",
  );
}
if (process.platform === "linux" && files.worker) {
  cpSync(join(output, files.engine), join(output, files.worker));
  for (const name of [
    "native-boundary-probe",
    "native-supervision-probe",
    "linux-boundary-probe",
  ])
    cpSync(join(root, "target/debug/examples", name), join(output, name));
}
const manifest = Object.fromEntries(
  Object.entries(files).map(([key, name]) => [
    key,
    createHash("sha256")
      .update(readFileSync(join(output, name)))
      .digest("hex"),
  ]),
);
writeFileSync(join(output, "integrity.json"), `${JSON.stringify(manifest)}\n`);
