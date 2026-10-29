import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { Operation } from "../desktop/contract";
import type {
  AssetView,
  GraphNode,
  PipelineDefinition,
  PipelineView,
  TrailView,
} from "../desktop/workspace-contract";
import {
  type Bindings,
  BindingsEditor,
  bindingRefs,
  completeBindings,
  selectedBindings,
} from "./BindingsEditor";
import { GraphInputs, initialInputs, Select } from "./GraphInputs";
import { PipelineGraph } from "./PipelineGraph";
import { PipelineInputs } from "./PipelineInputs";
export type PipelineMode = "create" | "revise" | "fork";
export interface PipelineDraft {
  title: string;
  definition: PipelineDefinition;
  key: string;
  bindingKey: string;
  bindings: Bindings;
}
export function PipelineEditor({
  mode,
  pipeline,
  operations,
  inputType,
  trail,
  assets,
  error,
  onSave,
  onClose,
  onDirty,
}: {
  mode: PipelineMode;
  pipeline: PipelineView | null;
  operations: Operation[];
  inputType: string;
  trail: TrailView;
  assets: AssetView[];
  error: string;
  onSave: (draft: PipelineDraft) => Promise<boolean>;
  onClose: () => void;
  onDirty: (dirty: boolean) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleField = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const paramsField = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const separatorField = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const [title, setTitle] = useState(
    mode === "fork"
      ? `${pipeline?.title} (copy)`
      : mode === "revise"
        ? (pipeline?.title ?? "")
        : "New Pipeline",
  );
  const [definition, setDefinition] = useState<PipelineDefinition>(() => {
    if (pipeline && mode !== "create")
      return structuredClone(pipeline.definition);
    const op = operations.find((o) => o.input_type === inputType);
    if (!op) throw new Error("catalog_unavailable");
    const id = crypto.randomUUID();
    return {
      formatVersion: 4,
      inputs: [{ name: "source", semanticType: inputType }],
      nodes: [
        {
          id,
          operation: op.operation,
          operationVersion: op.operation_version,
          parameters: op.parameters,
          inputs: initialInputs(op, [
            { name: "source", semanticType: inputType },
          ]),
          position: { x: 40, y: 60 },
        },
      ],
      output: id,
    };
  });
  const [bindings, setBindings] = useState<Bindings>(() =>
    mode === "create" ? {} : bindingRefs(trail),
  );
  const initialBindings = useRef(JSON.stringify(bindings));
  const initial = useRef(JSON.stringify(definition));
  const [selected, setSelected] = useState(definition.output);
  const [saving, setSaving] = useState(false);
  const [invalid, setInvalid] = useState(false);
  const key = useRef(crypto.randomUUID());
  const bindingKey = useRef(crypto.randomUUID());
  const node = definition.nodes.find((n) => n.id === selected);
  const operation = operations.find((o) => o.operation === node?.operation);
  const [parameters, setParameters] = useState(
    JSON.stringify(node?.parameters ?? {}, null, 2),
  );
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const changed =
    JSON.stringify(definition) !== initial.current ||
    JSON.stringify(bindings) !== initialBindings.current ||
    mode !== "revise" ||
    invalid;
  useEffect(() => onDirty(changed), [changed, onDirty]);
  const change = (next: PipelineDefinition) => {
    key.current = crypto.randomUUID();
    bindingKey.current = crypto.randomUUID();
    setDefinition(next);
  };
  const update = (next: GraphNode) =>
    change({
      ...definition,
      nodes: definition.nodes.map((n) => (n.id === next.id ? next : n)),
    });
  const select = (id: string) => {
    setSelected(id);
    setParameters(
      JSON.stringify(
        definition.nodes.find((n) => n.id === id)?.parameters ?? {},
        null,
        2,
      ),
    );
    setInvalid(false);
  };
  const readonly = saving || mode === "fork";
  const valid =
    !invalid &&
    completeBindings(definition.inputs, bindings) &&
    title.trim() &&
    new TextEncoder().encode(title).byteLength <= 120 &&
    new TextEncoder().encode(JSON.stringify(definition)).byteLength <=
      limits.pipeline_definition_bytes;
  const save = async () => {
    if (!valid || saving) return;
    setSaving(true);
    try {
      if (
        await onSave({
          title,
          definition,
          key: key.current,
          bindingKey: bindingKey.current,
          bindings: selectedBindings(definition.inputs, bindings),
        })
      )
        onClose();
    } finally {
      setSaving(false);
    }
  };
  const heading =
    mode === "create"
      ? "New Pipeline"
      : mode === "fork"
        ? "Fork Pipeline"
        : "Edit Pipeline";
  return (
    <dialog
      ref={dialog}
      className="content-dialog pipeline-editor"
      aria-label={heading}
      onCancel={(e) => {
        e.preventDefault();
        if (!saving) onClose();
      }}
    >
      <h2>{heading}</h2>
      {error ? <p role="alert">{error}</p> : null}
      {mode === "revise" ? (
        <p>
          {pipeline?.title} · based on revision {pipeline?.revision}
        </p>
      ) : (
        <>
          <label htmlFor="pipeline-title">Pipeline name</label>
          <VscodeTextfield
            id="pipeline-title"
            label="Pipeline name"
            ref={titleField}
            value={title}
            disabled={saving}
            onInput={() => {
              key.current = crypto.randomUUID();
              bindingKey.current = crypto.randomUUID();
              setTitle(titleField.current?.value ?? "");
            }}
          />
        </>
      )}
      <p>
        Connect typed ports, select the Pipeline output, and move nodes by
        dragging or using the arrow keys. Runs execute the output’s dependencies
        in Rust.
      </p>
      <PipelineInputs
        definition={definition}
        disabled={readonly}
        onChange={change}
      />
      <BindingsEditor
        inputs={definition.inputs}
        assets={assets}
        pinned={trail.bindings}
        bindings={bindings}
        disabled={saving}
        onChange={(next) => {
          bindingKey.current = crypto.randomUUID();
          setBindings(next);
        }}
      />
      <div className="graph-editor-layout">
        <div>
          <PipelineGraph
            definition={definition}
            operations={operations}
            selected={selected}
            onSelect={invalid ? undefined : select}
            onMove={
              readonly
                ? undefined
                : (id, x, y) => {
                    const n = definition.nodes.find((n) => n.id === id);
                    if (n) update({ ...n, position: { x, y } });
                  }
            }
          />
          <VscodeButton
            secondary
            disabled={
              readonly ||
              invalid ||
              definition.nodes.length >= limits.pipeline_node_limit
            }
            onClick={() => {
              const op =
                operations.find((o) => o.operation === node?.operation) ??
                operations[0];
              if (!op) return;
              const id = crypto.randomUUID();
              const port = operation?.contract.outputs.find((p) =>
                op.contract.inputs.some(
                  (i) => i.semantic_type === p.semantic_type,
                ),
              );
              const inputs = initialInputs(op, definition.inputs);
              const consumer = op.contract.inputs.find(
                (p) => p.semantic_type === port?.semantic_type,
              );
              if (node && port && consumer)
                inputs[consumer.name] = {
                  kind: "output",
                  nodeId: node.id,
                  port: port.name,
                };
              const next: GraphNode = {
                id,
                operation: op.operation,
                operationVersion: op.operation_version,
                parameters: op.parameters,
                inputs,
                position: {
                  x: 40 + (definition.nodes.length % 3) * 300,
                  y: 60 + Math.floor(definition.nodes.length / 3) * 160,
                },
              };
              change({
                ...definition,
                nodes: [...definition.nodes, next],
                output: id,
              });
              setSelected(id);
              setParameters(JSON.stringify(next.parameters, null, 2));
            }}
          >
            Add node
          </VscodeButton>
          <small>
            {definition.nodes.length} / {limits.pipeline_node_limit} nodes ·
            source: {inputType}
          </small>
        </div>
        {node ? (
          <aside className="graph-inspector" aria-label="Selected node">
            <h3>Node {definition.nodes.indexOf(node) + 1}</h3>
            <Select
              id="pipeline-operation"
              label="Operation"
              value={node.operation}
              disabled={readonly || invalid}
              options={operations.map((o) => ({
                value: o.operation,
                label: o.name,
              }))}
              onChange={(id) => {
                const op = operations.find((o) => o.operation === id);
                if (!op) return;
                update({
                  ...node,
                  operation: id,
                  operationVersion: op.operation_version,
                  parameters: op.parameters,
                  inputs: initialInputs(op, definition.inputs),
                });
                setParameters(JSON.stringify(op.parameters, null, 2));
              }}
            />
            <p>{operation?.description}</p>
            <GraphInputs
              node={node}
              definition={definition}
              operations={operations}
              disabled={readonly || invalid}
              onChange={(inputs) => update({ ...node, inputs })}
            />
            {operation?.operation === "text.concat" ? (
              <>
                <label htmlFor="pipeline-separator">Separator</label>
                <VscodeTextarea
                  id="pipeline-separator"
                  label="Separator"
                  ref={separatorField}
                  rows={2}
                  value={String(node.parameters.separator ?? "")}
                  disabled={readonly}
                  onInput={() => {
                    const next = {
                      separator: separatorField.current?.value ?? "",
                    };
                    update({ ...node, parameters: next });
                    setParameters(JSON.stringify(next));
                    setInvalid(
                      new TextEncoder().encode(JSON.stringify(next))
                        .byteLength > limits.pipeline_parameter_bytes,
                    );
                  }}
                />
                <p>
                  Included once between left and right. Leave blank to join
                  directly.
                </p>
              </>
            ) : (
              <>
                <label htmlFor="pipeline-parameters">Parameters (JSON)</label>
                <VscodeTextarea
                  id="pipeline-parameters"
                  label="Parameters (JSON)"
                  ref={paramsField}
                  value={parameters}
                  rows={5}
                  disabled={readonly}
                  onInput={() => {
                    const text = paramsField.current?.value ?? "";
                    setParameters(text);
                    try {
                      const v: unknown = JSON.parse(text);
                      if (
                        !v ||
                        typeof v !== "object" ||
                        Array.isArray(v) ||
                        new TextEncoder().encode(JSON.stringify(v)).byteLength >
                          limits.pipeline_parameter_bytes
                      )
                        throw new Error("invalid_parameters");
                      setInvalid(false);
                      update({
                        ...node,
                        parameters: v as Record<string, unknown>,
                      });
                    } catch {
                      setInvalid(true);
                    }
                  }}
                />
              </>
            )}
            {invalid ? (
              <p role="alert">
                {operation?.operation === "text.concat"
                  ? "The separator is too long for this Pipeline."
                  : "Enter a bounded JSON object for the parameters before changing nodes."}
              </p>
            ) : null}
            <div className="dialog-actions">
              <VscodeButton
                secondary
                disabled={readonly || definition.output === node.id}
                onClick={() => change({ ...definition, output: node.id })}
              >
                Use as output
              </VscodeButton>
              <VscodeButton
                secondary
                disabled={
                  readonly ||
                  invalid ||
                  definition.nodes.length === 1 ||
                  definition.nodes.some((n) =>
                    Object.values(n.inputs).some(
                      (s) => s.kind === "output" && s.nodeId === node.id,
                    ),
                  )
                }
                onClick={() => {
                  const nodes = definition.nodes.filter(
                    (n) => n.id !== node.id,
                  );
                  const first = nodes[0];
                  if (!first) return;
                  change({
                    ...definition,
                    nodes,
                    output:
                      definition.output === node.id
                        ? first.id
                        : definition.output,
                  });
                  select(first.id);
                }}
              >
                Remove node
              </VscodeButton>
            </div>
          </aside>
        ) : null}
      </div>
      <small>
        Saving freezes the full graph, versions, parameters and layout. Only
        this Trail selects the saved revision; previous Runs preserve their
        evidence.
      </small>
      <div className="dialog-actions">
        <VscodeButton secondary disabled={saving} onClick={onClose}>
          Cancel
        </VscodeButton>
        <VscodeButton
          icon="save"
          disabled={saving || !valid}
          onClick={() => void save()}
        >
          {mode === "create"
            ? "Create Pipeline"
            : mode === "fork"
              ? "Fork Pipeline"
              : "Save revision"}
        </VscodeButton>
      </div>
    </dialog>
  );
}
