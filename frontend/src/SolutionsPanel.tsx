import { useEffect, useRef, useState } from "react";
import type { CandidateState } from "./candidate-api";
import { SolutionDetails } from "./SolutionDetails";
import { SolutionEditor } from "./SolutionEditor";
import {
  type CandidateRef,
  candidateSelection,
  type FinalSelection,
  ports,
  type RetainedResult,
  retainedResult,
  retainedResults,
  type SolutionHeader,
  type SolutionRequest,
  type SolutionState,
  saveSolution,
  solution,
  solutions,
} from "./solution-api";

export type SolutionIntent = { result_id: string; port: string | null };
const message = (error: unknown) =>
  error instanceof Error
    ? error.message
    : "The private solution is unavailable. Existing evidence was preserved.";
export function SolutionsPanel({
  wanted,
  onExplore,
}: {
  wanted: SolutionIntent | null;
  onExplore: () => void;
}) {
  const [items, setItems] = useState<SolutionHeader[]>([]),
    [loaded, setLoaded] = useState(false),
    [state, setState] = useState<SolutionState | null>(null),
    [draft, setDraft] = useState<SolutionRequest | null>(null),
    [results, setResults] = useState<RetainedResult[]>([]),
    [picking, setPicking] = useState(false),
    [busy, setBusy] = useState(false),
    [saving, setSaving] = useState(false),
    [error, setError] = useState<string | null>(null),
    [status, setStatus] = useState("Loading private solutions…");
  const active = useRef<AbortController | null>(null),
    savingNow = useRef(false);
  useEffect(() => {
    const controller = new AbortController();
    solutions(controller.signal).then(
      (value) => {
        if (!controller.signal.aborted) {
          setItems(value);
          setLoaded(true);
          setStatus("Private solutions loaded.");
        }
      },
      (error) => {
        if (!controller.signal.aborted) {
          setError(message(error));
          setStatus("Saved solutions could not be loaded.");
        }
      },
    );
    return () => {
      controller.abort();
      active.current?.abort();
    };
  }, []);
  useEffect(() => {
    if (!draft) return;
    const before = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", before);
    return () => window.removeEventListener("beforeunload", before);
  }, [draft]);
  function begin() {
    active.current?.abort();
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    return controller;
  }
  function include(result: RetainedResult) {
    setResults((previous) => [
      result,
      ...previous.filter((r) => r.id !== result.id),
    ]);
  }
  async function refresh() {
    const controller = begin();
    try {
      const value = await solutions(controller.signal);
      if (!controller.signal.aborted) {
        setItems(value);
        setLoaded(true);
        setStatus("Private solutions loaded.");
      }
    } catch (error) {
      if (!controller.signal.aborted) setError(message(error));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  async function open(id: string, revision?: number) {
    if (draft) return;
    const controller = begin();
    try {
      const value = await solution(id, controller.signal, revision);
      if (!controller.signal.aborted) {
        setState(value);
        setStatus("Immutable solution revision opened.");
      }
    } catch (error) {
      if (!controller.signal.aborted) setError(message(error));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function choose(result: RetainedResult, port: string | null = null) {
    const names = ports(result);
    if (port !== null && !names.includes(port))
      throw new Error(
        "The selected named output is unavailable in this retained result.",
      );
    const selected: FinalSelection = {
      result_id: result.id,
      port: port ?? names[0] ?? null,
      field: null,
    };
    setDraft((previous) =>
      previous
        ? {
            ...previous,
            source: { type: "result", selection: selected, candidate: null },
          }
        : {
            id: crypto.randomUUID(),
            revision: 0,
            title: `Finding · ${result.provenance.operation}`,
            explanation: "",
            local_status: "finding",
            source: { type: "result", selection: selected, candidate: null },
          },
    );
    include(result);
    setPicking(false);
    setState(null);
    setError(null);
    setStatus(
      "Write an explanation and preserve the selected evidence explicitly.",
    );
  }
  async function browse() {
    const controller = begin();
    setPicking(true);
    try {
      const values = await retainedResults(controller.signal);
      if (!controller.signal.aborted) {
        setResults(values);
        setStatus(
          values.length
            ? "Choose a retained result."
            : "Run a Lab operation to produce a retained result.",
        );
      }
    } catch (error) {
      if (!controller.signal.aborted) setError(message(error));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  async function fromWanted() {
    if (!wanted || draft) return;
    const controller = begin();
    try {
      const root = await retainedResult(wanted.result_id, controller.signal);
      if (!controller.signal.aborted) choose(root, wanted.port);
    } catch (error) {
      if (!controller.signal.aborted) setError(message(error));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  async function save() {
    if (!draft || savingNow.current) return;
    savingNow.current = true;
    const sent = draft,
      controller = begin();
    setSaving(true);
    setStatus("Preserving the selected derivation…");
    try {
      const value = await saveSolution(sent, controller.signal);
      if (!controller.signal.aborted) {
        setState(value);
        setDraft(null);
        setPicking(false);
        setStatus("Solution saved locally. No answer was submitted.");
        try {
          const values = await solutions(controller.signal);
          if (!controller.signal.aborted) {
            setItems(values);
            setLoaded(true);
          }
        } catch (error) {
          if (!controller.signal.aborted) {
            setError(message(error));
            setStatus(
              "Solution saved locally. Refresh the saved list when storage is available.",
            );
          }
        }
      }
    } catch (error) {
      if (!controller.signal.aborted) {
        setError(message(error));
        setStatus("The solution draft and earlier revisions were preserved.");
      }
    } finally {
      savingNow.current = false;
      if (!controller.signal.aborted) {
        setBusy(false);
        setSaving(false);
      }
    }
  }
  async function reloadDraft() {
    if (!draft) return;
    const controller = begin();
    try {
      const value = await solution(draft.id, controller.signal);
      if (!controller.signal.aborted) {
        setState(value);
        setDraft((previous) =>
          previous ? { ...previous, revision: value.latest_revision } : null,
        );
        setStatus(
          `Draft retained. Saving will append revision ${value.latest_revision + 1}.`,
        );
      }
    } catch (error) {
      if (!controller.signal.aborted) setError(message(error));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  async function candidate(
    reference: CandidateRef,
    candidate: CandidateState["history"][number],
    signal: AbortSignal,
  ) {
    if (!draft) return;
    const current = draft.id;
    setBusy(true);
    setError(null);
    try {
      const root = await retainedResult(candidate.result_id, signal);
      const selected = candidateSelection(root, candidate.selection);
      if (!signal.aborted) {
        include(root);
        setDraft((previous) =>
          previous?.id === current
            ? {
                ...previous,
                source: {
                  type: "result",
                  selection: selected,
                  candidate: reference,
                },
              }
            : previous,
        );
        setStatus(
          "Existing local candidate revision selected. Save explicitly to preserve it.",
        );
      }
    } finally {
      if (!signal.aborted) setBusy(false);
    }
  }
  return (
    <section aria-label="Private solutions" className="solutions-panel">
      <h2>Private solutions</h2>
      <p>
        Preserve a selected derivation and your explanation while continuing the
        wider investigation.
      </p>
      <div className="lab-controls">
        <button
          type="button"
          className="button"
          disabled={busy || draft !== null}
          onClick={() => void browse()}
        >
          New finding from retained results
        </button>
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={() => void refresh()}
        >
          Refresh saved solutions
        </button>
        <button type="button" className="button secondary" onClick={onExplore}>
          Open Explorations
        </button>
      </div>
      {wanted && (
        <div className="selected-asset">
          <span>
            Selected Lab output{wanted.port ? `: ${wanted.port}` : ""}
          </span>
          <button
            type="button"
            className="button secondary"
            disabled={busy || draft !== null}
            onClick={() => void fromWanted()}
          >
            Create finding from selected output
          </button>
        </div>
      )}
      <nav className="solution-list" aria-label="Saved solutions">
        {!loaded ? (
          <p>
            {error
              ? "Saved solutions are unavailable. Retry without deleting local state."
              : "Loading saved solutions…"}
          </p>
        ) : items.length ? (
          items.map((item) => (
            <button
              type="button"
              className="button secondary"
              key={item.id}
              disabled={busy || draft !== null}
              onClick={() => void open(item.id)}
            >
              {item.title} · revision {item.revision} ·{" "}
              {item.local_status === "resolved"
                ? "Resolved locally"
                : "Finding"}
            </button>
          ))
        ) : (
          <p>No private solutions are saved yet.</p>
        )}
      </nav>
      {picking && (
        <section
          className="notebook-panel"
          aria-label="Retained result selection"
        >
          <h3>Choose retained evidence</h3>
          {busy ? (
            <p role="status">Reading retained results…</p>
          ) : results.length ? (
            <ul className="solution-results">
              {results.map((result) => (
                <li key={result.id}>
                  <button
                    type="button"
                    className="button secondary"
                    onClick={() => choose(result)}
                  >
                    {result.provenance.operation} ·{" "}
                    {new Date(result.provenance.created_ms).toLocaleString()} ·{" "}
                    {result.id.slice(0, 8)}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p>
              No retained results are available. Create a result in
              Explorations; private output needs explicit retention.
            </p>
          )}
        </section>
      )}
      {draft && (
        <SolutionEditor
          draft={draft}
          results={results}
          busy={busy}
          saving={saving}
          error={error}
          onChange={setDraft}
          onSave={() => void save()}
          onCancel={() => {
            setDraft(null);
            setPicking(false);
            setError(null);
            setStatus(
              "Draft discarded. Earlier solution revisions were preserved.",
            );
          }}
          onBrowse={() => void browse()}
          onReload={() => void reloadDraft()}
          onCandidate={candidate}
        />
      )}
      {state && !draft && (
        <SolutionDetails
          key={`${state.solution.id}:${state.solution.revision}`}
          state={state}
          busy={busy}
          onRevision={(revision) => void open(state.solution.id, revision)}
          onRevise={() => {
            const s = state.solution;
            setDraft({
              id: s.id,
              revision: state.latest_revision,
              title: s.title,
              explanation: s.explanation,
              local_status: s.local_status,
              source: { type: "revision", revision: s.revision },
            });
            setError(null);
          }}
          onExplore={onExplore}
        />
      )}
      {error && !draft && <p role="alert">{error}</p>}
      <p role="status">{status}</p>
    </section>
  );
}
