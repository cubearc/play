import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SolutionDetails } from "./SolutionDetails";
import { finalWire, solutionState } from "./test/solution-fixture";

afterEach(() => vi.unstubAllGlobals());
it("preserves immutable explanations when final content is missing or differs and explicitly reloads evidence", async () => {
  const fetcher = vi
    .fn()
    .mockRejectedValueOnce(new Error("Artifact is missing"))
    .mockResolvedValueOnce(
      Response.json({
        ...finalWire(),
        metadata: { ...finalWire().metadata, sha256: "1".repeat(64) },
      }),
    )
    .mockResolvedValueOnce(Response.json(finalWire()));
  vi.stubGlobal("fetch", fetcher);
  const state = solutionState(),
    revision = vi.fn();
  state.solution.final_output.field = { pointer: "/text", conversion: "text" };
  state.solution.candidate = { reference: { stage: "one", revision: 1 } };
  render(
    <SolutionDetails
      state={state}
      busy={false}
      onRevision={revision}
      onRevise={vi.fn()}
      onExplore={vi.fn()}
    />,
  );
  const inspect = screen.getByRole("button", { name: "Inspect final value" });
  fireEvent.click(inspect);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Artifact is missing",
  );
  fireEvent.click(inspect);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "differs from the saved solution",
  );
  expect(screen.getByLabelText("Saved explanation")).toHaveValue(
    state.solution.explanation,
  );
  fireEvent.click(inspect);
  expect(await screen.findByLabelText("Exact text output")).toHaveValue("A6");
  fireEvent.click(screen.getByText("Frozen local stage candidate"));
  expect(
    screen.getByLabelText<HTMLTextAreaElement>("Frozen stage candidate").value,
  ).toContain('"stage": "one"');
  fireEvent.click(screen.getByText(/Selected execution path ·/));
  expect(
    screen.getByLabelText<HTMLTextAreaElement>(
      "Recorded output schema identities",
    ).value,
  ).toContain('"name": "text"');
  fireEvent.click(
    screen.getByRole("button", { name: "Check solution evidence again" }),
  );
  expect(screen.queryByLabelText("Exact text output")).toBeNull();
  expect(revision).toHaveBeenCalledWith(1);
});
it("shows binary selections and fails clearly when a retained image cannot be rendered", async () => {
  const state = solutionState();
  state.solution.local_status = "resolved";
  state.solution.final_output.field = {
    pointer: "",
    conversion: "base64_bytes",
  };
  state.replay_current = { status: "inputs_available", gaps: [] };
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      Response.json({
        ...finalWire(),
        metadata: { ...finalWire().metadata, representation: "image" },
        value: { base64: "AP8=" },
      }),
    ),
  );
  render(
    <SolutionDetails
      state={state}
      busy={false}
      onRevision={vi.fn()}
      onRevise={vi.fn()}
      onExplore={vi.fn()}
    />,
  );
  expect(
    screen.getByText(/Resolved locally · Private · Unverified/),
  ).toBeVisible();
  expect(
    screen.getByText(/Recorded inputs are currently available/),
  ).toBeVisible();
  expect(screen.getByText(/the selected output itself/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Inspect final value" }));
  fireEvent.error(await screen.findByAltText("Selected solution artifact"));
  expect(screen.getByText(/Image preview is unavailable/)).toBeVisible();
});
