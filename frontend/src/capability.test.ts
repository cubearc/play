import { afterEach, expect, it, vi } from "vitest";
import {
  apiFetch,
  readLink,
  sessionToken,
  setSessionToken,
  terminalProtocols,
} from "./capability";

vi.unmock("./capability");
afterEach(() => vi.unstubAllGlobals());
it("keeps API authority in headers and origin-scoped storage, never cookies or URLs", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(new Response(null, { status: 204 }));
  vi.stubGlobal("fetch", fetcher);
  setSessionToken("private-capability");
  expect(sessionToken()).toBe("private-capability");
  expect(window.sessionStorage.getItem("cube-play-session-capability")).toBe(
    "private-capability",
  );
  await apiFetch("/api/health");
  const [url, options] = fetcher.mock.calls[0] ?? [];
  expect(url).toBe("/api/health");
  expect(options.credentials).toBe("omit");
  expect(options.headers.get("Authorization")).toBe(
    "Bearer private-capability",
  );
  expect(terminalProtocols()).toEqual([
    "cube-play.v1",
    "cube-play.auth.private-capability",
  ]);
  expect(() => apiFetch("https://outside.invalid/api/health")).toThrow(
    "same-origin",
  );
  setSessionToken(null);
  expect(terminalProtocols()).toEqual([]);
});
it("requests only one bounded read URL and rejects mismatched scopes and server errors", async () => {
  const uri = "/api/workspace/media?store=source&path=sample.mp4&offset=0";
  const url = `${uri}&read_cap=6d434f94-89f3-4fb8-a7d8-c345a0000001`;
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(Response.json({ url, expires_in: 3600 }))
    .mockResolvedValueOnce(
      Response.json({ url: "https://outside.invalid/", expires_in: 3600 }),
    )
    .mockResolvedValueOnce(Response.json({ url: "wrong", expires_in: 3600 }))
    .mockResolvedValueOnce(Response.json({ error: "quota" }, { status: 429 }));
  vi.stubGlobal("fetch", fetcher);
  const signal = new AbortController().signal;
  expect(await readLink(uri, signal)).toBe(url);
  expect(JSON.parse(fetcher.mock.calls[0]?.[1].body)).toEqual({ uri });
  await expect(readLink(uri, signal)).rejects.toThrow("scope");
  await expect(readLink(uri, signal)).rejects.toThrow("scope");
  await expect(readLink(uri, signal)).rejects.toThrow("unavailable");
});
