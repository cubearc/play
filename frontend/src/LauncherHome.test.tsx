import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { LauncherHome } from "./LauncherHome";
import { openWorkspace, recentWorkspaces } from "./launcher-api";

vi.mock("./launcher-api", () => ({
  recentWorkspaces: vi.fn(),
  openWorkspace: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(recentWorkspaces).mockResolvedValue([]);
  vi.mocked(openWorkspace).mockResolvedValue(null);
});
it("renders real recent entries, rejects inert markup and leaves unavailable sources explicit", async () => {
  vi.mocked(recentWorkspaces).mockResolvedValue([
    { id: "one", name: "<img src='outside'>", available: true },
    { id: "two", name: "Missing source", available: false },
  ]);
  const navigate = vi.fn();
  render(<LauncherHome navigate={navigate} />);
  const recent = await screen.findByRole("button", {
    name: /<img src='outside'>/,
  });
  expect(document.querySelector("img")).toBeNull();
  expect(screen.getByRole("button", { name: /Missing source/ })).toBeDisabled();
  vi.mocked(openWorkspace).mockResolvedValue("http://127.0.0.1:7331/");
  await userEvent.click(recent);
  expect(openWorkspace).toHaveBeenCalledWith(
    { intent: "recent", id: "one" },
    expect.any(AbortSignal),
  );
  await waitFor(() =>
    expect(navigate).toHaveBeenCalledWith("http://127.0.0.1:7331/"),
  );
});
it("creates only a named intent, waits for readiness and preserves the name when the native action is cancelled", async () => {
  const navigate = vi.fn();
  let finish: ((value: string | null) => void) | undefined;
  vi.mocked(openWorkspace).mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const user = userEvent.setup();
  render(<LauncherHome navigate={navigate} />);
  await screen.findByText("No local workspaces are registered yet.");
  expect(screen.queryByLabelText("Workspace folder name")).toBeNull();
  await user.click(screen.getByRole("button", { name: "New workspace" }));
  const name = screen.getByLabelText("Workspace folder name");
  expect(name).toHaveFocus();
  await user.clear(name);
  await user.type(name, "Private investigation");
  await user.click(screen.getByRole("button", { name: "Create workspace" }));
  expect(screen.getByRole("button", { name: "Open folder" })).toBeDisabled();
  expect(navigate).not.toHaveBeenCalled();
  expect(openWorkspace).toHaveBeenCalledWith(
    { intent: "new", name: "Private investigation" },
    expect.any(AbortSignal),
  );
  finish?.(null);
  expect(await screen.findByText(/Folder selection cancelled/)).toBeVisible();
  expect(name).toHaveValue("Private investigation");
  expect(navigate).not.toHaveBeenCalled();
});
it("recovers recent loading and reports a failed open without discarding investigations", async () => {
  vi.mocked(recentWorkspaces).mockRejectedValueOnce(
    new Error("Reopen the private launcher link"),
  );
  const user = userEvent.setup();
  render(<LauncherHome navigate={vi.fn()} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Reopen");
  await user.click(
    screen.getByRole("button", { name: "Refresh recent workspaces" }),
  );
  expect(
    await screen.findByText("No local workspaces are registered yet."),
  ).toBeVisible();
  vi.mocked(openWorkspace).mockRejectedValueOnce(new Error("Directory moved"));
  await user.click(screen.getByRole("button", { name: "Open folder" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Directory moved");
  expect(screen.getByRole("status")).toHaveTextContent("preserved");
});

it("cancels naming without opening a system dialog and restores keyboard focus", async () => {
  const user = userEvent.setup();
  render(<LauncherHome navigate={vi.fn()} />);
  const create = screen.getByRole("button", { name: "New workspace" });
  await user.click(create);
  await user.clear(screen.getByLabelText("Workspace folder name"));
  await user.type(screen.getByLabelText("Workspace folder name"), "Draft case");
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  expect(create).toHaveFocus();
  expect(screen.queryByLabelText("Workspace folder name")).toBeNull();
  expect(openWorkspace).not.toHaveBeenCalled();
  await user.click(create);
  expect(screen.getByLabelText("Workspace folder name")).toHaveValue(
    "Draft case",
  );
});
it("opens an initial ready session only after authenticated local state loads and can recover invalid bootstrap state", async () => {
  const navigate = vi.fn();
  const result = render(
    <LauncherHome initialLink="http://127.0.0.1:7331/" navigate={navigate} />,
  );
  await waitFor(() => expect(navigate).toHaveBeenCalledOnce());
  result.unmount();
  render(
    <LauncherHome initialError="Invalid launcher link" navigate={navigate} />,
  );
  expect(screen.getByRole("alert")).toHaveTextContent("Invalid launcher link");
  fireEvent.click(
    screen.getByRole("button", { name: "Refresh recent workspaces" }),
  );
  expect(
    await screen.findByText("No local workspaces are registered yet."),
  ).toBeVisible();
});
