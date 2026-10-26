import { afterEach, expect, it, vi } from "vitest";
import { loadInvestigation, saveInvestigation } from "./investigation-api";

const snapshot = {
  id: "investigation",
  title: "Case",
  revision: 1,
  view_revision: 0,
  interrupted_sessions: 0,
  warnings: [],
  view: { store: "source", path: "", offset: 0, selected: null },
};
afterEach(() => vi.unstubAllGlobals());
it("restores typed local state and saves explicit revisions without external requests", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({
        ...snapshot,
        view: {
          ...snapshot.view,
          selected: { store: "imports", path: "saved.bin" },
        },
      }),
    )
    .mockResolvedValueOnce(Response.json({ revision: 2 }));
  vi.stubGlobal("fetch", fetcher);
  const signal = new AbortController().signal;
  expect((await loadInvestigation(signal)).view.selected).toEqual({
    store: "imports",
    path: "saved.bin",
  });
  expect(await saveInvestigation("", { title: "Case", revision: 1 })).toBe(2);
  expect(fetcher).toHaveBeenNthCalledWith(1, "/api/investigation", {
    credentials: "same-origin",
    signal,
  });
  expect(fetcher).toHaveBeenNthCalledWith(
    2,
    "/api/investigation",
    expect.objectContaining({
      method: "PUT",
      body: '{"title":"Case","revision":1}',
    }),
  );
});
it.each([
  null,
  [],
  { ...snapshot, title: 0 },
  { ...snapshot, title: "x".repeat(4097) },
  { ...snapshot, revision: -1 },
  { ...snapshot, revision: 1.5 },
  { ...snapshot, revision: "1" },
  { ...snapshot, view: { ...snapshot.view, store: "other" } },
  { ...snapshot, warnings: null },
  { ...snapshot, warnings: Array(17).fill("warning") },
  { ...snapshot, warnings: [{}] },
  { ...snapshot, view: null },
])("rejects invalid state responses: %j", async (value) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(value)));
  await expect(
    loadInvestigation(new AbortController().signal),
  ).rejects.toThrow();
});
it.each([409, 503])(
  "retains a useful fallback for non-JSON errors (%s)",
  async (status) => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response("unavailable", { status })),
    );
    await expect(saveInvestigation("/view", {})).rejects.toThrow(
      status === 409 ? "another tab" : "Local state",
    );
  },
);
it("shows a bounded server recovery message and propagates connection errors", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValueOnce(
        Response.json({ message: "Restore the database" }, { status: 503 }),
      )
      .mockRejectedValueOnce(new Error("offline")),
  );
  await expect(loadInvestigation(new AbortController().signal)).rejects.toThrow(
    "Restore the database",
  );
  await expect(saveInvestigation("/view", {})).rejects.toThrow("offline");
});
