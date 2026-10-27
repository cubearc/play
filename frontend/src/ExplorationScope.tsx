import { useEffect, useRef, useState } from "react";
import { challengeRequest } from "./challenge-api";
import type { Exploration } from "./exploration-api";

type Choice = { challenge: string; stage: string; label: string };
export function ExplorationScope({
  scope,
  onChange,
  disabled,
}: {
  scope: Exploration["scope"];
  onChange: (scope: Exploration["scope"]) => void;
  disabled: boolean;
}) {
  const [choices, setChoices] = useState<Choice[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  async function load() {
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setNotice("");
    try {
      const state = await challengeRequest("", undefined, controller.signal);
      if (controller.signal.aborted) return;
      const challenge = state.active?.definition;
      setChoices(
        challenge?.manifest.stages.map((stage) => ({
          challenge: challenge.key,
          stage: stage.id,
          label: `${challenge.manifest.title} → ${stage.title}`,
        })) ?? [],
      );
      setNotice(
        challenge
          ? "Choose a stage from the active local challenge. Switch challenges in Local challenges to choose another."
          : "Import a local challenge in Local challenges first, or keep this exploration free.",
      );
    } catch (error) {
      if (!controller.signal.aborted)
        setNotice(
          error instanceof Error
            ? error.message
            : "Local stages are unavailable.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  const value = scope ? `${scope.challenge}:${scope.stage}` : "free";
  return (
    <fieldset disabled={disabled || busy} className="exploration-scope">
      <legend>Investigation scope</legend>
      <label>
        Exploration scope
        <select
          value={value}
          onChange={(event) => {
            if (event.target.value === "free") onChange(null);
            else {
              const choice = choices.find(
                (c) => `${c.challenge}:${c.stage}` === event.target.value,
              );
              if (choice)
                onChange({ challenge: choice.challenge, stage: choice.stage });
            }
          }}
        >
          <option value="free">Free exploration</option>
          {scope &&
            !choices.some(
              (c) => c.challenge === scope.challenge && c.stage === scope.stage,
            ) && (
              <option value={value}>Saved local stage: {scope.stage}</option>
            )}
          {choices.map((c) => (
            <option
              key={`${c.challenge}:${c.stage}`}
              value={`${c.challenge}:${c.stage}`}
            >
              {c.label}
            </option>
          ))}
        </select>
      </label>
      <button
        type="button"
        className="button secondary"
        onClick={() => void load()}
      >
        {busy ? "Reading local stages…" : "Load local stages"}
      </button>
      {notice && <p role="status">{notice}</p>}
      <p>
        Association does not run operations, change stage progress or submit a
        solution.
      </p>
    </fieldset>
  );
}
