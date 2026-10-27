import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { InvestigationWorkspace } from "./InvestigationWorkspace";
import {
  type Investigation,
  loadInvestigation,
  saveInvestigation,
} from "./investigation-api";
import { entry, inspection, listing } from "./test/workspace";

vi.mock("./ChallengePanel", () => ({ ChallengePanel: () => null }));
vi.mock("./ExplorationLab", () => ({ ExplorationLab: () => null }));
vi.mock("./ArtifactLibrary", () => ({
  ArtifactLibrary: () => null,
  TextLab: () => null,
}));
vi.mock("./InvestigationNotes", () => ({ InvestigationNotes: () => null }));
vi.mock("./investigation-api", () => ({
  loadInvestigation: vi.fn(),
  saveInvestigation: vi.fn(),
}));
const snapshot: Investigation = {
  id: "id",
  title: "Saved case",
  revision: 1,
  view_revision: 4,
  interrupted_sessions: 0,
  warnings: [],
  view: { store: "source", path: "", offset: 0, selected: null },
};
function api(value: Investigation = snapshot) {
  vi.mocked(loadInvestigation).mockResolvedValue(value);
  vi.mocked(saveInvestigation).mockResolvedValue(5);
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      const path = new URL(url, "http://localhost");
      if (path.pathname.endsWith("/inspect"))
        return Response.json(
          inspection(path.searchParams.get("path") ?? "notes.txt"),
        );
      return Response.json(
        listing([entry("notes.txt"), entry("second.txt")], {
          store:
            path.searchParams.get("store") === "imports" ? "imports" : "source",
          path: path.searchParams.get("path") ?? "",
        }),
      );
    }),
  );
}
afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});
it("restores the actual workspace section and saves section changes with existing file navigation", async () => {
  api({ ...snapshot, view: { ...snapshot.view, panel: "notes" } });
  const user = userEvent.setup();
  render(<InvestigationWorkspace initialSelection={null} />);
  expect(await screen.findByRole("tab", { name: "Notes" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await user.click(screen.getByRole("tab", { name: "Explorations" }));
  await waitFor(() =>
    expect(saveInvestigation).toHaveBeenCalledWith("/view", {
      revision: 4,
      view: { ...snapshot.view, panel: "explorations" },
    }),
  );
  await user.click(screen.getByRole("tab", { name: "Files" }));
  await waitFor(() =>
    expect(saveInvestigation).toHaveBeenLastCalledWith("/view", {
      revision: 5,
      view: snapshot.view,
    }),
  );
});
it("protects an unsaved workspace name until the actual save succeeds", async () => {
  api();
  const user = userEvent.setup();
  render(<InvestigationWorkspace initialSelection={null} />);
  await user.click(await screen.findByText("Rename"));
  const name = await screen.findByLabelText("Investigation name");
  await user.clear(name);
  await user.type(name, "Unsaved case");
  const dirty = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(dirty);
  expect(dirty.defaultPrevented).toBe(true);
  await user.click(screen.getByRole("button", { name: "Save name" }));
  await waitFor(() =>
    expect(saveInvestigation).toHaveBeenCalledWith("", {
      revision: 1,
      title: "Unsaved case",
    }),
  );
  const saved = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(saved);
  expect(saved.defaultPrevented).toBe(false);
});
it("restores selected files, saves the name and keeps recovered state visible", async () => {
  api({
    ...snapshot,
    interrupted_sessions: 1,
    warnings: ["One artifact is missing"],
    view: {
      store: "imports",
      path: "",
      offset: 0,
      selected: { store: "imports", path: "notes.txt" },
    },
  });
  const user = userEvent.setup();
  render(<InvestigationWorkspace initialSelection={null} />);
  expect(
    await screen.findByRole("heading", { name: "notes.txt" }),
  ).toBeVisible();
  expect(screen.getByLabelText("Investigation name")).toHaveValue("Saved case");
  expect(
    screen.getByText(/recovered after an interrupted session/),
  ).toBeVisible();
  expect(screen.getByText("One artifact is missing")).toBeVisible();
  expect(saveInvestigation).not.toHaveBeenCalled();
  await user.click(screen.getByText("Rename"));
  await user.clear(screen.getByLabelText("Investigation name"));
  await user.type(screen.getByLabelText("Investigation name"), "Renamed case");
  await user.click(screen.getByRole("button", { name: "Save name" }));
  await waitFor(() =>
    expect(saveInvestigation).toHaveBeenCalledWith("", {
      title: "Renamed case",
      revision: 1,
    }),
  );
});
it("serializes and coalesces navigation writes while a request is outstanding", async () => {
  api();
  let finish: ((value: number) => void) | undefined;
  vi.mocked(saveInvestigation)
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    )
    .mockResolvedValue(6);
  const user = userEvent.setup();
  render(<InvestigationWorkspace initialSelection={null} />);
  await user.click(
    await screen.findByRole("button", { name: "Inspect notes.txt" }),
  );
  expect(await screen.findByText("Saving navigation…")).toBeVisible();
  await user.click(screen.getByRole("button", { name: "Inspect second.txt" }));
  await user.click(screen.getByRole("button", { name: "Inspect notes.txt" }));
  expect(saveInvestigation).toHaveBeenCalledTimes(1);
  await act(async () => finish?.(5));
  await waitFor(() => expect(saveInvestigation).toHaveBeenCalledTimes(2));
  expect(saveInvestigation).toHaveBeenLastCalledWith("/view", {
    revision: 5,
    view: {
      ...snapshot.view,
      selected: { store: "source", path: "notes.txt" },
    },
  });
  expect(await screen.findByText("Navigation saved locally")).toBeVisible();
});
it("stops conflicting navigation writes until an explicit reload", async () => {
  api();
  vi.mocked(saveInvestigation).mockRejectedValue(
    new Error("Changed in another tab"),
  );
  const user = userEvent.setup();
  render(<InvestigationWorkspace initialSelection={null} />);
  await user.click(
    await screen.findByRole("button", { name: "Inspect notes.txt" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Changed in another tab",
  );
  await user.click(screen.getByRole("button", { name: "Inspect second.txt" }));
  expect(saveInvestigation).toHaveBeenCalledTimes(1);
  await user.click(
    screen.getByRole("button", { name: "Reload investigation" }),
  );
  await waitFor(() => expect(loadInvestigation).toHaveBeenCalledTimes(2));
});
it("handles failed restoration and retries without overwriting stored state", async () => {
  api();
  vi.mocked(loadInvestigation).mockRejectedValueOnce(
    new Error("Database is unavailable"),
  );
  const user = userEvent.setup();
  render(<InvestigationWorkspace initialSelection={null} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Database is unavailable",
  );
  expect(saveInvestigation).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Reload investigation" }),
  );
  expect(await screen.findByLabelText("Investigation name")).toHaveValue(
    "Saved case",
  );
});
it("prioritizes an explicit launcher selection over saved navigation", async () => {
  api({
    ...snapshot,
    view: {
      store: "imports",
      path: "",
      offset: 0,
      selected: { store: "imports", path: "old.txt" },
    },
  });
  render(
    <InvestigationWorkspace
      initialSelection={{ name: "requested.txt", intent: "file" }}
    />,
  );
  expect(
    await screen.findByRole("heading", { name: "requested.txt" }),
  ).toBeVisible();
  await waitFor(() =>
    expect(saveInvestigation).toHaveBeenCalledWith(
      "/view",
      expect.objectContaining({
        view: {
          store: "source",
          path: "",
          offset: 0,
          selected: { store: "source", path: "requested.txt" },
        },
      }),
    ),
  );
});
it("reports rename failure and aborts outstanding restoration when unmounted", async () => {
  api();
  vi.mocked(saveInvestigation).mockRejectedValue(
    new Error("Name could not be saved"),
  );
  const user = userEvent.setup();
  const result = render(<InvestigationWorkspace initialSelection={null} />);
  await user.click(await screen.findByText("Rename"));
  await user.click(await screen.findByRole("button", { name: "Save name" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Name could not be saved",
  );
  result.unmount();
  const signal = vi.mocked(loadInvestigation).mock.calls[0]?.[0];
  expect(signal?.aborted).toBe(true);
});
