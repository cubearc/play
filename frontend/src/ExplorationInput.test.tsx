import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { InputField } from "./ExplorationInput";
import type { CatalogEntry, Exploration, Port, Step } from "./exploration-api";
import { library } from "./notebook-api";

vi.mock("./notebook-api", () => ({ library: vi.fn() }));
const port: Port = {
  name: "input",
  semantic_type: "text",
  required: true,
  schema: { type: "string" },
};
const step: Step = {
  id: "target",
  operation: "consumer",
  operation_version: 1,
  inputs: {},
  parameters: {},
  private_parameters: [],
  position: [0, 0],
};
const graph: Exploration = {
  schema_version: 1,
  id: "graph",
  revision: 1,
  title: "Fields",
  hypothesis: "",
  scope: null,
  steps: [
    step,
    { ...step, id: "producer", operation: "report" },
    {
      ...step,
      id: "descendant",
      inputs: { input: { type: "step", step: "target", port: "output" } },
    },
  ],
};
const catalog: CatalogEntry[] = [
  {
    execution: { available: true, reason: null, adapter_version: 2 },
    contract: {
      schema_version: 1,
      operation_version: 1,
      category: "text",
      description: "Report",
      inputs: [],
      parameters: {},
      errors: {},
      policy: {
        private_input: false,
        private_output: false,
        sensitive_parameters: [],
      },
      operation: "report",
      outputs: [
        {
          ...port,
          name: "output",
          semantic_type: "text.traversal.report",
          schema: { type: "object" },
        },
      ],
    },
  },
];
const report = catalog[0];
if (report)
  catalog.push({
    execution: report.execution,
    contract: {
      ...report.contract,
      operation: "consumer",
      outputs: [{ ...port, name: "output" }],
    },
  });
it("loads retained typed artifacts and makes field conversion explicit without allowing cycles", async () => {
  vi.mocked(library).mockResolvedValue([
    {
      entry: { id: "saved", title: "Saved report", revision: 1 },
      error: null,
      record: {
        semantic_type: "operation.outputs",
        metadata: { ports: [{ name: "output", semantic_type: "report" }] },
      },
    },
  ]);
  const onChange = vi.fn();
  const user = userEvent.setup();
  const { rerender } = render(
    <InputField
      port={port}
      step={step}
      exploration={graph}
      catalog={catalog}
      asset={null}
      onChange={onChange}
      onValidityChange={vi.fn()}
      privateInput={false}
      privateValue={undefined}
    />,
  );
  expect(
    screen.queryByRole("option", { name: "consumer → output" }),
  ).toBeNull();
  await user.click(screen.getByRole("button", { name: "Load saved inputs" }));
  await screen.findByRole("option", {
    name: "Saved report → output · select a string field",
  });
  await user.selectOptions(
    screen.getByLabelText("input · text"),
    "artifact:saved:output",
  );
  expect(onChange).toHaveBeenLastCalledWith({
    type: "artifact",
    id: "saved",
    port: "output",
    selection: { pointer: "/", conversion: "text" },
  });
  const configured = {
    ...step,
    inputs: {
      input: {
        type: "artifact" as const,
        id: "saved",
        port: "output",
        selection: { pointer: "/extracted", conversion: "text" as const },
      },
    },
  };
  rerender(
    <InputField
      port={port}
      step={configured}
      exploration={graph}
      catalog={catalog}
      asset={null}
      onChange={onChange}
      onValidityChange={vi.fn()}
      privateInput={false}
      privateValue={undefined}
    />,
  );
  expect(screen.getByLabelText("Field JSON Pointer")).toHaveValue("/extracted");
  expect(screen.getByLabelText("Field conversion")).toHaveValue("text");
  expect(
    screen.getByRole("option", { name: "Whole named output" }),
  ).toBeDisabled();
});

it("requires explicit Base64 conversion when a structured field feeds a byte reader", async () => {
  const onChange = vi.fn();
  const user = userEvent.setup();
  render(
    <InputField
      port={{
        ...port,
        semantic_type: "byte_reader",
        schema: { type: "object" },
      }}
      step={step}
      exploration={graph}
      catalog={catalog}
      asset={null}
      onChange={onChange}
      onValidityChange={vi.fn()}
      privateInput={false}
      privateValue={undefined}
    />,
  );
  await user.selectOptions(
    screen.getByLabelText("input · byte_reader"),
    "step:producer:output",
  );
  expect(onChange).toHaveBeenLastCalledWith({
    type: "step",
    step: "producer",
    port: "output",
    selection: { pointer: "/", conversion: "base64_bytes" },
  });
});
