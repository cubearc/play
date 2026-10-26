export type Health = { status: "ok"; version: string };

export class SessionError extends Error {}

export function takeSessionLink(): string | null {
  const session = new URLSearchParams(window.location.hash.slice(1)).get(
    "session",
  );
  if (session !== null) {
    window.history.replaceState(
      null,
      "",
      window.location.pathname + window.location.search,
    );
  }
  return session;
}

export async function connect(
  session: string | null,
  signal: AbortSignal,
): Promise<Health> {
  if (session) {
    const response = await fetch("/api/session", {
      method: "POST",
      credentials: "same-origin",
      headers: { Authorization: `Bearer ${session}` },
      signal,
    });
    if (!response.ok)
      throw new SessionError("The local session link is missing or expired.");
  }
  const response = await fetch("/api/health", {
    credentials: "same-origin",
    signal,
  });
  if (response.status === 401 || response.status === 403) {
    throw new SessionError(
      "Open the session link shown by Cube Play to connect securely.",
    );
  }
  if (!response.ok) throw new Error("The local application is unavailable.");
  const data: unknown = await response.json();
  if (
    typeof data !== "object" ||
    data === null ||
    !("status" in data) ||
    data.status !== "ok" ||
    !("version" in data) ||
    typeof data.version !== "string"
  ) {
    throw new Error("The local application returned an unexpected response.");
  }
  return { status: "ok", version: data.version };
}
