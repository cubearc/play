import { useState } from "react";
import type { LabResult } from "./lab-api";
import { request } from "./notebook-api";
import { CandidateControls } from "./StageCandidate";
export function SavedResultActions({ result }: { result: LabResult }) {
  const [title, setTitle] = useState(""),
    [status, setStatus] = useState(""),
    [saving, setSaving] = useState(false);
  const extension =
    result.representation === "image"
      ? "png"
      : result.representation === "text"
        ? "txt"
        : result.representation === "structured"
          ? "json"
          : "bin";
  return (
    <section aria-label="Preserve selected result">
      <label>
        Artifact name
        <input
          value={title}
          maxLength={512}
          onChange={(event) => setTitle(event.target.value)}
        />
      </label>
      <div className="lab-controls">
        <button
          type="button"
          className="button secondary"
          disabled={!result.retained || !title.trim() || saving}
          onClick={() => {
            setSaving(true);
            void request("/lab/library", undefined, {
              id: result.id,
              title,
              revision: 0,
            })
              .then(
                () =>
                  setStatus(
                    "Artifact saved locally. Refresh the artifact library to reopen it.",
                  ),
                (error) =>
                  setStatus(
                    error instanceof Error
                      ? error.message
                      : "Artifact could not be saved.",
                  ),
              )
              .finally(() => setSaving(false));
          }}
        >
          Save named artifact
        </button>
        <a
          className="button secondary"
          href={`/api/lab/results/${result.id}/export?name=cube-play-${result.id}.${extension}`}
          download
        >
          Export selected artifact
        </a>
      </div>
      <p role="status">{status}</p>
      <CandidateControls result={result} />
      <p>
        Export uses your browser’s download destination. The application
        receives no host write path.
      </p>
    </section>
  );
}
