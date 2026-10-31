import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { RunDetail, TrailView } from "../desktop/workspace-contract";
import { useExecutions } from "./useExecutions";

const trail: TrailView = {
  id: "trail",
  workspaceId: "workspace",
  explorationId: "exploration",
  stageId: "stage",
  revision: 1,
  archived: false,
  title: "Clue",
  hypothesis: "",
  pipelineId: null,
  pipeline: null,
  source: {
    id: "source",
    explorationId: "exploration",
    revision: 1,
    title: "Cipher",
    content: { kind: "text", text: "Uryyb" },
  },
  bindings: {},
};
const running: RunDetail = {
  id: "run",
  sequence: 1,
  caller: "cli",
  trailId: trail.id,
  pipelineId: "pipeline",
  pipelineRevision: 1,
  status: "running",
  createdMs: 1,
  finishedMs: null,
  retainOutput: true,
  operation: { operation: "cipher.rot13", operationVersion: 1, name: "ROT13" },
  plan: {
    pipeline: { id: "pipeline", revision: 1, definition: {} },
    contract: {
      operation: "cipher.rot13",
      operation_version: 1,
      inputs: [],
      outputs: [],
    },
    contractDigest: "digest",
    targetNodes: ["node"],
    reusePolicy: "none",
    reusedOutputs: [],
    inputs: {},
    parameters: {},
    limits: {},
  },
  result: null,
  attempts: [],
};

it("shows a CLI Run that finishes between history selection and watcher attachment", async () => {
  const completed: RunDetail = {
    ...running,
    status: "completed",
    finishedMs: 2,
    result: { status: "completed", outputs: { text: "Hello" } },
  };
  let reads = 0;
  const workspace = vi.fn(async (request: { kind: string }) => {
    if (request.kind === "list_runs") return { runs: [running], before: null };
    if (request.kind === "run_detail")
      return ++reads === 1 ? running : completed;
    throw new Error("Unexpected request");
  });
  vi.stubGlobal("play", { catalog: async () => [], workspace });
  const refresh = vi.fn(async () => {});
  const { result, unmount } = renderHook(() =>
    useExecutions(null, refresh, { busy: false, save: async () => {} }),
  );
  try {
    await act(async () => result.current.load(trail));
    await waitFor(() =>
      expect(result.current.history[trail.id]?.detail?.status).toBe(
        "completed",
      ),
    );
    expect(result.current.history[trail.id]?.runs[0]?.status).toBe("completed");
    expect(
      result.current.history[trail.id]?.detail?.result?.outputs?.text,
    ).toBe("Hello");
    expect(result.current.isBusy(trail)).toBe(false);
    expect(refresh).toHaveBeenCalledWith(trail.workspaceId);
    expect(reads).toBe(2);
  } finally {
    unmount();
    vi.unstubAllGlobals();
  }
});
