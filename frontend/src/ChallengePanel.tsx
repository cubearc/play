import { useEffect, useRef, useState } from "react";
import {
  type ChallengeState,
  challengeRequest,
  importChallenge,
} from "./challenge-api";
import { StageCandidate } from "./StageCandidate";
import type { Store } from "./workspace-api";
export type FileSelection = { store: Store; path: string };
export function ChallengePanel({
  selected,
  launch,
  onOpen,
}: {
  selected: FileSelection | null;
  launch: { name: string; intent: "file" | "challenge" } | null;
  onOpen: (file: FileSelection) => void;
}) {
  const [state, setState] = useState<ChallengeState | null>(null),
    [error, setError] = useState<string | null>(null),
    [busy, setBusy] = useState(false);
  const imported = useRef(false);
  useEffect(() => {
    const controller = new AbortController();
    void (async () => {
      try {
        let value: ChallengeState;
        if (launch?.intent === "challenge" && !imported.current) {
          imported.current = true;
          value = await importChallenge(
            { store: "source", path: launch.name },
            controller.signal,
          );
        } else value = await challengeRequest("", undefined, controller.signal);
        if (!controller.signal.aborted) setState(value);
      } catch (error) {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error ? error.message : "Challenge unavailable.",
          );
      }
    })();
    return () => controller.abort();
  }, [launch]);
  async function run(action: () => Promise<ChallengeState>) {
    setBusy(true);
    setError(null);
    try {
      setState(await action());
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Challenge request failed.",
      );
    } finally {
      setBusy(false);
    }
  }
  const active = state?.active,
    stage = active?.definition.manifest.stages.find(
      (stage) => stage.id === active.stage,
    ),
    progress = active?.progress.find((item) => item.stage_key === stage?.id);
  return (
    <section className="notebook-panel" aria-label="Local challenges">
      <h2>Local challenges</h2>
      <p>
        Optional structured investigations. Imports contain inert instructions;
        local progress is private and unverified.
      </p>
      <div className="lab-controls">
        <button
          type="button"
          className="button secondary"
          disabled={busy || !selected}
          onClick={() => {
            if (selected) void run(() => importChallenge(selected));
          }}
        >
          Import selected challenge
        </button>
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={() => void run(() => challengeRequest())}
        >
          Reload local challenges
        </button>
      </div>
      {error && (
        <p role="alert">{error} No challenge definition was executed.</p>
      )}
      {busy && <p role="status">Validating local challenge…</p>}
      {!active && (
        <p>No active challenge. Free Lab and notes work without one.</p>
      )}
      {state && state.challenges.length > 0 && (
        <label>
          Saved challenge
          <select
            value={active?.definition.key ?? ""}
            disabled={busy}
            onChange={(event) =>
              void run(() =>
                challengeRequest("/active", { key: event.target.value }),
              )
            }
          >
            <option value="">Choose a local challenge</option>
            {state.challenges.map((item) => (
              <option key={item.key} value={item.key}>
                {item.title}
              </option>
            ))}
          </select>
        </label>
      )}
      {active && stage && (
        <>
          <h3>{active.definition.manifest.title}</h3>
          <p className="inert-instructions">
            {active.definition.manifest.description}
          </p>
          <nav aria-label="Challenge stages" className="lab-controls">
            {active.definition.manifest.stages.map((item) => (
              <button
                type="button"
                className="button secondary"
                key={item.id}
                disabled={busy}
                aria-current={item.id === stage.id ? "step" : undefined}
                onClick={() =>
                  void run(() =>
                    challengeRequest("/stage", {
                      key: active.definition.key,
                      stage: item.id,
                      revision: active.revision,
                    }),
                  )
                }
              >
                {item.title}
              </button>
            ))}
          </nav>
          <h4>{stage.title}</h4>
          <p className="inert-instructions">{stage.instructions}</p>
          <ol>
            {stage.steps.map((step) => (
              <li key={step.id} className="inert-instructions">
                {step.instructions}
              </li>
            ))}
          </ol>
          {stage.hints.map((hint, index) => (
            <details key={hint}>
              <summary>Hint {index + 1}</summary>
              <p className="inert-instructions">{hint}</p>
            </details>
          ))}
          <div className="lab-controls">
            {stage.artifacts.map((id) => {
              const artifact = active.artifacts.find((item) => item.id === id);
              return (
                artifact && (
                  <button
                    className="button secondary"
                    type="button"
                    key={id}
                    onClick={() =>
                      onOpen({ store: artifact.store, path: artifact.path })
                    }
                  >
                    Open challenge artifact {id}
                  </button>
                )
              );
            })}
          </div>
          <p>
            Terminal paths:{" "}
            {stage.artifacts
              .map((id) => {
                const artifact = active.artifacts.find(
                  (item) => item.id === id,
                );
                return artifact
                  ? `${artifact.store === "source" ? "/workspace/source" : "/workspace/data/imports"}/${artifact.path}`
                  : id;
              })
              .join(", ") || "No stage artifacts"}
          </p>
          <label>
            Private local progress
            <select
              value={progress?.local_status ?? "unstarted"}
              disabled={busy}
              onChange={(event) =>
                void run(() =>
                  challengeRequest("/progress", {
                    key: active.definition.key,
                    stage: stage.id,
                    revision: progress?.revision ?? 0,
                    status: event.target.value,
                  }),
                )
              }
            >
              <option value="unstarted">Unstarted locally</option>
              <option value="in_progress">Investigating locally</option>
              <option value="completed">
                Marked complete locally — unverified
              </option>
            </select>
          </label>
          <StageCandidate
            challengeKey={active.definition.key}
            stage={stage.id}
          />
        </>
      )}
    </section>
  );
}
