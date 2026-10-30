import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useEffect, useRef, useState } from "react";
import type {
  TerminalIntent,
  TerminalView,
} from "../desktop/terminal-contract";
import type { Context, RunRequest } from "../desktop/workspace-contract";
import VscodeButton from "./VscodeButton";

// Serialize open/close across remounts and rapid context switches.
let lifecycle = Promise.resolve();
export function ContextTerminal({
  context,
  selection,
  title,
  assetCommand,
  onClose,
  onReconnect,
  onRun,
}: {
  context: Context;
  selection: RunRequest | null;
  title: string;
  assetCommand: string | null;
  onClose: () => void;
  onReconnect: () => void;
  onRun: (trailId: string) => void;
}) {
  const binding = useRef({ context, selection, title, assetCommand }).current;
  const output = useRef<HTMLDivElement>(null);
  const insert = useRef<(value: string) => void>(() => {});
  const changed = useRef(onRun);
  changed.current = onRun;
  const [status, setStatus] = useState("Connecting…");
  const [ready, setReady] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!output.current) return;
    const nonce = document.querySelector<HTMLMetaElement>(
      'meta[name="play-style-nonce"]',
    )?.content;
    const style = getComputedStyle(document.documentElement);
    const terminal = new Terminal({
      disableStdin: true,
      screenReaderMode: true,
      scrollback: 500,
      fontSize: 13,
      fontFamily: style.getPropertyValue("--vscode-editor-font-family"),
      theme: {
        background: style.getPropertyValue("--vscode-panel-background").trim(),
        foreground: style.getPropertyValue("--vscode-foreground").trim(),
        cursor: style.getPropertyValue("--vscode-foreground").trim(),
      },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    // xterm creates its styles synchronously during open, including one through
    // the global document. Authorize only that call, then restore the factory.
    const createElement = document.createElement;
    document.createElement = new Proxy(createElement, {
      apply(factory, receiver, args) {
        const element: HTMLElement = Reflect.apply(factory, receiver, args);
        if (args[0] === "style" && nonce) element.nonce = nonce;
        return element;
      },
    });
    try {
      terminal.open(output.current);
    } finally {
      document.createElement = createElement;
    }

    let alive = true,
      sessionId: string | undefined,
      queue = Promise.resolve();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const dimensions = () => ({
      columns: Math.max(20, Math.min(300, terminal.cols)),
      rows: Math.max(4, Math.min(100, terminal.rows)),
    });
    const view = (value: TerminalView) => {
      if (!alive) return;
      if (value.output)
        terminal.write(
          Uint8Array.from(atob(value.output), (c) => c.charCodeAt(0)),
        );
      if (value.run_id && value.trail_id) changed.current(value.trail_id);
      if (value.state === "closed") {
        terminal.options.disableStdin = true;
        setReady(false);
        setStatus(
          value.code === "closed"
            ? "Closed"
            : `Closed: ${value.code ?? "connection lost"}`,
        );
      }
    };
    const send = (intent: TerminalIntent) =>
      window.play.terminal(intent).then((value) => {
        view(value);
        return value;
      });
    const fail = () => {
      if (alive) {
        setError(
          "Terminal request was not confirmed. Reconnect to continue; saved Runs remain in history.",
        );
        setReady(false);
        terminal.options.disableStdin = true;
      }
    };
    const write = (input: string) => {
      if (!alive || !sessionId || terminal.options.disableStdin) return;
      if (new TextEncoder().encode(input).length > 1024) {
        setError("Paste at most 1,024 bytes at a time.");
        return;
      }
      const id = sessionId;
      queue = queue
        .then(() =>
          send({
            kind: "write",
            context: binding.context,
            session_id: id,
            input,
          }),
        )
        .then(() => {})
        .catch(fail);
    };
    insert.current = (text) => {
      write(`\x03${text}`);
      terminal.focus();
    };
    const data = terminal.onData(write);
    const poll = async () => {
      if (!alive || !sessionId) return;
      try {
        const result = await send({
          kind: "read",
          context: binding.context,
          session_id: sessionId,
        });
        if (alive && result.state === "open")
          timer = setTimeout(() => void poll(), 100);
      } catch {
        fail();
      }
    };
    const observer = new ResizeObserver(() => {
      fit.fit();
      if (alive && sessionId) {
        const id = sessionId,
          size = dimensions();
        queue = queue
          .then(() =>
            send({
              kind: "resize",
              context: binding.context,
              session_id: id,
              ...size,
            }),
          )
          .then(() => {})
          .catch(fail);
      }
    });
    observer.observe(output.current);
    lifecycle = lifecycle
      .then(async () => {
        fit.fit();
        const opened = await window.play.terminal({
          kind: "open",
          context: binding.context,
          selection: binding.selection,
          ...dimensions(),
        });
        sessionId = opened.id;
        if (!alive) return;
        view(opened);
        setStatus("Connected");
        setReady(true);
        terminal.options.disableStdin = false;
        terminal.focus();
        void poll();
      })
      .catch(() => {
        if (alive) {
          setStatus("Unavailable");
          setError(
            "A native terminal could not be opened. Close another session or reconnect. This preview requires macOS.",
          );
        }
      });
    return () => {
      alive = false;
      clearTimeout(timer);
      observer.disconnect();
      data.dispose();
      terminal.dispose();
      lifecycle = lifecycle
        .then(async () => {
          await queue;
          if (sessionId)
            await window.play.terminal({
              kind: "close",
              context: binding.context,
              session_id: sessionId,
            });
        })
        .catch(() => {});
    };
  }, [binding]);
  return (
    <section className="context-terminal" aria-label="Cube terminal">
      <div className="terminal-toolbar">
        <strong>Terminal</strong>
        <span className="terminal-context" title={binding.title}>
          {binding.title}
        </span>
        <span role="status">{status}</span>
        <VscodeButton
          secondary
          disabled={!ready || !binding.selection}
          onClick={() => insert.current("selected")}
        >
          Insert selected Run
        </VscodeButton>
        <VscodeButton
          secondary
          disabled={!ready || !binding.assetCommand}
          onClick={() =>
            binding.assetCommand && insert.current(binding.assetCommand)
          }
        >
          Insert Asset
        </VscodeButton>
        <VscodeButton secondary onClick={onReconnect}>
          Reconnect
        </VscodeButton>
        <VscodeButton secondary icon="close" onClick={onClose}>
          Close terminal
        </VscodeButton>
      </div>
      {error ? (
        <p role="alert" className="terminal-error">
          {error}
        </p>
      ) : null}
      <div className="terminal-surface" ref={output} />
    </section>
  );
}
