import { render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { CandidateProvider, CandidateSummary } from "./Candidates";

afterEach(() => vi.unstubAllGlobals());

it("shows an empty evidence context without starting a Stage read", () => {
  const workspace = vi.fn();
  vi.stubGlobal("play", { workspace });
  render(
    <CandidateProvider context={null} disabled={false} openRun={() => {}}>
      <CandidateSummary />
    </CandidateProvider>,
  );
  expect(
    screen.getByText("Open a Stage to inspect its candidate."),
  ).toBeVisible();
  expect(screen.queryByText("Loading Stage candidate…")).toBeNull();
  expect(workspace).not.toHaveBeenCalled();
});

it("keeps the loading state while a selected Stage is being read", () => {
  const workspace = vi.fn(() => new Promise(() => {}));
  vi.stubGlobal("play", { workspace });
  render(
    <CandidateProvider
      context={{
        workspace_id: "workspace",
        exploration_id: "exploration",
        stage_id: "stage",
      }}
      disabled={false}
      openRun={() => {}}
    >
      <CandidateSummary />
    </CandidateProvider>,
  );
  expect(screen.getByText("Loading Stage candidate…")).toBeVisible();
  expect(workspace).toHaveBeenCalledOnce();
});
