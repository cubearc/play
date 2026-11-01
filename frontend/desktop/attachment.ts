/** Explicit, revocable control attachment. No operation executes in this process. */
import { randomBytes, randomUUID, timingSafeEqual } from "node:crypto";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer, type Server, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { type CliIntent, cliIntent } from "./cli-contract.ts";
import { contract } from "./contract.ts";
import {
  type Context,
  context as parseContext,
  type RunRequest,
  runRequest,
} from "./workspace-contract.ts";

export class Attachment {
  readonly path: string;
  private readonly root: string;
  private readonly server: Server;
  private readonly connections = new Set<Socket>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private stopped = false;
  private constructor(root: string, server: Server) {
    this.root = root;
    this.server = server;
    this.path = join(root, "context.json");
  }
  static async create(
    context: Context,
    selection: RunRequest | null,
    dispatch: (intent: CliIntent) => Promise<unknown>,
  ): Promise<Attachment> {
    if (process.platform === "win32")
      throw new Error("attachment_policy_unavailable");
    parseContext(context);
    if (selection) {
      runRequest(selection);
      if (!isDeepStrictEqual(selection.context, context))
        throw new Error("context_mismatch");
    }
    const root = await mkdtemp(join(tmpdir(), "cube-play-"));
    const server = createServer();
    const attachment = new Attachment(root, server);
    const socketPath = join(root, "control.sock");
    const token = randomBytes(32).toString("hex");
    try {
      await chmod(root, 0o700);
      server.on("connection", (socket) =>
        attachment.accept(socket, token, context, selection, dispatch),
      );
      await new Promise<void>((resolve, reject) => {
        server.once("error", reject);
        server.listen(socketPath, () => {
          server.removeListener("error", reject);
          resolve();
        });
      });
      server.on("error", () => void attachment.close());
      await chmod(socketPath, 0o600);
      const expires = Date.now() + 10 * 60_000;
      await writeFile(
        attachment.path,
        `${JSON.stringify({ format: "cubearc.play.context", version: 1, socket: socketPath, token, expires })}\n`,
        { flag: "wx", mode: 0o600 },
      );
      attachment.timer = setTimeout(
        () => void attachment.close(),
        expires - Date.now(),
      );
      attachment.timer.unref();
      return attachment;
    } catch (error) {
      await attachment.close();
      throw error;
    }
  }
  private accept(
    socket: Socket,
    token: string,
    context: Context,
    selection: RunRequest | null,
    dispatch: (intent: CliIntent) => Promise<unknown>,
  ) {
    if (this.stopped || this.connections.size >= 1) {
      socket.destroy();
      return;
    }
    this.connections.add(socket);
    socket.once("close", () => this.connections.delete(socket));
    socket.on("error", () => socket.destroy());
    socket.setTimeout(contract.handshake_ms, () => socket.destroy());
    let buffer = Buffer.alloc(0),
      phase = 0;
    const nonce = randomBytes(16).toString("hex");
    const selected = selection ? { ...selection, key: randomUUID() } : null;
    const scope = { ...context, generation: contract.storage_generation };
    const send = (value: unknown) => {
      const bytes = Buffer.from(`${JSON.stringify(value)}\n`);
      if (bytes.length > contract.frame_bytes) socket.destroy();
      else socket.write(bytes);
    };
    socket.on("data", (chunk: Buffer) => {
      buffer = Buffer.concat([buffer, chunk]);
      if (buffer.length > contract.frame_bytes) {
        socket.destroy();
        return;
      }
      const end = buffer.indexOf(10);
      if (end < 0) return;
      if (end !== buffer.length - 1 || phase === 2) {
        socket.destroy();
        return;
      }
      let frame: Record<string, unknown>;
      try {
        frame = JSON.parse(
          new TextDecoder("utf8", { fatal: true }).decode(buffer),
        );
      } catch {
        socket.destroy();
        return;
      }
      buffer = Buffer.alloc(0);
      if (phase === 0) {
        if (
          !frame ||
          Object.keys(frame).sort().join() !== "protocol,token,type" ||
          frame.type !== "attach" ||
          frame.protocol !== 1 ||
          typeof frame.token !== "string" ||
          !/^[a-f0-9]{64}$/.test(frame.token) ||
          !timingSafeEqual(Buffer.from(frame.token), Buffer.from(token))
        ) {
          socket.destroy();
          return;
        }
        phase = 1;
        send({
          type: "context",
          protocol: 1,
          cli: "0.18.0",
          nonce,
          context: scope,
          run: selected,
        });
        return;
      }
      let intent: CliIntent;
      try {
        if (
          Object.keys(frame).sort().join() !==
            "command,context,nonce,protocol,type" ||
          frame.type !== "request" ||
          frame.protocol !== 1 ||
          frame.nonce !== nonce ||
          !isDeepStrictEqual(frame.context, scope)
        )
          throw new Error("invalid_frame");
        const action = frame.command as Record<string, unknown>;
        intent = cliIntent(
          action?.kind === "start" ? action : { ...action, context },
        );
        if (
          intent.kind === "start" &&
          (!selected || !isDeepStrictEqual(intent.request, selected))
        )
          throw new Error("selection_mismatch");
      } catch {
        socket.destroy();
        return;
      }
      phase = 2;
      socket.setTimeout(contract.handshake_ms + contract.run_ms, () =>
        socket.destroy(),
      );
      void dispatch(intent)
        .then(
          (data) =>
            send({
              type: "reply",
              protocol: 1,
              nonce,
              result: { status: "ok", data },
            }),
          (error) =>
            send({
              type: "reply",
              protocol: 1,
              nonce,
              result: {
                status: "rejected",
                code:
                  error instanceof Error && /^[a-z_]{1,64}$/.test(error.message)
                    ? error.message
                    : "command_rejected",
              },
            }),
        )
        .finally(() => socket.end());
    });
  }
  get isClosed(): boolean {
    return this.stopped;
  }
  async close(): Promise<void> {
    if (this.stopped) return;
    this.stopped = true;
    clearTimeout(this.timer);
    for (const socket of this.connections) socket.destroy();
    await new Promise<void>((resolve) => {
      if (this.server.listening) this.server.close(() => resolve());
      else resolve();
    });
    await rm(this.root, { recursive: true, force: true });
  }
}
