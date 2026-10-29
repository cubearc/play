import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useRef } from "react";
import limits from "../../desktop/protocol.json";
import type { NoteView } from "../desktop/workspace-contract";
export interface NoteDraftState {
  value: string;
  dirty: boolean;
  saving: boolean;
}
export interface NoteEditing {
  state: (note: NoteView) => NoteDraftState;
  edit: (note: NoteView, value: string) => void;
  save: (id?: string) => Promise<boolean>;
  discard: (id: string) => void;
}
export function NoteCell({
  note,
  editing,
}: {
  note: NoteView;
  editing: NoteEditing;
}) {
  const field = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const draft = editing.state(note);
  const reasoning = note.kind === "reasoning";
  const bytes = new TextEncoder().encode(draft.value).byteLength;
  return (
    <>
      {note.origin ? (
        <p className="copy-origin">
          Copied from {note.origin.title} · r{note.origin.revision}
        </p>
      ) : null}
      <VscodeTextarea
        ref={field}
        label={reasoning ? "Stage note" : "Annotation text"}
        value={draft.value}
        rows={3}
        disabled={draft.saving}
        resize="vertical"
        onInput={() => editing.edit(note, field.current?.value ?? "")}
      />
      <div className="note-save">
        <VscodeButton
          icon="save"
          disabled={!draft.dirty || draft.saving || bytes > limits.note_bytes}
          onClick={() => void editing.save(note.id)}
        >
          {reasoning ? "Save reasoning" : "Save annotation"}
        </VscodeButton>
        {draft.dirty ? (
          <VscodeButton
            secondary
            disabled={draft.saving}
            onClick={() => editing.discard(note.id)}
          >
            Discard draft
          </VscodeButton>
        ) : null}
        <span role="status" aria-label="Note save status">
          {draft.saving ? "Saving…" : draft.dirty ? "Unsaved changes" : "Saved"}
        </span>
        <span className="muted">
          {bytes} / {limits.note_bytes} UTF-8 bytes
        </span>
      </div>
    </>
  );
}
