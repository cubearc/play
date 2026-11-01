import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useRef } from "react";
import limits from "../../desktop/protocol.json";
import type { Representation } from "../desktop/contract";
import { parseExactJson } from "./exactJson";

export interface LiteralDraft {
  raw: string;
  error: string;
}
export function LiteralValue({
  id,
  value,
  representation,
  schema,
  disabled,
  draft,
  onDraft,
  onChange,
}: {
  id: string;
  value: unknown;
  representation: Representation;
  schema: unknown;
  disabled: boolean;
  draft: LiteralDraft | undefined;
  onDraft: (draft: LiteralDraft) => void;
  onChange: (value: unknown) => void;
}) {
  const area = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const field = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const integer =
    representation === "json" &&
    typeof schema === "object" &&
    schema !== null &&
    "type" in schema &&
    schema.type === "integer";
  const text = representation === "text",
    bytes = representation === "bytes",
    json = representation === "json";
  const label = integer
    ? "Integer value"
    : text
      ? "Text value"
      : bytes
        ? "Bytes (Base64)"
        : representation === "byte_array"
          ? "Byte values (JSON)"
          : "JSON value";
  const encoded =
    bytes && value !== null && typeof value === "object" && "base64" in value
      ? String(value.base64)
      : "";
  const display =
    draft?.raw ??
    (bytes
      ? encoded
      : json
        ? JSON.stringify(value)
        : typeof value === "string"
          ? value
          : JSON.stringify(value));
  const input = () => {
    const raw = (integer ? field.current?.value : area.current?.value) ?? "";
    try {
      if (
        new TextEncoder().encode(raw).byteLength > limits.pipeline_literal_bytes
      )
        throw new Error("This literal is too long for the Pipeline.");
      let parsed: unknown;
      if (integer) {
        if (
          !/^-?(0|[1-9][0-9]*)$/.test(raw) ||
          !Number.isSafeInteger(Number(raw))
        )
          throw new Error(
            "Enter an exact integer from -9007199254740991 to 9007199254740991.",
          );
        parsed = Number(raw);
      } else
        parsed = bytes
          ? { base64: raw }
          : text
            ? raw
            : json
              ? parseExactJson(raw)
              : JSON.parse(raw);
      if (
        new TextEncoder().encode(JSON.stringify(parsed)).byteLength >
        limits.pipeline_literal_bytes
      )
        throw new Error("This literal is too long for the Pipeline.");
      onDraft({ raw, error: "" });
      onChange(parsed);
    } catch (error) {
      onDraft({
        raw,
        error:
          error instanceof SyntaxError
            ? "Enter valid JSON. Strings need quotes; arrays use [ ]."
            : error instanceof Error
              ? error.message
              : "Enter a valid value.",
      });
    }
  };
  return (
    <>
      <label htmlFor={id}>{label}</label>
      {integer ? (
        <VscodeTextfield
          ref={field}
          id={id}
          label={label}
          value={display}
          disabled={disabled}
          aria-invalid={draft?.error ? "true" : "false"}
          aria-describedby={draft?.error ? `${id}-error` : undefined}
          onInput={input}
        />
      ) : (
        <VscodeTextarea
          ref={area}
          id={id}
          label={label}
          value={display}
          rows={2}
          disabled={disabled}
          aria-invalid={draft?.error ? "true" : "false"}
          aria-describedby={draft?.error ? `${id}-error` : undefined}
          onInput={input}
        />
      )}
      {draft?.error ? (
        <p id={`${id}-error`} role="alert">
          {draft.error}
        </p>
      ) : null}
    </>
  );
}
