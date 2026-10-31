import { useEffect, useRef, useState } from "react";
import type {
  CatalogEntry,
  Exploration,
  Json,
  Port,
  Source,
  Step,
} from "./exploration-api";
import { library } from "./notebook-api";
import { SchemaField } from "./SchemaField";
import type { Store } from "./workspace-api";

type Choice = {
  value: string;
  label: string;
  source: Source;
  semantic: string;
};
export function InputField({
  port,
  step,
  exploration,
  catalog,
  asset,
  onChange,
  onValidityChange,
  privateValue,
  privateInput,
}: {
  privateValue: Json | undefined;
  privateInput: boolean;
  port: Port;
  step: Step;
  exploration: Exploration;
  catalog: CatalogEntry[];
  asset: { store: Store; path: string } | null;
  onChange: (value: Source) => void;
  onValidityChange: (valid: boolean) => void;
}) {
  const source = step.inputs[port.name];
  const filesystemInput = ["file_path", "file.source"].includes(
    port.semantic_type,
  );
  const [saved, setSaved] = useState<Choice[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  // Descendants cannot feed an ancestor. The server independently checks cycles.
  const descendants = new Set([step.id]);
  for (let i = 0; i < exploration.steps.length; i++)
    for (const s of exploration.steps)
      if (
        Object.values(s.inputs).some(
          (input) => input.type === "step" && descendants.has(input.step),
        )
      )
        descendants.add(s.id);
  const options: Choice[] = exploration.steps
    .filter((s) => !descendants.has(s.id))
    .flatMap(
      (s) =>
        catalog
          .find((e) => e.contract.operation === s.operation)
          ?.contract.outputs.map((p) => ({
            value: `step:${s.id}:${p.name}`,
            label: `${s.operation} → ${p.name}`,
            semantic: p.semantic_type,
            source: { type: "step" as const, step: s.id, port: p.name },
          })) ?? [],
    );
  const choices = [...options, ...saved].filter(
    (o) =>
      compatible(o.semantic, port.semantic_type) ||
      ["text", "bytes", "byte_reader"].includes(port.semantic_type),
  );
  const fileRepresentation =
    port.semantic_type === "text"
      ? "text"
      : port.semantic_type === "image.samples"
        ? "image"
        : ["bytes", "byte_reader", "file_path", "file.source"].includes(
              port.semantic_type,
            )
          ? "bytes"
          : null;
  const chosen =
    source?.type === "step"
      ? `step:${source.step}:${source.port}`
      : source?.type === "artifact"
        ? `artifact:${source.id}:${source.port}`
        : source?.type === "file"
          ? asset?.path === source.path && asset.store === source.store
            ? "file"
            : "saved_file"
          : "literal";
  const reference =
    source?.type === "step" || source?.type === "artifact" ? source : null;
  const selectedChoice = choices.find((c) => c.value === chosen);
  const requiresSelection =
    selectedChoice && !compatible(selectedChoice.semantic, port.semantic_type);
  async function loadSaved() {
    const controller = new AbortController();
    active.current?.abort();
    active.current = controller;
    setLoading(true);
    setError(null);
    try {
      const entries = await library(controller.signal);
      const choices: Choice[] = entries.flatMap(({ entry, record, error }) => {
        if (error || !record || record.semantic_type !== "operation.outputs")
          return [];
        const metadata = record.metadata;
        if (
          !metadata ||
          typeof metadata !== "object" ||
          !("ports" in metadata) ||
          !Array.isArray(metadata.ports)
        )
          return [];
        return metadata.ports.flatMap((p: unknown) => {
          if (
            !p ||
            typeof p !== "object" ||
            !("name" in p) ||
            !("semantic_type" in p) ||
            typeof p.name !== "string" ||
            typeof p.semantic_type !== "string"
          )
            return [];
          return [
            {
              value: `artifact:${entry.id}:${p.name}`,
              label: `${entry.title} → ${p.name}`,
              semantic: p.semantic_type,
              source: { type: "artifact" as const, id: entry.id, port: p.name },
            },
          ];
        });
      });
      if (!controller.signal.aborted) setSaved(choices);
    } catch (error) {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "Saved inputs are unavailable.",
        );
    } finally {
      if (!controller.signal.aborted) setLoading(false);
    }
  }
  return (
    <div className="port-input">
      <label>
        {port.name} · {port.semantic_type}
        <select
          value={chosen}
          onChange={(event) => {
            onValidityChange(true);
            if (event.target.value === "literal")
              onChange({
                type: "literal",
                value: port.schema.type === "string" ? "" : null,
              });
            else if (
              event.target.value === "file" &&
              asset &&
              fileRepresentation
            )
              onChange({
                type: "file",
                ...asset,
                representation: fileRepresentation,
              });
            else {
              const option = choices.find(
                (o) => o.value === event.target.value,
              );
              if (option) {
                const value = option.source;
                if (
                  (value.type === "step" || value.type === "artifact") &&
                  !compatible(option.semantic, port.semantic_type)
                ) {
                  onChange({
                    ...value,
                    selection: {
                      pointer: "/",
                      conversion:
                        port.semantic_type === "text" ? "text" : "base64_bytes",
                    },
                  });
                } else onChange(value);
              }
            }
          }}
        >
          <option value="literal" disabled={filesystemInput}>
            {filesystemInput
              ? "Choose a workspace file"
              : privateInput
                ? "Private value for this run"
                : "Enter a value"}
          </option>
          {asset && fileRepresentation && (
            <option value="file">Selected asset: {asset.path}</option>
          )}
          {source?.type === "file" &&
            (!asset ||
              asset.path !== source.path ||
              asset.store !== source.store) && (
              <option value="saved_file">{source.path}</option>
            )}
          {reference && !selectedChoice && (
            <option value={chosen}>Retained reference: {reference.port}</option>
          )}
          {choices.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
              {!compatible(o.semantic, port.semantic_type)
                ? " · select a string field"
                : ""}
            </option>
          ))}
        </select>
      </label>
      <button
        type="button"
        className="button secondary"
        disabled={loading}
        onClick={() => void loadSaved()}
      >
        {loading ? "Loading saved inputs…" : "Load saved inputs"}
      </button>
      {error && <p role="alert">{error}</p>}
      {reference &&
        ["text", "bytes", "byte_reader"].includes(port.semantic_type) && (
          <label>
            Field conversion
            <select
              value={reference.selection?.conversion ?? "whole"}
              onChange={(event) => {
                if (event.target.value === "whole") {
                  const { selection: _selection, ...whole } = reference;
                  onChange(whole);
                } else
                  onChange({
                    ...reference,
                    selection: {
                      pointer: reference.selection?.pointer ?? "/",
                      conversion: event.target.value as "text" | "base64_bytes",
                    },
                  });
              }}
            >
              <option value="whole" disabled={Boolean(requiresSelection)}>
                Whole named output
              </option>
              <option
                value={port.semantic_type === "text" ? "text" : "base64_bytes"}
              >
                {port.semantic_type === "text"
                  ? "String field as exact text"
                  : "String field decoded from Base64 bytes"}
              </option>
            </select>
          </label>
        )}
      {reference?.selection && (
        <label>
          Field JSON Pointer
          <input
            aria-label="Field JSON Pointer"
            aria-invalid={!validPointer(reference.selection.pointer)}
            maxLength={512}
            value={reference.selection.pointer}
            placeholder="/field or /items/0/name"
            onChange={(event) => {
              onValidityChange(validPointer(event.target.value));
              onChange({
                ...reference,
                selection: {
                  conversion: reference.selection?.conversion ?? "text",
                  pointer: event.target.value,
                },
              });
            }}
          />
          <small>
            Relative to the selected named output. Only string fields are
            accepted. Use ~1 for / and ~0 for ~. No expressions or implicit text
            decoding.
          </small>
        </label>
      )}
      {filesystemInput && chosen === "literal" && (
        <p>
          Select an asset in Files. Typed paths do not grant filesystem access.
        </p>
      )}
      {privateInput && (!source || source.type === "literal") && (
        <p>
          This private value is used only for this run. It is not saved in the
          configuration and must be entered again.
        </p>
      )}
      {!filesystemInput && (!source || source.type === "literal") && (
        <SchemaField
          label={`${port.name} value`}
          schema={port.schema}
          value={
            privateInput
              ? privateValue
              : source?.type === "literal"
                ? source.value
                : undefined
          }
          required={port.required}
          onValidityChange={onValidityChange}
          onChange={(value) => {
            onChange({ type: "literal", value: value ?? null });
          }}
        />
      )}
    </div>
  );
}
function validPointer(pointer: string): boolean {
  return (
    pointer.startsWith("/") &&
    new TextEncoder().encode(pointer).byteLength <= 512 &&
    pointer.split("/").length <= 17 &&
    pointer
      .split("~")
      .slice(1)
      .every((part) => part.startsWith("0") || part.startsWith("1"))
  );
}

function compatible(source: string, target: string): boolean {
  return source === target || (source === "bytes" && target === "byte_reader");
}
