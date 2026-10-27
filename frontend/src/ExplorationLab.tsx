import { useEffect, useId, useRef, useState } from "react";
import { CaptureImport } from "./CaptureImport";
import { ExplorationFlow, stepState } from "./ExplorationFlow";
import { InputField } from "./ExplorationInput";
import { ExplorationScope } from "./ExplorationScope";
import {
  type CatalogEntry,
  type Execution,
  type Exploration,
  executeStep,
  executions,
  type Json,
  catalog as loadCatalog,
  explorations as loadExplorations,
  type OperationResult,
  openExecution,
  type Schema,
  type Step,
  saveExploration,
} from "./exploration-api";
import type { EvidenceTarget } from "./notebook-api";
import { OperationParameters } from "./OperationParameters";
import { TypedOutputs } from "./TypedOutputs";
import type { Store } from "./workspace-api";

export function ExplorationLab({
  asset,
  onAnnotate,
  onPreserve,
}: {
  asset: { store: Store; path: string } | null;
  onAnnotate?: ((target: EvidenceTarget) => void) | undefined;
  onPreserve?: ((id: string, port: string) => void) | undefined;
}) {
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [items, setItems] = useState<Exploration[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      loadCatalog(controller.signal),
      loadExplorations(controller.signal),
    ])
      .then(([catalog, items]) => {
        if (controller.signal.aborted) return;
        setCatalog(catalog);
        setItems(items);
        setSelected(items[0]?.id ?? null);
        setLoading(false);
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          setError(message(error));
          setLoading(false);
        }
      });
    return () => controller.abort();
  }, []);
  function saved(value: Exploration) {
    setItems((items) => [value, ...items.filter((e) => e.id !== value.id)]);
    setSelected(value.id);
  }
  async function create() {
    setCreating(true);
    setError(null);
    try {
      saved(
        await saveExploration({
          schema_version: 1,
          id: crypto.randomUUID(),
          revision: 0,
          title: `Exploration ${items.length + 1}`,
          hypothesis: "",
          scope: null,
          steps: [],
        }),
      );
    } catch (error) {
      setError(message(error));
    } finally {
      setCreating(false);
    }
  }
  const exploration = items.find((e) => e.id === selected);
  return (
    <section className="exploration-lab" aria-labelledby="explorations-title">
      <header className="exploration-toolbar">
        <div>
          <p className="eyebrow">VISUAL INVESTIGATION</p>
          <h2 id="explorations-title">Explorations</h2>
        </div>
        <button
          type="button"
          className="button"
          onClick={() => void create()}
          disabled={creating || loading || editing}
        >
          {creating ? "Creating…" : "New exploration"}
        </button>
      </header>
      {error && <p role="alert">{error}</p>}
      {asset && (
        <CaptureImport key={`${asset.store}:${asset.path}`} asset={asset} />
      )}
      {loading ? (
        <p role="status">Loading operation contracts and explorations…</p>
      ) : (
        <div className="exploration-layout">
          <nav className="exploration-list" aria-label="Saved explorations">
            {items.length === 0 ? (
              <p>
                Start a hypothesis, connect operations and preserve the results.
                A challenge is optional.
              </p>
            ) : (
              items.map((item) => (
                <button
                  type="button"
                  key={item.id}
                  aria-current={selected === item.id ? "true" : undefined}
                  disabled={creating || editing}
                  onClick={() => setSelected(item.id)}
                >
                  {item.title}
                  <small>
                    {item.steps.length} steps ·{" "}
                    {item.scope ? "Local stage" : "Free exploration"}
                  </small>
                </button>
              ))
            )}
          </nav>
          {creating ? (
            <p role="status">Creating exploration…</p>
          ) : exploration ? (
            <Editor
              key={exploration.id}
              initial={exploration}
              catalog={catalog}
              asset={asset}
              onSaved={saved}
              onEditingChange={setEditing}
              onAnnotate={onAnnotate}
              onPreserve={onPreserve}
            />
          ) : (
            <p className="exploration-empty">
              Your private investigation starts here.
            </p>
          )}
        </div>
      )}
    </section>
  );
}
function message(error: unknown): string {
  return error instanceof Error
    ? error.message
    : "The exploration could not be saved or executed.";
}
function properties(schema: Schema): Record<string, Json> {
  const value = schema.properties;
  return value && typeof value === "object" && !Array.isArray(value)
    ? value
    : {};
}
function initialStep(entry: CatalogEntry, index: number): Step {
  const parameters: Record<string, Json> = {};
  for (const [name, schema] of Object.entries(
    properties(entry.contract.parameters),
  ))
    if (
      schema &&
      typeof schema === "object" &&
      !Array.isArray(schema) &&
      schema.default !== undefined &&
      !entry.contract.policy.sensitive_parameters.includes(name)
    )
      parameters[name] = schema.default;
  return {
    id: crypto.randomUUID(),
    operation: entry.contract.operation,
    operation_version: entry.contract.operation_version,
    inputs: {},
    parameters,
    private_parameters: entry.contract.policy.sensitive_parameters,
    position: [24 + (index % 3) * 250, 24 + Math.floor(index / 3) * 140],
  };
}
function Editor({
  initial,
  catalog,
  asset,
  onSaved,
  onEditingChange,
  onAnnotate,
  onPreserve,
}: {
  initial: Exploration;
  catalog: CatalogEntry[];
  asset: { store: Store; path: string } | null;
  onSaved: (value: Exploration) => void;
  onEditingChange: (editing: boolean) => void;
  onAnnotate?: ((target: EvidenceTarget) => void) | undefined;
  onPreserve?: ((id: string, port: string) => void) | undefined;
}) {
  const formId = useId();
  const [draft, setDraft] = useState(initial);
  const [selected, setSelected] = useState<string | null>(
    initial.steps[0]?.id ?? null,
  );
  const [mode, setMode] = useState<"flow" | "steps">("flow");
  const [runs, setRuns] = useState<Execution[]>([]);
  const [query, setQuery] = useState("");
  const [operation, setOperation] = useState("");
  const [privateValues, setPrivateValues] = useState<Record<string, Json>>({});
  const [privateInputs, setPrivateInputs] = useState<Record<string, Json>>({});
  const [retainPrivateOutput, setRetainPrivateOutput] = useState(false);
  const [invalidFields, setInvalidFields] = useState<Record<string, boolean>>(
    {},
  );
  const [output, setOutput] = useState<OperationResult | null>(null);
  const [running, setRunning] = useState(false);
  const [pathStates, setPathStates] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState("Saved locally");
  const active = useRef<AbortController | null>(null);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    const controller = new AbortController();
    executions(initial.id, controller.signal)
      .then(setRuns)
      .catch((error) => {
        if (!controller.signal.aborted) setError(message(error));
      });
    return () => {
      mounted.current = false;
      controller.abort();
      active.current?.abort();
    };
  }, [initial.id]);
  const step = draft.steps.find((s) => s.id === selected);
  const entry = catalog.find((e) => e.contract.operation === step?.operation);
  const dirty = JSON.stringify(draft) !== JSON.stringify(initial);
  useEffect(() => {
    if (!dirty && !busy) return;
    const guard = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", guard);
    return () => window.removeEventListener("beforeunload", guard);
  }, [dirty, busy]);
  useEffect(() => {
    onEditingChange(dirty || busy);
    return () => onEditingChange(false);
  }, [dirty, busy, onEditingChange]);
  const invalid = Object.values(invalidFields).some(Boolean);
  function validity(name: string, valid: boolean) {
    setInvalidFields((fields) => ({ ...fields, [name]: !valid }));
  }
  function updateStep(update: (step: Step) => Step) {
    setDraft((draft) => ({
      ...draft,
      steps: draft.steps.map((s) => (s.id === selected ? update(s) : s)),
    }));
    setStatus("Unsaved changes");
    setOutput(null);
  }
  async function save() {
    setBusy(true);
    setError(null);
    try {
      const saved = await saveExploration(draft);
      setDraft(saved);
      setPathStates({});
      onSaved(saved);
      setStatus("Configuration saved locally");
    } catch (error) {
      setError(message(error));
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  function add() {
    const entry = catalog.find(
      (e) => e.contract.operation === operation && e.execution.available,
    );
    if (!entry) return;
    const step = initialStep(entry, draft.steps.length);
    setDraft((draft) => ({ ...draft, steps: [...draft.steps, step] }));
    setSelected(step.id);
    setPrivateValues({});
    setPrivateInputs({});
    setRetainPrivateOutput(false);
    setStatus("Unsaved changes");
  }
  async function run() {
    if (!step || busy) return;
    if (dirty) {
      setError(
        "Save the configuration before running. Earlier results remain available in history.",
      );
      return;
    }
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setRunning(true);
    setError(null);
    setOutput(null);
    const deadline = window.setTimeout(() => controller.abort(), 60_000);
    const queued: Record<string, string> = {};
    function queue(current: Step) {
      if (queued[current.id]) return;
      const cached =
        current.id !== step?.id &&
        runs.some(
          (r) =>
            r.step === current.id &&
            r.configuration_revision === draft.revision &&
            r.status === "succeeded" &&
            typeof r.evidence.result_id === "string",
        );
      queued[current.id] = cached ? "succeeded" : "queued";
      if (cached) return;
      for (const source of Object.values(current.inputs))
        if (source.type === "step") {
          const producer = draft.steps.find((s) => s.id === source.step);
          if (producer) queue(producer);
        }
    }
    queue(step);
    setPathStates(queued);
    let history = runs;
    let selectedRetained = false;
    const visited = new Set<string>();
    async function execute(current: Step): Promise<void> {
      if (visited.has(current.id)) return;
      visited.add(current.id);
      if (
        current.id !== step?.id &&
        catalog.find((e) => e.contract.operation === current.operation)
          ?.contract.policy.private_output
      )
        throw new Error(
          "Run the upstream private-output step explicitly and choose whether to retain its result before chaining it.",
        );
      if (
        current.id !== step?.id &&
        (current.private_parameters.length ||
          catalog.find((e) => e.contract.operation === current.operation)
            ?.contract.policy.private_input)
      )
        throw new Error(
          "Run the upstream step with its private inputs and parameters first.",
        );
      for (const source of Object.values(current.inputs))
        if (source.type === "step") {
          const producer = draft.steps.find((s) => s.id === source.step);
          if (!producer) throw new Error("A connected step is missing.");
          if (
            !history.some(
              (r) =>
                r.step === producer.id &&
                r.configuration_revision === draft.revision &&
                r.status === "succeeded",
            )
          )
            await execute(producer);
        }
      if (controller.signal.aborted) throw new Error("Execution cancelled.");
      setPathStates((states) => ({ ...states, [current.id]: "running" }));
      setStatus(`Running ${current.operation}…`);
      const result = await executeStep(
        draft,
        current,
        history,
        current.id === step?.id ? privateValues : {},
        controller.signal,
        current.id === step?.id && retainPrivateOutput,
        current.id === step?.id ? privateInputs : {},
      );
      setPathStates((states) => ({ ...states, [current.id]: "succeeded" }));
      history = await executions(draft.id, controller.signal);
      if (mounted.current) {
        setRuns(history);
        if (current.id === step?.id) {
          setOutput(result);
          selectedRetained = result.record.retained;
        }
      }
    }
    try {
      await execute(step);
      if (mounted.current)
        setStatus(
          selectedRetained
            ? "Execution and output preserved locally"
            : "Execution recorded locally. Private output was not retained.",
        );
    } catch (error) {
      if (mounted.current) {
        setPathStates((states) =>
          Object.fromEntries(
            Object.entries(states).map(([id, state]) => [
              id,
              state === "succeeded"
                ? state
                : controller.signal.aborted
                  ? "cancelled"
                  : state === "running"
                    ? "failed"
                    : "not run",
            ]),
          ),
        );
        setError(
          controller.signal.aborted
            ? "Execution cancelled; prior results were preserved."
            : message(error),
        );
        executions(draft.id)
          .then(setRuns)
          .catch(() => {});
      }
    } finally {
      window.clearTimeout(deadline);
      active.current = null;
      if (mounted.current) {
        setBusy(false);
        setRunning(false);
        setPrivateValues({});
        setPrivateInputs({});
        setRetainPrivateOutput(false);
      }
    }
  }
  async function open(run: Execution) {
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    setOutput(null);
    try {
      const result = await openExecution(run, controller.signal);
      if (mounted.current) {
        setOutput(result);
        setStatus(
          `Viewing execution from configuration ${run.configuration_revision}`,
        );
      }
    } catch (error) {
      if (mounted.current) setError(message(error));
    } finally {
      active.current = null;
      if (mounted.current) setBusy(false);
    }
  }
  const matches = catalog.filter((e) =>
    `${e.contract.operation} ${e.contract.description}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
  const chosenOperation = catalog.find(
    (e) => e.contract.operation === operation,
  );
  const outputTypes = new Set(
    entry?.contract.outputs.map((p) => p.semantic_type) ?? [],
  );
  const compatibleMatches = matches.filter((e) =>
    e.contract.inputs.some(
      (p) =>
        outputTypes.has(p.semantic_type) ||
        (p.semantic_type === "byte_reader" && outputTypes.has("bytes")),
    ),
  );
  const otherMatches = matches.filter((e) => !compatibleMatches.includes(e));
  return (
    <div className="exploration-editor" aria-busy={busy}>
      <div className="exploration-heading">
        <label>
          Exploration name
          <input
            maxLength={512}
            value={draft.title}
            disabled={busy}
            onChange={(event) =>
              setDraft({ ...draft, title: event.target.value })
            }
          />
        </label>
        <button
          type="button"
          className="button secondary"
          disabled={busy || !dirty || invalid}
          onClick={() => void save()}
        >
          Save configuration
        </button>
        {dirty && (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() => {
              setDraft(initial);
              setSelected(initial.steps[0]?.id ?? null);
              setInvalidFields({});
              setPrivateValues({});
              setPrivateInputs({});
              setRetainPrivateOutput(false);
              setError(null);
              setStatus("Restored the last saved configuration");
            }}
          >
            Discard configuration edits
          </button>
        )}
      </div>
      <div className="hypothesis">
        <ExplorationScope
          scope={draft.scope}
          disabled={busy}
          onChange={(scope) => setDraft({ ...draft, scope })}
        />
        <label htmlFor={`${formId}-hypothesis`}>Hypothesis</label>
        <textarea
          id={`${formId}-hypothesis`}
          rows={2}
          maxLength={8192}
          value={draft.hypothesis}
          disabled={busy}
          onChange={(event) =>
            setDraft({ ...draft, hypothesis: event.target.value })
          }
        />
      </div>
      <div className="operation-menu">
        <label>
          Find an operation
          <input
            value={query}
            disabled={busy}
            onChange={(event) => {
              setQuery(event.target.value);
              setOperation("");
            }}
            placeholder="Search by name or purpose"
          />
        </label>
        <label>
          Operation
          <select
            value={operation}
            disabled={busy}
            onChange={(event) => setOperation(event.target.value)}
          >
            <option value="">Choose an operation…</option>
            {compatibleMatches.length > 0 && (
              <optgroup label="Compatible with selected step outputs">
                {compatibleMatches.map((e) => (
                  <option
                    key={e.contract.operation}
                    value={e.contract.operation}
                  >
                    {e.contract.operation}
                    {e.execution.available ? "" : " · unavailable"}
                  </option>
                ))}
              </optgroup>
            )}
            <optgroup label="All other operations">
              {otherMatches.map((e) => (
                <option key={e.contract.operation} value={e.contract.operation}>
                  {e.contract.operation}
                  {e.execution.available ? "" : " · unavailable"}
                </option>
              ))}
            </optgroup>
          </select>
        </label>
        <button
          type="button"
          className="button secondary"
          disabled={
            busy ||
            !operation ||
            chosenOperation?.execution.available !== true ||
            draft.steps.length >= 64
          }
          onClick={add}
        >
          Add step
        </button>
      </div>
      {chosenOperation && !chosenOperation.execution.available && (
        <p role="status">
          {chosenOperation.execution.reason ??
            "This operation is unavailable in the current runtime."}
        </p>
      )}
      {entry && (
        <p>
          Suggestions match declared named-port types. Actual formats,
          dimensions and processing limits are checked when the operation runs.
        </p>
      )}
      <div className="exploration-toolbar">
        <fieldset className="view-switch" aria-label="Exploration view">
          <button
            type="button"
            aria-pressed={mode === "flow"}
            onClick={() => setMode("flow")}
          >
            Flow
          </button>
          <button
            type="button"
            aria-pressed={mode === "steps"}
            onClick={() => setMode("steps")}
          >
            Steps
          </button>
        </fieldset>
        <span role="status">{dirty ? "Unsaved changes" : status}</span>
      </div>
      {draft.steps.length === 0 ? (
        <p className="exploration-empty">
          Add an operation and choose its inputs to begin.
        </p>
      ) : mode === "flow" ? (
        <ExplorationFlow
          exploration={draft}
          catalog={catalog}
          runs={runs}
          pathStates={pathStates}
          dirty={dirty}
          selected={selected}
          disabled={busy}
          onSelect={(id) => {
            setSelected(id);
            setInvalidFields({});
            setPrivateValues({});
            setPrivateInputs({});
            setRetainPrivateOutput(false);
            setOutput(null);
          }}
        />
      ) : (
        <ol className="step-list" aria-label="Exploration steps">
          {draft.steps.map((s) => (
            <li key={s.id}>
              <button
                type="button"
                aria-pressed={selected === s.id}
                disabled={busy}
                onClick={() => {
                  setSelected(s.id);
                  setInvalidFields({});
                  setPrivateValues({});
                  setPrivateInputs({});
                  setRetainPrivateOutput(false);
                  setOutput(null);
                }}
              >
                {s.operation}
              </button>
              <span aria-live="polite">
                {stepState(draft.revision, s.id, runs, pathStates, dirty)}
              </span>
            </li>
          ))}
        </ol>
      )}
      {step && entry && (
        <div className="step-inspector" key={step.id}>
          <h3>{step.operation}</h3>
          <p>{entry.contract.description}</p>
          <fieldset disabled={busy}>
            <legend>Named inputs</legend>
            {entry.contract.inputs.map((port) => (
              <InputField
                key={port.name}
                port={port}
                step={step}
                exploration={draft}
                catalog={catalog}
                asset={asset}
                onValidityChange={(valid) =>
                  validity(`input:${port.name}`, valid)
                }
                privateValue={
                  entry.contract.policy.private_input
                    ? privateInputs[port.name]
                    : undefined
                }
                privateInput={entry.contract.policy.private_input}
                onChange={(value) => {
                  if (
                    entry.contract.policy.private_input &&
                    value.type === "literal"
                  ) {
                    if (step.inputs[port.name])
                      updateStep((step) => ({
                        ...step,
                        inputs: Object.fromEntries(
                          Object.entries(step.inputs).filter(
                            ([name]) => name !== port.name,
                          ),
                        ),
                      }));
                    setPrivateInputs((values) => ({
                      ...values,
                      [port.name]: value.value,
                    }));
                  } else {
                    setPrivateInputs((values) =>
                      Object.fromEntries(
                        Object.entries(values).filter(
                          ([name]) => name !== port.name,
                        ),
                      ),
                    );
                    updateStep((step) => ({
                      ...step,
                      inputs: { ...step.inputs, [port.name]: value },
                    }));
                  }
                }}
              />
            ))}
          </fieldset>
          <fieldset disabled={busy}>
            <legend>Parameters</legend>
            <OperationParameters
              contract={entry.contract}
              values={step.parameters}
              privateValues={privateValues}
              onChange={(parameters) =>
                updateStep((step) => ({ ...step, parameters }))
              }
              onPrivateChange={setPrivateValues}
              onValidityChange={(name, valid) =>
                validity(`parameter:${name}`, valid)
              }
            />
          </fieldset>
          {entry.contract.policy.private_output && (
            <fieldset disabled={busy} className="private-output-choice">
              <legend>Private output for the next run</legend>
              <label>
                <input
                  type="checkbox"
                  checked={retainPrivateOutput}
                  onChange={(event) =>
                    setRetainPrivateOutput(event.target.checked)
                  }
                />
                Keep this private output for this run
              </label>
              <p>
                Leave unchecked to view the result without retaining its
                content. Keeping it locally enables chaining, notes on a
                selected output and private solutions. This choice resets after
                each attempt and never saves private parameters.
              </p>
            </fieldset>
          )}
          <div className="exploration-toolbar">
            <button
              type="button"
              className="button"
              disabled={busy || dirty || invalid}
              onClick={() => void run()}
            >
              Run selected path
            </button>
            {running && (
              <button
                type="button"
                className="button secondary"
                onClick={() => active.current?.abort()}
              >
                Cancel execution
              </button>
            )}
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => {
                setDraft({
                  ...draft,
                  steps: draft.steps
                    .filter((s) => s.id !== step.id)
                    .map((s) => ({
                      ...s,
                      inputs: Object.fromEntries(
                        Object.entries(s.inputs).filter(
                          ([, source]) =>
                            source.type !== "step" || source.step !== step.id,
                        ),
                      ),
                    })),
                });
                setSelected(null);
                setInvalidFields({});
              }}
            >
              Remove step
            </button>
          </div>
          {entry.contract.policy.sensitive_parameters.length > 0 && (
            <p>
              Private parameters are used for this run and must be entered again
              to replay it.
            </p>
          )}
          {output !== null && (
            <TypedOutputs
              key={output.id}
              result={output}
              onPreserve={
                onPreserve ? (port) => onPreserve(output.id, port) : undefined
              }
              onAnnotate={
                onAnnotate &&
                runs.some((r) => r.evidence.result_id === output.id)
                  ? (port) => {
                      const run = runs.find(
                        (r) => r.evidence.result_id === output.id,
                      );
                      if (run)
                        onAnnotate({ execution_id: run.id, output_port: port });
                    }
                  : undefined
              }
            />
          )}
          <details>
            <summary>
              Execution history ·{" "}
              {runs.filter((r) => r.step === step.id).length}
            </summary>
            <ol>
              {runs
                .filter((r) => r.step === step.id)
                .map((run) => (
                  <li key={run.id}>
                    {onAnnotate && (
                      <button
                        type="button"
                        className="button secondary"
                        onClick={() =>
                          onAnnotate({
                            execution_id: run.id,
                            output_port: null,
                          })
                        }
                      >
                        Annotate execution
                      </button>
                    )}
                    {run.status}
                    {run.stale || run.configuration_revision !== draft.revision
                      ? " · earlier configuration"
                      : ""}{" "}
                    · {new Date(run.started_ms).toLocaleString()}
                    <code>{run.id}</code>
                    {run.status === "succeeded" &&
                    typeof run.evidence.result_id === "string" ? (
                      <button
                        type="button"
                        className="button secondary"
                        disabled={busy}
                        onClick={() => void open(run)}
                      >
                        View retained output
                      </button>
                    ) : run.status === "succeeded" ? (
                      <span> · Private output was not retained</span>
                    ) : null}
                  </li>
                ))}
            </ol>
          </details>
        </div>
      )}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
