import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import * as api from "./exploration-api";
import { LinkedExecution } from "./LinkedExecution";
import { parseResult } from "./lab-api";
import { id, outputPort, typedWire } from "./test/lab-fixture";

vi.mock("./exploration-api", async (original) => ({
  ...(await original<typeof api>()),
  execution: vi.fn(),
  openExecution: vi.fn(),
}));
afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
const run: api.Execution & { exploration_id: string } = {
  id,
  exploration_id: crypto.randomUUID(),
  configuration_revision: 1,
  step: crypto.randomUUID(),
  status: "succeeded",
  stale: true,
  evidence: { retained: true, result_id: id },
  started_ms: 1,
  ended_ms: 2,
};
it("opens the referenced old output without changing its execution or choosing a newer configuration", async () => {
  vi.mocked(api.execution).mockResolvedValue(run);
  vi.mocked(api.openExecution).mockResolvedValue({
    id,
    record: parseResult(typedWire()),
    ports: [{ ...outputPort, name: "other" }, outputPort],
    output: { other: "Another named output", text: "Original evidence" },
  });
  const fetcher = vi.fn().mockResolvedValue(
    Response.json({
      schema_version: 1,
      artifact_id: id,
      status: "inputs_available",
      checked_artifacts: 1,
      gaps: [],
    }),
  );
  vi.stubGlobal("fetch", fetcher);
  render(
    <LinkedExecution target={{ execution_id: id, output_port: "text" }} />,
  );
  expect(api.execution).not.toHaveBeenCalled();
  await userEvent.click(
    screen.getByRole("button", { name: "Inspect linked execution" }),
  );
  expect(await screen.findByLabelText("Exact text output")).toHaveValue(
    "Original evidence",
  );
  expect(screen.getByText(/earlier configuration/)).toBeVisible();
  expect(api.execution).toHaveBeenCalledWith(id, expect.any(AbortSignal));
  expect(api.openExecution).toHaveBeenCalledWith(run, expect.any(AbortSignal));
  expect(
    fetcher.mock.calls.every(
      ([, options]) => options.method === undefined || options.method === "GET",
    ),
  ).toBe(true);
});
it("retains failed-attempt evidence without requesting a nonexistent result", async () => {
  vi.mocked(api.execution).mockResolvedValue({
    ...run,
    status: "failed",
    evidence: {
      error: { code: "invalid_input", message: "Exact recorded failure" },
    },
  });
  render(<LinkedExecution target={{ execution_id: id, output_port: null }} />);
  await userEvent.click(
    screen.getByRole("button", { name: "Inspect linked execution" }),
  );
  expect(await screen.findByText(/no retained output/)).toBeVisible();
  expect(api.openExecution).not.toHaveBeenCalled();
  await userEvent.click(screen.getByText("Recorded execution evidence"));
  expect(screen.getByLabelText("Execution evidence JSON")).toHaveValue(
    JSON.stringify(
      { error: { code: "invalid_input", message: "Exact recorded failure" } },
      null,
      2,
    ),
  );
});
it("reports unavailable artifacts while keeping the linked execution visible", async () => {
  vi.mocked(api.execution).mockResolvedValue(run);
  vi.mocked(api.openExecution).mockRejectedValue(
    new Error("Retained result integrity failed"),
  );
  render(
    <LinkedExecution target={{ execution_id: id, output_port: "text" }} />,
  );
  await userEvent.click(
    screen.getByRole("button", { name: "Inspect linked execution" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "The note and its reference were preserved",
  );
  expect(screen.getByText(/earlier configuration/)).toBeVisible();
});
