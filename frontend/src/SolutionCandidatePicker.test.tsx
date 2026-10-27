import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SolutionCandidatePicker } from "./SolutionCandidatePicker";
import { SolutionsPanel } from "./SolutionsPanel";
import { id } from "./test/lab-fixture";
import {
  challengeKey,
  localCandidate,
  localCandidates,
  localChallenge,
} from "./test/solution-candidate-fixture";
import { retained, solutionWire } from "./test/solution-fixture";

afterEach(() => vi.unstubAllGlobals());
it("selects the exact existing candidate revision, supports empty stages and preserves references after errors", async () => {
  const choose = vi
    .fn()
    .mockRejectedValueOnce(new Error("Selected artifact is missing"))
    .mockResolvedValue(undefined);
  let stageError = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (url.endsWith("/challenges")) return Response.json(localChallenge());
      if (url.includes("stage=two")) {
        if (stageError) throw new Error("Candidate storage is unavailable");
        return Response.json({
          schema_version: 1,
          candidate: null,
          history: [],
          error: null,
        });
      }
      return Response.json(localCandidates());
    }),
  );
  render(<SolutionCandidatePicker disabled={false} onChoose={choose} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Choose local stage candidate" }),
  );
  await screen.findByLabelText("Candidate revision");
  expect(screen.getByRole("alert")).toHaveTextContent(
    "artifact needs recovery",
  );
  fireEvent.change(screen.getByLabelText("Candidate revision"), {
    target: { value: "1" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Use candidate revision" }),
  );
  await screen.findByText("Selected artifact is missing");
  expect(choose.mock.calls[0]?.[0]).toEqual({
    key: challengeKey,
    stage: "one",
    revision: 1,
  });
  expect(choose.mock.calls[0]?.[1]).toEqual(localCandidate(1));
  fireEvent.click(
    screen.getByRole("button", { name: "Use candidate revision" }),
  );
  await waitFor(() => expect(choose).toHaveBeenCalledTimes(2));
  await waitFor(() =>
    expect(screen.getByLabelText("Candidate stage")).toBeEnabled(),
  );
  fireEvent.change(screen.getByLabelText("Candidate stage"), {
    target: { value: "two" },
  });
  await screen.findByText(/No candidate has been selected/);
  fireEvent.change(screen.getByLabelText("Candidate stage"), {
    target: { value: "one" },
  });
  await screen.findByLabelText("Candidate revision");
  stageError = true;
  fireEvent.change(screen.getByLabelText("Candidate stage"), {
    target: { value: "two" },
  });
  await screen.findByText("Candidate storage is unavailable");
});
it("keeps free findings usable without a challenge and reports challenge-read failures", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockRejectedValueOnce(new Error("Local challenge unreadable"))
      .mockResolvedValueOnce(
        Response.json({ schema_version: 1, challenges: [], active: null }),
      ),
  );
  render(<SolutionCandidatePicker disabled={false} onChoose={vi.fn()} />);
  const open = screen.getByRole("button", {
    name: "Choose local stage candidate",
  });
  fireEvent.click(open);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Local challenge unreadable",
  );
  fireEvent.click(open);
  await screen.findByText(/A free finding does not require one/);
});
it("binds the chosen stage revision to its real retained result and clears the binding when selection changes", async () => {
  const writes: unknown[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) => {
      if (url.endsWith("/challenges")) return Response.json(localChallenge());
      if (url.includes("/candidates?")) return Response.json(localCandidates());
      if (url.includes("/lab/results/"))
        return Response.json({
          schema_version: 1,
          retained: true,
          result: retained,
        });
      if (options?.method === "PUT") {
        writes.push(JSON.parse(String(options.body)));
        return Response.json(solutionWire());
      }
      return Response.json({ schema_version: 1, solutions: [] });
    }),
  );
  render(
    <SolutionsPanel
      wanted={{ result_id: id, port: "text" }}
      onExplore={vi.fn()}
    />,
  );
  await screen.findByText("No private solutions are saved yet.");
  fireEvent.click(
    screen.getByRole("button", { name: "Create finding from selected output" }),
  );
  await screen.findByLabelText("Solution explanation");
  fireEvent.change(screen.getByLabelText("Solution explanation"), {
    target: { value: "This stage uses the selected text." },
  });
  const choose = async () => {
    fireEvent.click(
      screen.getByRole("button", { name: "Choose local stage candidate" }),
    );
    await screen.findByLabelText("Candidate revision");
    fireEvent.click(
      screen.getByRole("button", { name: "Use candidate revision" }),
    );
    await screen.findByText(/Local stage one · candidate revision 2/);
  };
  await choose();
  expect(screen.getByLabelText("Final field JSON pointer")).toHaveValue("");
  fireEvent.click(
    screen.getByRole("button", { name: "Remove stage association" }),
  );
  expect(
    screen.queryByText(/Local stage one · candidate revision 2/),
  ).toBeNull();
  await choose();
  fireEvent.change(screen.getByLabelText("Final output port"), {
    target: { value: "text" },
  });
  expect(
    screen.queryByText(/Local stage one · candidate revision 2/),
  ).toBeNull();
  await choose();
  fireEvent.click(
    screen.getByRole("button", { name: "Save private solution" }),
  );
  await screen.findByLabelText("Saved explanation");
  expect(writes[0]).toMatchObject({
    source: {
      candidate: { key: challengeKey, stage: "one", revision: 2 },
      selection: {
        result_id: id,
        port: "text",
        field: { pointer: "", conversion: "text" },
      },
    },
  });
});
