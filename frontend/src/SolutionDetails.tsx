import { useEffect, useRef, useState } from "react";
import {
  type FinalPreview,
  finalPreview,
  type SolutionState,
} from "./solution-api";
import { TypedValue } from "./TypedOutputs";

export function SolutionDetails({
  state,
  busy,
  onRevision,
  onRevise,
  onExplore,
}: {
  state: SolutionState;
  busy: boolean;
  onRevision: (revision: number) => void;
  onRevise: () => void;
  onExplore: () => void;
}) {
  const s = state.solution;
  const [preview, setPreview] = useState<FinalPreview | null>(null),
    [error, setError] = useState<string | null>(null),
    [reading, setReading] = useState(false),
    [imageError, setImageError] = useState(false);
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  async function inspect() {
    active.current?.abort();
    const controller = new AbortController();
    active.current = controller;
    setReading(true);
    setError(null);
    setPreview(null);
    setImageError(false);
    try {
      const value = await finalPreview(s.id, s.revision, controller.signal);
      if (
        value.metadata.sha256 !== s.final_metadata.sha256 ||
        value.metadata.byte_size !== s.final_metadata.byte_size
      )
        throw new Error(
          "The selected value differs from the saved solution. Its snapshot was preserved.",
        );
      if (!controller.signal.aborted) setPreview(value);
    } catch (error) {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "The final value is unavailable. The saved explanation and references were preserved.",
        );
    } finally {
      if (!controller.signal.aborted) setReading(false);
    }
  }
  const report = state.replay_current;
  const gaps = Array.isArray(report.gaps) ? report.gaps.slice(0, 256) : [];
  return (
    <section className="notebook-panel" aria-label="Saved solution">
      <h3>{s.title}</h3>
      <p>
        {s.local_status === "resolved" ? "Resolved locally" : "Finding"} ·
        Private · Unverified
      </p>
      <p>
        This records your assessment. Saving does not submit an answer or change
        stage progress.
      </p>
      <label>
        Solution revision
        <select
          value={s.revision}
          disabled={busy}
          onChange={(event) => onRevision(Number(event.target.value))}
        >
          {state.history.map((h) => (
            <option key={h.revision} value={h.revision}>
              Revision {h.revision} · {h.title}
            </option>
          ))}
        </select>
      </label>
      <label>
        Saved explanation
        <textarea
          className="text-preview"
          readOnly
          rows={6}
          value={s.explanation}
        />
      </label>
      <div className="lab-controls">
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={onRevise}
        >
          Revise this solution
        </button>
        <button type="button" className="button secondary" onClick={onExplore}>
          Continue in Explorations
        </button>
      </div>
      <h4>Final selection</h4>
      <p>
        {String(s.final_metadata.semantic_type)} ·{" "}
        {String(s.final_metadata.representation)} ·{" "}
        {String(s.final_metadata.byte_size)} bytes
        {s.final_output.port ? ` · output ${s.final_output.port}` : ""}
      </p>
      {s.final_output.field && (
        <p>
          Explicit{" "}
          {s.final_output.field.conversion === "text"
            ? "text selection"
            : "Base64 to bytes conversion"}{" "}
          · field{" "}
          <code>
            {s.final_output.field.pointer || "the selected output itself"}
          </code>
        </p>
      )}
      <details>
        <summary>Final reference and digest</summary>
        <p>
          Artifact <code>{s.final_output.result_id}</code>
        </p>
        <p>
          SHA-256 <code>{String(s.final_metadata.sha256)}</code>
        </p>
      </details>
      <button
        type="button"
        className="button secondary"
        disabled={reading}
        onClick={() => void inspect()}
      >
        {reading ? "Reading final value…" : "Inspect final value"}
      </button>
      {error && <p role="alert">{error}</p>}
      {preview && (
        <section aria-label="Solution final value">
          {preview.metadata.representation === "image" &&
          preview.value &&
          typeof preview.value === "object" &&
          !Array.isArray(preview.value) &&
          typeof preview.value.base64 === "string" ? (
            imageError ? (
              <p>
                Image preview is unavailable. The artifact and its recorded
                digest remain available.
              </p>
            ) : (
              <img
                className="solution-image"
                src={`data:image/png;base64,${preview.value.base64}`}
                alt="Selected solution artifact"
                onError={() => setImageError(true)}
              />
            )
          ) : (
            <TypedValue
              semantic={preview.metadata.semantic_type}
              value={preview.value}
            />
          )}
        </section>
      )}
      <h4>Replay availability</h4>
      <button
        type="button"
        className="button secondary"
        disabled={busy}
        onClick={() => {
          setPreview(null);
          onRevision(s.revision);
        }}
      >
        Check solution evidence again
      </button>
      <p>
        {report.status === "inputs_available"
          ? "Recorded inputs are currently available."
          : "Replay has gaps that need attention."}{" "}
        This check reads evidence without running operations and does not verify
        correctness.
      </p>
      {gaps.length > 0 && (
        <ul>
          {gaps.map((gap) => (
            <li key={JSON.stringify(gap)}>
              {gap && typeof gap === "object" && "message" in gap
                ? String(gap.message)
                : "A recorded input or version is unavailable."}
            </li>
          ))}
        </ul>
      )}
      <details>
        <summary>
          Selected execution path · {s.evidence_path.length} retained results
        </summary>
        <p>
          The path contains the actual results used by this finding. Your
          exploration retains its other branches.
        </p>
        <ul className="solution-evidence">
          {s.evidence_path.map((record) => (
            <li key={record.id}>
              <h4>{record.provenance.operation}</h4>
              <p>
                {new Date(record.provenance.created_ms).toLocaleString()} · Core{" "}
                {record.provenance.core_version}
              </p>
              <p>
                Result <code>{record.id}</code>
              </p>
              <label>
                Recorded parameters
                <textarea
                  className="text-preview"
                  readOnly
                  rows={5}
                  value={JSON.stringify(
                    record.provenance.parameters,
                    null,
                    2,
                  ).slice(0, 65536)}
                />
              </label>
              <label>
                Recorded inputs and digests
                <textarea
                  className="text-preview"
                  readOnly
                  rows={6}
                  value={JSON.stringify(
                    record.provenance.inputs,
                    null,
                    2,
                  ).slice(0, 65536)}
                />
              </label>
              <label>
                Recorded output schema identities
                <textarea
                  className="text-preview"
                  readOnly
                  rows={5}
                  value={JSON.stringify(record.metadata, null, 2).slice(
                    0,
                    65536,
                  )}
                />
              </label>
            </li>
          ))}
        </ul>
        <p>Each metadata preview is limited to 65,536 characters.</p>
        <label>
          Selected steps from immutable configurations
          <textarea
            className="text-preview"
            readOnly
            rows={8}
            value={JSON.stringify(s.configurations, null, 2).slice(0, 65536)}
          />
        </label>
      </details>
      {s.candidate !== null && (
        <details>
          <summary>Frozen local stage candidate</summary>
          <p>
            The candidate revision remains private and unverified. Saving does
            not unlock a stage.
          </p>
          <textarea
            className="text-preview"
            readOnly
            rows={8}
            aria-label="Frozen stage candidate"
            value={JSON.stringify(s.candidate, null, 2).slice(0, 65536)}
          />
        </details>
      )}
    </section>
  );
}
