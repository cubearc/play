import type { CatalogEntry, Execution, Exploration } from "./exploration-api";

export function ExplorationFlow({
  exploration,
  catalog,
  runs,
  selected,
  onSelect,
  disabled = false,
  pathStates = {},
  dirty = false,
}: {
  exploration: Exploration;
  catalog: CatalogEntry[];
  runs: Execution[];
  selected: string | null;
  onSelect: (id: string) => void;
  disabled?: boolean;
  pathStates?: Record<string, string>;
  dirty?: boolean;
}) {
  const width = Math.max(
    680,
    ...exploration.steps.map((s) => s.position[0] + 240),
  );
  const height = Math.max(
    240,
    ...exploration.steps.map((s) => s.position[1] + 130),
  );
  return (
    <section className="exploration-canvas" aria-label="Exploration flow">
      <div className="flow-surface" style={{ width, height }}>
        <svg
          width={width}
          height={height}
          aria-hidden="true"
          className="flow-edges"
        >
          <title>Connections between named operation ports</title>
          {exploration.steps.flatMap((step) =>
            Object.entries(step.inputs).flatMap(([name, source]) => {
              if (source.type !== "step") return [];
              const parent = exploration.steps.find(
                (s) => s.id === source.step,
              );
              if (!parent) return [];
              return (
                <path
                  key={`${step.id}:${name}`}
                  d={`M ${parent.position[0] + 200} ${parent.position[1] + 42} C ${parent.position[0] + 245} ${parent.position[1] + 42}, ${step.position[0] - 45} ${step.position[1] + 42}, ${step.position[0]} ${step.position[1] + 42}`}
                  fill="none"
                  stroke="#87b3a4"
                  strokeWidth="2"
                />
              );
            }),
          )}
        </svg>
        {exploration.steps.map((step, index) => {
          const contract = catalog.find(
            (e) => e.contract.operation === step.operation,
          )?.contract;
          return (
            <button
              type="button"
              disabled={disabled}
              key={step.id}
              className={`flow-node ${selected === step.id ? "selected" : ""}`}
              style={{ left: step.position[0], top: step.position[1] }}
              onClick={() => onSelect(step.id)}
              aria-pressed={selected === step.id}
            >
              <span className="flow-node-caption">
                Step {index + 1} ·{" "}
                {stepState(
                  exploration.revision,
                  step.id,
                  runs,
                  pathStates,
                  dirty,
                )}
              </span>
              <strong>{step.operation}</strong>
              <small>
                {contract?.inputs.map((p) => p.semantic_type).join(" + ") ||
                  "Parameters"}{" "}
                → {contract?.outputs.map((p) => p.semantic_type).join(" + ")}
              </small>
            </button>
          );
        })}
      </div>
    </section>
  );
}

export function stepState(
  revision: number,
  id: string,
  runs: Execution[],
  states: Record<string, string>,
  dirty: boolean,
): string {
  if (dirty) return "Changed";
  if (states[id]) return states[id];
  const run = runs.filter((r) => r.step === id).at(-1);
  return !run
    ? "Not run"
    : run.stale || run.configuration_revision !== revision
      ? "Changed"
      : run.status;
}
