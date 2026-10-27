import { useState } from "react";
import type { Contract, Json, Schema } from "./exploration-api";
import { resolved, SchemaField } from "./SchemaField";

function properties(schema: Schema): Record<string, Json> {
  const value = schema.properties;
  return value && typeof value === "object" && !Array.isArray(value)
    ? value
    : {};
}
export function OperationParameters({
  contract,
  values,
  privateValues,
  onChange,
  onPrivateChange,
  onValidityChange,
}: {
  contract: Contract;
  values: Record<string, Json>;
  privateValues: Record<string, Json>;
  onChange: (values: Record<string, Json>) => void;
  onPrivateChange: (values: Record<string, Json>) => void;
  onValidityChange: (name: string, valid: boolean) => void;
}) {
  const root = contract.parameters;
  const alternatives = root.anyOf ?? root.oneOf;
  const variants = Array.isArray(alternatives)
    ? alternatives.filter(
        (value): value is Schema =>
          !!value && typeof value === "object" && !Array.isArray(value),
      )
    : [];
  const [variant, setVariant] = useState(() =>
    Math.max(
      0,
      variants.findIndex((schema) =>
        Object.keys(values).every(
          (name) => name in properties(resolved(schema, root)),
        ),
      ),
    ),
  );
  const schema = resolved(variants[variant] ?? root, root);
  const fields = properties(schema);
  function changeVariant(index: number) {
    const fields = properties(resolved(variants[index] ?? root, root));
    setVariant(index);
    const next: Record<string, Json> = {};
    for (const [name, schema] of Object.entries(fields)) {
      if (values[name] !== undefined) next[name] = values[name];
      else if (
        schema &&
        typeof schema === "object" &&
        !Array.isArray(schema) &&
        schema.default !== undefined
      )
        next[name] = schema.default;
      onValidityChange(name, true);
    }
    onChange(next);
    onPrivateChange({});
  }
  return (
    <>
      {variants.length > 0 && (
        <label>
          Parameter form
          <select
            value={variant}
            onChange={(event) => changeVariant(Number(event.target.value))}
          >
            {variants.map((schema, index) => (
              <option
                key={
                  typeof schema.$ref === "string" ? schema.$ref : String(index)
                }
                value={index}
              >
                {typeof schema.$ref === "string"
                  ? schema.$ref.split("/").at(-1)
                  : `Form ${index + 1}`}
              </option>
            ))}
          </select>
        </label>
      )}
      {Object.entries(fields).map(([name, schema]) => {
        if (!schema || typeof schema !== "object" || Array.isArray(schema))
          return null;
        const secret = contract.policy.sensitive_parameters.includes(name);
        return (
          <SchemaField
            key={`${variant}:${name}`}
            label={name}
            schema={schema}
            root={root}
            required={
              Array.isArray(
                resolved(variants[variant] ?? root, root).required,
              ) &&
              (
                resolved(variants[variant] ?? root, root).required as Json[]
              ).includes(name)
            }
            secret={secret}
            value={secret ? privateValues[name] : values[name]}
            onValidityChange={(valid) => onValidityChange(name, valid)}
            onChange={(value) => {
              const next = { ...(secret ? privateValues : values) };
              if (value === undefined) delete next[name];
              else next[name] = value;
              if (secret) onPrivateChange(next);
              else onChange(next);
            }}
          />
        );
      })}
      {Object.keys(fields).length === 0 &&
        (root.type === "object" ? (
          <p>No parameters required.</p>
        ) : (
          <SchemaField
            label="Operation parameters"
            schema={root}
            value={values}
            onValidityChange={(valid) => onValidityChange("parameters", valid)}
            onChange={(value) => {
              if (value && typeof value === "object" && !Array.isArray(value))
                onChange(value);
            }}
          />
        ))}
    </>
  );
}
