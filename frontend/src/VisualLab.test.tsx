import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { catalog, counts, metadata } from "./test/lab-fixture";
import VisualLab from "./VisualLab";

vi.mock("./LabHistogram", () => ({
  default: ({ data }: { data: unknown }) => (
    <p>Histogram: {JSON.stringify(data)}</p>
  ),
}));
afterEach(() => vi.unstubAllGlobals());
function peer(model = "rgb") {
  const records = new Map<string, unknown>();
  const calls: Record<string, unknown>[] = [];
  let failure = "";
  const fetcher = vi.fn(async (url: string, options?: RequestInit) => {
    if (url.endsWith("operations")) return Response.json(catalog());
    if (options?.method !== "POST") return Response.json(records.get(url));
    const request = JSON.parse(String(options.body));
    calls.push(request);
    if (failure)
      return Response.json(
        { schema_version: 1, error: { message: failure } },
        { status: 400 },
      );
    const operation = request.operation;
    const image = ![
      "image.histogram",
      "image.compare",
      "image.pixels.read",
      "image.channel.extract",
    ].includes(operation);
    const data =
      operation === "image.histogram"
        ? counts
        : operation === "image.compare"
          ? { equal: false, differing_pixels: 4, complete: true }
          : {
              bit_depth: 8,
              channels: ["green"],
              region: { x: 0, y: 0, width: 2, height: 2 },
              preview_png_base64: "aGVsbG8=",
            };
    const info = {
      ...metadata,
      color_model: operation === "image.palette.expand" ? "rgba" : model,
    };
    const record = {
      id: request.id,
      metadata: image ? info : null,
      representation: image
        ? "image"
        : operation === "image.pixels.read"
          ? "bytes"
          : "structured",
      semantic_type: image ? "image.samples" : "bytes",
      byte_size: 80,
      sha256: "0".repeat(64),
      provenance: { operation, parameters: request.parameters },
    };
    const content_url = `/api/lab/results/${request.id}/content`;
    records.set(content_url, data);
    return Response.json({
      schema_version: 1,
      result: record,
      retained: request.retain_result,
      content_url: request.retain_result ? content_url : null,
      inline: image
        ? null
        : operation === "image.pixels.read"
          ? { type: "bytes", base64: "FFA0" }
          : { type: "structured", data },
    });
  });
  vi.stubGlobal("fetch", fetcher);
  return {
    calls,
    fail: (message: string) => {
      failure = message;
    },
    fetcher,
  };
}
function expectValue(label: string, fragment: string) {
  expect((screen.getByLabelText(label) as HTMLTextAreaElement).value).toContain(
    fragment,
  );
}
async function open() {
  render(<VisualLab store="source" path="sample.png" />);
  fireEvent.click(screen.getByRole("button", { name: "Decode image in Core" }));
  await screen.findByLabelText("Pixel X");
}
it("uses typed Core probes, generated channels/transforms, comparison and result selection", async () => {
  const protocol = peer();
  await open();
  fireEvent.change(screen.getByLabelText("Pixel X"), {
    target: { value: "1" },
  });
  fireEvent.change(screen.getByLabelText("Pixel Y"), {
    target: { value: "1" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Read pixel" }));
  await screen.findByText(/Core pixel \(1, 1\).*20, 80, 52/);
  expectValue("Result provenance", '"x"');
  fireEvent.change(screen.getByLabelText("Analysis channel"), {
    target: { value: "green" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Compute histogram" }));
  await screen.findByText(/Histogram:/);
  expect(protocol.calls.at(-1)?.retain_result).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Extract channel" }));
  await screen.findByAltText("Core-generated grayscale channel preview");
  expectValue("Structured result content", "preview_png_base64");
  fireEvent.click(screen.getByRole("button", { name: "Invert" }));
  await waitFor(() => expectValue("Result provenance", "image.invert"));
  fireEvent.click(
    screen.getByRole("button", { name: "Compare with original" }),
  );
  await waitFor(() =>
    expectValue("Structured result content", '"differing_pixels": 4'),
  );
  const list = screen.getByRole("region", { name: "Generated Lab results" });
  fireEvent.click(
    within(list).getByRole("button", { name: /1. image.decode/ }),
  );
  await waitFor(() =>
    expect(
      within(list).getByRole("button", { name: /1. image.decode/ }),
    ).toHaveAttribute("aria-pressed", "true"),
  );
  fireEvent.click(screen.getByRole("button", { name: "Rotate 90°" }));
  await waitFor(() =>
    expect(protocol.calls.at(-1)?.operation).toBe("image.rotate"),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Mirror horizontally" }),
    ).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Mirror horizontally" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Grayscale" })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Grayscale" }));
  await waitFor(() =>
    expect(protocol.calls.at(-1)?.operation).toBe("image.grayscale"),
  );
  await waitFor(() =>
    expect(
      within(list).getByRole("button", { name: /2. image.channel.extract/ }),
    ).toBeEnabled(),
  );
  fireEvent.click(
    within(list).getByRole("button", { name: /2. image.channel.extract/ }),
  );
  await waitFor(() =>
    expectValue("Structured result content", "preview_png_base64"),
  );
});
it.each(["gray", "gray_alpha", "rgba", "indexed", "unknown"])(
  "uses explicit %s channel semantics and palette expansion",
  async (model) => {
    peer(model);
    await open();
    const options = within(
      screen.getByLabelText("Analysis channel"),
    ).queryAllByRole("option");
    expect(options).toHaveLength(
      model === "gray" || model === "indexed"
        ? 1
        : model === "gray_alpha"
          ? 2
          : model === "rgba"
            ? 4
            : 0,
    );
    if (model === "indexed") {
      fireEvent.click(
        screen.getByRole("button", { name: "Expand palette to RGBA" }),
      );
      await waitFor(() =>
        expect(
          within(screen.getByLabelText("Analysis channel")).getAllByRole(
            "option",
          ),
        ).toHaveLength(4),
      );
    }
    fireEvent.change(screen.getByLabelText("Pixel X"), {
      target: { value: "20" },
    });
    expect(screen.getByRole("button", { name: "Read pixel" })).toBeDisabled();
  },
);
it("keeps failures explicit and retries without modifying the original", async () => {
  const protocol = peer();
  protocol.fail("Invalid PNG.");
  render(<VisualLab store="source" path="bad.png" />);
  fireEvent.click(screen.getByRole("button", { name: "Decode image in Core" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Invalid PNG");
  protocol.fail("");
  fireEvent.click(screen.getByRole("button", { name: "Decode image in Core" }));
  await screen.findByLabelText("Pixel X");
  protocol.fail("Resource limit.");
  fireEvent.click(screen.getByRole("button", { name: "Invert" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Resource limit");
  expect(
    screen.getByAltText("Original Core-decoded raster"),
  ).toBeInTheDocument();
});
it("cancels and aborts on unmount, with recovery for missing catalog support", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValueOnce(Response.json(catalog([]))),
  );
  const view = render(<VisualLab store="source" path="sample.png" />);
  fireEvent.click(screen.getByRole("button", { name: "Decode image in Core" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "does not expose image.decode",
  );
  vi.stubGlobal(
    "fetch",
    vi.fn(
      (_url, options) =>
        new Promise((_resolve, reject) =>
          options.signal.addEventListener("abort", () =>
            reject(new DOMException("aborted", "AbortError")),
          ),
        ),
    ),
  );
  fireEvent.click(screen.getByRole("button", { name: "Decode image in Core" }));
  fireEvent.click(screen.getByRole("button", { name: "Cancel operation" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("cancelled");
  fireEvent.click(screen.getByRole("button", { name: "Decode image in Core" }));
  view.unmount();
});
