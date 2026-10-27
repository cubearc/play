import { useState } from "react";
import { readLink } from "./capability";
import type { LabResult } from "./lab-api";
import { request } from "./notebook-api";
import { CandidateControls } from "./StageCandidate";
export function SavedResultActions({ result }: { result: LabResult }) {
  const [title, setTitle] = useState(""),
    [status, setStatus] = useState(""),
    [saving, setSaving] = useState(false);
  const [exporting, setExporting] = useState(false);
  const extension =
    result.representation === "image"
      ? "png"
      : result.representation === "text"
        ? "txt"
        : result.representation === "structured"
          ? "json"
          : "bin";
  async function exportResult() {
    setExporting(true);
    setStatus("");
    try {
      const uri = `/api/lab/results/${result.id}/export?name=cube-play-${result.id}.${extension}`;
      const url = await readLink(uri, new AbortController().signal);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `cube-play-${result.id}.${extension}`;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      setStatus("The browser download was requested.");
    } catch (error) {
      setStatus(
        error instanceof Error ? error.message : "Export could not start.",
      );
    } finally {
      setExporting(false);
    }
  }
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
        <button
          type="button"
          className="button secondary"
          disabled={!result.retained || exporting}
          onClick={() => void exportResult()}
        >
          Export selected artifact
        </button>
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
