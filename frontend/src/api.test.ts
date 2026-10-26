import { afterEach, describe, expect, it, vi } from "vitest";
import { connect, SessionError, takeSessionLink } from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

describe("local connection", () => {
  it("removes session credentials from the URL before any request", () => {
    window.history.replaceState(
      null,
      "",
      "/?view=lab#session=private-capability",
    );
    expect(takeSessionLink()).toBe("private-capability");
    expect(window.location.hash).toBe("");
    expect(window.location.search).toBe("?view=lab");
    expect(takeSessionLink()).toBeNull();
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
      credentials: "same-origin",
      headers: { Authorization: "Bearer capability" },
      signal,
    });
    expect(fetcher).toHaveBeenNthCalledWith(2, "/api/health", {
      credentials: "same-origin",
      signal,
    });
  });
  it("uses the existing cookie without a capability in the URL", async () => {
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
