import { useEffect, useId, useState } from "react";
import { type Json, json, type Schema } from "./exploration-api";

export function resolved(schema: Schema, root: Schema): Schema {
  let current = schema;
  for (let depth = 0; typeof current.$ref === "string"; depth++) {
    if (depth === 16 || !current.$ref.startsWith("#/"))
      throw new Error("This schema reference is unsupported.");
    let value: Json = root;
    for (const part of current.$ref.slice(2).split("/")) {
      if (!value || typeof value !== "object" || Array.isArray(value))
        throw new Error("This schema reference is unavailable.");
      value = value[part.replace(/~1/g, "/").replace(/~0/g, "~")] ?? null;
    }
    if (!value || typeof value !== "object" || Array.isArray(value))
      throw new Error("This schema reference is unavailable.");
    current = value;
  }
  return current;
}
export function SchemaField({
  label,
  schema,
  root = schema,
  value,
  onChange,
  secret = false,
  required = false,
  onValidityChange,
}: {
  label: string;
  schema: Schema;
  root?: Schema;
  value: Json | undefined;
  onChange: (value: Json | undefined) => void;
  secret?: boolean;
  required?: boolean;
  onValidityChange?: (valid: boolean) => void;
}) {
  const id = useId();
  const [error, setError] = useState<string | null>(null);
  function report(message: string | null) {
    setError(message);
    onValidityChange?.(message === null);
  }
  const field = resolved(schema, root);
  const type = field["x-cube-integer"] ? "exact_integer" : field.type;
  const choices = Array.isArray(field.enum) ? field.enum : null;
  const description =
    typeof field.description === "string" ? field.description : null;
  function scalar(raw: string) {
    if (!raw && type !== "string") {
      onChange(undefined);
      report(null);
      return;
    }
    if (type === "integer" || type === "number" || type === "exact_integer") {
      if (type !== "number" && !/^-?(0|[1-9][0-9]*)$/.test(raw)) {
        report("Enter a canonical decimal integer.");
        return;
      }
      const number = Number(raw);
      if (!Number.isFinite(number)) {
        report("Enter a finite number.");
        return;
      }
      if (type === "integer" && !Number.isSafeInteger(number)) {
        report("Enter an integer within this field's exact numeric range.");
        return;
      }
      onChange(
        type === "exact_integer" && !Number.isSafeInteger(number)
          ? raw
          : number,
      );
      report(null);
    } else {
      onChange(raw);
      report(null);
    }
  }
  return (
    <div className="schema-field">
      <label htmlFor={id}>
        {label}
        {required ? " *" : ""}
        {secret ? " · private for this run" : ""}
      </label>
      {choices ? (
        <select
          id={id}
          value={
            value === undefined
              ? ""
              : String(
                  choices.findIndex(
                    (choice) =>
                      JSON.stringify(choice) === JSON.stringify(value),
                  ),
                )
          }
          onChange={(event) =>
            onChange(
              event.target.value === ""
                ? undefined
                : choices[Number(event.target.value)],
            )
          }
        >
          <option value="">Choose…</option>
          {choices.map((choice, index) => (
            <option key={JSON.stringify(choice)} value={index}>
              {String(choice)}
            </option>
          ))}
        </select>
      ) : type === "boolean" ? (
        <input
          id={id}
          type="checkbox"
          checked={value === true}
          onChange={(event) => onChange(event.target.checked)}
        />
      ) : type === "string" ||
        type === "integer" ||
        type === "number" ||
        type === "exact_integer" ? (
        <input
          id={id}
          type={secret ? "password" : "text"}
          inputMode={
            type === "integer" || type === "exact_integer"
              ? "numeric"
              : undefined
          }
          autoComplete="off"
          value={
            typeof value === "string" || typeof value === "number"
              ? String(value)
              : ""
          }
          onChange={(event) => scalar(event.target.value)}
        />
      ) : (
        <StructuredField
          id={id}
          value={value}
          onChange={onChange}
          onError={report}
        />
      )}
      {description && <small>{description}</small>}
      {type === "exact_integer" && (
        <small>Large integers are preserved as exact decimal strings.</small>
      )}
      {error && <span role="alert">{error}</span>}
    </div>
  );
}
function StructuredField({
  id,
  value,
  onChange,
  onError,
}: {
  id: string;
  value: Json | undefined;
  onChange: (value: Json | undefined) => void;
  onError: (message: string | null) => void;
}) {
  const [draft, setDraft] = useState(() =>
    value === undefined ? "" : JSON.stringify(value, null, 2),
  );
  useEffect(() => {
    if (value === undefined) setDraft("");
  }, [value]);
  return (
    <textarea
      id={id}
      spellCheck={false}
      rows={4}
      value={draft}
      onChange={(event) => {
        const raw = event.target.value;
        setDraft(raw);
        if (!raw.trim()) {
          onChange(undefined);
          onError(null);
          return;
        }
        try {
          onChange(json(JSON.parse(raw)));
          onError(null);
        } catch {
          onError(
            "Enter valid JSON with exact integers; use strings for values outside the safe integer range.",
          );
        }
      }}
    />
  );
}
