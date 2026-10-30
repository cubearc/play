import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { nativeFiles } from "./native-files.ts";

export async function verifyNative(directory: URL): Promise<void> {
  const manifest = JSON.parse(
    await readFile(new URL("integrity.json", directory), "utf8"),
  ) as Record<string, string>;
  for (const [key, name] of Object.entries(nativeFiles())) {
    const hash = createHash("sha256")
      .update(await readFile(new URL(name, directory)))
      .digest("hex");
    if (hash !== manifest[key]) throw new Error("binary_mismatch");
  }
  if (process.platform === "darwin") {
    await promisify(execFile)(
      "/usr/bin/codesign",
      [
        "--verify",
        "--strict",
        fileURLToPath(new URL("Terminal.app", directory)),
      ],
      { env: {}, timeout: 3000, maxBuffer: 65536 },
    );
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
