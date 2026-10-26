import { useEffect, useState } from "react";
import { type CandidateState, candidates } from "./candidate-api";
import { type ChallengeState, challengeRequest } from "./challenge-api";
import type { LabResult } from "./lab-api";

function CandidateDetails({ state }: { state: CandidateState }) {
  return (
    <>
      {state.error && (
        <p role="alert">
          {state.error} Candidate reference retained for recovery.
        </p>
      )}
      {state.candidate ? (
        <>
          <p>
            Private candidate revision {state.candidate.revision} ·{" "}
            {state.candidate.representation} · {state.candidate.byte_size} bytes
            · unverified
          </p>
          <p>
            Artifact: <code>{state.candidate.result_id}</code>
          </p>
          <p>
            Integrity digest: <code>{state.candidate.sha256}</code>
          </p>
          <details>
            <summary>Candidate selection history</summary>
            <textarea
              className="text-preview"
              readOnly
              aria-label="Candidate provenance history"
              rows={8}
              value={JSON.stringify(state.history, null, 2)}
            />
          </details>
        </>
      ) : (
        <p>No private candidate selected for this stage.</p>
      )}
      <p>
        Local association only. No submission, answer checking, official
        verification or stage unlock occurs.
      </p>
    </>
  );
}
export function StageCandidate({
  challengeKey,
  stage,
}: {
  challengeKey: string;
  stage: string;
}) {
  const [state, setState] = useState<CandidateState | null>(null),
    [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setState(null);
    setError(null);
    candidates(challengeKey, stage, controller.signal).then(
      setState,
      (error) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error ? error.message : "Candidate unavailable.",
          );
      },
    );
    return () => controller.abort();
  }, [challengeKey, stage]);
  return (
    <section aria-label="Stage candidate">
      <h4>Private stage candidate</h4>
      <button
        type="button"
        className="button secondary"
        onClick={() => {
          setError(null);
          void candidates(challengeKey, stage).then(setState, (error) =>
            setError(
              error instanceof Error ? error.message : "Candidate unavailable.",
            ),
          );
        }}
      >
        Refresh stage candidate
      </button>
      {error && <p role="alert">{error}</p>}
      {!state && !error && <p role="status">Restoring private candidate…</p>}
      {state && <CandidateDetails state={state} />}
    </section>
  );
}
export function CandidateControls({ result }: { result: LabResult }) {
  const [opened, setOpened] = useState(false),
    [context, setContext] = useState<ChallengeState | null>(null),
    [stage, setStage] = useState(""),
    [snapshot, setSnapshot] = useState<CandidateState | null>(null),
    [error, setError] = useState<string | null>(null),
    [busy, setBusy] = useState(false),
    [kind, setKind] = useState("whole"),
    [pointer, setPointer] = useState(""),
    [conversion, setConversion] = useState("text");
  const key = context?.active?.definition.key;
  useEffect(() => {
    if (!key || !stage) return;
    const controller = new AbortController();
    setSnapshot(null);
    setError(null);
    candidates(key, stage, controller.signal).then(setSnapshot, (error) => {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error ? error.message : "Candidate unavailable.",
        );
    });
    return () => controller.abort();
  }, [key, stage]);
  async function open() {
    setBusy(true);
    setOpened(true);
    setError(null);
    try {
      const value = await challengeRequest();
      setContext(value);
      setStage(value.active?.stage ?? "");
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Challenge unavailable.",
      );
    } finally {
      setBusy(false);
    }
  }
  const selected = context?.active?.definition.manifest.stages.find(
    (item) => item.id === stage,
  );
  return (
    <section aria-label="Stage candidate association">
      <button
        type="button"
        className="button secondary"
        disabled={busy || !result.retained}
        onClick={() => void open()}
      >
        Use result as stage candidate
      </button>
      {opened && (
        <>
          {!context?.active && (
            <p>
              Import or activate a local challenge first. Save this result as a
              named artifact before association.
            </p>
          )}
          {context?.active && (
            <>
              <p>
                Save the named artifact first. Candidate conversions are
                explicit and bounded to 64 KiB.
              </p>
              <label>
                Candidate stage
                <select
                  value={stage}
                  disabled={busy}
                  onChange={(event) => {
                    setSnapshot(null);
                    setStage(event.target.value);
                  }}
                >
                  {context.active.definition.manifest.stages.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.title} · {item.answer_type}
                    </option>
                  ))}
                </select>
              </label>
              <p>Selected stage expects {selected?.answer_type}.</p>
              <label>
                Candidate selection
                <select
                  value={kind}
                  disabled={busy}
                  onChange={(event) => setKind(event.target.value)}
                >
                  <option value="whole">Whole text or byte artifact</option>
                  {result.representation === "structured" && (
                    <option value="field">
                      Explicit structured string field
                    </option>
                  )}
                </select>
              </label>
              {kind === "field" && (
                <>
                  <label>
                    JSON field pointer
                    <input
                      value={pointer}
                      maxLength={512}
                      placeholder="/channels/0"
                      onChange={(event) => setPointer(event.target.value)}
                    />
                  </label>
                  <label>
                    Field interpretation
                    <select
                      value={conversion}
                      onChange={(event) => setConversion(event.target.value)}
                    >
                      <option value="text">String as UTF-8 text</option>
                      <option value="base64_bytes">
                        Base64 decoded by Core as bytes
                      </option>
                    </select>
                  </label>
                </>
              )}
              <div className="lab-controls">
                <button
                  type="button"
                  className="button secondary"
                  disabled={
                    busy ||
                    !snapshot ||
                    result.representation === "image" ||
                    (kind === "field" && !pointer.startsWith("/"))
                  }
                  onClick={() => {
                    if (!key) return;
                    setBusy(true);
                    setError(null);
                    void candidates(key, stage, undefined, {
                      key,
                      stage,
                      result_id: result.id,
                      revision: snapshot?.candidate?.revision ?? 0,
                      selection:
                        kind === "whole"
                          ? { kind: "whole" }
                          : { kind: "field", pointer, conversion },
                    })
                      .then(setSnapshot, (error) =>
                        setError(
                          error instanceof Error
                            ? error.message
                            : "Candidate association failed.",
                        ),
                      )
                      .finally(() => setBusy(false));
                  }}
                >
                  Associate private candidate
                </button>
                <button
                  type="button"
                  className="button secondary"
                  disabled={busy}
                  onClick={() => {
                    if (key)
                      void candidates(key, stage).then(setSnapshot, (error) =>
                        setError(
                          error instanceof Error
                            ? error.message
                            : "Candidate unavailable.",
                        ),
                      );
                  }}
                >
                  Reload candidate revision
                </button>
              </div>
              {result.representation === "image" && (
                <p>
                  Images require an explicit text/byte or structured result
                  before candidate selection.
                </p>
              )}
              {snapshot && <CandidateDetails state={snapshot} />}
            </>
          )}
          {error && (
            <p role="alert">
              {error} Existing candidate revisions were preserved.
            </p>
          )}
          {busy && <p role="status">Saving private association…</p>}
        </>
      )}
    </section>
  );
}
