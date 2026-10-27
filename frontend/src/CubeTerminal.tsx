import type { Terminal } from "@xterm/xterm";
import {
  type FormEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { terminalProtocols } from "./capability";
import { type OperationResult, openTypedResult } from "./exploration-api";
import { TypedOutputs } from "./TypedOutputs";
import {
  type TerminalStatus,
  type TerminalStore,
  terminalEnd,
  terminalStatus,
  terminalUrl,
} from "./terminal-api";
import "@xterm/xterm/css/xterm.css";

type Request = { id: number; store: TerminalStore; path: string };
export function CubeTerminal() {
  const [status, setStatus] = useState<TerminalStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const controller = useRef<AbortController | null>(null);
  const [store, setStore] = useState<TerminalStore>("source");
  const [path, setPath] = useState("");
  const [request, setRequest] = useState<Request | null>(null);
  const checkRunner = useCallback(() => {
    controller.current?.abort();
    const request = new AbortController();
    controller.current = request;
    setStatus(null);
    setError(null);
    terminalStatus(request.signal).then(
      (value) => {
        if (!request.signal.aborted) setStatus(value);
      },
      (error: unknown) => {
        if (!request.signal.aborted)
          setError(
            error instanceof Error
              ? error.message
              : "Terminal status is unavailable.",
          );
      },
    );
  }, []);
  useEffect(() => {
    checkRunner();
    return () => controller.current?.abort();
  }, [checkRunner]);
  function start(event: FormEvent) {
    event.preventDefault();
    setRequest({ id: Date.now(), store, path });
  }
  return (
    <section className="terminal-panel" aria-labelledby="terminal-title">
      <div className="terminal-heading">
        <div>
          <p className="eyebrow">ISOLATED CUBE RUNNER</p>
          <h2 id="terminal-title">Cube terminal</h2>
        </div>
        <span>Private · source protected</span>
      </div>
      <p>
        Execute real Cube commands. Outputs stay local; this session is not
        saved. Disconnecting ends its processes.
      </p>
      {error ? (
        <p role="alert">{error}</p>
      ) : !status ? (
        <p role="status">Checking runner…</p>
      ) : !status.available ? (
        <p>
          Open this workspace with cube play to use the isolated terminal.
          Native development does not execute host commands.
        </p>
      ) : (
        <>
          {!request && (
            <form className="terminal-controls" onSubmit={start}>
              <label>
                Working directory
                <select
                  value={store}
                  onChange={(event) =>
                    setStore(event.target.value as TerminalStore)
                  }
                >
                  <option value="source">Source · read-only</option>
                  <option value="imports">Imports · read-only</option>
                  <option value="output">Output · writable</option>
                </select>
              </label>
              <label>
                Relative directory
                <input
                  value={path}
                  onChange={(event) => setPath(event.target.value)}
                  maxLength={4096}
                  placeholder="Workspace root"
                />
              </label>
              <button className="button" type="submit">
                Start terminal
              </button>
            </form>
          )}
          <p className="terminal-limits">
            Up to {status.limit} sessions · {status.idle_seconds / 60} minute
            idle limit · {status.lifetime_seconds / 60} minute maximum ·{" "}
            {status.output_limit / (1024 * 1024)} MiB output
          </p>
        </>
      )}
      {!request && (
        <button
          className="button secondary"
          type="button"
          onClick={checkRunner}
        >
          Refresh runner
        </button>
      )}
      {request && (
        <TerminalSession
          key={request.id}
          request={request}
          close={() => setRequest(null)}
        />
      )}
    </section>
  );
}
function TerminalSession({
  request,
  close,
}: {
  request: Request;
  close: () => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const socket = useRef<WebSocket | null>(null);
  const [message, setMessage] = useState("Starting isolated terminal…");
  const [connected, setConnected] = useState(false);
  const [ended, setEnded] = useState(false);
  const [size, setSize] = useState("");
  const [captures, setCaptures] = useState<string[]>([]);
  const [publishing, setPublishing] = useState(false);
  const [published, setPublished] = useState<OperationResult | null>(null);
  const [captureMessage, setCaptureMessage] = useState("");
  const reading = useRef<AbortController | null>(null);
  useEffect(() => () => reading.current?.abort(), []);
  useEffect(() => {
    let active = true;
    let cleanup = () => {};
    void (async () => {
      try {
        const [{ Terminal }, { FitAddon }] = await Promise.all([
          import("@xterm/xterm"),
          import("@xterm/addon-fit"),
        ]);
        if (!active || !container.current) return;
        const term = new Terminal({
          cols: 80,
          rows: 18,
          scrollback: 1000,
          screenReaderMode: true,
          allowProposedApi: false,
          fontSize: 14,
          fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
          theme: {
            background: "#14251e",
            foreground: "#eef5ef",
            cursor: "#b8e4ca",
          },
        });
        const fit = new FitAddon();
        term.loadAddon(fit);
        term.open(container.current);
        term.textarea?.setAttribute("aria-label", "Cube terminal input");
        // Terminal output cannot write the clipboard or open links through addons.
        const clipboard = term.parser.registerOscHandler(52, () => true);
        const links = term.parser.registerOscHandler(8, () => true);
        const resize = () => {
          const proposed = fit.proposeDimensions();
          const cols = Math.max(20, Math.min(240, proposed?.cols ?? term.cols));
          const rows = Math.max(5, Math.min(80, proposed?.rows ?? term.rows));
          if (cols !== term.cols || rows !== term.rows) term.resize(cols, rows);
        };
        resize();
        const ws = new WebSocket(
          terminalUrl(request.store, request.path, term.cols, term.rows),
          terminalProtocols(),
        );
        ws.binaryType = "arraybuffer";
        socket.current = ws;
        terminal.current = term;
        let pending = 0;
        const write = (data: Uint8Array) => {
          pending += data.byteLength;
          if (pending > 256 * 1024) {
            ws.close();
            setMessage(
              "Terminal rendering could not keep up. The session was stopped.",
            );
            return;
          }
          term.write(data, () => {
            pending -= data.byteLength;
          });
        };
        const input = term.onData((data) => {
          if (ws.readyState !== WebSocket.OPEN) return;
          // Bound buffered input; UTF-8 byte lengths are checked again by both Rust boundaries.
          if (
            ws.bufferedAmount > 32768 ||
            new TextEncoder().encode(data).byteLength > 4096
          ) {
            setMessage(
              "Input is too large or waiting to be sent. Paste smaller portions.",
            );
            return;
          }
          ws.send(JSON.stringify({ type: "input", data }));
        });
        const sizing = term.onResize((size) => {
          if (ws.readyState === WebSocket.OPEN)
            ws.send(
              JSON.stringify({
                type: "resize",
                size: { cols: size.cols, rows: size.rows },
              }),
            );
        });
        const observer = new ResizeObserver(resize);
        observer.observe(container.current);
        ws.onmessage = (event) => {
          if (!active) return;
          if (event.data instanceof ArrayBuffer) {
            write(new Uint8Array(event.data));
            return;
          }
          try {
            const value: unknown = JSON.parse(String(event.data));
            if (!value || typeof value !== "object" || !("type" in value))
              throw new Error("Invalid terminal message");
            if (value.type === "ready" || value.type === "resized") {
              if (
                !("size" in value) ||
                !value.size ||
                typeof value.size !== "object" ||
                !("cols" in value.size) ||
                !("rows" in value.size) ||
                typeof value.size.cols !== "number" ||
                typeof value.size.rows !== "number" ||
                !Number.isInteger(value.size.cols) ||
                !Number.isInteger(value.size.rows) ||
                value.size.cols < 20 ||
                value.size.cols > 240 ||
                value.size.rows < 5 ||
                value.size.rows > 80
              )
                throw new Error("Invalid terminal size");
              setSize(`${value.size.cols} × ${value.size.rows}`);
              if (value.type === "ready") {
                setConnected(true);
                setMessage("Connected to Cube Runner");
                term.focus();
              }
            } else if (
              value.type === "captures" &&
              "ids" in value &&
              Array.isArray(value.ids) &&
              value.ids.length <= 16 &&
              value.ids.every(
                (id) =>
                  typeof id === "string" && /^[A-Za-z0-9_-]{1,96}$/.test(id),
              )
            ) {
              setCaptures(value.ids);
              setCaptureMessage(
                value.ids.length
                  ? "Choose a capture to publish explicitly."
                  : "No staged captures. Run a supported command with --lab-capture first.",
              );
            } else if (
              value.type === "publication_error" &&
              "message" in value &&
              typeof value.message === "string"
            ) {
              setPublishing(false);
              setCaptureMessage(value.message);
            } else if (
              value.type === "published" &&
              "id" in value &&
              typeof value.id === "string"
            ) {
              reading.current?.abort();
              const controller = new AbortController();
              reading.current = controller;
              void openTypedResult(value.id, controller.signal)
                .then((result) => {
                  if (active && !controller.signal.aborted) {
                    setPublished(result);
                    setCaptureMessage(
                      "Published locally without rerunning. Original inputs are unavailable; provenance is an unverified CLI declaration.",
                    );
                  }
                })
                .catch((error) => {
                  if (active && !controller.signal.aborted)
                    setCaptureMessage(
                      error instanceof Error
                        ? error.message
                        : "Reopen the retained artifact to inspect it.",
                    );
                })
                .finally(() => {
                  if (active && !controller.signal.aborted)
                    setPublishing(false);
                });
            } else if (value.type === "end") {
              setMessage(terminalEnd("reason" in value ? value.reason : null));
              setConnected(false);
              setEnded(true);
            } else throw new Error("Terminal protocol error");
          } catch {
            setMessage(
              "The terminal returned an invalid response. Its session was closed.",
            );
            ws.close();
          }
        };
        ws.onerror = () => {
          if (active)
            setMessage(
              "The runner could not open this terminal. Check its availability, session limit and working directory.",
            );
        };
        ws.onclose = () => {
          if (active) {
            setConnected(false);
            setEnded(true);
            setPublishing(false);
            setMessage((previous) =>
              previous === "Connected to Cube Runner" ||
              previous === "Starting isolated terminal…"
                ? terminalEnd(null)
                : previous,
            );
          }
        };
        cleanup = () => {
          observer.disconnect();
          input.dispose();
          sizing.dispose();
          clipboard.dispose();
          links.dispose();
          ws.onmessage = null;
          ws.onerror = null;
          ws.onclose = null;
          ws.close();
          term.dispose();
          terminal.current = null;
          socket.current = null;
        };
      } catch {
        socket.current?.close();
        terminal.current?.dispose();
        if (active) {
          setEnded(true);
          setMessage(
            "The terminal could not be initialized. Close it and try again.",
          );
        }
      }
    })();
    return () => {
      active = false;
      cleanup();
    };
  }, [request]);
  return (
    <div className="terminal-session">
      <div className="terminal-session-bar">
        <p role="status">{message}</p>
        <span>Terminal size: {size || "measuring…"}</span>
        <button className="button secondary" type="button" onClick={close}>
          {ended ? "Close terminal" : "End session"}
        </button>
        <button
          className="button secondary"
          type="button"
          disabled={!connected}
          onClick={() => terminal.current?.focus()}
        >
          Focus terminal
        </button>
      </div>
      <section
        className="terminal-screen"
        ref={container}
        aria-label="Cube terminal output"
      />
      <section aria-label="Terminal to Lab">
        <h3>Send a result to the Lab</h3>
        <p>
          Use <code>cube --lab-capture text reverse --text CUBE</code> to stage
          a supported result privately. Captures are limited to 128 KiB and
          discarded when this session ends. Terminal history is never recorded.
        </p>
        <button
          type="button"
          className="button secondary"
          disabled={!connected || publishing}
          onClick={() =>
            socket.current?.send(JSON.stringify({ type: "capture_list" }))
          }
        >
          Refresh staged captures
        </button>
        <ul>
          {captures.map((id) => (
            <li key={id}>
              <code>{id}</code>
              <button
                type="button"
                className="button secondary"
                disabled={!connected || publishing}
                onClick={() => {
                  setPublishing(true);
                  setCaptureMessage("Publishing selected capture…");
                  socket.current?.send(
                    JSON.stringify({ type: "capture_read", id }),
                  );
                }}
              >
                Send to Lab
              </button>
            </li>
          ))}
        </ul>
        {captureMessage && <p role="status">{captureMessage}</p>}
        {published && <TypedOutputs key={published.id} result={published} />}
      </section>
      <p className="terminal-help">
        Try <code>cube encode base64 hello</code> or <code>cube --help</code>.
        Use Ctrl+C to interrupt a command and Ctrl+D at an empty prompt to exit.
        Shell scripts and pipes are unavailable.
      </p>
    </div>
  );
}
