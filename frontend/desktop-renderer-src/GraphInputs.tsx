import VscodeOption from "@vscode-elements/react-elements/dist/components/VscodeOption.js";
import VscodeSingleSelect from "@vscode-elements/react-elements/dist/components/VscodeSingleSelect.js";
import { useEffect, useRef, useState } from "react";
import type { Operation } from "../desktop/contract";
import type {
  GraphNode,
  GraphSource,
  PipelineDefinition,
} from "../desktop/workspace-contract";
import { type LiteralDraft, LiteralValue } from "./LiteralValue";
export function Select({
  id,
  label,
  value,
  disabled,
  options,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  disabled: boolean;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  const ref = useRef<HTMLElementTagNameMap["vscode-single-select"]>(null);
  const [position, setPosition] = useState<"above" | "below">("below");
  const place = () => {
    const box = ref.current?.getBoundingClientRect();
    if (box)
      setPosition(
        box.top > window.innerHeight - box.bottom ? "above" : "below",
      );
  };
  return (
    <>
      <label htmlFor={id}>{label}</label>
      <VscodeSingleSelect
        id={id}
        label={label}
        ref={ref}
        position={position}
        onFocusCapture={place}
        onPointerDownCapture={place}
        onKeyDownCapture={place}
        value={value}
        disabled={disabled}
        aria-disabled={disabled ? "true" : "false"}
        onChange={() => onChange(ref.current?.value ?? "")}
      >
        {options.map((o) => (
          <VscodeOption key={o.value} value={o.value}>
            {o.label}
          </VscodeOption>
        ))}
      </VscodeSingleSelect>
    </>
  );
}
function emptyLiteral(semanticType: string, schema?: unknown): unknown {
  if (semanticType === "json")
    return schema !== null &&
      typeof schema === "object" &&
      "type" in schema &&
      schema.type === "integer"
      ? 0
      : null;
  return semanticType === "bytes"
    ? { base64: "" }
    : semanticType === "bytes.array"
      ? []
      : "";
}
export function initialInputs(
  operation: Operation,
  slots: PipelineDefinition["inputs"],
): GraphNode["inputs"] {
  return Object.fromEntries(
    operation.contract.inputs.map((p) => [
      p.name,
      slots.some((s) => s.semanticType === p.semantic_type)
        ? {
            kind: "input",
            name:
              slots.find((s) => s.semanticType === p.semantic_type)?.name ??
              "source",
          }
        : {
            kind: "literal",
            value: emptyLiteral(p.semantic_type, p.schema),
          },
    ]),
  );
}
export function GraphInputs({
  node,
  definition,
  operations,
  disabled,
  onChange,
  onInvalidChange,
}: {
  node: GraphNode;
  definition: PipelineDefinition;
  operations: Operation[];
  disabled: boolean;
  onChange: (inputs: GraphNode["inputs"]) => void;
  onInvalidChange: (invalid: boolean) => void;
}) {
  const op = operations.find((o) => o.operation === node.operation);
  const [drafts, setDrafts] = useState<Record<string, LiteralDraft>>({});
  const draftState = useRef(drafts);
  const literalKey = (n: GraphNode, port: string) =>
    `${n.id}:${n.operation}:${port}`;
  const activeKeys = definition.nodes
    .flatMap((n) =>
      Object.entries(n.inputs)
        .filter(([, s]) => s.kind === "literal")
        .map(([port]) => literalKey(n, port)),
    )
    .sort()
    .join("|");
  useEffect(() => {
    const keys = new Set(activeKeys.split("|"));
    const next = Object.fromEntries(
      Object.entries(draftState.current).filter(([key]) => keys.has(key)),
    );
    draftState.current = next;
    setDrafts(next);
    onInvalidChange(Object.values(next).some((d) => Boolean(d.error)));
  }, [activeKeys, onInvalidChange]);
  const saveDraft = (key: string, draft: LiteralDraft) => {
    const next = { ...draftState.current, [key]: draft };
    draftState.current = next;
    setDrafts(next);
    onInvalidChange(Object.values(next).some((d) => Boolean(d.error)));
  };
  return (
    <>
      {op?.contract.inputs.map((port) => {
        const source = node.inputs[port.name];
        const value =
          source?.kind === "output"
            ? `${source.nodeId}:${source.port}`
            : source?.kind === "input"
              ? `input:${source.name}`
              : "literal";
        const choices = [
          ...definition.inputs
            .filter((s) => s.semanticType === port.semantic_type)
            .map((s) => ({
              value: `input:${s.name}`,
              label: `${s.name} · ${s.semanticType}`,
            })),
          ...(port.semantic_type !== "image.samples"
            ? [{ value: "literal", label: `Literal · ${port.semantic_type}` }]
            : []),
          ...definition.nodes
            .filter((n) => n.id !== node.id)
            .flatMap(
              (n) =>
                operations
                  .find((o) => o.operation === n.operation)
                  ?.contract.outputs.filter(
                    (p) => p.semantic_type === port.semantic_type,
                  )
                  .map((p) => ({
                    value: `${n.id}:${p.name}`,
                    label: `${definition.nodes.indexOf(n) + 1}. ${operations.find((o) => o.operation === n.operation)?.name} · ${p.name}`,
                  })) ?? [],
            ),
        ];
        const update = (binding: GraphSource) =>
          onChange({ ...node.inputs, [port.name]: binding });
        return (
          <div className="graph-input" key={port.name}>
            <Select
              id={`graph-${node.id}-${port.name}`}
              label={`${port.name} · ${port.semantic_type}`}
              value={value}
              disabled={disabled}
              options={choices}
              onChange={(v) => {
                if (v.startsWith("input:"))
                  update({ kind: "input", name: v.slice(6) });
                else if (v === "literal")
                  update({
                    kind: "literal",
                    value: emptyLiteral(port.semantic_type, port.schema),
                  });
                else {
                  const [nodeId, port] = v.split(":");
                  if (nodeId && port) update({ kind: "output", nodeId, port });
                }
              }}
            />
            {source?.kind === "literal" ? (
              <LiteralValue
                key={literalKey(node, port.name)}
                draft={drafts[literalKey(node, port.name)]}
                onDraft={(draft) =>
                  saveDraft(literalKey(node, port.name), draft)
                }
                schema={port.schema}
                id={`literal-${node.id}-${port.name}`}
                value={source.value}
                semanticType={port.semantic_type}
                disabled={disabled}
                onChange={(v) => update({ kind: "literal", value: v })}
              />
            ) : null}
          </div>
        );
      })}
    </>
  );
}
