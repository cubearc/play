import { useRef } from "react";
import type { TrailView } from "../desktop/workspace-contract";
import { InlineResult } from "./InlineResult";
import type { Executions } from "./useExecutions";
import VscodeButton from "./VscodeButton";
import VscodeCheckbox from "./VscodeCheckbox";

export function TrailCell({
  trail,
  executions,
}: {
  trail: TrailView;
  executions: Executions;
}) {
  const pipeline = trail.pipeline;
  const keep = useRef<HTMLInputElement>(null);
  const operation = executions.operationFor(trail);
  const history = executions.history[trail.id];
  const running =
    history?.runs.some(
      (run) => run.status === "running" || run.status === "queued",
    ) ?? false;
  const cancel = running ? (
    <VscodeButton
      secondary
      icon="debug-stop"
      onClick={() => void executions.cancel(trail)}
    >
      Cancel
    </VscodeButton>
  ) : null;
  if (!pipeline)
    return (
      <>
        {cancel}
        <InlineResult trail={trail} executions={executions} />
      </>
    );
  return (
    <div className="operation-cell">
      <div className="cell-code">
        <div className="cell-code-header">
          <span>In [{history?.runs[0]?.sequence ?? " "}]</span>
          <span>Pipeline · {pipeline?.title}</span>
        </div>
        <div className="cell-controls">
          <strong className="operation-name" title={operation?.operation}>
            {pipeline.definition.nodes.length > 1
              ? `${pipeline.definition.nodes.length} nodes → ${operation?.name ?? "Unavailable operation"}`
              : (operation?.name ?? "Unavailable operation")}
          </strong>
          <VscodeButton
            icon="play"
            disabled={!operation || executions.isBusy(trail) || trail.archived}
            aria-disabled={
              !operation || executions.isBusy(trail) || trail.archived
                ? "true"
                : "false"
            }
            onClick={() => {
              if (operation && !trail.archived)
                void executions.run(trail, operation);
            }}
          >
            Run cell
          </VscodeButton>
          <VscodeButton
            secondary
            icon="terminal"
            disabled={!operation || executions.isBusy(trail) || trail.archived}
            onClick={() => {
              if (operation && !trail.archived)
                void executions.run(
                  trail,
                  operation,
                  pipeline.outputNodeId,
                  "cli",
                );
            }}
          >
            Run with Cube CLI
          </VscodeButton>
          {cancel}
        </div>
        <VscodeCheckbox
          ref={keep}
          checked={executions.keepOutput(trail)}
          disabled={running || trail.archived}
          onChange={() =>
            !trail.archived &&
            executions.setKeepOutput(trail, keep.current?.checked ?? true)
          }
        >
          Keep result in private history
        </VscodeCheckbox>
        <p className="binding">
          <span className="port-dot" /> source ← {trail.source.title}{" "}
          <span className="muted">@ r{trail.source.revision}</span>
        </p>
        <p className="cell-description">
          {operation?.description ??
            (executions.catalogError || "Connecting to Core…")}
        </p>
        {operation && trail.source.content.kind === "image" ? (
          <p className="binding">
            Saved parameters:{" "}
            <code>
              {JSON.stringify(
                pipeline.definition.nodes.find(
                  (n) => n.id === pipeline.outputNodeId,
                )?.parameters,
              )}
            </code>
          </p>
        ) : null}
      </div>
      <InlineResult trail={trail} executions={executions} />
    </div>
  );
}
