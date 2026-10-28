import { useCallback, useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { Operation, RunResult } from "../desktop/contract";
import { type TrailView, textPipeline } from "./exampleWorkspace";

export interface Attempt {
  readonly id: string;
  readonly operation: Operation;
  readonly assetId: string;
  readonly assetRevision: number;
  readonly result?: RunResult;
  readonly error?: string;
  readonly elapsed: number;
  readonly running: boolean;
}

export function useExecutions() {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [catalogError, setCatalogError] = useState("");
  const [selected, setSelected] = useState<Record<string, string>>({});
  const [attempts, setAttempts] = useState<Record<string, readonly Attempt[]>>(
    {},
  );
  const [busy, setBusy] = useState(false);
  const active = useRef(false);

  useEffect(() => {
    let mounted = true;
    void window.play
      .catalog()
      .then((entries) => {
        if (mounted) setOperations(entries);
      })
      .catch(() => {
        if (mounted) setCatalogError("Engine catalog unavailable.");
      });
    return () => {
      mounted = false;
    };
  }, []);

  const operationFor = (trail: TrailView) =>
    operations.find(
      (entry) =>
        entry.operation ===
        (selected[trail.id] ?? textPipeline.initialOperation),
    );
  const selectOperation = (trail: TrailView, operation: string) => {
    setSelected((previous) => ({ ...previous, [trail.id]: operation }));
  };
  const run = useCallback(async (trail: TrailView, operation: Operation) => {
    const content = trail.source.content;
    if (
      active.current ||
      content.kind !== "text" ||
      new TextEncoder().encode(content.text).byteLength > limits.input_bytes
    )
      return;
    active.current = true;
    setBusy(true);
    const attempt: Attempt = {
      id: crypto.randomUUID(),
      operation,
      assetId: trail.source.id,
      assetRevision: trail.source.revision,
      elapsed: 0,
      running: true,
    };
    // Capture the originating trail, not the currently selected tab/Exploration.
    setAttempts((previous) => ({
      ...previous,
      [trail.id]: [...(previous[trail.id] ?? []), attempt],
    }));
    const start = performance.now();
    let finished: Attempt;
    try {
      const result = await window.play.run(
        operation.operation,
        operation.operation_version,
        content.text,
      );
      finished = {
        ...attempt,
        result,
        elapsed: Math.round(performance.now() - start),
        running: false,
      };
    } catch {
      finished = {
        ...attempt,
        error: "Execution rejected. Native invocation is unavailable.",
        elapsed: Math.round(performance.now() - start),
        running: false,
      };
    }
    setAttempts((previous) => ({
      ...previous,
      [trail.id]: [...(previous[trail.id] ?? []).slice(0, -1), finished],
    }));
    active.current = false;
    setBusy(false);
  }, []);

  return {
    operations,
    catalogError,
    attempts,
    busy,
    operationFor,
    selectOperation,
    run,
  };
}

export type Executions = ReturnType<typeof useExecutions>;
