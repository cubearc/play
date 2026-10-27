import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import { copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export default function setup() {
  if (process.env.CUBE_PLAY_DOCKER_TEST_URL)
    process.env.CUBE_PLAY_TEST_URL = process.env.CUBE_PLAY_DOCKER_TEST_URL;
}

// Each serial project worker owns an investigation within the production quotas.
export async function startLocalServer() {
  const directory = await mkdtemp(resolve(tmpdir(), "cube-play-browser-"));
  const source = resolve(directory, "source");
  await mkdir(source);
  await writeFile(
    resolve(source, "sample 空.mp4"),
    "original selection fixture",
  );
  await mkdir(resolve(source, "nested"));
  await writeFile(
    resolve(source, "notes.txt"),
    "<script>window.fixtureExecuted = true</script>\nPrivate original notes.",
  );
  await writeFile(
    resolve(source, "nested", "single.txt"),
    "One compatible file opens automatically.",
  );
  await writeFile(
    resolve(source, "unknown.xyz"),
    Buffer.from([0, 255, 17, 42]),
  );
  for (const name of [
    "sample.png",
    "indexed.png",
    "sample.mp4",
    "sample.wav",
    "rabbit.cube",
  ])
    await copyFile(resolve("../tests/fixtures", name), resolve(source, name));
  await writeFile(
    resolve(source, "malformed.png"),
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10, 0]),
  );
  await writeFile(
    resolve(source, "over-limit.png"),
    Buffer.alloc(4 * 1024 * 1024 + 1),
  );
  await mkdir(resolve(source, "empty"));
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
      CUBE_PLAY_DATA: resolve(directory, "imports"),
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
