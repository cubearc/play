import VscodeOption from "@vscode-elements/react-elements/dist/components/VscodeOption.js";
import VscodeSingleSelect from "@vscode-elements/react-elements/dist/components/VscodeSingleSelect.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useRef, useState } from "react";
import type { Operation } from "../desktop/contract";
import type {
  GraphNode,
  GraphSource,
  PipelineDefinition,
} from "../desktop/workspace-contract";
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
function Literal({
  id,
  value,
  semanticType,
  disabled,
  onChange,
}: {
  id: string;
  value: unknown;
  semanticType: string;
  disabled: boolean;
  onChange: (value: unknown) => void;
}) {
  const ref = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const text = semanticType === "text";
  const bytes = semanticType === "bytes";
  const label = text
    ? "Text value"
    : bytes
      ? "Bytes (Base64)"
      : "Byte values (JSON)";
  const encoded =
    bytes && value !== null && typeof value === "object" && "base64" in value
      ? String(value.base64)
      : "";
  return (
    <>
      <label htmlFor={id}>{label}</label>
      <VscodeTextarea
        id={id}
        label={label}
        ref={ref}
        value={
          bytes
            ? encoded
            : typeof value === "string"
              ? value
              : JSON.stringify(value)
        }
        rows={2}
        disabled={disabled}
        onInput={() => {
          const raw = ref.current?.value ?? "";
          if (bytes) onChange({ base64: raw });
          else if (text) onChange(raw);
          else {
            try {
              onChange(JSON.parse(raw));
            } catch {
              onChange(raw);
            }
          }
        }}
      />
    </>
  );
}
function emptyLiteral(semanticType: string): unknown {
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
            value: emptyLiteral(p.semantic_type),
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
}: {
  node: GraphNode;
  definition: PipelineDefinition;
  operations: Operation[];
  disabled: boolean;
  onChange: (inputs: GraphNode["inputs"]) => void;
}) {
  const op = operations.find((o) => o.operation === node.operation);
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
                    value: emptyLiteral(port.semantic_type),
                  });
                else {
                  const [nodeId, port] = v.split(":");
                  if (nodeId && port) update({ kind: "output", nodeId, port });
                }
              }}
            />
            {source?.kind === "literal" ? (
              <Literal
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
