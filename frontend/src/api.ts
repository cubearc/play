import { apiFetch, setSessionToken } from "./capability";

export type Health = {
  status: "ok";
  version: string;
  workspace?: { id: string; name: string };
};
export type LaunchLink = {
  session: string | null;
  selection: { name: string; intent: "file" | "challenge" } | null;
};

export class SessionError extends Error {}

export function takeLaunchLink(): LaunchLink {
  const hash = window.location.hash.slice(1);
  if (hash) {
    window.history.replaceState(
      null,
      "",
      window.location.pathname + window.location.search,
    );
  }
  if (hash.length > 16384) return { session: null, selection: null };
  const parameters = new URLSearchParams(hash);
  const name = parameters.get("file");
  const selection =
    name &&
    name.length <= 4096 &&
    name !== "." &&
    name !== ".." &&
    !name.includes("/") &&
    !name.includes("\0")
      ? {
          name,
          intent:
            parameters.get("intent") === "challenge"
              ? ("challenge" as const)
              : ("file" as const),
        }
      : null;
  return { session: parameters.get("session"), selection };
}

export async function connect(
  session: string | null,
  signal: AbortSignal,
): Promise<Health> {
  if (session) {
    const response = await fetch("/api/session", {
      method: "POST",
      credentials: "omit",
      redirect: "error",
      headers: { Authorization: `Bearer ${session}` },
      signal,
    });
    if (!response.ok)
      throw new SessionError("The local session link is missing or expired.");
    setSessionToken(session);
  }
  const response = await apiFetch("/api/health", {
    credentials: "omit",
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
  let workspace: Health["workspace"];
  if ("workspace" in data) {
    const value = data.workspace;
    if (
      typeof value !== "object" ||
      value === null ||
      !("id" in value) ||
      typeof value.id !== "string" ||
      !("name" in value) ||
      typeof value.name !== "string" ||
      !value.name ||
      value.name.length > 4096
    ) {
      throw new Error(
        "The local application returned an unexpected workspace context.",
      );
    }
    workspace = { id: value.id, name: value.name };
  }
  return {
    status: "ok",
    version: data.version,
    ...(workspace ? { workspace } : {}),
  };
}
