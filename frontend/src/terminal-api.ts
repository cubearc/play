import { apiFetch } from "./capability";
export type TerminalStore = "source" | "imports" | "output";
export type TerminalStatus = {
  available: boolean;
  active: number;
  limit: number;
  idle_seconds: number;
  lifetime_seconds: number;
  input_limit: number;
  output_limit: number;
};
export async function terminalStatus(
  signal: AbortSignal,
): Promise<TerminalStatus> {
  const response = await apiFetch("/api/terminal", {
    signal,
    credentials: "omit",
  });
  if (!response.ok)
    throw new Error(
      "The isolated runner is unavailable. Reopen this workspace through cube play.",
    );
  const value: unknown = await response.json();
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("Invalid terminal status response.");
  const record = value as Record<string, unknown>;
  if (
    typeof record.available !== "boolean" ||
    [
      "active",
      "limit",
      "idle_seconds",
      "lifetime_seconds",
      "input_limit",
      "output_limit",
    ].some(
      (key) =>
        typeof record[key] !== "number" ||
        !Number.isSafeInteger(record[key]) ||
        Number(record[key]) < 0,
    )
  )
    throw new Error("Invalid terminal status response.");
  return record as TerminalStatus;
}
export function terminalUrl(
  store: TerminalStore,
  path: string,
  cols: number,
  rows: number,
): string {
  const url = new URL("/api/terminal/ws", window.location.href);
  url.protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  url.search = new URLSearchParams({
    store,
    path,
    cols: String(cols),
    rows: String(rows),
  }).toString();
  return url.toString();
}
export function terminalEnd(reason: unknown): string {
  switch (reason) {
    case "closed":
    case "exited":
      return "Session ended. Start a new terminal to continue.";
    case "idle_timeout":
      return "Session closed after five minutes without input.";
    case "lifetime_limit":
      return "Session reached its 30-minute lifetime limit.";
    case "input_limit":
    case "input_backpressure":
      return "Terminal input exceeded its rate or buffering limit. Start a new session.";
    case "output_limit":
      return "Session stopped after reaching its 8 MiB output limit.";
    case "shutdown":
      return "The local runner stopped. Reopen the workspace to continue.";
    default:
      return "Terminal connection ended. Its processes are terminated; start a new session to continue.";
  }
}
