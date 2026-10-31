import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import limits from "../../desktop/protocol.json" with { type: "json" };
import type {
  NoteView,
  SolutionSummary,
  SolutionView,
  WorkspaceRequest,
} from "../desktop/workspace-contract";
import { CandidateEvidence, useCandidate } from "./Candidates";
import { ContributingPath, SolutionLineage } from "./SolutionLineage";
import { NotePicker, SavedSolutionNotes } from "./SolutionNotes";
import { errorMessage } from "./useWorkspace";
import VscodeButton from "./VscodeButton";

type Intent = Extract<WorkspaceRequest, { kind: "save_solution" }>;
function useSolutions(notes: NoteView[]) {
  const candidate = useCandidate();
  const owner = useRef(candidate?.context).current;
  const [items, setItems] = useState<SolutionSummary[]>();
  const [value, setValue] = useState<SolutionView>();
  const [draft, setDraft] = useState<Intent>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [expanded, setExpanded] = useState(false);
  const active = useRef(true);
  const lock = useRef(false);
  const intent = useRef<Intent | undefined>(undefined);
  const read = useCallback(
    async (id?: string, revision: number | null = null) => {
      if (!owner || lock.current) return;
      lock.current = true;
      setBusy(true);
      try {
        const result = await window.play.workspace({
          kind: "list_solutions",
          context: owner,
        });
        const selected = id ?? result.solutions.at(-1)?.id;
        const detail = selected
          ? await window.play.workspace({
              kind: "get_solution",
              context: owner,
              solution_id: selected,
              revision,
            })
          : undefined;
        if (active.current) {
          setItems(result.solutions);
          setValue(detail);
          setError("");
        }
      } catch (e) {
        if (active.current) setError(errorMessage(e));
      } finally {
        lock.current = false;
        if (active.current) setBusy(false);
      }
    },
    [owner],
  );
  useEffect(() => {
    active.current = true;
    void read();
    return () => {
      active.current = false;
    };
  }, [read]);
  const begin = (existing?: SolutionView) => {
    if (!owner || !candidate?.value || busy) return;
    intent.current = undefined;
    setDraft({
      kind: "save_solution",
      context: owner,
      key: crypto.randomUUID(),
      solution_id: existing?.id ?? crypto.randomUUID(),
      expected_revision: existing?.revision ?? 0,
      notes:
        existing?.snapshot.notes.map((note) => ({
          note_id: note.id,
          revision: note.revision,
        })) ?? [],
      candidate_revision:
        existing?.snapshot.candidate.revision ??
        candidate.value.selectedRevision,
      title: existing?.title ?? "",
      explanation: existing?.explanation ?? "",
    });
    setError("");
  };
  const save = async () => {
    if (!draft || !owner || lock.current) return;
    lock.current = true;
    setBusy(true);
    setError("");
    const previous = intent.current;
    const command =
      previous &&
      previous.title === draft.title &&
      previous.explanation === draft.explanation &&
      previous.candidate_revision === draft.candidate_revision &&
      JSON.stringify(previous.notes) === JSON.stringify(draft.notes)
        ? previous
        : { ...draft, key: crypto.randomUUID() };
    intent.current = command;
    try {
      const saved = await window.play.workspace(command);
      const next = await window.play.workspace({
        kind: "get_solution",
        context: owner,
        solution_id: saved.solutionId,
        revision: saved.revision,
      });
      const list = await window.play.workspace({
        kind: "list_solutions",
        context: owner,
      });
      intent.current = undefined;
      if (active.current) {
        setValue(next);
        setItems(list.solutions);
        setDraft(undefined);
      }
    } catch (e) {
      let message = errorMessage(e);
      if (e instanceof Error && e.message.includes("state_conflict")) {
        intent.current = undefined;
        const latest = await window.play
          .workspace({
            kind: "get_solution",
            context: owner,
            solution_id: command.solution_id,
            revision: null,
          })
          .catch(() => undefined);
        if (active.current && latest) {
          setValue(latest);
          setDraft({ ...draft, expected_revision: latest.revision });
        }
        message =
          "This Solution changed. Your draft is preserved. Review the saved revision before saving again.";
      }
      if (e instanceof Error && /solution_(revision_)?limit/.test(e.message))
        message =
          "The Solution history limit was reached. Earlier walkthroughs remain available.";
      if (active.current) setError(message);
    } finally {
      lock.current = false;
      if (active.current) setBusy(false);
    }
  };
  return {
    notes,
    expanded,
    setExpanded,
    candidate,
    items,
    value,
    draft,
    setDraft,
    busy,
    error,
    read,
    begin,
    save,
  };
}
const SolutionsContext = createContext<ReturnType<typeof useSolutions> | null>(
  null,
);
export function SolutionsProvider({
  children,
  notes,
}: {
  children: ReactNode;
  notes: NoteView[];
}) {
  const state = useSolutions(notes);
  return (
    <SolutionsContext.Provider value={state}>
      {children}
    </SolutionsContext.Provider>
  );
}
export function SolutionsPanel() {
  const state = useContext(SolutionsContext);
  const titleField = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const explanationField =
    useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  if (!state?.candidate?.context) return null;
  const {
    expanded,
    setExpanded,
    candidate,
    items,
    value,
    draft,
    setDraft,
    busy,
    error,
    read,
    begin,
    save,
  } = state;
  const context = candidate.context;
  if (!context) return null;
  const canCreate =
    !!candidate.value &&
    candidate.value.evidence.kind !== "unavailable" &&
    items !== undefined &&
    items.length < limits.solution_limit;
  const path = value?.snapshot.path;
  return (
    <section
      className="candidate-panel solution-panel"
      aria-label="Stage solutions"
    >
      <details
        open={expanded}
        onToggle={(event) => setExpanded(event.currentTarget.open)}
      >
        <summary className="solution-summary">
          <h2>Solutions</h2>
          <span className="muted">
            {items?.length ?? 0}{" "}
            {items?.length === 1 ? "walkthrough" : "walkthroughs"} · Private ·
            Unverified
          </span>
        </summary>
        {error ? <p role="alert">{error}</p> : null}
        {items === undefined ? (
          <p role="status">
            {error ? "Solutions unavailable." : "Loading solutions…"}
          </p>
        ) : items.length === 0 ? (
          <p className="muted">
            Explain a selected candidate and preserve its contributing evidence
            as a walkthrough.
          </p>
        ) : (
          <label className="candidate-history">
            Walkthrough{" "}
            <select
              aria-label="Solution"
              value={value?.id ?? ""}
              disabled={busy || !!draft}
              onChange={(e) => void read(e.target.value)}
            >
              {items.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.title} · r{item.revision}
                </option>
              ))}
            </select>
          </label>
        )}
        {value ? (
          <article aria-label="Saved solution">
            <label className="candidate-history">
              Revision{" "}
              <select
                aria-label="Solution revision"
                value={value.selectedRevision}
                disabled={busy || !!draft}
                onChange={(e) => void read(value.id, Number(e.target.value))}
              >
                {value.history.map((entry) => (
                  <option key={entry.revision} value={entry.revision}>
                    Revision {entry.revision}
                    {entry.revision === value.revision
                      ? " · current"
                      : " · earlier"}
                  </option>
                ))}
              </select>
            </label>
            <h3>{value.title}</h3>
            <p className="solution-explanation">{value.explanation}</p>
            <p className="muted">
              Candidate r{value.snapshot.candidate.revision} · Saved evidence
            </p>
            <CandidateEvidence
              evidence={value.evidence}
              context={context}
              openRun={candidate.openRun}
            />
            {path ? (
              <>
                <h4>Contributing path</h4>
                <ContributingPath path={path} />
                {value.evidence.kind === "unavailable" ? (
                  <VscodeButton
                    secondary
                    onClick={() =>
                      candidate.openRun({
                        trailId: path.trail.id,
                        runId: path.runId,
                      })
                    }
                  >
                    Open source Run
                  </VscodeButton>
                ) : null}
              </>
            ) : (
              <p className="muted">Exact Asset revision.</p>
            )}
            <SolutionLineage
              lineage={value.snapshot.lineage}
              openRun={candidate.openRun}
            />
            <SavedSolutionNotes notes={value.snapshot.notes} />
            {value.gaps.length ? (
              <ul role="alert">
                {value.gaps.map((gap) => (
                  <li key={gap.reference}>
                    Evidence unavailable: {gap.source} ({gap.code})
                  </li>
                ))}
              </ul>
            ) : null}
            <details>
              <summary>Snapshot references</summary>
              <pre>{JSON.stringify(value.snapshot, null, 2)}</pre>
            </details>
            {!draft ? (
              <VscodeButton
                secondary
                disabled={
                  busy || value.revision >= limits.solution_revision_limit
                }
                onClick={() => begin(value)}
              >
                Revise explanation
              </VscodeButton>
            ) : null}
          </article>
        ) : null}
        {draft ? (
          <form
            aria-label="Solution draft"
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <p>
              Saving Candidate r{draft.candidate_revision} in{" "}
              {draft.expected_revision
                ? `Solution revision ${draft.expected_revision + 1}`
                : "a new Solution"}
              .
            </p>
            <label htmlFor="solution-title">Solution title</label>
            <VscodeTextfield
              id="solution-title"
              ref={titleField}
              label="Solution title"
              value={draft.title}
              disabled={busy}
              maxlength={120}
              onInput={() =>
                setDraft({ ...draft, title: titleField.current?.value ?? "" })
              }
            />
            <label htmlFor="solution-explanation">Explanation</label>
            <VscodeTextarea
              id="solution-explanation"
              ref={explanationField}
              label="Solution explanation"
              rows={5}
              value={draft.explanation}
              disabled={busy}
              onInput={() =>
                setDraft({
                  ...draft,
                  explanation: explanationField.current?.value ?? "",
                })
              }
            />
            <small>
              {new TextEncoder().encode(draft.explanation).length} /{" "}
              {limits.note_bytes} bytes
            </small>
            <p className="muted">
              Explain how the selected evidence supports your answer. This saves
              locally.
            </p>
            <NotePicker
              notes={state.notes}
              selected={draft.notes}
              disabled={busy}
              onChange={(notes) => setDraft({ ...draft, notes })}
            />
            <div className="solution-actions">
              <VscodeButton
                disabled={
                  busy ||
                  !draft.title.trim() ||
                  !draft.explanation.trim() ||
                  new TextEncoder().encode(draft.title).length > 120 ||
                  new TextEncoder().encode(draft.explanation).length >
                    limits.note_bytes
                }
                onClick={() => void save()}
              >
                {busy ? "Saving solution…" : "Save solution"}
              </VscodeButton>
              <VscodeButton
                secondary
                disabled={busy}
                onClick={() => setDraft(undefined)}
              >
                Cancel
              </VscodeButton>
            </div>
          </form>
        ) : (
          <VscodeButton
            secondary
            disabled={busy || candidate.busy || !canCreate}
            onClick={() => begin()}
          >
            New solution from Candidate
            {candidate.value ? ` r${candidate.value.selectedRevision}` : ""}
          </VscodeButton>
        )}
        {error ? (
          <VscodeButton
            secondary
            disabled={busy}
            onClick={() => void read(value?.id)}
          >
            Reload solutions
          </VscodeButton>
        ) : null}
      </details>
    </section>
  );
}
