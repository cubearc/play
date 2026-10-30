import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

export async function verifyNative(directory: URL): Promise<void> {
  const manifest = JSON.parse(
    await readFile(new URL("integrity.json", directory), "utf8"),
  ) as Record<string, string>;
  const files: [string, string][] = [
    ["cli", process.platform === "win32" ? "cube.exe" : "cube"],
    [
      "engine",
      process.platform === "win32" ? "play-engine.exe" : "play-engine",
    ],
  ];
  if (process.platform === "darwin")
    files.push(["worker", "Worker.app/Contents/MacOS/play-engine"]);
  for (const [key, name] of files) {
    const hash = createHash("sha256")
      .update(await readFile(new URL(name, directory)))
      .digest("hex");
    if (hash !== manifest[key]) throw new Error("binary_mismatch");
  }
  if (process.platform === "darwin") {
    await promisify(execFile)(
      "/usr/bin/codesign",
      ["--verify", "--strict", fileURLToPath(new URL("Worker.app", directory))],
      {
        env: {},
        timeout: 3000,
        maxBuffer: 65536,
      },
    );
  }
}
