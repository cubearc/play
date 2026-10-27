import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { CubeTerminal } from "./CubeTerminal";
import { terminalStatus } from "./terminal-api";

vi.mock("./exploration-api", async (original) => ({
  ...(await original<typeof import("./exploration-api")>()),
  openTypedResult: vi.fn(),
}));
const mocks = vi.hoisted(() => ({
  data: null as null | ((data: string) => void),
  resize: null as null | ((size: { cols: number; rows: number }) => void),
  write: vi.fn(),
  dispose: vi.fn(),
  focus: vi.fn(),
}));
vi.mock("./terminal-api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./terminal-api")>()),
  terminalStatus: vi.fn(),
}));
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 18;
    textarea = document.createElement("textarea");
    parser = { registerOscHandler: () => ({ dispose: vi.fn() }) };
    loadAddon() {}
    open(element: HTMLElement) {
      element.append(this.textarea);
    }
    focus = mocks.focus;
    dispose = mocks.dispose;
    resize(cols: number, rows: number) {
      this.cols = cols;
      this.rows = rows;
      mocks.resize?.({ cols, rows });
    }
    onData(callback: (data: string) => void) {
      mocks.data = callback;
      return { dispose: vi.fn() };
    }
    onResize(callback: (size: { cols: number; rows: number }) => void) {
      mocks.resize = callback;
      return { dispose: vi.fn() };
    }
    write(data: Uint8Array, done: () => void) {
      mocks.write(data);
      done();
    }
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    proposeDimensions() {
      return { cols: 100, rows: 24 };
    }
  },
}));
class Socket {
  static OPEN = 1;
  static instances: Socket[] = [];
  readyState = 1;
  bufferedAmount = 0;
  binaryType = "";
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  send = vi.fn();
  close = vi.fn(() => this.onclose?.());
  constructor(public url: string) {
    Socket.instances.push(this);
  }
}
function setup(available = true) {
  vi.mocked(terminalStatus).mockResolvedValue({
    available,
    active: 0,
    limit: 2,
    idle_seconds: 300,
    lifetime_seconds: 1800,
    input_limit: 4194304,
    output_limit: 8388608,
  });
  vi.stubGlobal("WebSocket", Socket);
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
}
afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
  Socket.instances = [];
  mocks.data = null;
  mocks.resize = null;
});
it("explains native unavailability and retries a failed runner check", async () => {
  setup(false);
  vi.mocked(terminalStatus).mockRejectedValueOnce(new Error("Runner stopped"));
  const user = userEvent.setup();
  render(<CubeTerminal />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Runner stopped");
  await user.click(screen.getByRole("button", { name: "Refresh runner" }));
  expect(
    await screen.findByText(
      /Native development does not execute host commands/,
    ),
  ).toBeVisible();
  expect(
    screen.queryByRole("button", { name: "Start terminal" }),
  ).not.toBeInTheDocument();
});
it("opens the selected directory, streams bytes, resizes and ends the isolated session", async () => {
  setup();
  const user = userEvent.setup();
  const view = render(<CubeTerminal />);
  await user.selectOptions(
    await screen.findByLabelText("Working directory"),
    "output",
  );
  await user.type(screen.getByLabelText("Relative directory"), "case-1");
  await user.click(screen.getByRole("button", { name: "Start terminal" }));
  await waitFor(() => expect(Socket.instances.length).toBe(1));
  const socket = Socket.instances[0];
  expect(socket).toBeDefined();
  if (!socket) return;
  expect(new URL(socket.url).searchParams.get("store")).toBe("output");
  expect(new URL(socket.url).searchParams.get("path")).toBe("case-1");
  await act(async () =>
    socket.onmessage?.({
      data: JSON.stringify({ type: "ready", size: { cols: 100, rows: 24 } }),
    }),
  );
  expect(await screen.findByText("Connected to Cube Runner")).toBeVisible();
  expect(screen.getByText("Terminal size: 100 × 24")).toBeVisible();
  act(() => mocks.data?.("cube --help\r"));
  expect(socket.send).toHaveBeenCalledWith(
    JSON.stringify({ type: "input", data: "cube --help\r" }),
  );
  act(() => mocks.resize?.({ cols: 110, rows: 30 }));
  expect(socket.send).toHaveBeenCalledWith(
    JSON.stringify({ type: "resize", size: { cols: 110, rows: 30 } }),
  );
  act(() =>
    socket.onmessage?.({
      data: JSON.stringify({ type: "resized", size: { cols: 110, rows: 30 } }),
    }),
  );
  expect(screen.getByText("Terminal size: 110 × 30")).toBeVisible();
  act(() => socket.onmessage?.({ data: new Uint8Array([0, 255]).buffer }));
  expect(mocks.write).toHaveBeenCalledWith(new Uint8Array([0, 255]));
  act(() => mocks.data?.("x".repeat(4097)));
  expect(await screen.findByText(/Paste smaller portions/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "Focus terminal" }));
  expect(mocks.focus).toHaveBeenCalled();
  act(() =>
    socket.onmessage?.({
      data: JSON.stringify({ type: "end", reason: "idle_timeout" }),
    }),
  );
  expect(await screen.findByText(/five minutes without input/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "Close terminal" }));
  expect(socket.close).toHaveBeenCalled();
  expect(mocks.dispose).toHaveBeenCalled();
  view.unmount();
});
it("handles transport errors and malformed responses without injecting output into HTML", async () => {
  setup();
  const user = userEvent.setup();
  render(<CubeTerminal />);
  await user.click(
    await screen.findByRole("button", { name: "Start terminal" }),
  );
  await waitFor(() => expect(Socket.instances.length).toBe(1));
  const socket = Socket.instances[0];
  if (!socket) return;
  act(() => socket.onerror?.());
  expect(await screen.findByText(/could not open this terminal/)).toBeVisible();
  act(() => socket.onmessage?.({ data: "<script>untrusted</script>" }));
  expect(await screen.findByText(/invalid response/)).toBeVisible();
  expect(screen.queryByText("untrusted")).not.toBeInTheDocument();
});

