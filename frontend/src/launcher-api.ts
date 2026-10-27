const KEY = "cube-play-workspace-host-capability";
const uuid =
  /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
export type RecentWorkspace = { id: string; name: string; available: boolean };
export function applicationLink(
  value: unknown,
  origin = window.location.origin,
): string {
  if (typeof value !== "string" || value.length > 16384)
    throw new Error("Invalid local workspace link.");
  const url = new URL(value);
  const params = new URLSearchParams(url.hash.slice(1));
  if (
    url.protocol !== "http:" ||
    url.hostname !== "127.0.0.1" ||
    !url.port ||
    Number(url.port) === 0 ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.origin === origin ||
    !uuid.test(params.get("session") ?? "") ||
    params.getAll("session").length !== 1
  )
    throw new Error("Invalid local workspace link.");
  params.set("launcher_origin", origin);
  url.hash = params.toString();
  return url.toString();
}
export function bootstrapHost(): string | null {
  const hash = window.location.hash.slice(1);
  window.history.replaceState(null, "", window.location.pathname);
  if (hash.length > 32768)
    throw new Error("The private launcher link exceeds its limit.");
  const values = new URLSearchParams(hash),
    token = values.get("session");
  if (values.has("session")) {
    if (!uuid.test(token ?? "") || values.getAll("session").length !== 1)
      throw new Error("The private launcher link is invalid.");
    sessionStorage.setItem(KEY, token ?? "");
  }
  if (values.getAll("open").length > 1)
    throw new Error("The private launcher link is invalid.");
  return values.has("open") ? applicationLink(values.get("open")) : null;
}
async function request(
  path: string,
  signal: AbortSignal,
  body?: unknown,
): Promise<Record<string, unknown>> {
  const token = sessionStorage.getItem(KEY);
  if (!token)
    throw new Error(
      "Open the private workspace chooser link shown by cube play to authorize this tab.",
    );
  let response: Response;
  try {
    response = await fetch(path, {
      method: body === undefined ? "GET" : "POST",
      credentials: "omit",
      redirect: "error",
      signal,
      headers: {
        Authorization: `Bearer ${token}`,
        ...(body === undefined ? {} : { "Content-Type": "application/json" }),
      },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
  } catch {
    throw new Error(
      "Could not reach the workspace chooser. Run cube play again.",
    );
  }
  let value: unknown;
  try {
    value = await response.json();
  } catch {
    throw new Error(
      "The workspace chooser returned an unreadable response. Run cube play again.",
    );
  }
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("The workspace chooser returned an invalid response.");
  const v = value as Record<string, unknown>;
  if (!response.ok) {
    const e = v.error as { message?: unknown } | undefined;
    throw new Error(
      typeof e?.message === "string" && e.message.length <= 1024
        ? e.message
        : "The workspace chooser is unavailable. Run cube play again.",
    );
  }
  if (v.schema_version !== 1)
    throw new Error("The workspace chooser uses an unsupported protocol.");
  return v;
}
export async function recentWorkspaces(
  signal: AbortSignal,
): Promise<RecentWorkspace[]> {
  const v = await request("/api/workspaces", signal);
  if (!Array.isArray(v.workspaces) || v.workspaces.length > 256)
    throw new Error("Invalid recent workspaces.");
  const ids = new Set<string>();
  return v.workspaces.map((raw: unknown) => {
    const item = raw as RecentWorkspace;
    if (
      !item ||
      typeof item !== "object" ||
      typeof item.id !== "string" ||
      !uuid.test(item.id) ||
      ids.has(item.id) ||
      typeof item.name !== "string" ||
      !item.name ||
      item.name.length > 4096 ||
      typeof item.available !== "boolean"
    )
      throw new Error("Invalid recent workspace.");
    ids.add(item.id);
    return { id: item.id, name: item.name, available: item.available };
  });
}
export type WorkspaceIntent =
  | { intent: "open" }
  | { intent: "new"; name: string }
  | { intent: "recent"; id: string };
export async function openWorkspace(
  intent: WorkspaceIntent,
  signal: AbortSignal,
): Promise<string | null> {
  const v = await request("/api/intents", signal, {
    schema_version: 1,
    ...intent,
  });
  if (v.cancelled === true) return null;
  return applicationLink(v.url);
}
