import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ChallengePanel } from "./ChallengePanel";
import { parseChallenge } from "./challenge-api";

vi.mock("./StageCandidate", () => ({ StageCandidate: () => null }));
const key = "a".repeat(64);
function state() {
  return {
    schema_version: 1,
    challenges: [{ key, id: "local", title: "Local challenge" }],
    active: {
      definition: {
        key,
        store: "source",
        path: "rabbit.cube",
        manifest: {
          id: "local",
          title: "Local challenge",
          description: "Private progress",
          stages: [
            {
              id: "first",
              title: "First stage",
              instructions: "<script>inert instructions</script>",
              steps: [{ id: "step", instructions: "Read bytes." }],
              hints: ["Green"],
              artifacts: ["image"],
              answer_type: "bytes",
            },
            {
              id: "second",
              title: "Second stage",
              instructions: "Write notes.",
              steps: [],
              hints: [],
              artifacts: [],
              answer_type: "text",
            },
          ],
        },
      },
      stage: "first",
      revision: 1,
      artifacts: [{ id: "image", store: "source", path: "sample.png" }],
      progress: [],
    },
  };
}
afterEach(() => vi.unstubAllGlobals());
it("imports launcher intent once, renders inert stages/hints and opens an authorized artifact", async () => {
  let current = state();
  const fetcher = vi.fn(async (_url: string, options?: RequestInit) => {
    const body = options?.body ? JSON.parse(String(options.body)) : null;
    if (body?.stage && body?.status)
      current = { ...current, active: { ...current.active, progress: [] } };
    else if (body?.stage)
      current = {
        ...current,
        active: { ...current.active, stage: body.stage, revision: 2 },
      };
    return Response.json(current);
  });
  vi.stubGlobal("fetch", fetcher);
  const open = vi.fn();
  render(
    <ChallengePanel
      selected={{ store: "source", path: "rabbit.cube" }}
      launch={{ name: "rabbit.cube", intent: "challenge" }}
      onOpen={open}
    />,
  );
  await screen.findByText("<script>inert instructions</script>");
  expect(fetcher.mock.calls[0]?.[1]?.method).toBe("POST");
  fireEvent.click(screen.getByText("Hint 1"));
  expect(screen.getByText("Green")).toBeInTheDocument();
  fireEvent.click(
    screen.getByRole("button", { name: "Open challenge artifact image" }),
  );
  expect(open).toHaveBeenCalledWith({ store: "source", path: "sample.png" });
  fireEvent.change(screen.getByLabelText("Private local progress"), {
    target: { value: "in_progress" },
  });
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(2));
  fireEvent.click(screen.getByRole("button", { name: "Second stage" }));
  await screen.findByText("Write notes.");
  fireEvent.change(screen.getByLabelText("Saved challenge"), {
    target: { value: key },
  });
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(4));
  fireEvent.click(
    screen.getByRole("button", { name: "Reload local challenges" }),
  );
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(5));
  fireEvent.click(
    screen.getByRole("button", { name: "Import selected challenge" }),
  );
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(6));
});
it("keeps Free Lab optional and exposes import failures without executing definitions", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({ schema_version: 1, challenges: [], active: null }),
    )
    .mockResolvedValueOnce(
      Response.json({ message: "Invalid version." }, { status: 400 }),
    );
  vi.stubGlobal("fetch", fetcher);
  render(
    <ChallengePanel
      selected={{ store: "source", path: "bad.cube" }}
      launch={null}
      onOpen={() => {}}
    />,
  );
  await screen.findByText(/Free Lab and notes work without one/);
  fireEvent.click(
    screen.getByRole("button", { name: "Import selected challenge" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent("Invalid version");
});
it("rejects incompatible challenge responses and reports recovery errors", async () => {
  for (const value of [
    {},
    { ...state(), schema_version: 2 },
    { ...state(), challenges: null },
    { ...state(), active: { ...state().active, revision: -1 } },
  ])
    expect(() => parseChallenge(value)).toThrow();
  vi.stubGlobal(
    "fetch",
    vi.fn().mockRejectedValue(new Error("Local storage unavailable.")),
  );
  render(<ChallengePanel selected={null} launch={null} onOpen={() => {}} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Local storage unavailable",
  );
  expect(
    screen.getByRole("button", { name: "Import selected challenge" }),
  ).toBeDisabled();
});
