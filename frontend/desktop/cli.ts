import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { isDeepStrictEqual } from "node:util";
import {
  type CliIntent,
  type CliResponses,
  cliIntent,
} from "./cli-contract.ts";
import { contract } from "./contract.ts";
import type { Engine } from "./engine.ts";

const protocol = 1;
const version = "0.18.0";
function command(value: CliIntent): { args: string[]; action: object } {
  switch (value.kind) {
    case "asset":
      return {
        args: [
          "asset",
          "show",
          value.asset_id,
          "--revision",
          String(value.revision),
        ],
        action: {
          kind: value.kind,
          asset_id: value.asset_id,
          revision: value.revision,
        },
      };
    case "pipeline":
      return {
        args: [
          "pipeline",
          "show",
          value.pipeline_id,
          "--revision",
          String(value.revision),
        ],
        action: {
          kind: value.kind,
          pipeline_id: value.pipeline_id,
          revision: value.revision,
        },
      };
    case "start":
      return {
        args: [
          "run",
          "start",
          "--pipeline",
          value.request.pipeline_id,
          "--revision",
          String(value.request.pipeline_revision),
          "--target",
          value.request.target_node_id,
        ],
        action: value,
      };
    case "status":
    case "cancel":
      return {
        args: ["run", value.kind, value.run_id],
        action: { kind: value.kind, run_id: value.run_id },
      };
  }
}
/** Trusted control client. Algorithms execute only in the engine's restricted worker. */
export class Cli {
  private active:
    | { child: ChildProcessWithoutNullStreams; done: Promise<unknown> }
    | undefined;
  private closed = false;
  constructor(
    private readonly binary: string,
    private readonly engine: Engine,
  ) {}
  execute<K extends CliIntent["kind"]>(
    input: Extract<CliIntent, { kind: K }>,
  ): Promise<CliResponses[K]>;
  execute(input: CliIntent): Promise<CliResponses[CliIntent["kind"]]>;
  execute(input: CliIntent): Promise<CliResponses[CliIntent["kind"]]> {
    if (this.closed) return Promise.reject(new Error("cli_stopped"));
    if (this.active) return Promise.reject(new Error("busy"));
    const value = cliIntent(input);
    const selected = command(value);
    const nonce = randomBytes(16).toString("hex");
    const scope =
      value.kind === "start" ? value.request.context : value.context;
    const context = { ...scope, generation: contract.storage_generation };
    const expected = {
      type: "request",
      protocol,
      nonce,
      context,
      command: selected.action,
    };
    const child = spawn(
      this.binary,
      ["--json-errors", "play", ...selected.args],
      { env: {}, stdio: "pipe", windowsHide: true },
    );
    const done = new Promise<CliResponses[CliIntent["kind"]]>(
      (resolve, reject) => {
        let buffer = Buffer.alloc(0),
          bytes = 0,
          errors = 0,
          phase = 0;
        let failure: string | undefined;
        let reply: Record<string, unknown> | undefined;
        let data: CliResponses[CliIntent["kind"]] | undefined;
        const fail = (code: string) => {
          failure ??= code;
          child.kill("SIGKILL");
        };
        const timer = setTimeout(
          () => fail("cli_timeout"),
          contract.handshake_ms + contract.run_ms,
        );
        const write = (frame: object) => {
          const bytes = Buffer.from(`${JSON.stringify(frame)}\n`);
          if (bytes.length > contract.frame_bytes) return fail("output_limit");
          child.stdin.write(bytes);
        };
        child.once("error", () => fail("cli_unavailable"));
        child.stdin.on("error", () => fail("transport_closed"));
        child.stderr.on("data", (chunk: Buffer) => {
          errors += chunk.length;
          if (errors > contract.output_bytes) fail("output_limit");
        });
        child.stdout.on("data", (chunk: Buffer) => {
          bytes += chunk.length;
          if (bytes > 2 * contract.frame_bytes) return fail("output_limit");
          buffer = Buffer.concat([buffer, chunk]);
          while (buffer.includes(10)) {
            const end = buffer.indexOf(10);
            const line = buffer.subarray(0, end);
            buffer = buffer.subarray(end + 1);
            if (end + 1 > contract.frame_bytes) return fail("output_limit");
            let frame: unknown;
            try {
              frame = JSON.parse(
                new TextDecoder("utf8", { fatal: true }).decode(line),
              );
            } catch {
              return fail("invalid_response");
            }
            if (phase === 0 && isDeepStrictEqual(frame, expected)) {
              phase = 1;
              void this.dispatch(value).then(
                (result) => {
                  if (failure) return;
                  data = result;
                  reply = {
                    type: "reply",
                    protocol,
                    nonce,
                    result: { status: "ok", data },
                  };
                  phase = 2;
                  write(reply);
                  child.stdin.end();
                },
                (error) => {
                  const code =
                    error instanceof Error &&
                    /^[a-z_]{1,64}$/.test(error.message)
                      ? error.message
                      : "command_rejected";
                  reply = {
                    type: "reply",
                    protocol,
                    nonce,
                    result: { status: "rejected", code },
                  };
                  failure = code;
                  phase = 2;
                  write(reply);
                  child.stdin.end();
                },
              );
            } else if (
              phase === 2 &&
              reply &&
              isDeepStrictEqual(frame, { ...reply, type: "result" })
            ) {
              phase = 3;
            } else return fail("invalid_response");
          }
          if (buffer.length >= contract.frame_bytes) fail("output_limit");
        });
        child.once("close", (code) => {
          clearTimeout(timer);
          this.active = undefined;
          if (failure) reject(new Error(failure));
          else if (code !== 0 || phase !== 3 || buffer.length || !data)
            reject(new Error("invalid_response"));
          else resolve(data);
        });
        write({
          type: "context",
          protocol,
          cli: version,
          nonce,
          context,
          run: value.kind === "start" ? value.request : null,
        });
      },
    );
    this.active = { child, done };
    return done;
  }
  private async dispatch(
    value: CliIntent,
  ): Promise<CliResponses[CliIntent["kind"]]> {
    switch (value.kind) {
      case "asset":
      case "pipeline":
        return this.engine.cliRead(value);
      case "start":
        return this.engine.startCliRun(value.request);
      case "status":
        return (await this.engine.workspace({
          kind: "run_detail",
          context: value.context,
          run_id: value.run_id,
        })) as CliResponses["status"];
      case "cancel":
        return this.engine.cancelRun({
          context: value.context,
          run_id: value.run_id,
        });
    }
  }
  async close(): Promise<void> {
    this.closed = true;
    const active = this.active;
    active?.child.kill("SIGKILL");
    await active?.done.catch(() => {});
  }
}
