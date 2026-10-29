import VscodeCheckbox from "@vscode-elements/react-elements/dist/components/VscodeCheckbox.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useRef } from "react";
import type { Operation } from "../desktop/contract";

interface FieldSchema {
  type?: string;
  description?: string;
}
function properties(operation: Operation): Record<string, FieldSchema> {
  const schema = operation.contract.parameters;
  return schema !== null &&
    typeof schema === "object" &&
    "properties" in schema &&
    typeof schema.properties === "object" &&
    schema.properties !== null
    ? (Object.fromEntries(
        Object.entries(schema.properties).filter(([name]) =>
          Object.hasOwn(operation.parameters, name),
        ),
      ) as Record<string, FieldSchema>)
    : {};
}
export function parameterTypesValid(
  operation: Operation,
  parameters: Record<string, unknown>,
): boolean {
  return Object.entries(properties(operation)).every(([name, schema]) => {
    const value = parameters[name];
    return schema.type === "integer"
      ? typeof value === "number" && Number.isSafeInteger(value)
      : schema.type === "boolean"
        ? typeof value === "boolean"
        : schema.type === "string"
          ? typeof value === "string"
          : true;
  });
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
  const label =
    name === "pointer"
      ? "Field path"
      : name
          .split("_")
          .map((part, i) =>
            i === 0 ? part[0]?.toUpperCase() + part.slice(1) : part,
          )
          .join(" ");
  const id = `parameter-${name}`;
  if (schema.type === "boolean")
    return (
      <VscodeCheckbox
        ref={check}
        checked={Boolean(value)}
        disabled={disabled}
        aria-disabled={disabled ? "true" : "false"}
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
              schema.type === "integer" &&
                /^-?(0|[1-9][0-9]*)$/.test(raw) &&
                Number.isSafeInteger(Number(raw))
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
  return fields.length ? (
    fields.map(([name, schema]) => (
      <Field
        key={name}
        name={name}
        schema={schema}
        value={parameters[name]}
        disabled={disabled}
        onChange={(value) => onChange({ ...parameters, [name]: value })}
      />
    ))
  ) : (
    <p>No parameters.</p>
  );
}
