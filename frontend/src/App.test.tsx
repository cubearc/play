import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

it.each(["file", "challenge"])(
  "shows the launcher's workspace and selected %s",
  async (intent) => {
    window.history.replaceState(
      null,
      "",
      `/#session=private&file=sample.cube&intent=${intent}`,
    );
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(new Response(null, { status: 204 }))
        .mockResolvedValueOnce(
          Response.json({
            status: "ok",
            version: "0.2.0",
            workspace: { id: "local-id", name: "Investigation 空" },
          }),
        ),
    );
    render(<App />);
    expect(
      await screen.findByRole("heading", { name: "Investigation 空" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        intent === "challenge" ? "Opened challenge:" : "Opened file:",
      ),
    ).toBeVisible();
    expect(window.location.hash).toBe("");
  },
);

it("shows a launched workspace without pretending discovery has selected a file", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      Response.json({
        status: "ok",
        version: "0.2.0",
        workspace: { id: "local-id", name: "Investigation" },
      }),
    ),
  );
  render(<App />);
  expect(await screen.findByText("Source files are read-only.")).toBeVisible();
});

it("shows the real connection and version without external requests", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(Response.json({ status: "ok", version: "0.2.0" }));
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  expect(
    screen.getByRole("heading", { level: 1, name: "Investigation workspace" }),
  ).toBeVisible();
  expect(await screen.findByText("Your connection is ready")).toBeVisible();
  expect(screen.getByText("Cube Play 0.2.0")).toBeVisible();
  expect(fetcher).toHaveBeenCalledWith(
    "/api/health",
    expect.objectContaining({ credentials: "omit" }),
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
    .mockResolvedValueOnce(Response.json({ status: "ok", version: "0.2.0" }));
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

it("accepts a fresh launcher capability while recovering in the same document", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(new Response(null, { status: 401 }))
    .mockResolvedValueOnce(new Response(null, { status: 204 }))
    .mockResolvedValueOnce(Response.json({ status: "ok", version: "0.2.0" }));
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  await screen.findByRole("button", { name: "Try again" });
  window.history.replaceState(null, "", "/#session=fresh");
  fireEvent(window, new HashChangeEvent("hashchange"));
  expect(await screen.findByText("Your connection is ready")).toBeVisible();
  expect(window.location.hash).toBe("");
});

it("keeps an active investigation mounted until the received session is explicitly opened", async () => {
  window.history.replaceState(null, "", "/#session=first");
  const fetcher = vi.fn(async (url: string) =>
    url === "/api/session"
      ? new Response(null, { status: 204 })
      : Response.json({ status: "ok", version: "0.2.0" }),
  );
  vi.stubGlobal("fetch", fetcher);
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Your connection is ready");
  const count = fetcher.mock.calls.length;
  window.history.replaceState(null, "", "/#session=second");
  fireEvent(window, new HashChangeEvent("hashchange"));
  expect(
    screen.getByRole("region", { name: "Received launcher link" }),
  ).toBeVisible();
  expect(fetcher).toHaveBeenCalledTimes(count);
  expect(window.location.hash).toBe("");
  await user.click(
    screen.getByRole("button", { name: "Open received session" }),
  );
  await screen.findByText("Your connection is ready");
  expect(fetcher).toHaveBeenCalledTimes(count + 2);
});
