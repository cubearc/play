import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useRef } from "react";
import type { Operation } from "../desktop/contract";
import { Select } from "./GraphInputs";
import VscodeCheckbox from "./VscodeCheckbox";

interface FieldSchema {
  type?: string;
  description?: string;
  enum?: unknown[];
  minimum?: number;
  maximum?: number;
  minItems?: number;
  maxItems?: number;
  uniqueItems?: boolean;
  items?: FieldSchema;
}
function properties(operation: Operation): Record<string, FieldSchema> {
  const schema = operation.contract.parameters;
  return schema !== null &&
    typeof schema === "object" &&
    "properties" in schema &&
    typeof schema.properties === "object" &&
    schema.properties !== null
    ? (schema.properties as Record<string, FieldSchema>)
    : {};
}
function required(operation: Operation): string[] {
  const s = operation.contract.parameters;
  return s !== null &&
    typeof s === "object" &&
    "required" in s &&
    Array.isArray(s.required)
    ? (s.required as string[])
    : [];
}
function valid(schema: FieldSchema, value: unknown): boolean {
  if (schema.enum && !schema.enum.some((v) => v === value)) return false;
  switch (schema.type) {
    case "integer":
      return (
        typeof value === "number" &&
        Number.isSafeInteger(value) &&
        (schema.minimum === undefined || value >= schema.minimum) &&
        (schema.maximum === undefined || value <= schema.maximum)
      );
    case "number":
      return (
        typeof value === "number" &&
        Number.isFinite(value) &&
        (schema.minimum === undefined || value >= schema.minimum) &&
        (schema.maximum === undefined || value <= schema.maximum)
      );
    case "boolean":
      return typeof value === "boolean";
    case "string":
      return typeof value === "string";
    case "array":
      return (
        Array.isArray(value) &&
        (schema.minItems === undefined || value.length >= schema.minItems) &&
        (schema.maxItems === undefined || value.length <= schema.maxItems) &&
        (!schema.uniqueItems ||
          new Set(value.map((v) => JSON.stringify(v))).size === value.length) &&
        value.every((v) => !schema.items || valid(schema.items, v))
      );
    case "object":
      return (
        value !== null && typeof value === "object" && !Array.isArray(value)
      );
    default:
      return true;
  }
}
export function parameterTypesValid(
  operation: Operation,
  parameters: Record<string, unknown>,
): boolean {
  const fields = properties(operation),
    mandatory = required(operation);
  return (
    Object.keys(parameters).every((n) => Object.hasOwn(fields, n)) &&
    Object.entries(fields).every(([n, s]) =>
      parameters[n] === undefined
        ? !mandatory.includes(n)
        : valid(s, parameters[n]),
    )
  );
}
function readable(value: string): string {
  const words = value
    .replaceAll("_", " ")
    .replace(/\bcw\b/g, "clockwise")
    .replace(/\bccw\b/g, "counterclockwise");
  return words.charAt(0).toUpperCase() + words.slice(1);
}
function Field({
  name,
  schema,
  value,
  disabled,
  onChange,
}: {
  name: string;
  schema: FieldSchema;
  value: unknown;
  disabled: boolean;
  onChange: (value: unknown) => void;
}) {
  const field = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const area = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const check = useRef<HTMLElementTagNameMap["vscode-checkbox"]>(null);
  const label = name === "pointer" ? "Field path" : readable(name);
  const id = `parameter-${name}`;
  if (schema.enum)
    return (
      <div className="graph-parameter">
        <Select
          id={id}
          label={label}
          value={JSON.stringify(value) ?? ""}
          disabled={disabled}
          options={schema.enum.map((v) => ({
            value: JSON.stringify(v),
            label: readable(String(v)),
          }))}
          onChange={(raw) => onChange(JSON.parse(raw))}
        />
        {schema.description ? <p>{schema.description}</p> : null}
      </div>
    );
  if (!["string", "boolean", "integer", "number"].includes(schema.type ?? ""))
    return (
      <div className="graph-parameter">
        <label htmlFor={id}>{label} (JSON)</label>
        <VscodeTextarea
          ref={area}
          id={id}
          label={`${label} (JSON)`}
          rows={2}
          value={typeof value === "string" ? value : JSON.stringify(value)}
          disabled={disabled}
          aria-invalid={valid(schema, value) ? "false" : "true"}
          onInput={() => {
            const raw = area.current?.value ?? "";
            try {
              onChange(JSON.parse(raw));
            } catch {
              onChange(raw);
            }
          }}
        />
        {!valid(schema, value) ? (
          <p role="alert">
            Enter a valid {schema.type} matching this parameter.
          </p>
        ) : null}
        {schema.description ? <p>{schema.description}</p> : null}
      </div>
    );
  if (schema.type === "boolean")
    return (
      <VscodeCheckbox
        ref={check}
        checked={Boolean(value)}
        disabled={disabled}
        onChange={() => onChange(check.current?.checked ?? false)}
      >
        {label}
      </VscodeCheckbox>
    );
  return (
    <div className="graph-parameter">
      <label htmlFor={id}>{label}</label>
      {name === "separator" ? (
        <VscodeTextarea
          ref={area}
          id={id}
          label={label}
          rows={2}
          value={String(value ?? "")}
          disabled={disabled}
          onInput={() => onChange(area.current?.value ?? "")}
        />
      ) : (
        <VscodeTextfield
          ref={field}
          id={id}
          label={label}
          value={String(value ?? "")}
          disabled={disabled}
          onInput={() => {
            const raw = field.current?.value ?? "";
            onChange(
              (schema.type === "integer" &&
                /^-?(0|[1-9][0-9]*)$/.test(raw) &&
                Number.isSafeInteger(Number(raw))) ||
                (schema.type === "number" &&
                  /^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$/.test(
                    raw,
                  ) &&
                  Number.isFinite(Number(raw)))
                ? Number(raw)
                : raw,
            );
          }}
        />
      )}
      {schema.description ? <p>{schema.description}</p> : null}
    </div>
  );
}
export function ParameterFields({
  operation,
  parameters,
  disabled,
  onChange,
}: {
  operation: Operation;
  parameters: Record<string, unknown>;
  disabled: boolean;
  onChange: (parameters: Record<string, unknown>) => void;
}) {
  const fields = Object.entries(properties(operation));
  const mandatory = required(operation);
  return fields.length ? (
    fields.map(([name, schema]) => {
      const optional =
        !mandatory.includes(name) && !Object.hasOwn(operation.parameters, name);
      const present = Object.hasOwn(parameters, name);
      return (
        <div key={name}>
          {optional ? (
            <VscodeCheckbox
              checked={present}
              disabled={disabled}
              onChange={() => {
                const next = { ...parameters };
                if (present) delete next[name];
                else
                  next[name] =
                    schema.type === "array"
                      ? []
                      : schema.type === "object"
                        ? {}
                        : "";
                onChange(next);
              }}
            >
              Use {readable(name).toLowerCase()}
            </VscodeCheckbox>
          ) : null}
          {!optional || present ? (
            <Field
              name={name}
              schema={schema}
              value={parameters[name]}
              disabled={disabled}
              onChange={(value) => onChange({ ...parameters, [name]: value })}
            />
          ) : null}
        </div>
      );
    })
  ) : (
    <p>No parameters.</p>
  );
}
