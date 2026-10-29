import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import { useState } from "react";
import type { RunDetail, TrailView } from "../desktop/workspace-contract";
import type { Executions } from "./useExecutions";

function value(run: RunDetail): string {
  if (run.status === "queued" || run.status === "running")
    return `${run.status === "queued" ? "Queued" : "Running"}…`;
  if (run.status !== "completed")
    return `${run.status[0]?.toUpperCase()}${run.status.slice(1)}${run.result?.code ? `: ${run.result.code}` : ""}`;
  if (run.result?.replayGap) return "Completed · output was not retained.";
  const outputs = run.result?.outputs;
  return outputs
    ? Object.entries(outputs)
        .map(
          ([name, value]) =>
            `${name}: ${typeof value === "string" ? value : `Base64 ${value.base64}`}`,
        )
        .join("\n")
    : "Output unavailable.";
}
export function InlineResult({
  trail,
  executions,
}: {
  trail: TrailView;
  executions: Executions;
}) {
  const [expanded, setExpanded] = useState(false);
  const history = executions.history[trail.id];
  if (!history) return null;
  const run = history.detail?.id === history.selected ? history.detail : null;
  const input = run
    ? Object.values(run.plan.inputs).find((value) => "assetId" in value)
    : null;
  const earlier =
    input &&
    "assetId" in input &&
    (input.assetId !== trail.source.id ||
      input.revision !== trail.source.revision);
  return (
    <>
      {history.error ? <p role="alert">{history.error}</p> : null}
      {run ? (
        <section className="inline-result" aria-label="Cell result">
          <div className="cell-result-label">
            <span>Out [{run.sequence}]</span>
            <span>
              {run.operation.name} · {run.status} · v
              {run.operation.operationVersion}
            </span>
          </div>
          <pre role="status">{value(run)}</pre>
          {run.status === "completed" &&
          run.retainOutput &&
          run.attempts.length === 1 &&
          run.attempts[0]?.status === "completed"
            ? Object.keys(run.result?.outputs ?? {})
                .filter((port) =>
                  run.plan.contract.outputs.some(
                    (output) =>
                      output.name === port &&
                      ["text", "bytes"].includes(output.semantic_type),
                  ),
                )
                .map((port) => {
                  const saved = executions.outputSaved(run, port);
                  return (
                    <div className="result-actions" key={port}>
                      <span>{port}</span>
                      <VscodeButton
                        secondary
                        icon={saved ? "check" : "notebook"}
                        disabled={executions.saveBusy || saved}
                        aria-disabled={
                          executions.saveBusy || saved ? "true" : "false"
                        }
                        aria-label={`${saved ? "Saved" : "Save"} ${port} as block`}
                        onClick={() =>
                          void executions.saveOutput(trail, run, port)
                        }
                      >
                        {saved ? "Saved as block" : "Save as block"}
                      </VscodeButton>
                    </div>
                  );
                })
            : null}
          {earlier && input && "assetId" in input ? (
            <p className="input-revision" role="note">
              Earlier input · r{input.revision}. The current source is r
              {trail.source.revision}; run again to use it.
            </p>
          ) : null}
          {run.pipelineId !== trail.pipeline?.id ||
          run.pipelineRevision !== trail.pipeline.revision ? (
            <p className="input-revision" role="note">
              Earlier Pipeline · r{run.pipelineRevision}. This result preserves
              its accepted configuration.
            </p>
          ) : null}
          <details
            open={expanded}
            onToggle={(event) => setExpanded(event.currentTarget.open)}
          >
            <summary>
              Saved Run & input evidence ({history.runs.length})
            </summary>
            <dl className="run-evidence">
              <dt>Run</dt>
              <dd>
                <code>{run.id}</code>
              </dd>
              <dt>Pipeline</dt>
              <dd>
                <code>
                  {run.pipelineId} @ r{run.pipelineRevision}
                </code>
              </dd>
              <dt>Inputs</dt>
              <dd>
                {Object.entries(run.plan.inputs).map(([port, input]) => (
                  <div key={port}>
                    <code>
                      {port} ←{" "}
                      {"assetId" in input
                        ? `${input.assetId} @ r${input.revision}`
                        : JSON.stringify(input.literal)}
                    </code>
                  </div>
                ))}
              </dd>
              <dt>Attempts</dt>
              <dd>
                {run.attempts.length} actual node attempt(s) · reuse: none
              </dd>
            </dl>
            <ol>
              {history.runs.map((record) => (
                <li key={record.id}>
                  <button
                    type="button"
                    aria-label={`Open Run ${record.sequence}`}
                    aria-current={
                      history.selected === record.id ? "true" : undefined
                    }
                    onClick={() => void executions.show(trail, record.id)}
                  >
                    {record.operation.name} · {record.status} · Run{" "}
                    {record.sequence}
                  </button>
                </li>
              ))}
            </ol>
            {history.before ? (
              <button
                type="button"
                onClick={() => void executions.load(trail, true)}
              >
                Load older Runs
              </button>
            ) : null}
          </details>
        </section>
      ) : history.loading ? (
        <p role="status">Loading saved Run…</p>
      ) : null}
    </>
  );
}
