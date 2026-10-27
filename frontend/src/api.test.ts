import { afterEach, describe, expect, it, vi } from "vitest";
import { connect, SessionError, takeLaunchLink } from "./api";
import { setSessionToken } from "./capability";

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

describe("local connection", () => {
  it("takes a private file or challenge selection from the fragment", () => {
    window.history.replaceState(
      null,
      "",
      "/#session=capability&file=rabbit.cube&intent=challenge",
    );
    expect(takeLaunchLink()).toEqual({
      session: "capability",
      selection: { name: "rabbit.cube", intent: "challenge" },
    });
    window.history.replaceState(null, "", "/#file=sample+%E7%A9%BA.mp4");
    expect(takeLaunchLink().selection).toEqual({
      name: "sample 空.mp4",
      intent: "file",
    });
    expect(window.location.hash).toBe("");
  });
  it.each(["..", ".", "../private", "a/b", "bad\0name", "x".repeat(4097)])(
    "ignores unsafe or excessive selection hints %#",
    (name) => {
      window.history.replaceState(
        null,
        "",
        `/#file=${encodeURIComponent(name)}`,
      );
      expect(takeLaunchLink().selection).toBeNull();
    },
  );
  it("bounds the complete startup fragment", () => {
    window.history.replaceState(null, "", `/#session=${"x".repeat(16385)}`);
    expect(takeLaunchLink()).toEqual({ session: null, selection: null });
  });
  it("accepts real workspace metadata in authenticated health", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        Response.json({
          status: "ok",
          version: "0.1.0",
          workspace: { id: "local-id", name: "Investigation" },
        }),
      ),
    );
    expect(
      (await connect(null, new AbortController().signal)).workspace,
    ).toEqual({ id: "local-id", name: "Investigation" });
  });
  it.each([
    null,
    "workspace",
    {},
    { id: 1, name: "name" },
    { id: "id", name: 1 },
    { id: "id", name: "" },
    { id: "id", name: "x".repeat(4097) },
  ])("rejects invalid workspace metadata %#", async (workspace) => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          Response.json({ status: "ok", version: "0.1.0", workspace }),
        ),
    );
    await expect(connect(null, new AbortController().signal)).rejects.toThrow(
      "unexpected workspace",
    );
  });
  it("removes session credentials from the URL before any request", () => {
    window.history.replaceState(
      null,
      "",
      "/?view=lab#session=private-capability",
    );
    expect(takeLaunchLink()).toEqual({
      session: "private-capability",
      selection: null,
    });
    expect(window.location.hash).toBe("");
    expect(window.location.search).toBe("?view=lab");
    expect(takeLaunchLink()).toEqual({ session: null, selection: null });
  });
  it("exchanges the capability then fetches authenticated health", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(Response.json({ status: "ok", version: "0.1.0" }));
    vi.stubGlobal("fetch", fetcher);
    const signal = new AbortController().signal;
    expect(await connect("capability", signal)).toEqual({
      status: "ok",
      version: "0.1.0",
    });
    expect(fetcher).toHaveBeenNthCalledWith(1, "/api/session", {
      method: "POST",
      credentials: "omit",
      redirect: "error",
      headers: { Authorization: "Bearer capability" },
      signal,
    });
    expect(fetcher.mock.calls[1]?.[0]).toBe("/api/health");
    expect(fetcher.mock.calls[1]?.[1]).toMatchObject({
      credentials: "omit",
      signal,
    });
    expect(fetcher.mock.calls[1]?.[1].headers.get("Authorization")).toBe(
      "Bearer capability",
    );
  });
  it("uses the origin-scoped session without a capability in the URL", async () => {
    setSessionToken("remembered");
    const fetcher = vi
      .fn()
      .mockResolvedValue(Response.json({ status: "ok", version: "0.1.0" }));
    vi.stubGlobal("fetch", fetcher);
    await connect(null, new AbortController().signal);
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
  it("reports expired bootstrap credentials", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(null, { status: 401 })),
    );
    await expect(
      connect("expired", new AbortController().signal),
    ).rejects.toBeInstanceOf(SessionError);
  });
  it.each([401, 403])("reports session denial (%i)", async (status) => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(null, { status })),
    );
    await expect(
      connect(null, new AbortController().signal),
    ).rejects.toBeInstanceOf(SessionError);
  });
  it("rejects server failures", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(null, { status: 500 })),
    );
    await expect(connect(null, new AbortController().signal)).rejects.toThrow(
      "unavailable",
    );
  });
  it.each([
    null,
    {},
    "ok",
    { status: "failed", version: "0.1.0" },
    { status: "ok", version: 1 },
  ])("rejects malformed health %#", async (data) => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(data)));
    await expect(connect(null, new AbortController().signal)).rejects.toThrow(
      "unexpected response",
    );
  });
});
