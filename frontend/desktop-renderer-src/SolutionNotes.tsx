import type { NoteRevisionRef, NoteView } from "../desktop/workspace-contract";
import { LinkedEvidence } from "./EvidenceNotes";
import VscodeButton from "./VscodeButton";
import VscodeCheckbox from "./VscodeCheckbox";

export function NotePicker({
  notes,
  selected,
  disabled,
  onChange,
}: {
  notes: NoteView[];
  selected: NoteRevisionRef[];
  disabled: boolean;
  onChange: (notes: NoteRevisionRef[]) => void;
}) {
  return (
    <fieldset className="solution-note-picker">
      <legend>Include saved notes</legend>
      <p className="muted">
        Choose the reasoning to preserve. Save notebook edits before selecting
        their revisions.
      </p>
      {notes.map((note) => {
        const chosen = selected.find((item) => item.note_id === note.id);
        const toggle = () =>
          onChange(
            chosen
              ? selected.filter((item) => item.note_id !== note.id)
              : [...selected, { note_id: note.id, revision: note.revision }],
          );
        return (
          <div key={note.id}>
            <VscodeCheckbox
              checked={!!chosen}
              disabled={disabled}
              onChange={toggle}
            >
              {note.title} · r{chosen?.revision ?? note.revision}
            </VscodeCheckbox>
            {chosen && chosen.revision !== note.revision ? (
              <VscodeButton
                secondary
                disabled={disabled}
                onClick={() =>
                  onChange(
                    selected.map((item) =>
                      item.note_id === note.id
                        ? { ...item, revision: note.revision }
                        : item,
                    ),
                  )
                }
              >
                Use {note.title} r{note.revision}
              </VscodeButton>
            ) : null}
          </div>
        );
      })}
    </fieldset>
  );
}
export function SavedSolutionNotes({
  notes,
}: {
  notes: NoteView[] | undefined;
}) {
  if (!notes?.length) return null;
  return (
    <section aria-label="Included solution notes">
      <h4>Included notes</h4>
      {notes.map((note) => (
        <article
          key={`${note.id}:${note.revision}`}
          aria-label={`${note.title} revision ${note.revision}`}
        >
          <h5>
            {note.title} · r{note.revision}
          </h5>
          <p className="solution-explanation">
            {note.text || "Empty saved note."}
          </p>
          {note.target ? (
            <LinkedEvidence noteId={note.id} link={note.target} />
          ) : null}
        </article>
      ))}
    </section>
  );
}
