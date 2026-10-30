import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Operation } from "../../desktop/contract";
import type { RunDetail, TrailView } from "../../desktop/workspace-contract";
import { useExecutions } from "../../desktop-renderer-src/useExecutions";

const operation = {
  operation: "cipher.rot13",
  operation_version: 1,
} as Operation;
const trail = (id: string, exploration: string) =>
  ({
    id,
    workspaceId: "workspace",
    explorationId: exploration,
    stageId: exploration,
    revision: 1,
    archived: false,
    source: { id: `${id}-asset`, revision: 1 },
    pipeline: { id: "pipeline", revision: 1, outputNodeId: "node" },
  }) as TrailView;
const run = (id: string, status: string) => ({ id, status }) as RunDetail;

afterEach(() => {
  vi.unstubAllGlobals();
});

it("admits another Trail while one Run is watched and keeps duplicate clicks scoped", async () => {
  let finishA: (run: RunDetail) => void = () => {};
  const held = new Promise<RunDetail>((resolve) => {
    finishA = resolve;
  });
  const start = vi
    .fn()
    .mockResolvedValueOnce(run("a", "running"))
    .mockResolvedValueOnce(run("b", "completed"));
  vi.stubGlobal("play", undefined);
  Object.defineProperty(window, "play", {
    configurable: true,
    value: {
      catalog: async () => [],
      startRun: start,
      workspace: () => held,
    },
  });
  const { result, unmount } = renderHook(() =>
    useExecutions(null, async () => {}, { busy: false, save: async () => {} }),
  );
  const a = trail("a", "first");
  const b = trail("b", "second");
  let first: Promise<void>;
  await act(async () => {
    first = result.current.run(a, operation);
  });
  await waitFor(() => expect(result.current.busy).toBe(true));
  await act(async () => {
    void result.current.run(a, operation);
  });
  expect(start).toHaveBeenCalledTimes(1);
  await act(async () => {
    await result.current.run(b, operation);
  });
  expect(start).toHaveBeenCalledTimes(2);
  expect(result.current.history.b?.detail?.status).toBe("completed");
  await act(async () => {
    finishA(run("a", "completed"));
    await first;
  });
  expect(result.current.history.a?.detail?.status).toBe("completed");
  unmount();
});

it("queue rejection leaves no phantom Run and retry submits a fresh intent", async () => {
  const start = vi
    .fn()
    .mockRejectedValueOnce(new Error("queue_full"))
    .mockResolvedValueOnce(run("accepted", "completed"));
  vi.stubGlobal("play", { catalog: async () => [], startRun: start });
  const { result, unmount } = renderHook(() =>
    useExecutions(null, async () => {}, { busy: false, save: async () => {} }),
  );
  const a = trail("a", "first");
  await act(async () => {
    await result.current.run(a, operation);
  });
  expect(result.current.history.a?.error).toContain("queue is full");
  expect(result.current.history.a?.runs).toEqual([]);
  expect(result.current.isBusy(a)).toBe(false);
  await act(async () => {
    await result.current.run(a, operation);
  });
  expect(start.mock.calls[0]?.[0].key).not.toEqual(
    start.mock.calls[1]?.[0].key,
  );
  expect(result.current.history.a?.detail?.id).toBe("accepted");
  unmount();
});
