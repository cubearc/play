import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { ExplorationLab } from "./ExplorationLab";
import * as api from "./exploration-api";
import { parseResult } from "./lab-api";
import { id, outputPort, typedWire } from "./test/lab-fixture";

vi.mock("./exploration-api", async (original) => ({
  ...(await original<typeof api>()),
  catalog: vi.fn(),
  explorations: vi.fn(),
  executions: vi.fn(),
  saveExploration: vi.fn(),
  executeStep: vi.fn(),
  openExecution: vi.fn(),
}));
const entry: api.CatalogEntry = {
  contract: {
    schema_version: 1,
    operation_version: 1,
    operation: "cipher.rot5",
    category: "ciphers",
    description: "Rotate decimal digits",
    inputs: [
      {
        name: "text",
        semantic_type: "text",
        required: true,
        schema: { type: "string" },
      },
    ],
    outputs: [
      {
        name: "text",
        semantic_type: "text",
        required: true,
        schema: { type: "string" },
      },
    ],
    parameters: { type: "object", properties: {}, additionalProperties: false },
    errors: { type: "object" },
    policy: {
      private_input: false,
      private_output: false,
      sensitive_parameters: [],
    },
  },
  execution: { available: true, reason: null, adapter_version: 2 },
};
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.catalog).mockResolvedValue([entry]);
  vi.mocked(api.explorations).mockResolvedValue([]);
  vi.mocked(api.executions).mockResolvedValue([]);
  vi.mocked(api.saveExploration).mockImplementation(async (value) => ({
    ...value,
    revision: value.revision + 1,
  }));
  vi.mocked(api.executeStep).mockResolvedValue({
    id,
    record: { ...parseResult(typedWire()), retained: false },
    ports: [outputPort],
    output: { text: "A6" },
  });
});
it("requires a fresh explicit choice to retain each private output and preserves it only for the selected run", async () => {
  const privateEntry = {
    ...entry,
    contract: {
      ...entry.contract,
      operation: "private.example",
      policy: { ...entry.contract.policy, private_output: true },
    },
  };
  const graph: api.Exploration = {
    schema_version: 1,
    id: "private-case",
    revision: 1,
    title: "Private result",
    hypothesis: "",
    scope: null,
    steps: [
      {
        id: "one",
        operation: "private.example",
        operation_version: 1,
        inputs: { text: { type: "literal", value: "A1" } },
        parameters: {},
        private_parameters: [],
        position: [0, 0],
      },
    ],
  };
  vi.mocked(api.catalog).mockResolvedValue([privateEntry]);
  vi.mocked(api.explorations).mockResolvedValue([graph]);
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} onPreserve={vi.fn()} />);
  const choice = await screen.findByRole("checkbox", {
    name: "Keep this private output for this run",
  });
  expect(choice).not.toBeChecked();
  const run = screen.getByRole("button", { name: "Run selected path" });
  await user.click(run);
  await screen.findByText(
    "Execution recorded locally. Private output was not retained.",
  );
  expect(vi.mocked(api.executeStep).mock.calls[0]?.[5]).toBe(false);
  expect(
    screen.queryByRole("button", { name: "Preserve as solution" }),
  ).toBeNull();
  await user.click(choice);
  vi.mocked(api.executeStep).mockResolvedValueOnce({
    id,
    record: { ...parseResult(typedWire()), retained: true },
    ports: [outputPort],
    output: { text: "A6" },
  });
  await user.click(run);
  await screen.findByText("Execution and output preserved locally");
  expect(vi.mocked(api.executeStep).mock.calls[1]?.[5]).toBe(true);
  expect(choice).not.toBeChecked();
  expect(
    screen.getByRole("button", { name: "Preserve as solution" }),
  ).toBeEnabled();
  await user.click(choice);
  vi.mocked(api.executeStep).mockRejectedValueOnce(
    new Error("Private execution failed"),
  );
  await user.click(run);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Private execution failed",
  );
  expect(choice).not.toBeChecked();
  expect(api.saveExploration).not.toHaveBeenCalled();
  expect(graph.steps[0]?.parameters).toEqual({});
});
it("does not execute an upstream private-output operation implicitly while chaining", async () => {
  const privateEntry = {
    ...entry,
    contract: {
      ...entry.contract,
      operation: "private.example",
      policy: { ...entry.contract.policy, private_output: true },
    },
  };
  const graph: api.Exploration = {
    schema_version: 1,
    id: "private-path",
    revision: 1,
    title: "Private branch",
    hypothesis: "",
    scope: null,
    steps: [
      {
        id: "one",
        operation: "private.example",
        operation_version: 1,
        inputs: { text: { type: "literal", value: "A1" } },
        parameters: {},
        private_parameters: [],
        position: [0, 0],
      },
      {
        id: "two",
        operation: entry.contract.operation,
        operation_version: 1,
        inputs: { text: { type: "step", step: "one", port: "text" } },
        parameters: {},
        private_parameters: [],
        position: [300, 0],
      },
    ],
  };
  vi.mocked(api.catalog).mockResolvedValue([privateEntry, entry]);
  vi.mocked(api.explorations).mockResolvedValue([graph]);
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  await screen.findByRole("checkbox", {
    name: "Keep this private output for this run",
  });
  await user.click(screen.getByRole("button", { name: "Steps" }));
  await user.click(
    within(screen.getByRole("list", { name: "Exploration steps" })).getByRole(
      "button",
      { name: "cipher.rot5" },
    ),
  );
  expect(
    screen.queryByRole("checkbox", {
      name: "Keep this private output for this run",
    }),
  ).toBeNull();
  await user.click(screen.getByRole("button", { name: "Run selected path" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Run the upstream private-output step explicitly",
  );
  expect(api.executeStep).not.toHaveBeenCalled();
});
it("creates and saves an actual graph configuration before explicit execution", async () => {
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  await user.click(
    await screen.findByRole("button", { name: "New exploration" }),
  );
  await user.clear(screen.getByLabelText("Exploration name"));
  await user.type(
    screen.getByLabelText("Exploration name"),
    "Digit hypothesis",
  );
  await user.type(
    screen.getByLabelText("Hypothesis"),
    "ROT5 changes only digits",
  );
  await user.selectOptions(
    screen.getByLabelText("Operation", { exact: true }),
    "cipher.rot5",
  );
  await user.click(screen.getByRole("button", { name: "Add step" }));
  await user.type(screen.getByLabelText("text value *"), "A1");
  expect(api.executeStep).not.toHaveBeenCalled();
  expect(
    screen.getByRole("button", { name: "Run selected path" }),
  ).toBeDisabled();
  await user.click(screen.getByRole("button", { name: "Save configuration" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Run selected path" }),
    ).toBeEnabled(),
  );
  const saved = vi.mocked(api.saveExploration).mock.calls.at(-1)?.[0];
  expect(saved?.steps[0]?.inputs.text).toEqual({
    type: "literal",
    value: "A1",
  });
  expect(saved?.hypothesis).toBe("ROT5 changes only digits");
  await user.click(screen.getByRole("button", { name: "Run selected path" }));
  expect(
    await screen.findByRole("region", { name: "Selected operation outputs" }),
  ).toHaveTextContent("A6");
  await user.click(screen.getByRole("button", { name: "Steps" }));
  const list = screen.getByRole("list", { name: "Exploration steps" });
  expect(
    within(list).getByRole("button", { name: "cipher.rot5" }),
  ).toBeVisible();
});
it("reopens a saved exploration and preserves server errors without inventing results", async () => {
  const value: api.Exploration = {
    schema_version: 1,
    id: "case",
    revision: 3,
    title: "Existing exploration",
    hypothesis: "Private hypothesis",
    scope: null,
    steps: [
      {
        id: "step",
        operation: "cipher.rot5",
        operation_version: 1,
        inputs: { text: { type: "literal", value: "A1" } },
        parameters: {},
        private_parameters: [],
        position: [0, 0],
      },
    ],
  };
  vi.mocked(api.explorations).mockResolvedValue([value]);
  vi.mocked(api.executeStep).mockRejectedValue(
    new Error("Source changed; choose a current input"),
  );
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  expect(await screen.findByLabelText("Hypothesis")).toHaveValue(
    "Private hypothesis",
  );
  expect(api.executeStep).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Run selected path" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Source changed");
  expect(
    screen.queryByRole("region", { name: "Selected operation outputs" }),
  ).not.toBeInTheDocument();
  await user.type(screen.getByLabelText("Hypothesis"), " revised");
  vi.mocked(api.saveExploration).mockRejectedValueOnce(
    new Error("Changed in another tab"),
  );
  await user.click(screen.getByRole("button", { name: "Save configuration" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Changed in another tab",
  );
});
it("reports loading and creation failures as actionable local errors", async () => {
  vi.mocked(api.catalog).mockRejectedValueOnce(
    new Error("Catalog unavailable"),
  );
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Catalog unavailable",
  );
  vi.mocked(api.saveExploration).mockRejectedValueOnce(
    new Error("Local storage full"),
  );
  await user.click(screen.getByRole("button", { name: "New exploration" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Local storage full",
  );
});

it("keeps a saved source reference stable when browsing another asset or store", async () => {
  const value: api.Exploration = {
    schema_version: 1,
    id: "case",
    revision: 2,
    title: "File exploration",
    hypothesis: "",
    scope: null,
    steps: [
      {
        id: "step",
        operation: "cipher.rot5",
        operation_version: 1,
        inputs: {
          text: {
            type: "file",
            store: "source",
            path: "original.txt",
            representation: "text",
          },
        },
        parameters: {},
        private_parameters: [],
        position: [0, 0],
      },
    ],
  };
  vi.mocked(api.explorations).mockResolvedValue([value]);
  const user = userEvent.setup();
  const { rerender } = render(
    <ExplorationLab asset={{ store: "source", path: "other.txt" }} />,
  );
  const input = await screen.findByRole("combobox", { name: "text · text" });
  expect(input).toHaveValue("saved_file");
  expect(api.saveExploration).not.toHaveBeenCalled();
  await user.selectOptions(input, "file");
  await user.click(screen.getByRole("button", { name: "Save configuration" }));
  await waitFor(() => expect(api.saveExploration).toHaveBeenCalled());
  expect(
    vi.mocked(api.saveExploration).mock.calls[0]?.[0].steps[0]?.inputs.text,
  ).toEqual({
    type: "file",
    store: "source",
    path: "other.txt",
    representation: "text",
  });
  rerender(<ExplorationLab asset={{ store: "imports", path: "other.txt" }} />);
  expect(input).toHaveValue("saved_file");
});

it("protects an unsaved draft and waits for the new exploration before showing its editor", async () => {
  const existing: api.Exploration = {
    schema_version: 1,
    id: "existing",
    revision: 1,
    title: "Existing exploration",
    hypothesis: "",
    scope: null,
    steps: [],
  };
  vi.mocked(api.explorations).mockResolvedValue([existing]);
  let complete: ((value: api.Exploration) => void) | undefined;
  vi.mocked(api.saveExploration).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
  );
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  await user.type(
    await screen.findByLabelText("Hypothesis"),
    "Keep this draft",
  );
  expect(
    screen.getByRole("button", { name: "New exploration" }),
  ).toBeDisabled();
  await user.click(
    screen.getByRole("button", { name: "Discard configuration edits" }),
  );
  expect(screen.getByLabelText("Hypothesis")).toHaveValue("");
  await user.click(screen.getByRole("button", { name: "New exploration" }));
  expect(screen.queryByLabelText("Exploration name")).not.toBeInTheDocument();
  expect(screen.getByText("Creating exploration…")).toBeVisible();
  await act(async () =>
    complete?.({ ...existing, id: "new", title: "New exploration" }),
  );
  expect(await screen.findByLabelText("Exploration name")).toHaveValue(
    "New exploration",
  );
});

it("keeps private inputs outside saved configuration and clears them after each attempt", async () => {
  const privateEntry = {
    ...entry,
    contract: {
      ...entry.contract,
      policy: { ...entry.contract.policy, private_input: true },
    },
  };
  const graph: api.Exploration = {
    schema_version: 1,
    id: "private-inputs",
    revision: 1,
    title: "Private material",
    hypothesis: "",
    scope: null,
    steps: [
      {
        id: "one",
        operation: entry.contract.operation,
        operation_version: 1,
        inputs: {},
        parameters: {},
        private_parameters: [],
        position: [0, 0],
      },
    ],
  };
  vi.mocked(api.catalog).mockResolvedValue([privateEntry]);
  vi.mocked(api.explorations).mockResolvedValue([graph]);
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  const input = await screen.findByLabelText(/^text value/);
  await user.type(input, "private-run-only");
  expect(screen.queryByText("Unsaved changes")).toBeNull();
  await user.click(screen.getByRole("button", { name: "Run selected path" }));
  await waitFor(() => expect(api.executeStep).toHaveBeenCalledTimes(1));
  expect(vi.mocked(api.executeStep).mock.calls[0]?.[1].inputs).toEqual({});
  expect(vi.mocked(api.executeStep).mock.calls[0]?.[6]).toEqual({
    text: "private-run-only",
  });
  await waitFor(() => expect(input).toHaveValue(""));
  expect(api.saveExploration).not.toHaveBeenCalled();
});

it("keeps unavailable contracts discoverable and explains why they cannot execute", async () => {
  vi.mocked(api.catalog).mockResolvedValue([
    {
      ...entry,
      execution: {
        available: false,
        adapter_version: 2,
        reason: "Runtime capability unavailable",
      },
    },
  ]);
  vi.mocked(api.explorations).mockResolvedValue([
    {
      schema_version: 1,
      id: "unavailable",
      revision: 1,
      title: "Availability",
      hypothesis: "",
      scope: null,
      steps: [],
    },
  ]);
  const user = userEvent.setup();
  render(<ExplorationLab asset={null} />);
  await user.selectOptions(
    await screen.findByRole("combobox", { name: "Operation" }),
    entry.contract.operation,
  );
  expect(
    await screen.findByText("Runtime capability unavailable"),
  ).toBeVisible();
  expect(screen.getByRole("button", { name: "Add step" })).toBeDisabled();
  expect(api.executeStep).not.toHaveBeenCalled();
});
