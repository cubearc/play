import { afterEach, expect, it, vi } from "vitest";
import { chooserOrigin } from "./chooser-origin";
import {
  applicationLink,
  bootstrapHost,
  openWorkspace,
  recentWorkspaces,
} from "./launcher-api";

const id = "6d434f94-89f3-4fb8-a7d8-c345a0000001";
const app = `http://127.0.0.1:7331/#session=${id}&file=sample.png&intent=file`;
afterEach(() => {
  vi.unstubAllGlobals();
  sessionStorage.clear();
  window.history.replaceState(null, "", "/");
});
it("consumes the private host fragment, scopes authority to its origin and forwards only a non-secret chooser origin", async () => {
  window.history.replaceState(
    null,
    "",
    `/#session=${id}&open=${encodeURIComponent(app)}`,
  );
  const link = bootstrapHost();
  expect(window.location.hash).toBe("");
  expect(sessionStorage.getItem("cube-play-workspace-host-capability")).toBe(
    id,
  );
  expect(sessionStorage.getItem("cube-play-session-capability")).toBeNull();
  const url = new URL(link ?? "");
  expect(url.origin).toBe("http://127.0.0.1:7331");
  expect(new URLSearchParams(url.hash.slice(1)).get("launcher_origin")).toBe(
    window.location.origin,
  );
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({
        schema_version: 1,
        workspaces: [{ id, name: "Private directory", available: true }],
      }),
    )
    .mockResolvedValueOnce(Response.json({ schema_version: 1, url: app }))
    .mockResolvedValueOnce(
      Response.json({ schema_version: 1, cancelled: true }),
    );
  vi.stubGlobal("fetch", fetcher);
  const signal = new AbortController().signal;
  expect(await recentWorkspaces(signal)).toEqual([
    { id, name: "Private directory", available: true },
  ]);
  expect(await openWorkspace({ intent: "recent", id }, signal)).toBe(link);
  expect(await openWorkspace({ intent: "open" }, signal)).toBeNull();
  for (const [, options] of fetcher.mock.calls) {
    expect(options.credentials).toBe("omit");
    expect(options.redirect).toBe("error");
    expect(options.headers.Authorization).toBe(`Bearer ${id}`);
  }
  expect(JSON.parse(fetcher.mock.calls[1]?.[1].body)).toEqual({
    schema_version: 1,
    intent: "recent",
    id,
  });
});
it.each([
  "https://outside.invalid/",
  `http://outside.invalid:7331/#session=${id}`,
  `http://127.0.0.1:7331/path#session=${id}`,
  `http://127.0.0.1:7331/?token=${id}#session=${id}`,
  "http://127.0.0.1:7331/#session=bad",
])("rejects an unsafe application handoff: %s", (value) =>
  expect(() => applicationLink(value)).toThrow(),
);
it("keeps the application backlink non-secret and rejects external or self origins", () => {
  expect(chooserOrigin("https://outside.invalid")).toBeNull();
  expect(chooserOrigin(window.location.origin)).toBeNull();
  expect(chooserOrigin("http://127.0.0.1:9000/unsafe")).toBeNull();
  expect(chooserOrigin("http://127.0.0.1:9000")).toBe("http://127.0.0.1:9000");
  expect(chooserOrigin()).toBe("http://127.0.0.1:9000");
  expect(sessionStorage.getItem("cube-play-chooser-origin")).not.toContain(id);
});
it("fails clearly on missing authority, bad links, malformed catalogs and server or network failures", async () => {
  const signal = new AbortController().signal;
  await expect(recentWorkspaces(signal)).rejects.toThrow("authorize");
  window.history.replaceState(null, "", `/#session=invalid`);
  expect(() => bootstrapHost()).toThrow("invalid");
  sessionStorage.setItem("cube-play-workspace-host-capability", id);
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(Response.json({ schema_version: 2 }))
    .mockResolvedValueOnce(
      Response.json({
        schema_version: 1,
        workspaces: [{ id, name: "Bad", available: "yes" }],
      }),
    )
    .mockResolvedValueOnce(
      Response.json(
        {
          schema_version: 1,
          error: { message: "Source directory unavailable" },
        },
        { status: 409 },
      ),
    )
    .mockResolvedValueOnce(new Response("not JSON", { status: 503 }))
    .mockRejectedValueOnce(new Error("offline"));
  vi.stubGlobal("fetch", fetcher);
  await expect(recentWorkspaces(signal)).rejects.toThrow(
    "unsupported protocol",
  );
  await expect(recentWorkspaces(signal)).rejects.toThrow(
    "Invalid recent workspace",
  );
  await expect(openWorkspace({ intent: "open" }, signal)).rejects.toThrow(
    "Source directory unavailable",
  );
  await expect(recentWorkspaces(signal)).rejects.toThrow("unreadable");
  await expect(recentWorkspaces(signal)).rejects.toThrow("Could not reach");
});
