import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import { useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { Operation } from "../desktop/contract";
import type { PipelineDefinition } from "../desktop/workspace-contract";
export function PipelineGraph({
  definition,
  operations,
  selected,
  onSelect,
  onMove,
  onRun,
  busy = false,
}: {
  definition: PipelineDefinition;
  operations: Operation[];
  selected?: string;
  onRun?: (id: string) => void;
  busy?: boolean;
  onSelect?: ((id: string) => void) | undefined;
  onMove?: ((id: string, x: number, y: number) => void) | undefined;
}) {
  const [inspected, setInspected] = useState<string>();
  const active = selected ?? inspected;
  const detail = definition.nodes.find((n) => n.id === inspected);
  const drag = useRef<{
    id: string;
    x: number;
    y: number;
    px: number;
    py: number;
  } | null>(null);
  const width = Math.max(
    900,
    ...definition.nodes.map((n) => n.position.x + 270),
  );
  const height = Math.max(
    280,
    ...definition.nodes.map((n) => n.position.y + 120),
  );
  return (
    <section className="pipeline-graph-scroll" aria-label="Pipeline graph">
      <div className="pipeline-graph" style={{ width, height }}>
        <svg width={width} height={height} aria-hidden="true">
          <title>Connections</title>
          {definition.nodes.flatMap((n) =>
            Object.entries(n.inputs).map(([port, source]) => {
              if (source.kind !== "output") return null;
              const from = definition.nodes.find((p) => p.id === source.nodeId);
              if (!from) return null;
              const x = from.position.x + 240,
                y = from.position.y + 45,
                x2 = n.position.x,
                y2 = n.position.y + 45;
              return (
                <path
                  key={`${n.id}:${port}`}
                  d={`M${x} ${y} C${x + 60} ${y},${x2 - 60} ${y2},${x2} ${y2}`}
                />
              );
            }),
          )}
        </svg>
        {definition.nodes.map((node, index) => (
          <button
            type="button"
            key={node.id}
            className={`graph-node${active === node.id ? " selected" : ""}`}
            style={{ left: node.position.x, top: node.position.y }}
            aria-label={`Node ${index + 1}: ${operations.find((o) => o.operation === node.operation)?.name ?? node.operation}`}
            aria-pressed={active === node.id}
            onClick={() =>
              onSelect ? onSelect(node.id) : setInspected(node.id)
            }
            onPointerDown={(e) => {
              if (!onMove || e.button !== 0) return;
              drag.current = {
                id: node.id,
                x: node.position.x,
                y: node.position.y,
                px: e.clientX,
                py: e.clientY,
              };
              e.currentTarget.setPointerCapture(e.pointerId);
            }}
            onPointerMove={(e) => {
              const d = drag.current;
              if (d?.id === node.id)
                onMove?.(
                  node.id,
                  Math.max(
                    0,
                    Math.min(
                      limits.pipeline_canvas_limit,
                      Math.round(d.x + e.clientX - d.px),
                    ),
                  ),
                  Math.max(
                    0,
                    Math.min(
                      limits.pipeline_canvas_limit,
                      Math.round(d.y + e.clientY - d.py),
                    ),
                  ),
                );
            }}
            onPointerUp={() => {
              drag.current = null;
            }}
            onPointerCancel={() => {
              drag.current = null;
            }}
            onKeyDown={(e) => {
              const delta: { [key: string]: [number, number] } = {
                ArrowLeft: [-20, 0],
                ArrowRight: [20, 0],
                ArrowUp: [0, -20],
                ArrowDown: [0, 20],
              };
              const d = delta[e.key];
              if (d && onMove) {
                e.preventDefault();
                onMove(
                  node.id,
                  Math.max(
                    0,
                    Math.min(
                      limits.pipeline_canvas_limit,
                      node.position.x + d[0],
                    ),
                  ),
                  Math.max(
                    0,
                    Math.min(
                      limits.pipeline_canvas_limit,
                      node.position.y + d[1],
                    ),
                  ),
                );
              }
            }}
          >
            <span className="graph-node-number">{index + 1}</span>
            <strong>
              {operations.find((o) => o.operation === node.operation)?.name ??
                node.operation}
            </strong>
            <small>
              {Object.keys(node.inputs).join(" · ")} →{" "}
              {operations
                .find((o) => o.operation === node.operation)
                ?.contract.outputs.map((p) => p.name)
                .join(" · ")}
            </small>
            {definition.output === node.id ? (
              <span className="badge">Pipeline output</span>
            ) : null}
          </button>
        ))}
      </div>
      {detail && !onSelect ? (
        <aside className="graph-node-detail" aria-label="Node details">
          <strong>
            {operations.find((o) => o.operation === detail.operation)?.name}
          </strong>
          {onRun ? (
            <VscodeButton
              icon="play"
              disabled={busy}
              onClick={() => onRun(detail.id)}
            >
              Run node
            </VscodeButton>
          ) : null}
          <pre>
            {JSON.stringify(
              { inputs: detail.inputs, parameters: detail.parameters },
              null,
              2,
            )}
          </pre>
        </aside>
      ) : null}
    </section>
  );
}
