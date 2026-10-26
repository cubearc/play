import { spawn } from "node:child_process";
import { once } from "node:events";
import { resolve } from "node:path";

export default async function setup() {
  const binary = process.platform === "win32" ? "play-local.exe" : "play-local";
  const server = spawn(resolve("../target/debug", binary), [], {
    env: {
      ...process.env,
      CUBE_PLAY_BIND: "127.0.0.1:17331",
      CUBE_PLAY_ORIGIN: "http://127.0.0.1:17331",
      CUBE_PLAY_CONTAINER: "0",
      CUBE_PLAY_FRONTEND_DIR: resolve("dist"),
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
    throw error;
  }
  return async () => {
    if (server.exitCode !== null) return;
    const exited = once(server, "exit");
    server.kill("SIGTERM");
    const timer = setTimeout(() => server.kill("SIGKILL"), 5_000);
    await exited;
    clearTimeout(timer);
  };
}