it("only publishes a selected staged capture and keeps the terminal alive after publication errors", async () => {
  setup();
  const { openTypedResult } = await import("./exploration-api");
  const { parseResult } = await import("./lab-api");
  const { typedWire, outputPort, id } = await import("./test/lab-fixture");
  vi.mocked(openTypedResult).mockResolvedValue({
    id,
    record: parseResult(typedWire()),
    ports: [outputPort],
    output: { text: "A6" },
  });
  const user = userEvent.setup();
  render(<CubeTerminal />);
  await user.click(
    await screen.findByRole("button", { name: "Start terminal" }),
  );
  await waitFor(() => expect(Socket.instances.length).toBe(1));
  const socket = Socket.instances[0];
  if (!socket) throw new Error("Missing socket");
  act(() =>
    socket.onmessage?.({
      data: JSON.stringify({ type: "ready", size: { cols: 80, rows: 18 } }),
    }),
  );
  expect(socket.send).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Refresh staged captures" }),
  );
  expect(socket.send).toHaveBeenLastCalledWith(
    JSON.stringify({ type: "capture_list" }),
  );
  act(() =>
    socket.onmessage?.({ data: JSON.stringify({ type: "captures", ids: [] }) }),
  );
  expect(await screen.findByText(/No staged captures/)).toBeVisible();
  act(() =>
    socket.onmessage?.({
      data: JSON.stringify({ type: "captures", ids: ["capture-1"] }),
    }),
  );
  await user.click(screen.getByRole("button", { name: "Send to Lab" }));
  expect(socket.send).toHaveBeenLastCalledWith(
    JSON.stringify({ type: "capture_read", id: "capture-1" }),
  );
  expect(screen.getByRole("button", { name: "Send to Lab" })).toBeDisabled();
  act(() =>
    socket.onmessage?.({
      data: JSON.stringify({
        type: "publication_error",
        message: "Capture is incomplete",
      }),
    }),
  );
  expect(await screen.findByText("Capture is incomplete")).toBeVisible();
  expect(socket.close).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Send to Lab" }));
  await act(async () =>
    socket.onmessage?.({ data: JSON.stringify({ type: "published", id }) }),
  );
  expect(await screen.findByLabelText("Exact text output")).toHaveValue("A6");
  expect(openTypedResult).toHaveBeenCalledWith(id, expect.any(AbortSignal));
  expect(
    await screen.findByText(/Published locally without rerunning/),
  ).toBeVisible();
});
