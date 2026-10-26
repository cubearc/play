import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

it("shows the real connection and version without external requests", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(Response.json({ status: "ok", version: "0.1.0" }));
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  expect(
    screen.getByRole("heading", { level: 1, name: "Investigation workspace" }),
  ).toBeVisible();
  expect(await screen.findByText("Your connection is ready")).toBeVisible();
  expect(screen.getByText("Cube Play 0.1.0")).toBeVisible();
  expect(fetcher).toHaveBeenCalledWith(
    "/api/health",
    expect.objectContaining({ credentials: "same-origin" }),
  );
});

it("shows progress while the local application responds", () => {
  vi.stubGlobal("fetch", vi.fn().mockReturnValue(new Promise(() => {})));
  render(<App />);
  expect(screen.getByRole("status")).toHaveTextContent(
    "Checking the local application",
  );
});

it("recovers from a connection failure using the keyboard", async () => {
  const fetcher = vi
    .fn()
    .mockRejectedValueOnce(new Error("offline"))
    .mockResolvedValueOnce(Response.json({ status: "ok", version: "0.1.0" }));
  vi.stubGlobal("fetch", fetcher);
  const user = userEvent.setup();
  render(<App />);
  const retry = await screen.findByRole("button", { name: "Try again" });
  retry.focus();
  await user.keyboard("{Enter}");
  expect(await screen.findByText("Your connection is ready")).toBeVisible();
});

it("explains a missing session without pretending to be connected", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response(null, { status: 401 })),
  );
  render(<App />);
  expect(await screen.findByText(/Open the session link/)).toBeVisible();
  expect(screen.queryByText("Connected locally")).not.toBeInTheDocument();
});

it("aborts outstanding work when unmounted", async () => {
  let signal: AbortSignal | undefined;
  vi.stubGlobal(
    "fetch",
    vi.fn().mockImplementation((_url, options: RequestInit) => {
      signal = options.signal ?? undefined;
      return new Promise(() => {});
    }),
  );
  const result = render(<App />);
  await waitFor(() => expect(signal).toBeDefined());
  result.unmount();
  expect(signal?.aborted).toBe(true);
});
