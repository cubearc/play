import { afterEach, expect, it, vi } from "vitest";
import { notes } from "./notebook-api";
import { id } from "./test/lab-fixture";

afterEach(() => vi.unstubAllGlobals());
it("preserves file and exact execution/output note references without inferring a newer run", async () => {
  const items = [
    { store: "source", path: "original.txt" },
    { execution_id: id, output_port: "output" },
    { execution_id: id, output_port: null },
    null,
  ].map((target, index) => ({
    id: `note-${index}`,
    revision: 1,
    title: "Saved note",
    body: "Private",
    target,
  }));
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(items)));
  expect(await notes(new AbortController().signal)).toEqual(items);
});
it.each([
  { execution_id: "../outside", output_port: null },
  { execution_id: id, output_port: "" },
  { execution_id: id, output_port: 7 },
  { execution_id: id, output_port: "x".repeat(129) },
  { store: "outside", path: "source" },
])("rejects incompatible note references %#", async (target) => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(
        Response.json([{ id, revision: 1, title: "Note", body: "", target }]),
      ),
  );
  await expect(notes(new AbortController().signal)).rejects.toThrow(
    "association",
  );
});
