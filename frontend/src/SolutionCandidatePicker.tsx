import { useEffect, useRef, useState } from "react";
import { type CandidateState, candidates } from "./candidate-api";
import { type ChallengeState, challengeRequest } from "./challenge-api";
import type { CandidateRef } from "./solution-api";

type Candidate = CandidateState["history"][number];
export function SolutionCandidatePicker({
  onChoose,
  disabled,
}: {
  onChoose: (
    reference: CandidateRef,
    candidate: Candidate,
    signal: AbortSignal,
  ) => Promise<void>;
  disabled: boolean;
}) {
  const [context, setContext] = useState<ChallengeState | null>(null),
    [stage, setStage] = useState(""),
    [state, setState] = useState<CandidateState | null>(null),
    [revision, setRevision] = useState(""),
    [error, setError] = useState<string | null>(null),
    [busy, setBusy] = useState(false),
    [opened, setOpened] = useState(false);
  const active = useRef<AbortController | null>(null);
  const key = context?.active?.definition.key;
  useEffect(() => () => active.current?.abort(), []);
  useEffect(() => {
    if (!key || !stage) return;
    const controller = new AbortController();
    active.current?.abort();
    active.current = controller;
    setState(null);
    setRevision("");
    setError(null);
    candidates(key, stage, controller.signal).then(
      (value) => {
        if (!controller.signal.aborted) {
          setState(value);
          setRevision(value.candidate ? String(value.candidate.revision) : "");
        }
      },
      (error) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error
              ? error.message
              : "Local candidates could not be read.",
          );
      },
    );
    return () => controller.abort();
  }, [key, stage]);
  async function open() {
    const controller = new AbortController();
    active.current?.abort();
    active.current = controller;
    setBusy(true);
    setError(null);
    setOpened(true);
    try {
      const value = await challengeRequest("", undefined, controller.signal);
      if (!controller.signal.aborted) {
        setContext(value);
        setStage(value.active?.definition.manifest.stages[0]?.id ?? "");
      }
    } catch (error) {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "Local challenges are unavailable.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  async function choose() {
    const candidate = state?.history.find(
      (c) => String(c.revision) === revision,
    );
    if (!candidate || !key) return;
    const controller = new AbortController();
    active.current?.abort();
    active.current = controller;
    setBusy(true);
    setError(null);
    try {
      await onChoose(
        { key, stage, revision: candidate.revision },
        candidate,
        controller.signal,
      );
    } catch (error) {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "This candidate cannot be used with the retained evidence.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  return (
    <section aria-label="Existing local stage candidate">
      <h4>Optional stage association</h4>
      <p>
        Use an existing private candidate revision from the active local
        challenge. Its history and stage progress are preserved.
      </p>
      <button
        type="button"
        className="button secondary"
        disabled={disabled || busy}
        onClick={() => void open()}
      >
        Choose local stage candidate
      </button>
      {opened && (
        <>
          {busy && <p role="status">Reading local candidate evidence…</p>}
          {context && !context.active && (
            <p>
              Import a local challenge in Local challenges to associate a stage.
              A free finding does not require one.
            </p>
          )}
          {context?.active && (
            <>
              <p>{context.active.definition.manifest.title}</p>
              <label>
                Candidate stage
                <select
                  value={stage}
                  disabled={disabled || busy}
                  onChange={(event) => setStage(event.target.value)}
                >
                  {context.active.definition.manifest.stages.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.title}
                    </option>
                  ))}
                </select>
              </label>
              {state ? (
                <>
                  {state.error && (
                    <p role="alert">
                      {state.error} Existing references remain available.
                    </p>
                  )}
                  {state.history.length ? (
                    <>
                      <label>
                        Candidate revision
                        <select
                          value={revision}
                          disabled={disabled || busy}
                          onChange={(event) => setRevision(event.target.value)}
                        >
                          {state.history.map((c) => (
                            <option key={c.revision} value={c.revision}>
                              Revision {c.revision} · {c.representation} ·{" "}
                              {c.byte_size} bytes
                            </option>
                          ))}
                        </select>
                      </label>
                      <button
                        type="button"
                        className="button secondary"
                        disabled={disabled || busy || !revision}
                        onClick={() => void choose()}
                      >
                        Use candidate revision
                      </button>
                    </>
                  ) : (
                    <p>
                      No candidate has been selected for this stage. Preserve a
                      Lab artifact and associate it in the existing candidate
                      controls first.
                    </p>
                  )}
                </>
              ) : (
                <p role="status">Loading candidate history…</p>
              )}
            </>
          )}
        </>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
