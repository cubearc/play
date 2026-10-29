import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeCheckbox from "@vscode-elements/react-elements/dist/components/VscodeCheckbox.js";
import { useRef } from "react";
import type { TrailView } from "../desktop/workspace-contract";
import { InlineResult } from "./InlineResult";
import type { Executions } from "./useExecutions";

export function OperationCell({
  trail,
  executions,
}: {
  trail: TrailView;
  executions: Executions;
}) {
  const pipeline = trail.pipeline;
  const keep = useRef<HTMLElementTagNameMap["vscode-checkbox"]>(null);
  const operation = executions.operationFor(trail);
  const history = executions.history[trail.id];
  const running =
    history?.runs.some(
      (run) => run.status === "running" || run.status === "queued",
    ) ?? false;
  return (
    <div className="operation-cell">
      <div className="cell-code">
        <div className="cell-code-header">
          <span>In [{history?.runs[0]?.sequence ?? " "}]</span>
          <span>Pipeline · {pipeline?.title}</span>
        </div>
        <div className="cell-controls">
          <strong className="operation-name" title={operation?.operation}>
            {operation?.name ?? "Unavailable operation"}
          </strong>
          <VscodeButton
            icon="play"
            disabled={!operation || executions.busy || trail.archived}
            aria-disabled={
              !operation || executions.busy || trail.archived ? "true" : "false"
            }
            onClick={() => {
              if (operation && !trail.archived)
                void executions.run(trail, operation);
            }}
          >
            Run cell
          </VscodeButton>
          {running ? (
            <VscodeButton
              secondary
              icon="debug-stop"
              onClick={() => void executions.cancel(trail)}
            >
              Cancel
            </VscodeButton>
          ) : null}
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
          <span className="port-dot" /> {pipeline?.input} ← {trail.source.title}{" "}
          <span className="muted">@ r{trail.source.revision}</span>
        </p>
        <p className="cell-description">
          {operation?.description ??
            (executions.catalogError || "Connecting to Core…")}
        </p>
        {operation && trail.source.content.kind === "image" ? (
          <p className="binding">
            Saved parameters:{" "}
            <code>{JSON.stringify(pipeline?.parameters)}</code>
          </p>
        ) : null}
      </div>
      <InlineResult trail={trail} executions={executions} />
    </div>
  );
}
