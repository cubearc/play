import {
  type FormEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { ArtifactLibrary, TextLab } from "./ArtifactLibrary";
import { ChallengePanel, type FileSelection } from "./ChallengePanel";
import { CubeTerminal } from "./CubeTerminal";
import { ExplorationLab } from "./ExplorationLab";
import { FileWorkspace } from "./FileWorkspace";
import { InvestigationNotes } from "./InvestigationNotes";
import {
  type Investigation,
  loadInvestigation,
  type Panel,
  saveInvestigation,
  type View,
} from "./investigation-api";
import type { EvidenceTarget } from "./notebook-api";
import { type SolutionIntent, SolutionsPanel } from "./SolutionsPanel";
import { WorkspacePanels } from "./WorkspacePanels";

function panelView(view: View, panel: Panel): View {
  const next = { ...view };
  if (panel === "files") delete next.panel;
  else next.panel = panel;
  return next;
}

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
  const [noteEvidence, setNoteEvidence] = useState<EvidenceTarget | null>(null);
  const [solutionIntent, setSolutionIntent] = useState<SolutionIntent | null>(
    null,
  );
  const [savedTitle, setSavedTitle] = useState(investigation.title);
  const [panel, setPanel] = useState<Panel>(
    initialSelection
      ? initialSelection.intent === "challenge"
        ? "challenges"
        : "files"
      : (investigation.view.panel ?? "files"),
  );
  const panelRef = useRef(panel);
  const viewRef = useRef(investigation.view);
  const [requestedSelection, setRequestedSelection] =
    useState<FileSelection | null>(null);
  const [selection, setSelection] = useState<View["selected"]>(
    initialSelection
      ? { store: "source", path: initialSelection.name }
      : investigation.view.selected,
  );
  const [titleRevision, setTitleRevision] = useState(investigation.revision);
  const [renaming, setRenaming] = useState(false);
  const renameDetails = useRef<HTMLDetailsElement | null>(null);
  useEffect(() => {
    if (title === savedTitle && !renaming) return;
    const guard = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", guard);
    return () => window.removeEventListener("beforeunload", guard);
  }, [title, savedTitle, renaming]);
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
    viewRef.current = view;
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
  const saveFilesView = useCallback(
    (view: View) => saveView(panelView(view, panelRef.current)),
    [saveView],
  );
  function changePanel(next: Panel) {
    panelRef.current = next;
    setPanel(next);
    saveView(panelView(viewRef.current, next));
  }
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
      if (active.current) {
        setTitleRevision(revision);
        setSavedTitle(title);
        if (renameDetails.current) {
          renameDetails.current.open = false;
          renameDetails.current.querySelector("summary")?.focus();
        }
      }
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
        <details ref={renameDetails} className="investigation-rename">
          <summary>
            <span className="investigation-caption">Investigation</span>
            <strong>{savedTitle}</strong>
            <span className="rename-hint">Rename</span>
          </summary>
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
        </details>
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
      <WorkspacePanels active={panel} onChange={changePanel}>
        {{
          files: (
            <>
              {selection && (
                <div className="selected-asset">
                  <span>
                    Selected asset:{" "}
                    <code>
                      {selection.store}/{selection.path}
                    </code>
                  </span>
                  <button
                    type="button"
                    className="button secondary"
                    onClick={() => changePanel("explorations")}
                  >
                    Explore selected file
                  </button>
                </div>
              )}
              <FileWorkspace
                requestedSelection={requestedSelection}
                initialSelection={initialSelection}
                initialView={initialSelection ? undefined : investigation.view}
                onViewChange={saveFilesView}
              />
            </>
          ),
          explorations: (
            <>
              <p className="selected-asset">
                {selection ? (
                  <>
                    Selected asset:{" "}
                    <code>
                      {selection.store}/{selection.path}
                    </code>
                  </>
                ) : (
                  "Choose a file in Files, or begin with a typed literal input."
                )}
              </p>
              <ExplorationLab
                asset={selection}
                onAnnotate={(target) => {
                  setNoteEvidence(target);
                  changePanel("notes");
                }}
                onPreserve={(id, port) => {
                  setSolutionIntent({ result_id: id, port });
                  changePanel("solutions");
                }}
              />
              <details className="quick-tools">
                <summary>Quick text and byte tools</summary>
                <TextLab
                  key={
                    selection
                      ? `${selection.store}:${selection.path}`
                      : "inline"
                  }
                  file={selection}
                />
              </details>
            </>
          ),
          notes: (
            <InvestigationNotes target={selection} evidence={noteEvidence} />
          ),
          artifacts: <ArtifactLibrary />,
          solutions: (
            <SolutionsPanel
              wanted={solutionIntent}
              onExplore={() => changePanel("explorations")}
            />
          ),
          challenges: (
            <ChallengePanel
              selected={selection}
              launch={initialSelection}
              onOpen={(value) => {
                setRequestedSelection(value);
                changePanel("files");
              }}
            />
          ),
          terminal: <CubeTerminal />,
        }}
      </WorkspacePanels>
    </>
  );
}
