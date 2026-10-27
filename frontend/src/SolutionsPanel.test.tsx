import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SolutionsPanel } from "./SolutionsPanel";
import { id } from "./test/lab-fixture";
import {
  finalWire,
  retained,
  solutionId,
  solutionWire,
} from "./test/solution-fixture";

afterEach(() => vi.unstubAllGlobals());
const header = {
  id: solutionId,
  revision: 1,
  title: "Digit finding",
  local_status: "finding",
  privacy: "private",
  verification: "unverified",
  updated_ms: 1,
};
const list = (items = [header]) => ({ schema_version: 1, solutions: items });
const root = () => ({ schema_version: 1, retained: true, result: retained });
const click = (name: string) =>
  fireEvent.click(screen.getByRole("button", { name }));

it("preserves an explicitly selected output, revises immutable evidence and retains a conflicting draft", async () => {
  const writes: Record<string, unknown>[] = [];
  let latest = 1,
    fail = false;
  const fetcher = vi.fn(async (url: string, options?: RequestInit) => {
    if (url.endsWith(`/lab/results/${id}`)) return Response.json(root());
    if (url.includes("/final?")) return Response.json(finalWire(latest));
    if (options?.method === "PUT") {
      writes.push(JSON.parse(String(options.body)));
      if (fail)
        return Response.json(
          { error: { message: "Changed by another editor." } },
          { status: 409 },
        );
      return Response.json(solutionWire(latest));
    }
    if (url.endsWith("/solutions")) return Response.json(list());
    return Response.json(solutionWire(latest));
  });
  vi.stubGlobal("fetch", fetcher);
  const explore = vi.fn();
  const view = render(
    <SolutionsPanel
      wanted={{ result_id: id, port: "text" }}
      onExplore={explore}
    />,
  );
  await screen.findByText(/Digit finding · revision 1/);
  click("Create finding from selected output");
  await screen.findByRole("heading", { name: "Preserve a private finding" });
  expect(
    screen.getByRole("button", { name: "Save private solution" }),
  ).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Solution title"), {
    target: { value: "Local deduction" },
  });
  fireEvent.change(screen.getByLabelText("Solution explanation"), {
    target: { value: "The original digits are preserved." },
  });
  fireEvent.click(screen.getByLabelText("Mark as resolved locally"));
  const leaving = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(leaving);
  expect(leaving.defaultPrevented).toBe(true);
  view.rerender(
    <SolutionsPanel
      wanted={{ result_id: id, port: "other" }}
      onExplore={explore}
    />,
  );
  expect(screen.getByLabelText("Solution explanation")).toHaveValue(
    "The original digits are preserved.",
  );
  click("Save private solution");
  await screen.findByLabelText("Saved explanation");
  expect(writes[0]).toMatchObject({
    revision: 0,
    local_status: "resolved",
    source: {
      type: "result",
      selection: { result_id: id, port: "text", field: null },
      candidate: null,
    },
  });
  click("Inspect final value");
  expect(await screen.findByLabelText("Exact text output")).toHaveValue("A6");
  click("Revise this solution");
  fireEvent.change(screen.getByLabelText("Solution explanation"), {
    target: { value: "A better explanation of the same evidence." },
  });
  fail = true;
  click("Save private solution");
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Changed by another editor",
  );
  latest = 2;
  click("Load latest revision and keep draft");
  await screen.findByText(/Saving will append revision 3/);
  expect(screen.getByLabelText("Solution explanation")).toHaveValue(
    "A better explanation of the same evidence.",
  );
  fail = false;
  latest = 3;
  click("Save private solution");
  await screen.findByLabelText("Saved explanation");
  expect(writes[2]).toMatchObject({
    id: solutionId,
    revision: 2,
    source: { type: "revision", revision: 1 },
  });
  fireEvent.change(screen.getByLabelText("Solution revision"), {
    target: { value: "1" },
  });
  await waitFor(() =>
    expect(
      fetcher.mock.calls.some(([url]) => url.includes("?revision=1")),
    ).toBe(true),
  );
  click("Continue in Explorations");
  expect(explore).toHaveBeenCalledOnce();
  click("Check solution evidence again");
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Revise this solution" }),
    ).toBeEnabled(),
  );
  click("Revise this solution");
  click("Cancel editing and discard draft");
  expect(screen.queryByLabelText("Solution explanation")).toBeNull();
});

