import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import {
  contract,
  type Invocation,
  type Operation,
  type RunResult,
} from "./contract.ts";

interface Pending {
  resolve: (value: Record<string, unknown>) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class Engine {
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly pending = new Map<number, Pending>();
  private buffer = Buffer.alloc(0);
  private nextId = 0;
  private active: number | undefined;
  private closed = false;
  private stopTimer: ReturnType<typeof setTimeout> | undefined;

  private constructor(binary: string, worker?: string) {
    this.child = spawn(
      binary,
      worker ? ["--ipc", "--worker", worker] : ["--ipc"],
      {
        env: {},
        stdio: "pipe",
        windowsHide: true,
      },
    );
    this.child.stdout.on("data", (chunk: Buffer) => this.receive(chunk));
    // Drain bounded diagnostic writes; never retain or forward private stderr.
    this.child.stderr.resume();
    this.child.once("error", () => this.fail("engine_unavailable"));
    this.child.once("exit", () => {
      clearTimeout(this.stopTimer);
      this.fail("engine_stopped");
    });
    this.child.stdin.on("error", () => this.fail("engine_stopped"));
  }

  static async connect(binary: string, worker?: string): Promise<Engine> {
    const engine = new Engine(binary, worker);
    const nonce = randomBytes(16).toString("hex");
    try {
      const reply = await engine.request({
        type: "hello",
        protocol: contract.protocol,
        engine: contract.engine,
        nonce,
      });
      if (
        reply.type !== "ready" ||
        reply.protocol !== contract.protocol ||
        reply.engine !== contract.engine ||
        reply.nonce !== nonce
      ) {
        throw new Error("binary_mismatch");
      }
      return engine;
    } catch (error) {
      await engine.close();
      throw error;
    }
  }

  run(invocation: Invocation): Promise<RunResult> {
    if (this.active !== undefined) return Promise.reject(new Error("busy"));
    const id = this.nextId + 1;
    this.active = id;
    return this.request({ type: "invoke", invocation })
      .then((reply) => {
        if (reply.type !== "finished") throw new Error("invalid_response");
        return reply as unknown as RunResult;
      })
      .finally(() => {
        if (this.active === id) this.active = undefined;
      });
  }

  async catalog(): Promise<Operation[]> {
    const reply = await this.request({ type: "catalog" });
    if (reply.type !== "catalog" || !Array.isArray(reply.operations))
      throw new Error("invalid_response");
    return reply.operations as Operation[];
  }

  async cancel(): Promise<void> {
    if (this.active === undefined) return;
    const reply = await this.request({ type: "cancel", run_id: this.active });
    if (reply.type !== "cancelled") throw new Error("invalid_response");
  }

  private request(
    command: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    if (this.closed) return Promise.reject(new Error("engine_stopped"));
    if (this.pending.size >= contract.queue_commands)
      return Promise.reject(new Error("busy"));
    const id = ++this.nextId;
    const bytes = Buffer.from(`${JSON.stringify({ ...command, id })}\n`);
    if (bytes.length > contract.frame_bytes)
      return Promise.reject(new Error("input_limit"));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => this.fail("engine_timeout"),
        contract.handshake_ms + contract.run_ms,
      );
      this.pending.set(id, { resolve, reject, timer });
      this.child.stdin.write(bytes);
    });
  }

  private receive(chunk: Buffer): undefined {
    if (this.closed) return;
    this.buffer = Buffer.concat([this.buffer, chunk]);
    while (true) {
      const end = this.buffer.indexOf(10);
      if (end < 0) break;
      if (end + 1 > contract.frame_bytes) return this.fail("invalid_response");
      const line = this.buffer.subarray(0, end);
      this.buffer = this.buffer.subarray(end + 1);
      try {
        const text = new TextDecoder("utf8", { fatal: true }).decode(line);
        const reply = JSON.parse(text) as Record<string, unknown>;
        if (
          !reply ||
          !Number.isSafeInteger(reply.id) ||
          typeof reply.type !== "string"
        ) {
          return this.fail("invalid_response");
        }
        const pending = this.pending.get(reply.id as number);
        if (!pending) return this.fail("invalid_response");
        if (reply.type === "started") continue;
        if (
          !["ready", "catalog", "finished", "cancelled", "rejected"].includes(
            reply.type,
          )
        ) {
          return this.fail("invalid_response");
        }
        clearTimeout(pending.timer);
        this.pending.delete(reply.id as number);
        if (reply.type === "rejected")
          pending.reject(new Error("command_rejected"));
        else pending.resolve(reply);
      } catch {
        return this.fail("invalid_response");
      }
    }
    if (this.buffer.length >= contract.frame_bytes)
      this.fail("invalid_response");
  }

  private fail(code: string): undefined {
    if (this.closed) return;
    this.closed = true;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(new Error(code));
    }
    this.pending.clear();
    this.child.stdin.end();
    if (this.child.exitCode === null && this.child.signalCode === null) {
      this.stopTimer = setTimeout(() => this.child.kill("SIGKILL"), 500);
      this.stopTimer.unref();
    }
  }

  async close(): Promise<void> {
    this.fail("engine_stopped");
    if (
      !this.child.pid ||
      this.child.exitCode !== null ||
      this.child.signalCode !== null
    )
      return;
    await new Promise<void>((resolve) => {
      this.child.once("exit", () => resolve());
    });
  }
}
