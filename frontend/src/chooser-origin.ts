const KEY = "cube-play-chooser-origin";
export function chooserOrigin(value?: string | null): string | null {
  try {
    const candidate = value ?? sessionStorage.getItem(KEY);
    if (!candidate || candidate.length > 128) return null;
    const url = new URL(candidate);
    if (
      url.protocol !== "http:" ||
      url.hostname !== "127.0.0.1" ||
      !url.port ||
      Number(url.port) < 1 ||
      url.username ||
      url.password ||
      url.pathname !== "/" ||
      url.search ||
      url.hash ||
      url.origin === window.location.origin
    )
      return null;
    if (value) sessionStorage.setItem(KEY, url.origin);
    return url.origin;
  } catch {
    return null;
  }
}
