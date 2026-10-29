import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";

export type DialogKind =
  | "exploration"
  | "stage"
  | "text"
  | "image"
  | "annotation"
  | "page";
const titles: Record<DialogKind, string> = {
  exploration: "New Exploration",
  stage: "New Stage",
  text: "Add text block",
  image: "Add image with caption",
  annotation: "Add annotation",
  page: "Add page",
};
export function ContentDialog({
  kind,
  onClose,
  onSubmit,
  error,
}: {
  kind: DialogKind;
  onClose: () => void;
  onSubmit: (title: string, text: string, key: string) => Promise<boolean>;
  error: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleField = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const textField = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const key = useRef(crypto.randomUUID());
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [submitting, setSubmitting] = useState(false);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const valid = kind === "image" || Boolean(title.trim());
  const submit = async () => {
    if (!valid || submitting) return;
    setSubmitting(true);
    const accepted = await onSubmit(title, text, key.current);
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
      {error ? <p role="alert">{error}</p> : null}
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
              key.current = crypto.randomUUID();
              setTitle(titleField.current?.value ?? "");
            }}
          />
        </>
      ) : null}
      {kind === "text" ||
      kind === "image" ||
      kind === "annotation" ||
      kind === "page" ? (
        <>
          <label htmlFor="content-text">
            {kind === "image"
              ? "Caption"
              : kind === "page"
                ? "Page source"
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
                  : "Text"
            }
            value={text}
            rows={4}
            onInput={() => {
              key.current = crypto.randomUUID();
              setText(textField.current?.value ?? "");
            }}
          />
          <small>
            Up to{" "}
            {(kind === "annotation"
              ? limits.note_bytes
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
                  : "Create"}
        </VscodeButton>
      </div>
    </dialog>
  );
}
