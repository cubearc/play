import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { EditableAssetView } from "../desktop/workspace-contract";

export function TextAssetEditor({
  asset,
  caption,
  error,
  onClose,
  onDirty,
  onSave,
}: {
  asset: EditableAssetView;
  caption: boolean;
  error: string;
  onClose: () => void;
  onDirty: (dirty: boolean) => void;
  onSave: (text: string, key: string) => Promise<boolean>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const field = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const key = useRef(crypto.randomUUID());
  const [text, setText] = useState(asset.content.text);
  const [saving, setSaving] = useState(false);
  const changed = text !== asset.content.text;
  const length = new TextEncoder().encode(text).byteLength;
  const valid = changed && length <= limits.text_bytes;
  const page = asset.content.kind === "page";
  const title = caption ? "Edit caption" : page ? "Edit page" : "Edit text";
  const label = caption ? "Caption" : page ? "Page source" : "Text";
  useEffect(() => {
    dialog.current?.showModal();
    field.current?.focus();
  }, []);
  useEffect(() => onDirty(changed), [changed, onDirty]);
  const save = async () => {
    if (!valid || saving) return;
    setSaving(true);
    try {
      if (await onSave(text, key.current)) onClose();
    } finally {
      setSaving(false);
    }
  };
  return (
    <dialog
      ref={dialog}
      className="content-dialog text-editor"
      aria-label={title}
      onCancel={(event) => {
        event.preventDefault();
        if (!saving) onClose();
      }}
      onKeyDown={(event) => {
        if (!(event.metaKey || event.ctrlKey) || event.altKey) return;
        if (event.key.toLowerCase() === "s") {
          event.preventDefault();
          event.stopPropagation();
          if (!event.repeat) void save();
        } else if (event.key === "Enter") {
          event.preventDefault();
          event.stopPropagation();
        }
      }}
    >
      <h2>{title}</h2>
      <p>
        {asset.title} · r{asset.revision}
      </p>
      {error ? <p role="alert">{error}</p> : null}
      <label htmlFor="asset-text">{label}</label>
      <VscodeTextarea
        ref={field}
        id="asset-text"
        label={label}
        value={text}
        rows={6}
        disabled={saving}
        onInput={() => {
          key.current = crypto.randomUUID();
          setText(field.current?.value ?? "");
        }}
      />
      <small>
        {length} / {limits.text_bytes} UTF-8 bytes. Saving preserves earlier
        revisions and Runs.
      </small>
      {length > limits.text_bytes ? (
        <p role="alert">
          Reduce the text to {limits.text_bytes} UTF-8 bytes to save.
        </p>
      ) : null}
      <div className="dialog-actions">
        <VscodeButton secondary disabled={saving} onClick={onClose}>
          Cancel
        </VscodeButton>
        <VscodeButton
          icon="save"
          disabled={!valid || saving}
          onClick={() => void save()}
        >
          Save changes
        </VscodeButton>
      </div>
    </dialog>
  );
}
