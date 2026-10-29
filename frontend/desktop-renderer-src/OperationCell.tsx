import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeCheckbox from "@vscode-elements/react-elements/dist/components/VscodeCheckbox.js";
import VscodeOption from "@vscode-elements/react-elements/dist/components/VscodeOption.js";
import VscodeSingleSelect from "@vscode-elements/react-elements/dist/components/VscodeSingleSelect.js";
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
  const selector = useRef<HTMLElementTagNameMap["vscode-single-select"]>(null);
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
          <VscodeSingleSelect
            ref={selector}
            label="Operation"
            title={operation?.operation}
            value={operation?.operation ?? ""}
            disabled={running || trail.archived}
            onChange={() =>
              !trail.archived &&
              executions.selectOperation(trail, selector.current?.value ?? "")
            }
          >
            {executions.operations.map((entry) => (
              <VscodeOption key={entry.operation} value={entry.operation}>
                {entry.name}
              </VscodeOption>
            ))}
          </VscodeSingleSelect>
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
        {operation && operation.operation !== pipeline?.initialOperation ? (
          <small>
            Selected operation will be frozen in this Run’s Pipeline definition
          </small>
        ) : null}
      </div>
      <InlineResult trail={trail} executions={executions} />
    </div>
  );
}
