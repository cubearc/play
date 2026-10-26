import { afterEach, expect, it, vi } from "vitest";
import { terminalEnd, terminalStatus, terminalUrl } from "./terminal-api";

afterEach(() => vi.unstubAllGlobals());
it("accepts bounded status and uses a same-origin websocket without credentials in its URL", async () => {
  const value = {
    available: true,
    active: 0,
    limit: 2,
    idle_seconds: 300,
    lifetime_seconds: 1800,
    input_limit: 4194304,
    output_limit: 8388608,
  };
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(value)));
  expect(await terminalStatus(new AbortController().signal)).toEqual(value);
  const url = new URL(terminalUrl("source", "空 dir", 80, 24));
  expect(url.origin.replace("ws:", "http:")).toBe(window.location.origin);
  expect(url.searchParams.get("path")).toBe("空 dir");
  expect(url.search).not.toContain("session");
});
it.each([null, [], { available: "yes" }, { available: true, active: -1 }])(
  "rejects invalid status %j",
  async (value) => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(value)));
    await expect(terminalStatus(new AbortController().signal)).rejects.toThrow(
      "Invalid terminal status",
    );
  },
);
it("explains runner failure and all bounded closure reasons", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response(null, { status: 503 })),
  );
  await expect(terminalStatus(new AbortController().signal)).rejects.toThrow(
    "isolated runner",
  );
  for (const reason of [
    "closed",
    "exited",
    "idle_timeout",
    "lifetime_limit",
    "input_limit",
    "input_backpressure",
    "output_limit",
    "shutdown",
    "unknown",
  ])
    expect(terminalEnd(reason).length).toBeGreaterThan(20);
});
