import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { FileInspector } from "./FileInspector";
import { FileWorkspace } from "./FileWorkspace";
import { entry, inspection, listing } from "./test/workspace";

afterEach(() => vi.unstubAllGlobals());
function api(
  handler: (url: URL, init?: RequestInit) => Response | Promise<Response>,
) {
  const fetcher = vi.fn((value: string, init?: RequestInit) =>
    Promise.resolve(handler(new URL(value, "http://localhost"), init)),
  );
  vi.stubGlobal("fetch", fetcher);
  return fetcher;
}

it("keeps an empty workspace actionable and imports only after explicit confirmation", async () => {
  let imported = false;
  const fetcher = api((url) => {
    if (url.pathname.endsWith("/import")) {
      imported = true;
      return Response.json({ store: "imports", path: "private.txt" });
    }
    if (url.pathname.endsWith("/inspect"))
      return Response.json(inspection("private.txt", { store: "imports" }));
    return Response.json(
      listing(imported ? [entry("private.txt")] : [], {
        store: imported ? "imports" : "source",
      }),
    );
  });
  const user = userEvent.setup();
  render(<FileWorkspace initialSelection={null} />);
  expect(await screen.findByText("This workspace is empty")).toBeVisible();
  await user.upload(
    screen.getByLabelText("Choose a file"),
    new File(["private"], "private.txt"),
  );
  expect(fetcher.mock.calls.some(([value]) => value.includes("/import?"))).toBe(
    false,
  );
  await user.clear(screen.getByLabelText("Save as"));
  await user.type(screen.getByLabelText("Save as"), "private.txt");
  await user.click(
    screen.getByRole("button", { name: "Import into workspace" }),
  );
  expect(await screen.findByText("Private fixture text.")).toBeVisible();
  expect(
    screen.getByRole("button", { name: "Imported files" }),
  ).toHaveAttribute("aria-pressed", "true");
  await user.click(screen.getByRole("button", { name: "Source files" }));
  await user.click(screen.getByRole("button", { name: "Imported files" }));
  expect(
    await screen.findByRole("button", { name: "Inspect private.txt" }),
  ).toBeVisible();
});

it("auto-selects one compatible file and preserves unknown siblings and blocked links", async () => {
  api((url) =>
    url.pathname.endsWith("/inspect")
      ? Response.json(inspection())
      : Response.json(
          listing([
            entry(),
            entry("unknown.xyz", { category: "unknown" }),
            entry("link", { kind: "symlink", accessible: false, path: null }),
          ]),
        ),
  );
  render(<FileWorkspace initialSelection={null} />);
  expect(
    await screen.findByRole("heading", { name: "notes.txt" }),
  ).toBeVisible();
  expect(
    screen.getByRole("button", { name: "Inspect unknown.xyz" }),
  ).toBeEnabled();
  expect(screen.getByRole("button", { name: "Inspect link" })).toBeDisabled();
});

it("retains direct-file selection without hiding siblings and never executes challenge content", async () => {
  api((url) =>
    url.pathname.endsWith("/inspect")
      ? Response.json(
          inspection("rabbit.cube", {
            text: {
              value: "<script>evil()</script>",
              bytes: 23,
              truncated: false,
            },
          }),
        )
      : Response.json(listing([entry("rabbit.cube"), entry("other.txt")])),
  );
  render(
    <FileWorkspace
      initialSelection={{ name: "rabbit.cube", intent: "challenge" }}
    />,
  );
  expect(await screen.findByText("<script>evil()</script>")).toBeVisible();
  expect(document.querySelector("script")).toBeNull();
  expect(
    screen.getByRole("button", { name: "Inspect other.txt" }),
  ).toBeVisible();
  expect(screen.getByText(/Imported content is never executed/)).toBeVisible();
});

it("navigates directories and bounded pages, then refreshes after a transient failure", async () => {
  let fail = true;
  const fetcher = api((url) => {
    if (fail) {
      fail = false;
      return Response.json({ message: "Directory changed." }, { status: 409 });
    }
    const path = url.searchParams.get("path") ?? "";
    const offset = Number(url.searchParams.get("offset"));
    return Response.json(
      listing(
        [
          entry(path ? "nested/deep.txt" : "nested", {
            kind: path ? "file" : "directory",
            path: path ? "nested/deep.txt" : "nested",
          }),
        ],
        { path, next_offset: offset === 0 ? 128 : null, truncated: true },
      ),
    );
  });
  const user = userEvent.setup();
  render(<FileWorkspace initialSelection={null} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Directory changed",
  );
  await user.click(screen.getByRole("button", { name: "Refresh files" }));
  await user.click(await screen.findByRole("button", { name: "Next files" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Previous files" }),
    ).toBeEnabled(),
  );
  await user.click(screen.getByRole("button", { name: "Previous files" }));
  await user.click(
    await screen.findByRole("button", { name: "Open directory nested" }),
  );
  expect(
    await screen.findByRole("button", { name: "Up one directory" }),
  ).toBeVisible();
  expect(screen.getByText(/Discovery stopped at/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "nested" }));
  await user.click(screen.getByRole("button", { name: "Up one directory" }));
  await user.click(screen.getByRole("button", { name: "Source" }));
  expect(
    fetcher.mock.calls.some(([value]) => value.includes("path=nested")),
  ).toBe(true);
});

