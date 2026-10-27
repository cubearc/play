import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ReplayStatus } from "./ReplayStatus";

afterEach(() => vi.unstubAllGlobals());
const id = "6d434f94-89f3-4fb8-a7d8-c345a0000001";
const report = {
  schema_version: 1,
  artifact_id: id,
  status: "inputs_available",
  checked_artifacts: 2,
  gaps: [],
};
it("checks actual evidence on open and explicitly rechecks changed sources without running operations", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(Response.json(report))
    .mockResolvedValueOnce(
      Response.json({
        ...report,
        status: "gaps",
        gaps: [
          {
            artifact_id: id,
            code: "source_changed",
            input_port: "input",
            message: "The current input differs from the recorded digest.",
          },
        ],
      }),
    );
  vi.stubGlobal("fetch", fetcher);
  render(<ReplayStatus id={id} />);
  expect(
    await screen.findByText(/Recorded inputs are currently available/),
  ).toBeInTheDocument();
  fireEvent.click(
    screen.getByRole("button", { name: "Check replay inputs again" }),
  );
  expect(
    await screen.findByText(/The current input differs/),
  ).toBeInTheDocument();
  for (const [url, options] of fetcher.mock.calls) {
    expect(url).toBe(`/api/lab/results/${id}/replay`);
    expect(options.method).toBeUndefined();
  }
});
it("keeps the retained result available when checks fail or the report is incompatible", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(new Response(null, { status: 429 }))
    .mockResolvedValueOnce(Response.json({ ...report, artifact_id: "other" }));
  vi.stubGlobal("fetch", fetcher);
  render(<ReplayStatus id={id} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "result is still available",
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Check replay inputs again" }),
  );
  await waitFor(() =>
    expect(screen.getByRole("alert")).toHaveTextContent("incompatible"),
  );
});
