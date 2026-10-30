import { useState } from "react";
import type { RunDetail, TrailView } from "../desktop/workspace-contract";
import { CandidateButton } from "./Candidates";
import { attemptOutputType, formatResultValue } from "./ResultValue";
import type { Executions } from "./useExecutions";
import VscodeButton from "./VscodeButton";

function value(run: RunDetail): string {
  if (run.result?.code === "invalid_encoding")
    return "Failed: The input bytes are not valid UTF-8.";
  if (run.result?.code === "missing_field")
    return "Failed: The selected field or array index does not exist. Check the field path.";
  if (run.result?.code === "numeric_overflow")
    return "Failed: The integer exceeds the exact range. Use smaller operands.";
  if (run.result?.code === "invalid_number")
    return "Failed: An exact integer is required. Use Parse integer for text.";
  if (run.result?.code === "invalid_json")
    return "Failed: The source is not valid JSON. Check its syntax and duplicate keys.";
  if (run.result?.code === "invalid_pointer")
    return "Failed: The field path is not a valid JSON Pointer. Use / between fields.";
  if (run.result?.code === "invalid_type")
    return "Failed: The selected value is not text. Choose a text field.";
  if (run.result?.code === "contract_mismatch")
    return "Failed: A value does not match its input type. Choose a compatible field or an explicit conversion.";
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
            `${name}: ${formatResultValue(value, run.plan.contract.outputs.find((p) => p.name === name)?.semantic_type)}`,
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
  const input = run ? (run.plan.inputs.source ?? run.plan.inputs.input) : null;
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
          {run.status === "completed" && run.retainOutput
            ? run.attempts
                .filter(
                  (a) =>
                    run.plan.targetNodes.includes(a.nodeId) &&
                    a.status === "completed",
                )
                .flatMap((attempt) =>
                  Object.keys(attempt.result?.outputs ?? {}).map((port) => (
                    <CandidateButton
                      key={`${attempt.id}/${port}`}
                      label={port}
                      selection={{
                        kind: "run_output",
                        run_id: run.id,
                        attempt_id: attempt.id,
                        port,
                      }}
                    />
                  )),
                )
            : null}
          {run.status === "completed" &&
          run.retainOutput &&
          run.attempts.some(
            (a) =>
              run.plan.targetNodes.includes(a.nodeId) &&
              a.status === "completed",
          )
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
              <dt>Started by</dt>
              <dd>{run.caller === "cli" ? "Cube CLI" : "Desktop"}</dd>
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
                        ? `${input.assetId} @ r${input.revision}${input.used === false ? " (not consumed)" : ""}`
                        : JSON.stringify(input.literal)}
                    </code>
                  </div>
                ))}
              </dd>
              <dt>Attempts</dt>
              <dd>
                {run.attempts.length} actual node attempt(s) · reuse: none
                <ol>
                  {run.attempts.map((a) => (
                    <li key={a.id}>
                      {run.plan.nodes?.find((n) => n.id === a.nodeId)?.operation
                        .name ?? run.operation.name}{" "}
                      · {a.status}
                      {a.result?.outputs ? (
                        <pre>
                          {Object.entries(a.result.outputs)
                            .map(
                              ([port, v]) =>
                                `${port}: ${formatResultValue(v, attemptOutputType(run, a.nodeId, port))}`,
                            )
                            .join("\n")}
                        </pre>
                      ) : null}
                    </li>
                  ))}
                </ol>
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
