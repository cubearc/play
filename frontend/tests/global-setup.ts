import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export default async function setup() {
  const directory = await mkdtemp(resolve(tmpdir(), "cube-play-browser-"));
  const source = resolve(directory, "source");
  await mkdir(source);
  await writeFile(
    resolve(source, "sample 空.mp4"),
    "original selection fixture",
  );
  const workspace = resolve(directory, "workspace.json");
  await writeFile(
    workspace,
    JSON.stringify({ id: randomUUID(), name: "Browser smoke workspace" }),
  );
  const binary = process.platform === "win32" ? "play-local.exe" : "play-local";
  const server = spawn(resolve("../target/debug", binary), [], {
    env: {
      ...process.env,
      CUBE_PLAY_BIND: "127.0.0.1:17331",
      CUBE_PLAY_ORIGIN: "http://127.0.0.1:17331",
      CUBE_PLAY_CONTAINER: "0",
      CUBE_PLAY_FRONTEND_DIR: resolve("dist"),
      CUBE_PLAY_WORKSPACE: workspace,
      CUBE_PLAY_SOURCE: source,
    },
    stdio: ["ignore", "pipe", "ignore"],
  });
  try {
    process.env.CUBE_PLAY_TEST_URL = await new Promise<string>(
      (accept, reject) => {
        const timer = setTimeout(
          () => reject(new Error("Local application did not become ready")),
          10_000,
        );
        server.once("error", (error) => {
          clearTimeout(timer);
          reject(error);
        });
        server.once("exit", () => {
          clearTimeout(timer);
          reject(new Error("Local application exited before readiness"));
        });
        let output = "";
        server.stdout.on("data", (chunk: Buffer) => {
          output += chunk.toString();
          const match = output.match(
            /Open Cube Play: (http:\/\/127\.0\.0\.1:17331\/#session=[a-f0-9-]+)/,
          );
          if (match?.[1]) {
            clearTimeout(timer);
            accept(match[1]);
          }
        });
      },
    );
  } catch (error) {
    server.kill();
    await rm(directory, { recursive: true, force: true });
    throw error;
  }
  return async () => {
    if (server.exitCode !== null) {
      await rm(directory, { recursive: true, force: true });
      return;
    }
    const exited = once(server, "exit");
    server.kill("SIGTERM");
    const timer = setTimeout(() => server.kill("SIGKILL"), 5_000);
    await exited;
    clearTimeout(timer);
    await rm(directory, { recursive: true, force: true });
  };
}
