// sessionStorage is scoped by scheme, host and port. Cookies are not port-scoped.
const KEY = "cube-play-session-capability";
let current: string | null = null;
export function sessionToken(): string | null {
  if (current) return current;
  try {
    return window.sessionStorage.getItem(KEY);
  } catch {
    return null;
  }
}
export function setSessionToken(value: string | null): void {
  current = value;
  try {
    if (value) window.sessionStorage.setItem(KEY, value);
    else window.sessionStorage.removeItem(KEY);
  } catch {
    // Memory-only sessions require the launcher link again after a reload.
  }
}
export function apiFetch(
  path: string,
  options: RequestInit = {},
): Promise<Response> {
  if (!path.startsWith("/api/"))
    throw new Error("Use an authorized same-origin API path.");
  const headers = new Headers(options.headers);
  const token = sessionToken();
  if (token) headers.set("Authorization", `Bearer ${token}`);
  return fetch(path, {
    ...options,
    headers,
    credentials: "omit",
    redirect: "error",
  });
}
export function terminalProtocols(): string[] {
  const token = sessionToken();
  return token ? ["cube-play.v1", `cube-play.auth.${token}`] : [];
}
export async function readLink(
  uri: string,
  signal: AbortSignal,
): Promise<string> {
  const response = await apiFetch("/api/read-links", {
    method: "POST",
    signal,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ uri }),
  });
  const value: unknown = await response.json();
  if (!response.ok) {
    const message =
      value &&
      typeof value === "object" &&
      "message" in value &&
      typeof value.message === "string" &&
      value.message.length <= 512
        ? value.message
        : "The private media link is unavailable. Reopen the preview to renew it.";
    throw new Error(message);
  }
  if (
    !value ||
    typeof value !== "object" ||
    !("url" in value) ||
    typeof value.url !== "string" ||
    !("expires_in" in value) ||
    typeof value.expires_in !== "number" ||
    !Number.isSafeInteger(value.expires_in) ||
    value.expires_in < 1 ||
    value.expires_in > 3600
  )
    throw new Error("Invalid private media link.");
  const prefix = `${uri}${uri.includes("?") ? "&" : "?"}read_cap=`;
  if (
    !value.url.startsWith(prefix) ||
    !/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(
      value.url.slice(prefix.length),
    )
  )
    throw new Error("Invalid private media link scope.");
  return value.url;
}
