import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ArtifactLibrary, TextLab } from "./ArtifactLibrary";
import { catalog, id, wire } from "./test/lab-fixture";

vi.mock("./VisualLab", () => ({
  default: () => <p>Existing visual controls</p>,
}));
afterEach(() => vi.unstubAllGlobals());
it("reopens named results, renames with optimistic revision, loads history and compares real protocol references", async () => {
  const second = crypto.randomUUID();
  let title = "Original";
  let revision = 1;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) => {
      if (url.endsWith("/library") && options?.method === "PUT") {
        const value = JSON.parse(String(options.body));
        expect(value.revision).toBe(1);
        title = value.title;
        revision = 2;
        return Response.json({ schema_version: 1, revision });
      }
      if (url.endsWith("/library"))
        return Response.json({
          entries: [
            {
              entry: { id, title, revision },
              record: { representation: "image" },
              error: null,
            },
            {
              entry: { id: second, title: "Second", revision: 1 },
              record: { representation: "image" },
              error: null,
            },
          ],
        });
      if (url.endsWith("/history"))
        return Response.json({ records: [wire().result] });
      if (url.endsWith("/operations")) return Response.json(catalog());
      if (url.endsWith("/execute"))
        return Response.json(
          wire("image.compare", {
            result: {
              ...wire().result,
              representation: "structured",
              metadata: null,
            },
            retained: false,
            content_url: null,
            inline: { type: "structured", data: { equal: false } },
          }),
        );
      return Response.json(wire());
    }),
  );
  render(<ArtifactLibrary />);
  fireEvent.click(await screen.findByRole("button", { name: "Original" }));
  await screen.findByText("Existing visual controls");
  fireEvent.change(screen.getByLabelText("Compare saved image"), {
    target: { value: second },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Compare saved artifacts" }),
  );
  await waitFor(() =>
    expect(
      (
        screen.getByLabelText(
          "Saved artifact comparison",
        ) as HTMLTextAreaElement
      ).value,
    ).toContain('"equal": false'),
  );
  fireEvent.change(screen.getByLabelText("Saved artifact name"), {
    target: { value: "Renamed" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Rename saved artifact" }),
  );
  await screen.findByRole("button", { name: "Renamed" });
  fireEvent.click(
    screen.getByRole("button", { name: "Load operation history" }),
  );
  fireEvent.click(
    await screen.findByRole("button", { name: /image.decode · image ·/ }),
  );
  await screen.findByText("Existing visual controls");
  fireEvent.click(
    screen.getByRole("button", { name: "Refresh artifact library" }),
  );
  await screen.findByRole("button", { name: "Renamed" });
});
it("preserves missing references, reports bad history and never hides read errors", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) =>
      url.endsWith("/library")
        ? Response.json({
            entries: [
              {
                entry: { id, title: "Missing", revision: 1 },
                record: null,
                error: "File missing.",
              },
            ],
          })
        : Response.json({ message: "Bundle is corrupt." }, { status: 503 }),
    ),
  );
  render(<ArtifactLibrary />);
  await screen.findByText(/File missing/);
  fireEvent.click(screen.getByRole("button", { name: "Missing" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Bundle is corrupt",
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Load operation history" }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Load operation history" }),
    ).toBeEnabled(),
  );
});
it("runs a discovered default Core text operation, saves a name and offers explicit export", async () => {
  let saved = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) => {
      if (url.endsWith("/operations"))
        return Response.json({
          schema_version: 1,
          operations: [
            {
              core: {
                id: "encoding.base64.decode",
                description: "Decode",
                parameters: [],
              },
              lab: { version: 1, input_type: "text" },
            },
          ],
        });
      if (url.endsWith("/execute")) {
        const request = JSON.parse(String(options?.body));
        expect(request.input).toEqual({ type: "text", data: "AP+A" });
        return Response.json(
          wire("encoding.base64.decode", {
            result: {
              ...wire().result,
              metadata: null,
              representation: "bytes",
            },
            inline: { type: "bytes", base64: "AP+A" },
          }),
        );
      }
      saved = true;
      return Response.json({ revision: 1 });
    }),
  );
  render(<TextLab file={null} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Discover text and byte operations" }),
  );
  await screen.findByRole("option", { name: "encoding.base64.decode" });
  fireEvent.change(screen.getByLabelText("Core operation"), {
    target: { value: "encoding.base64.decode" },
  });
  fireEvent.change(screen.getByLabelText("Private Lab text"), {
    target: { value: "AP+A" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Run Core operation" }));
  await screen.findByLabelText("Reopened result content");
  fireEvent.change(screen.getByLabelText("Artifact name"), {
    target: { value: "Bytes" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save named artifact" }));
  await waitFor(() => expect(saved).toBe(true));
  await screen.findByText(/Artifact saved locally/);
  expect(
    within(
      screen.getByRole("region", { name: "Preserve selected result" }),
    ).getByRole("link"),
  ).toHaveAttribute("download");
});
it("runs file input with the exact discovered representation and renders plain text inertly", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) => {
      if (url.endsWith("/operations"))
        return Response.json({
          schema_version: 1,
          operations: [
            {
              core: {
                id: "encoding.base64.encode",
                description: "Encode",
                parameters: [],
              },
              lab: { version: 1, input_type: "bytes" },
            },
          ],
        });
      if (url.endsWith("/execute")) {
        expect(JSON.parse(String(options?.body)).input.representation).toBe(
          "bytes",
        );
        return Response.json(
          wire("encoding.base64.encode", {
            result: {
              ...wire().result,
              metadata: null,
              representation: "text",
            },
            inline: { type: "text", data: "AP+A" },
          }),
        );
      }
      return new Response("<script>inert text</script>");
    }),
  );
  render(<TextLab file={{ store: "source", path: "unknown.xyz" }} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Discover text and byte operations" }),
  );
  await screen.findByRole("option", { name: "encoding.base64.encode" });
  fireEvent.change(screen.getByLabelText("Core operation"), {
    target: { value: "encoding.base64.encode" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Run Core operation" }));
  await waitFor(() =>
    expect(screen.getByLabelText("Reopened result content")).toHaveValue(
      "<script>inert text</script>",
    ),
  );
});
