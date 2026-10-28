import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
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
  const operation = executions.operationFor(trail);
  const attempts = executions.attempts[trail.id] ?? [];
  const running = attempts.at(-1)?.running ?? false;
  return (
    <div className="operation-cell">
      <div className="cell-code">
        <div className="cell-code-header">
          <span>In [{attempts.length || " "}]</span>
          <span>Pipeline · {pipeline?.title}</span>
        </div>
        <div className="cell-controls">
          <VscodeSingleSelect
            ref={selector}
            label="Operation"
            title={operation?.operation}
            value={operation?.operation ?? ""}
            disabled={running}
            onChange={() =>
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
            disabled={!operation || executions.busy}
            onClick={() => {
              if (operation) void executions.run(trail, operation);
            }}
          >
            Run cell
          </VscodeButton>
          {running ? (
            <VscodeButton
              secondary
              icon="debug-stop"
              onClick={() => void window.play.cancel().catch(() => {})}
            >
              Cancel
            </VscodeButton>
          ) : null}
        </div>
        <p className="binding">
          <span className="port-dot" /> {pipeline?.input} ← {trail.source.title}{" "}
          <span className="muted">@ r{trail.source.revision}</span>
        </p>
        <p className="cell-description">
          {operation?.description ??
            (executions.catalogError || "Connecting to Core…")}
        </p>
        {operation && operation.operation !== pipeline?.initialOperation ? (
          <small>Unsaved pipeline configuration · session only</small>
        ) : null}
      </div>
      <InlineResult attempts={attempts} />
    </div>
  );
}
