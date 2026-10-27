import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { expect, it, vi } from "vitest";
import { type ChallengeState, challengeRequest } from "./challenge-api";
import { ExplorationScope } from "./ExplorationScope";
import type { Exploration } from "./exploration-api";

vi.mock("./challenge-api", () => ({ challengeRequest: vi.fn() }));
function Harness({
  onChange,
}: {
  onChange: (value: Exploration["scope"]) => void;
}) {
  const [scope, setScope] = useState<Exploration["scope"]>(null);
  return (
    <ExplorationScope
      scope={scope}
      disabled={false}
      onChange={(value) => {
        onChange(value);
        setScope(value);
      }}
    />
  );
}
it("associates an existing local stage explicitly without importing or updating progress", async () => {
  const state: ChallengeState = {
    challenges: [{ id: "fixture", key: "key", title: "Local fixture" }],
    active: {
      definition: {
        key: "key",
        store: "source",
        path: "rabbit.cube",
        manifest: {
          id: "fixture",
          title: "Local fixture",
          description: "",
          stages: [
            {
              id: "one",
              title: "First stage",
              instructions: "Read",
              steps: [],
              hints: [],
              artifacts: [],
              answer_type: "text",
            },
          ],
        },
      },
      stage: "one",
      revision: 1,
      artifacts: [],
      progress: [],
    },
  };
  vi.mocked(challengeRequest).mockResolvedValue(state);
  const change = vi.fn();
  const user = userEvent.setup();
  render(<Harness onChange={change} />);
  expect(challengeRequest).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Load local stages" }));
  await screen.findByRole("option", { name: "Local fixture → First stage" });
  await user.selectOptions(
    screen.getByLabelText("Exploration scope"),
    "key:one",
  );
  expect(change).toHaveBeenLastCalledWith({ challenge: "key", stage: "one" });
  expect(challengeRequest).toHaveBeenCalledExactlyOnceWith(
    "",
    undefined,
    expect.any(AbortSignal),
  );
  await user.selectOptions(screen.getByLabelText("Exploration scope"), "free");
  expect(change).toHaveBeenLastCalledWith(null);
});
it("preserves a historical association when the active local challenge is missing", async () => {
  vi.mocked(challengeRequest).mockResolvedValue({
    challenges: [],
    active: null,
  });
  const user = userEvent.setup();
  render(
    <ExplorationScope
      scope={{ challenge: "old", stage: "retained" }}
      onChange={vi.fn()}
      disabled={false}
    />,
  );
  expect(screen.getByLabelText("Exploration scope")).toHaveValue(
    "old:retained",
  );
  await user.click(screen.getByRole("button", { name: "Load local stages" }));
  expect(await screen.findByText(/Import a local challenge/)).toBeVisible();
  expect(screen.getByLabelText("Exploration scope")).toHaveValue(
    "old:retained",
  );
});
