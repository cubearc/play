import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import { parseExactJson } from "./exactJson";
import { Select } from "./GraphInputs";
import VscodeButton from "./VscodeButton";

export type DialogKind =
  | "exploration"
  | "stage"
  | "text"
  | "image"
  | "annotation"
  | "page"
  | "structured";
const titles: Record<DialogKind, string> = {
  exploration: "New Exploration",
  stage: "New Stage",
  text: "Add text block",
  image: "Add image with caption",
  annotation: "Add annotation",
  page: "Add page",
  structured: "Add structured data",
};
export function ContentDialog({
  kind,
  onClose,
  onSubmit,
  error,
  description,
  onDirty,
  structuredTypes = [],
}: {
  kind: DialogKind;
  onClose: () => void;
  onSubmit: (
    title: string,
    text: string,
    key: string,
    semanticType?: string,
  ) => Promise<boolean>;
  structuredTypes?: string[];
  error: string;
  description?: string;
  onDirty?: (dirty: boolean) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleField = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const textField = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const key = useRef(crypto.randomUUID());
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [semanticType, setSemanticType] = useState(structuredTypes[0] ?? "");
  const [jsonError, setJsonError] = useState("");
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const valid =
    kind === "image" ||
    (Boolean(title.trim()) && (kind !== "structured" || Boolean(semanticType)));
  const submit = async () => {
    if (!valid || submitting) return;
    if (kind === "structured") {
      try {
        parseExactJson(text);
        if (new TextEncoder().encode(text).byteLength > limits.output_bytes)
          throw new Error("This value exceeds the supported data limit.");
        setJsonError("");
      } catch (error) {
        setJsonError(
          error instanceof Error ? error.message : "Enter a valid JSON value.",
        );
        return;
      }
    }
    setSubmitting(true);
    const accepted = await onSubmit(title, text, key.current, semanticType);
    setSubmitting(false);
    if (accepted) onClose();
  };
  return (
    <dialog
      ref={dialog}
      className="content-dialog"
      aria-label={titles[kind]}
      onCancel={onClose}
    >
      <h2>{titles[kind]}</h2>
      {description ? (
        <p className="linked-target-label">{description}</p>
      ) : null}
      {error || jsonError ? <p role="alert">{jsonError || error}</p> : null}
      {kind === "structured" ? (
        <Select
          id="content-type"
          label="Data type"
          value={semanticType}
          disabled={submitting}
          options={structuredTypes.map((value) => ({ value, label: value }))}
          onChange={(value) => {
            setSemanticType(value);
            key.current = crypto.randomUUID();
            onDirty?.(true);
          }}
        />
      ) : null}
      {kind !== "image" ? (
        <>
          <label htmlFor="content-title">Title</label>
          <VscodeTextfield
            ref={titleField}
            id="content-title"
            label="Title"
            value={title}
            maxlength={120}
            onInput={() => {
              onDirty?.(true);
              key.current = crypto.randomUUID();
              setTitle(titleField.current?.value ?? "");
            }}
          />
        </>
      ) : null}
      {kind === "text" ||
      kind === "image" ||
      kind === "annotation" ||
      kind === "page" ||
      kind === "structured" ? (
        <>
          <label htmlFor="content-text">
            {kind === "image"
              ? "Caption"
              : kind === "page"
                ? "Page source"
                : kind === "structured"
                  ? "JSON value"
                  : "Text"}
          </label>
          <VscodeTextarea
            ref={textField}
            id="content-text"
            label={
              kind === "image"
                ? "Caption"
                : kind === "page"
                  ? "Page source"
                  : kind === "structured"
                    ? "JSON value"
                    : "Text"
            }
            value={text}
            rows={4}
            onInput={() => {
              onDirty?.(true);
              key.current = crypto.randomUUID();
              setText(textField.current?.value ?? "");
            }}
          />
          <small>
            Up to{" "}
            {(kind === "annotation"
              ? limits.note_bytes
              : kind === "structured"
                ? limits.output_bytes
                : limits.text_bytes
            ).toLocaleString()}{" "}
            UTF-8 bytes
            {kind === "image" ? " · PNG images up to 128 KiB" : ""}.
          </small>
        </>
      ) : null}
      <div className="dialog-actions">
        <VscodeButton secondary disabled={submitting} onClick={onClose}>
          Cancel
        </VscodeButton>
        <VscodeButton
          disabled={!valid || submitting}
          onClick={() => void submit()}
        >
          {kind === "image"
            ? "Choose PNG"
            : kind === "text"
              ? "Add text"
              : kind === "page"
                ? "Add page"
                : kind === "annotation"
                  ? "Add annotation"
                  : kind === "structured"
                    ? "Add data"
                    : "Create"}
        </VscodeButton>
      </div>
    </dialog>
  );
}
