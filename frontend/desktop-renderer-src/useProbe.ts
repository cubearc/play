import { useCallback, useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { Operation, RunResult } from "../desktop/contract";

export function useProbe() {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [operation, setOperation] = useState("");
  const [input, setInput] = useState("Hello, CubeArc!");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<RunResult>();
  const [error, setError] = useState("");
  const [elapsed, setElapsed] = useState(0);
  const active = useRef(false);
  const bytes = new TextEncoder().encode(input).byteLength;
  const selected = operations.find((entry) => entry.operation === operation);
  const canRun = Boolean(selected) && bytes <= limits.input_bytes && !running;

  useEffect(() => {
    let mounted = true;
    void window.play
      .catalog()
      .then((entries) => {
        if (!mounted) return;
        setOperations(entries);
        setOperation(entries[0]?.operation ?? "");
      })
      .catch(() => {
        if (mounted) setError("Engine catalog unavailable.");
      });
    return () => {
      mounted = false;
    };
  }, []);

  const run = useCallback(async () => {
    if (!selected || active.current || bytes > limits.input_bytes) return;
    active.current = true;
    setRunning(true);
    setResult(undefined);
    setError("");
    const start = performance.now();
    try {
      setResult(
        await window.play.run(operation, selected.operation_version, input),
      );
    } catch {
      setError(
        "Execution rejected. The native policy or invocation is unavailable.",
      );
    } finally {
      setElapsed(Math.round(performance.now() - start));
      active.current = false;
      setRunning(false);
    }
  }, [selected, bytes, operation, input]);

  return {
    operations,
    operation,
    setOperation,
    selected,
    input,
    setInput,
    running,
    canRun,
    run,
    result,
    error,
    elapsed,
    bytes,
    limit: limits.input_bytes,
  };
}
