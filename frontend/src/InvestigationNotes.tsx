import { useEffect, useRef, useState } from "react";
import { type Note, notes, request } from "./notebook-api";
import type { Store } from "./workspace-api";

function validNote(value: Note) {
  return (
    value.title.trim().length > 0 &&
    new TextEncoder().encode(value.title).length <= 512 &&
    new TextEncoder().encode(value.body).length <= 65536
  );
}
type Target = { store: Store; path: string } | null;
export function InvestigationNotes({ target }: { target: Target }) {
  const [items, setItems] = useState<Note[]>([]),
    [active, setActive] = useState<Note | null>(null);
  const [error, setError] = useState<string | null>(null),
    [status, setStatus] = useState("Loading notes…"),
    [generation, setGeneration] = useState(0);
  const draft = useRef<Note | null>(null),
    running = useRef(false),
    blocked = useRef(false),
    mounted = useRef(true);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    mounted.current = true;
    const controller = new AbortController();
    notes(controller.signal).then(
      (value) => {
        setItems(value);
        setStatus("Notes saved locally");
      },
      (error) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error ? error.message : "Notes unavailable.",
          );
      },
    );
    return () => {
      mounted.current = false;
      controller.abort();
      if (timer.current) clearTimeout(timer.current);
    };
  }, []);
  async function save() {
    if (running.current || blocked.current || !draft.current) return;
    running.current = true;
    try {
      while (draft.current && validNote(draft.current) && !blocked.current) {
        const sent = draft.current;
        draft.current = null;
        const response = (await request(
          "/investigation/notes",
          undefined,
          sent,
        )) as { revision: number };
        const saved = { ...sent, revision: response.revision };
        if (draft.current && (draft.current as Note).id === sent.id)
          (draft.current as Note).revision = saved.revision;
        if (mounted.current) {
          setItems((previous) => [
            ...previous.filter((item) => item.id !== saved.id),
            saved,
          ]);
          setActive((previous) =>
            previous?.id === saved.id
              ? { ...previous, revision: saved.revision }
              : previous,
          );
        }
      }
      if (mounted.current)
        setStatus(
          draft.current
            ? "Use a title of 1–512 UTF-8 bytes and a body up to 64 KiB."
            : "Notes saved locally",
        );
    } catch (error) {
      blocked.current = true;
      if (mounted.current) {
        setError(
          error instanceof Error ? error.message : "Note could not be saved.",
        );
        setStatus("Unsaved draft retained in this editor");
      }
    } finally {
      running.current = false;
    }
  }
  function edit(value: Note) {
    setActive(value);
    draft.current = value;
    setStatus("Saving note…");
    if (timer.current) clearTimeout(timer.current);
    if (!validNote(value)) {
      setStatus("Use a title of 1–512 UTF-8 bytes and a body up to 64 KiB.");
      return;
    }
    timer.current = setTimeout(() => void save(), 400);
  }
  async function reload() {
    try {
      const value = await notes(new AbortController().signal);
      setItems(value);
      setActive(null);
      draft.current = null;
      blocked.current = false;
      setError(null);
      setStatus("Notes saved locally");
      setGeneration((v) => v + 1);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Notes unavailable.");
    }
  }
  const dirty = status !== "Notes saved locally";
  useEffect(() => {
    if (!active || !dirty) return;
    const handler = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [active, dirty]);
  return (
    <section className="notebook-panel" aria-label="Private notes">
      <h2>Private notes</h2>
      <p>
        Autosaved only in this investigation. Wait for the saved confirmation
        before leaving.
      </p>
      <div className="lab-controls">
        <button
          type="button"
          className="button secondary"
          disabled={status === "Loading notes…" || (dirty && active !== null)}
          onClick={() => {
            const note: Note = {
              id: crypto.randomUUID(),
              revision: 0,
              title: "Untitled note",
              body: "",
              target,
            };
            setActive(note);
            edit(note);
          }}
        >
          New note
        </button>
        {items.map((note) => (
          <button
            className="button secondary"
            type="button"
            key={note.id}
            disabled={dirty}
            aria-pressed={active?.id === note.id}
            onClick={() => setActive(note)}
          >
            {note.title}
          </button>
        ))}
      </div>
      {active && (
        <div key={generation} className="note-editor">
          <label htmlFor={`note-title-${active.id}`}>Note title</label>
          <input
            id={`note-title-${active.id}`}
            value={active.title}
            maxLength={512}
            onChange={(event) => edit({ ...active, title: event.target.value })}
          />
          <label htmlFor={`note-body-${active.id}`}>Note body</label>
          <textarea
            id={`note-body-${active.id}`}
            rows={8}
            maxLength={65536}
            value={active.body}
            onChange={(event) => edit({ ...active, body: event.target.value })}
          />
          <p>
            Associated with:{" "}
            {active.target
              ? `${active.target.store}/${active.target.path}`
              : "this investigation"}
          </p>
          <button
            type="button"
            className="button secondary"
            disabled={blocked.current}
            onClick={() => edit({ ...active, target })}
          >
            Associate with selected file or investigation
          </button>
        </div>
      )}
      <p role="status">{status}</p>
      {error && (
        <div>
          <p role="alert">
            {error} Copy your unsaved draft before reloading; newer stored
            content was preserved.
          </p>
          <button
            type="button"
            className="button secondary"
            onClick={() => void reload()}
          >
            Reload notes and discard local draft
          </button>
        </div>
      )}
    </section>
  );
}