it("shows unavailable managed storage and does not expose an import action", async () => {
  api(() => Response.json(listing([], { imports_available: false })));
  render(<FileWorkspace initialSelection={null} />);
  expect(
    await screen.findByText(/Managed storage is unavailable/),
  ).toBeVisible();
  expect(screen.getByRole("button", { name: "Imported files" })).toBeDisabled();
  expect(screen.queryByLabelText("Choose a file")).toBeNull();
});

it("reports import conflicts and allows canceling a pending request", async () => {
  let pending = false;
  let signal: AbortSignal | undefined;
  api((url, init) => {
    if (url.pathname.endsWith("/import")) {
      if (!pending) {
        pending = true;
        return Response.json(
          { message: "Name already exists." },
          { status: 409 },
        );
      }
      signal = init?.signal ?? undefined;
      return new Promise((_resolve, reject) =>
        signal?.addEventListener("abort", () =>
          reject(new DOMException("Canceled", "AbortError")),
        ),
      );
    }
    return Response.json(listing());
  });
  const user = userEvent.setup();
  render(<FileWorkspace initialSelection={null} />);
  await screen.findByText("This workspace is empty");
  await user.upload(
    screen.getByLabelText("Choose a file"),
    new File(["private"], "name.txt"),
  );
  await user.click(
    screen.getByRole("button", { name: "Import into workspace" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Name already exists",
  );
  await user.click(
    screen.getByRole("button", { name: "Import into workspace" }),
  );
  await user.click(
    await screen.findByRole("button", { name: "Cancel import" }),
  );
  expect(signal?.aborted).toBe(true);
  expect(await screen.findByRole("alert")).toHaveTextContent("Import canceled");
});

it("blocks an oversized import before sending its bytes and aborts pending work on unmount", async () => {
  let signal: AbortSignal | undefined;
  api((_url, init) => {
    signal = init?.signal ?? undefined;
    return new Promise(() => {});
  });
  const user = userEvent.setup();
  const view = render(<FileWorkspace initialSelection={null} />);
  const file = new File(["x"], "large.bin");
  Object.defineProperty(file, "size", { value: 32 * 1024 * 1024 + 1 });
  await user.upload(screen.getByLabelText("Choose a file"), file);
  expect(
    screen.getByRole("button", { name: "Import into workspace" }),
  ).toBeDisabled();
  expect(screen.getByRole("alert")).toHaveTextContent("32 MiB");
  view.unmount();
  expect(signal?.aborted).toBe(true);
});

it.each(["image", "video", "audio"] as const)(
  "renders an authorized %s preview and recovers from browser codec errors",
  async (kind) => {
    api(() =>
      Response.json(
        inspection("sample.file", {
          entry: entry("sample.file", { category: kind, modified_ms: null }),
          preview: { kind, available: true, mime: `${kind}/fixture` },
          text: null,
          image: kind === "image" ? { width: 2, height: 3 } : null,
          media: kind === "video" ? { tracks: [] } : null,
          notice: "Bounded Core evidence.",
          signatures: [
            {
              format: "fixture",
              mime: `${kind}/fixture`,
              rule: "header candidate",
            },
          ],
          extension_mismatch: true,
        }),
      ),
    );
    render(<FileInspector store="source" path="sample.file" />);
    await screen.findByText("Bounded Core evidence.");
    const media =
      kind === "image"
        ? screen.getByRole("img")
        : screen.getByLabelText(
            `${kind === "video" ? "Video" : "Audio"} preview of sample.file`,
          );
    fireEvent.error(media);
    expect(await screen.findByText(/browser could not decode/)).toBeVisible();
    expect(
      screen.getByRole("region", { name: "Byte inspection" }),
    ).toBeVisible();
  },
);

it("inspects bounded text and byte pages and retries an interrupted file request", async () => {
  let first = true;
  api((url) => {
    if (first) {
      first = false;
      return Response.json({ message: "File was removed." }, { status: 404 });
    }
    const offset = Number(url.searchParams.get("offset"));
    return Response.json(
      inspection("empty.txt", {
        text: { value: "", bytes: 65536, truncated: true },
        offset,
        next_offset: offset === 0 ? 4096 : null,
      }),
    );
  });
  const user = userEvent.setup();
  render(<FileInspector store="imports" path="empty.txt" />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "File was removed",
  );
  await user.click(screen.getByRole("button", { name: "Retry inspection" }));
  expect(await screen.findByText("(Empty text file)")).toBeVisible();
  expect(screen.getByText(/64 KiB text limit/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "Next bytes" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Next bytes" })).toBeDisabled(),
  );
  await user.click(screen.getByRole("button", { name: "Previous bytes" }));
  expect(await screen.findByText(/Offset 0/)).toBeVisible();
});

it("shows empty byte results and ignores canceled inspector responses", async () => {
  let resolve: ((value: Response) => void) | undefined;
  const fetcher = api(
    () =>
      new Promise<Response>((done) => {
        resolve = done;
      }),
  );
  const view = render(<FileInspector store="source" path="gone.txt" />);
  expect(screen.getByRole("status")).toHaveTextContent("Inspecting");
  view.unmount();
  resolve?.(Response.json(inspection("gone.txt", { rows: [], text: null })));
  expect(fetcher.mock.calls[0]?.[1]?.signal?.aborted).toBe(true);
  api(() =>
    Response.json(
      inspection("empty.bin", {
        rows: [],
        text: null,
        preview: {
          kind: "binary",
          available: false,
          mime: "application/octet-stream",
        },
      }),
    ),
  );
  render(<FileInspector store="source" path="empty.bin" />);
  expect(await screen.findByText("No bytes in this file.")).toBeVisible();
});
