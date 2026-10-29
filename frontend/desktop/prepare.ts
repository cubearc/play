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

const root = fileURLToPath(new URL("../../", import.meta.url));
const output = join(root, "frontend/desktop-native");

function command(binary: string, args: string[]): void {
  const result = spawnSync(binary, args, { cwd: root, stdio: "inherit" });
  if (result.error || result.status !== 0)
    throw new Error(`Desktop build failed: ${binary}`);
}

function bundle(name: string, binary: string, identifier: string): string {
  const app = join(output, `${name}.app`);
  mkdirSync(join(app, "Contents/MacOS"), { recursive: true });
  cpSync(binary, join(app, "Contents/MacOS/play-engine"));
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
  return join(app, "Contents/MacOS/play-engine");
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
  "--bin",
  "play-engine",
]);
rmSync(output, { recursive: true, force: true });
mkdirSync(output, { recursive: true });
const binary = process.platform === "win32" ? "play-engine.exe" : "play-engine";
cpSync(join(root, "target/debug", binary), join(output, binary));
const digest = (file: string) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
const manifest: Record<string, string> = {
  engine: digest(join(output, binary)),
};
if (process.platform === "darwin") {
  manifest.worker = digest(
    bundle(
      "Worker",
      join(root, "target/debug/play-engine"),
      "com.cubearc.play.boundary-worker",
    ),
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
writeFileSync(join(output, "integrity.json"), `${JSON.stringify(manifest)}\n`);
