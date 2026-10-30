import { useCallback, useEffect, useRef, useState } from "react";
import type { Operation } from "../desktop/contract";
import type {
  ExplorationView,
  RunDetail,
  RunPage,
  RunRequest,
  TrailView,
} from "../desktop/workspace-contract";

import { runSelection } from "./runSelection";

export interface History extends RunPage {
  selected: string | null;
  detail: RunDetail | null;
  loading: boolean;
  error: string;
}
const empty: History = {
  runs: [],
  before: null,
  selected: null,
  detail: null,
  loading: false,
  error: "",
};
const activeStatus = (status: string) =>
  status === "queued" || status === "running";
const contextFor = (trail: TrailView) => ({
  workspace_id: trail.workspaceId,
  exploration_id: trail.explorationId,
  stage_id: trail.stageId,
});
const pause = () => new Promise<void>((resolve) => setTimeout(resolve, 150));

export function useExecutions(
  exploration: ExplorationView | null,
  refreshSummary: (workspaceId: string) => Promise<void>,
  results: {
    busy: boolean;
    save: (trail: TrailView, run: RunDetail, port: string) => Promise<void>;
  },
) {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [catalogError, setCatalogError] = useState("");
  const [retention, setRetention] = useState<Record<string, boolean>>({});
  const [history, setHistory] = useState<Record<string, History>>({});
  const [busy, setBusy] = useState(false);
  const live = useRef(true);
  const active = useRef(false);
  const state = useRef(history);
  state.current = history;
  const epochs = useRef<Record<string, number>>({});
  const pending = useRef<Record<string, RunRequest>>({});
  const watching = useRef(new Set<string>());
  const update = useCallback(
    (id: string, transform: (value: History) => History) => {
      if (live.current)
        setHistory((previous) => ({
          ...previous,
          [id]: transform(previous[id] ?? empty),
        }));
    },
    [],
  );
  useEffect(() => {
    live.current = true;
    void window.play
      .catalog()
      .then((value) => {
        if (live.current) setOperations(value);
      })
      .catch(() => {
        if (live.current) setCatalogError("Engine catalog unavailable.");
      });
    return () => {
      live.current = false;
    };
  }, []);
  const show = useCallback(
    async (trail: TrailView, id: string) => {
      const epoch = (epochs.current[trail.id] ?? 0) + 1;
      epochs.current[trail.id] = epoch;
      update(trail.id, (previous) => ({
        ...previous,
        selected: id,
        loading: true,
        error: "",
      }));
      try {
        const detail = await window.play.workspace({
          kind: "run_detail",
          context: contextFor(trail),
          run_id: id,
        });
        if (epochs.current[trail.id] === epoch)
          update(trail.id, (previous) => ({
            ...previous,
            detail,
            loading: false,
          }));
      } catch {
        if (epochs.current[trail.id] === epoch)
          update(trail.id, (previous) => ({
            ...previous,
            loading: false,
            error: "Run evidence unavailable. Reopen the stage to retry.",
          }));
      }
    },
    [update],
  );
  const watch = useCallback(
    async (trail: TrailView, initial: RunDetail) => {
      if (watching.current.has(initial.id)) return;
      watching.current.add(initial.id);
      setBusy(true);
      let detail = initial;
      try {
        while (live.current && activeStatus(detail.status)) {
          await pause();
          if (!live.current) return;
          detail = await window.play.workspace({
            kind: "run_detail",
            context: contextFor(trail),
            run_id: detail.id,
          });
          const latest = detail;
          update(trail.id, (previous) => ({
            ...previous,
            runs: previous.runs.map((run) =>
              run.id === latest.id ? latest : run,
            ),
            detail: previous.selected === latest.id ? latest : previous.detail,
          }));
        }
      } catch {
        update(trail.id, (previous) => ({
          ...previous,
          error:
            "Connection lost. The engine owns this Run; reopen the stage to recover its recorded state.",
        }));
      } finally {
        if (live.current && !activeStatus(detail.status))
          void refreshSummary(trail.workspaceId);
        watching.current.delete(initial.id);
        if (live.current) setBusy(watching.current.size > 0);
      }
    },
    [update, refreshSummary],
  );
  const load = useCallback(
    async (trail: TrailView, older = false) => {
      const before = older ? (state.current[trail.id]?.before ?? null) : null;
      try {
        const page = await window.play.workspace({
          kind: "list_runs",
          context: contextFor(trail),
          trail_id: trail.id,
          before,
        });
        const previous = state.current[trail.id] ?? empty;
        const records = older
          ? [
              ...new Map(
                [...previous.runs, ...page.runs].map((run) => [run.id, run]),
              ).values(),
            ].sort((a, b) => b.sequence - a.sequence)
          : page.runs;
        const chosen =
          previous.selected &&
          records.some((run) => run.id === previous.selected)
            ? previous.selected
            : (records[0]?.id ?? null);
        update(trail.id, (value) => ({
          ...value,
          ...page,
          runs: records,
          selected: chosen,
          error: "",
        }));
        if (chosen) await show(trail, chosen);
        const running = page.runs.find((run) => activeStatus(run.status));
        if (running) {
          const detail = await window.play.workspace({
            kind: "run_detail",
            context: contextFor(trail),
            run_id: running.id,
          });
          void watch(trail, detail);
        }
      } catch {
        update(trail.id, (value) => ({
          ...value,
          error: "Run history unavailable. Reopen this stage to retry.",
        }));
      }
    },
    [show, update, watch],
  );
  useEffect(() => {
    if (!exploration) return;
    let current = true;
    void (async () => {
      for (const trail of exploration.trails) {
        if (!current) return;
        await load(trail);
      }
    })();
    return () => {
      current = false;
    };
  }, [exploration, load]);
  const operationsFor = (trail: TrailView) =>
    operations.filter(
      (operation) =>
        operation.input_type ===
        (["text", "page"].includes(trail.source.content.kind)
          ? "text"
          : trail.source.content.kind === "image"
            ? "image.samples"
            : null),
    );
  const operationFor = (trail: TrailView) =>
    operations.find(
      (operation) => operation.operation === trail.pipeline?.outputOperation,
    );
  const run = useCallback(
    async (
      trail: TrailView,
      operation: Operation,
      targetNodeId = trail.pipeline?.outputNodeId,
      caller: "desktop" | "cli" = "desktop",
    ) => {
      if (
        active.current ||
        watching.current.size > 0 ||
        !trail.pipeline ||
        !targetNodeId ||
        trail.archived
      )
        return;
      const keep = retention[trail.id] ?? true;
      const fresh = runSelection(trail, operation, keep, targetNodeId);
      if (!fresh) return;
      active.current = true;
      setBusy(true);
      const pendingKey = `${trail.id}:${caller}`;
      const previous = pending.current[pendingKey];
      const request: RunRequest =
        previous &&
        previous.operation === operation.operation &&
        previous.trail_revision === trail.revision &&
        previous.target_node_id === targetNodeId &&
        previous.pipeline_id === trail.pipeline.id &&
        previous.pipeline_revision === trail.pipeline.revision &&
        previous.source_asset_id === trail.source.id &&
        previous.source_revision === trail.source.revision &&
        previous.retain_output === keep
          ? previous
          : fresh;
      pending.current[pendingKey] = request;
      update(trail.id, (value) => ({ ...value, error: "" }));
      try {
        const detail =
          caller === "cli"
            ? await window.play.cli({ kind: "start", request })
            : await window.play.startRun(request);
        delete pending.current[pendingKey];
        update(trail.id, (value) => ({
          ...value,
          runs: [detail, ...value.runs.filter((run) => run.id !== detail.id)],
          detail,
          selected: detail.id,
        }));
        await watch(trail, detail);
      } catch (error) {
        const conflict =
          error instanceof Error && error.message.includes("state_conflict");
        if (conflict) delete pending.current[pendingKey];
        update(trail.id, (value) => ({
          ...value,
          error: conflict
            ? "The input or Pipeline changed. Reopen this stage before starting another Run."
            : "Run request was not confirmed. Retry to check the same intent; saved history is preserved.",
        }));
      } finally {
        active.current = false;
        if (live.current) setBusy(watching.current.size > 0);
      }
    },
    [retention, update, watch],
  );
  const cancel = async (trail: TrailView) => {
    const run = state.current[trail.id]?.runs.find((run) =>
      activeStatus(run.status),
    );
    if (!run) return;
    try {
      const detail = await window.play.cancelRun({
        context: contextFor(trail),
        run_id: run.id,
      });
      update(trail.id, (value) => ({
        ...value,
        runs: value.runs.map((run) => (run.id === detail.id ? detail : run)),
        detail: value.selected === detail.id ? detail : value.detail,
      }));
    } catch {
      update(trail.id, (value) => ({
        ...value,
        error:
          "Cancellation was not confirmed. The recorded Run state will be reloaded.",
      }));
    }
  };
  return {
    saveOutput: results.save,
    saveBusy: results.busy,
    outputSaved: (run: RunDetail, port: string) => {
      if (!exploration) return false;
      const assets = [
        ...exploration.stage.blocks
          .filter((b) => b.kind === "asset")
          .map((b) => b.asset),
        ...exploration.stage.archivedBlocks
          .map((b) => b.block)
          .filter((b) => b.kind === "asset")
          .map((b) => b.asset),
      ];
      return assets.some(
        (asset) =>
          asset.origin?.kind === "run_output" &&
          asset.origin.runId === run.id &&
          asset.origin.attemptId ===
            run.attempts.find((a) => run.plan.targetNodes.includes(a.nodeId))
              ?.id &&
          asset.origin.port === port,
      );
    },
    operations,
    catalogError,
    history,
    busy,
    operationFor,
    operationsFor,
    run,
    cancel,
    load,
    show,
    keepOutput: (trail: TrailView) => retention[trail.id] ?? true,
    setKeepOutput: (trail: TrailView, value: boolean) =>
      setRetention((previous) => ({ ...previous, [trail.id]: value })),
  };
}
export type Executions = ReturnType<typeof useExecutions>;
