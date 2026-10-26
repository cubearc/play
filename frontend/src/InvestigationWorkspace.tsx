import {
  type FormEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { ArtifactLibrary, TextLab } from "./ArtifactLibrary";
import { ChallengePanel, type FileSelection } from "./ChallengePanel";
import { FileWorkspace } from "./FileWorkspace";
import { InvestigationNotes } from "./InvestigationNotes";
import {
  type Investigation,
  loadInvestigation,
  saveInvestigation,
  type View,
} from "./investigation-api";

type Selection = { name: string; intent: "file" | "challenge" } | null;
export function InvestigationWorkspace({
  initialSelection,
}: {
  initialSelection: Selection;
}) {
  const [request, setRequest] = useState(0);
  return (
    <LoadedInvestigation
      key={request}
      initialSelection={initialSelection}
      reload={() => setRequest((value) => value + 1)}
    />
  );
}
function LoadedInvestigation({
  initialSelection,
  reload,
}: {
  initialSelection: Selection;
  reload: () => void;
}) {
  const [investigation, setInvestigation] = useState<Investigation | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setInvestigation(null);
    setError(null);
    loadInvestigation(controller.signal).then(
      (value) => {
        if (!controller.signal.aborted) setInvestigation(value);
      },
      (error: unknown) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error
              ? error.message
              : "The investigation could not be restored.",
          );
      },
    );
    return () => controller.abort();
  }, []);
  if (error)
    return (
      <section className="session-panel">
        <p role="alert">{error}</p>
        <button type="button" className="button secondary" onClick={reload}>
          Reload investigation
        </button>
      </section>
    );
  if (!investigation) return <p role="status">Restoring investigation…</p>;
  return (
    <SavedWorkspace
      key={investigation.id}
      investigation={investigation}
      initialSelection={initialSelection}
      reload={reload}
    />
  );
}
function SavedWorkspace({
  investigation,
  initialSelection,
  reload,
}: {
  investigation: Investigation;
  initialSelection: Selection;
  reload: () => void;
}) {
  const [title, setTitle] = useState(investigation.title);
  const [requestedSelection, setRequestedSelection] =
    useState<FileSelection | null>(null);
  const [selection, setSelection] = useState<View["selected"]>(
    initialSelection
      ? { store: "source", path: initialSelection.name }
      : investigation.view.selected,
  );
  const [titleRevision, setTitleRevision] = useState(investigation.revision);
  const [renaming, setRenaming] = useState(false);
  const [titleError, setTitleError] = useState<string | null>(null);
  const [status, setStatus] = useState("Navigation saved locally");
  const [error, setError] = useState<string | null>(null);
  const active = useRef(true);
  const queue = useRef<{
    pending: View | null;
    saving: boolean;
    revision: number;
    last: string;
    failed: boolean;
  }>({
    pending: null,
    saving: false,
    revision: investigation.view_revision,
    last: JSON.stringify(investigation.view),
    failed: false,
  });
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const saveView = useCallback((view: View) => {
    setSelection(view.selected);
    const state = queue.current;
    const encoded = JSON.stringify(view);
    if (state.failed || encoded === state.last) return;
    state.last = encoded;
    state.pending = view;
    if (state.saving) return;
    state.saving = true;
    setStatus("Saving navigation…");
    void (async () => {
      try {
        while (state.pending && active.current) {
          const next = state.pending;
          state.pending = null;
          state.revision = await saveInvestigation("/view", {
            revision: state.revision,
            view: next,
          });
        }
        if (active.current) setStatus("Navigation saved locally");
      } catch (error) {
        state.failed = true;
        if (active.current)
          setError(
            error instanceof Error
              ? error.message
              : "Navigation could not be saved. Reload the investigation.",
          );
      } finally {
        state.saving = false;
      }
    })();
  }, []);
  async function rename(event: FormEvent) {
    event.preventDefault();
    if (renaming) return;
    setRenaming(true);
    setTitleError(null);
    try {
      const revision = await saveInvestigation("", {
        revision: titleRevision,
        title,
      });
      if (active.current) setTitleRevision(revision);
    } catch (error) {
      if (active.current)
        setTitleError(
          error instanceof Error
            ? error.message
            : "The name could not be saved.",
        );
    } finally {
      if (active.current) setRenaming(false);
    }
  }
  return (
    <>
      <section className="investigation-state" aria-label="Saved investigation">
        <form onSubmit={rename}>
          <label htmlFor="investigation-name">Investigation name</label>
          <input
            id="investigation-name"
            required
            maxLength={512}
            value={title}
            onChange={(event) => setTitle(event.target.value)}
          />
          <button
            type="submit"
            className="button secondary"
            disabled={renaming}
          >
            {renaming ? "Saving name…" : "Save name"}
          </button>
        </form>
        {investigation.interrupted_sessions > 0 && (
          <p>
            Previous investigation data recovered after an interrupted session.
          </p>
        )}
        {investigation.warnings.map((warning) => (
          <p className="file-warning" key={warning}>
            {warning}
          </p>
        ))}
        {error || titleError ? (
          <div>
            <p role="alert">{error ?? titleError}</p>
            <button className="button secondary" type="button" onClick={reload}>
              Reload investigation
            </button>
          </div>
        ) : (
          <p role="status">{status}</p>
        )}
      </section>
      <ChallengePanel
        selected={selection}
        launch={initialSelection}
        onOpen={setRequestedSelection}
      />
      <FileWorkspace
        requestedSelection={requestedSelection}
        initialSelection={initialSelection}
        initialView={initialSelection ? undefined : investigation.view}
        onViewChange={saveView}
      />
      <TextLab
        key={selection ? `${selection.store}:${selection.path}` : "inline"}
        file={selection}
      />
      <InvestigationNotes target={selection} />
      <ArtifactLibrary />
    </>
  );
}
