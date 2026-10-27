import { useEffect, useRef, useState } from "react";
import {
  type Execution,
  execution,
  type OperationResult,
  openExecution,
} from "./exploration-api";
import type { EvidenceTarget } from "./notebook-api";
import { TypedOutputs } from "./TypedOutputs";

export function LinkedExecution({ target }: { target: EvidenceTarget }) {
  const [run, setRun] = useState<
    (Execution & { exploration_id: string }) | null
  >(null);
  const [output, setOutput] = useState<OperationResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  async function inspect() {
    const controller = new AbortController();
    active.current?.abort();
    active.current = controller;
    setBusy(true);
    setError(null);
    setOutput(null);
    try {
      const run = await execution(target.execution_id, controller.signal);
      if (controller.signal.aborted) return;
      setRun(run);
      if (run.status === "succeeded" && run.evidence.retained === true) {
        const output = await openExecution(run, controller.signal);
        if (!controller.signal.aborted) setOutput(output);
      }
    } catch (error) {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "The linked evidence is unavailable. The note reference was preserved.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  return (
    <section className="linked-execution" aria-label="Linked execution">
      <h3>Linked execution evidence</h3>
      <button
        type="button"
        className="button secondary"
        disabled={busy}
        onClick={() => void inspect()}
      >
        {busy ? "Reading linked evidence…" : "Inspect linked execution"}
      </button>
      <p>Inspecting reads retained evidence without running an operation.</p>
      {run && (
        <>
          <p>
            Execution <code>{run.id}</code> · {run.status} · configuration{" "}
            {run.configuration_revision}
            {run.stale ? " · earlier configuration" : ""}
          </p>
          <p>
            Exploration <code>{run.exploration_id}</code> · step{" "}
            <code>{run.step}</code>
          </p>
          <details>
            <summary>Recorded execution evidence</summary>
            <textarea
              className="text-preview"
              readOnly
              rows={8}
              aria-label="Execution evidence JSON"
              value={JSON.stringify(run.evidence, null, 2).slice(0, 65536)}
            />
          </details>
          {run.evidence.retained !== true && (
            <p>
              This attempt has no retained output. Its recorded status and
              errors remain available.
            </p>
          )}
        </>
      )}
      {error && (
        <p role="alert">{error} The note and its reference were preserved.</p>
      )}
      {output && (
        <TypedOutputs
          key={output.id}
          result={output}
          initialPort={target.output_port ?? undefined}
        />
      )}
    </section>
  );
}