it("browses retained roots, makes field conversions explicit and keeps successful saves when list refresh fails", async () => {
  let saved = false,
    empty = true;
  const writes: Record<string, unknown>[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) => {
      if (url.endsWith("/lab/history"))
        return Response.json({
          schema_version: 1,
          records: empty ? [] : [retained],
        });
      if (options?.method === "PUT") {
        saved = true;
        writes.push(JSON.parse(String(options.body)));
        return Response.json(solutionWire());
      }
      if (saved) throw new Error("Local list is unavailable");
      return Response.json(list([]));
    }),
  );
  render(<SolutionsPanel wanted={null} onExplore={vi.fn()} />);
  await screen.findByText("No private solutions are saved yet.");
  click("New finding from retained results");
  await screen.findByText(/No retained results are available/);
  empty = false;
  click("New finding from retained results");
  click(
    (await screen.findByRole("button", { name: /cipher.rot5 ·/ }))
      .textContent ?? "",
  );
  fireEvent.change(screen.getByLabelText("Solution explanation"), {
    target: { value: "A byte selection from the report." },
  });
  fireEvent.click(
    screen.getByLabelText("Select or convert a string field explicitly"),
  );
  fireEvent.change(screen.getByLabelText("Final field JSON pointer"), {
    target: { value: "/base64" },
  });
  fireEvent.change(screen.getByLabelText("Final field conversion"), {
    target: { value: "base64_bytes" },
  });
  click("Choose another retained result");
  await screen.findByRole("heading", { name: "Choose retained evidence" });
  await waitFor(() =>
    expect(screen.queryByText("Reading retained results…")).toBeNull(),
  );
  expect(screen.getByLabelText("Solution explanation")).toHaveValue(
    "A byte selection from the report.",
  );
  fireEvent.click(screen.getByRole("button", { name: /cipher.rot5 ·/ }));
  expect(screen.queryByLabelText("Final field JSON pointer")).toBeNull();
  fireEvent.click(
    screen.getByLabelText("Select or convert a string field explicitly"),
  );
  fireEvent.change(screen.getByLabelText("Final field JSON pointer"), {
    target: { value: "/base64" },
  });
  fireEvent.change(screen.getByLabelText("Final field conversion"), {
    target: { value: "base64_bytes" },
  });
  click("Save private solution");
  await screen.findByText(/Solution saved locally. Refresh the saved list/);
  expect(screen.getByLabelText("Saved explanation")).toBeVisible();
  expect(writes[0]).toMatchObject({
    source: {
      selection: { field: { pointer: "/base64", conversion: "base64_bytes" } },
    },
  });
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Local list is unavailable",
  );
});

it("recovers list and missing-output errors without discarding saved data", async () => {
  let broken = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (url.endsWith("/solutions")) {
        if (broken) throw new Error("Storage unavailable");
        return Response.json(list());
      }
      throw new Error("Retained output is missing");
    }),
  );
  render(
    <SolutionsPanel
      wanted={{ result_id: id, port: "text" }}
      onExplore={vi.fn()}
    />,
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Storage unavailable",
  );
  broken = false;
  click("Refresh saved solutions");
  await screen.findByText(/Digit finding · revision 1/);
  click("Create finding from selected output");
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Retained output is missing",
  );
  expect(screen.queryByLabelText("Solution explanation")).toBeNull();
  fireEvent.click(
    screen.getByRole("button", { name: /Digit finding · revision/ }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Retained output is missing",
  );
  click("New finding from retained results");
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Retained output is missing",
  );
});
