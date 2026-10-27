import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import type { OperationResult } from "./exploration-api";
import { parseResult } from "./lab-api";
import { TypedOutputs } from "./TypedOutputs";
import { counts, id, outputPort, typedWire } from "./test/lab-fixture";

vi.mock("echarts/core", () => ({
  use: vi.fn(),
  init: vi.fn(() => ({
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
  })),
}));
afterEach(() => vi.unstubAllGlobals());
it("annotates the explicitly selected retained port and refuses unretained output", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      Response.json({
        schema_version: 1,
        artifact_id: id,
        status: "inputs_available",
        checked_artifacts: 1,
        gaps: [],
      }),
    ),
  );
  const value = result(
    { text: "Text", bytes: { base64: "AP8=" } },
    { text: "text", bytes: "bytes" },
  );
  value.record.retained = true;
  const annotate = vi.fn();
  const view = render(<TypedOutputs result={value} onAnnotate={annotate} />);
  await userEvent.selectOptions(screen.getByLabelText("Output port"), "bytes");
  await userEvent.click(
    screen.getByRole("button", { name: "Annotate selected output" }),
  );
  expect(annotate).toHaveBeenCalledWith("bytes");
  view.rerender(
    <TypedOutputs
      result={{ ...value, record: { ...value.record, retained: false } }}
      onAnnotate={annotate}
    />,
  );
  expect(
    screen.queryByRole("button", { name: "Annotate selected output" }),
  ).toBeNull();
});
function result(
  output: OperationResult["output"],
  semantics: Record<string, string>,
): OperationResult {
  return {
    id,
    record: { ...parseResult(typedWire()), retained: false },
    output,
    ports: Object.entries(semantics).map(([name, semantic_type]) => ({
      ...outputPort,
      name,
      semantic_type,
    })),
  };
}
it("selects named outputs with exact inert text, bytes and a bounded structured fallback", async () => {
  const user = userEvent.setup();
  render(
    <TypedOutputs
      result={result(
        {
          text: "<img src='https://outside.invalid' onerror='alert(1)'>",
          bytes: { base64: "AP8=" },
          structured: { colors: counts.colors, pixels: counts.pixels },
          empty: "",
          large: "x".repeat(65540),
        },
        {
          text: "text",
          bytes: "bytes",
          structured: "custom.report",
          empty: "text",
          large: "text",
          optional: "custom.optional",
        },
      )}
    />,
  );
  expect(screen.getByLabelText("Exact text output")).toHaveValue(
    "<img src='https://outside.invalid' onerror='alert(1)'>",
  );
  expect(document.querySelector("img")).toBeNull();
  await user.selectOptions(screen.getByLabelText("Output port"), "bytes");
  expect(screen.getByLabelText("Base64 byte output")).toHaveValue("AP8=");
  await user.selectOptions(screen.getByLabelText("Output port"), "structured");
  expect(screen.getByLabelText("Structured output")).toHaveValue(
    JSON.stringify({ colors: counts.colors, pixels: counts.pixels }, null, 2),
  );
  expect(screen.queryByRole("region", { name: "Exact histogram" })).toBeNull();
  await user.selectOptions(screen.getByLabelText("Output port"), "empty");
  expect(screen.getByText("The result is empty text.")).toBeVisible();
  await user.selectOptions(screen.getByLabelText("Output port"), "large");
  expect(screen.getByLabelText("Exact text output")).toHaveValue(
    "x".repeat(65536),
  );
  await user.selectOptions(screen.getByLabelText("Output port"), "optional");
  expect(
    screen.getByText("This optional output was not produced."),
  ).toBeVisible();
});
it("uses exact histogram counts only for the known semantic type and supported shape", async () => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
  const view = render(
    <TypedOutputs
      result={result({ histogram: counts }, { histogram: "image.histogram" })}
    />,
  );
  expect(
    await screen.findByRole(
      "region",
      { name: "Exact histogram" },
      { timeout: 5000 },
    ),
  ).toHaveTextContent("4 pixels");
  expect(screen.getByRole("table", { hidden: true })).toHaveTextContent(
    "Core sample counts",
  );
  view.rerender(
    <TypedOutputs
      result={result(
        { histogram: { colors: "malformed" } },
        { histogram: "image.histogram" },
      )}
    />,
  );
  expect(screen.getByLabelText("Structured output")).toHaveValue(
    JSON.stringify({ colors: "malformed" }, null, 2),
  );
});
