import type { CandidateState } from "./candidate-api";
import { SolutionCandidatePicker } from "./SolutionCandidatePicker";
import {
  type CandidateRef,
  ports,
  type RetainedResult,
  type SolutionRequest,
} from "./solution-api";

export function SolutionEditor({
  draft,
  results,
  busy,
  saving,
  error,
  onChange,
  onSave,
  onCancel,
  onBrowse,
  onReload,
  onCandidate,
}: {
  draft: SolutionRequest;
  results: RetainedResult[];
  busy: boolean;
  saving: boolean;
  error: string | null;
  onChange: (value: SolutionRequest) => void;
  onSave: () => void;
  onCancel: () => void;
  onBrowse: () => void;
  onReload: () => void;
  onCandidate: (
    reference: CandidateRef,
    candidate: CandidateState["history"][number],
    signal: AbortSignal,
  ) => Promise<void>;
}) {
  const source = draft.source;
  const root =
    source.type === "result"
      ? results.find((r) => r.id === source.selection.result_id)
      : null;
  const names = root ? ports(root) : [];
  const valid =
    draft.title.trim().length > 0 &&
    new TextEncoder().encode(draft.title).length <= 512 &&
    draft.explanation.trim().length > 0 &&
    new TextEncoder().encode(draft.explanation).length <= 8192;
  return (
    <section className="notebook-panel" aria-label="Solution editor">
      <h3>
        {draft.revision
          ? "Revise a private solution"
          : "Preserve a private finding"}
      </h3>
      <p>
        Saving creates immutable revision {draft.revision + 1}. Your
        explanation, selected output and actual dependency path stay together.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (valid && !busy) onSave();
        }}
      >
        <fieldset disabled={busy}>
          <legend>Explanation and local assessment</legend>
          <label>
            Solution title
            <input
              required
              value={draft.title}
              maxLength={512}
              onChange={(event) =>
                onChange({ ...draft, title: event.target.value })
              }
            />
          </label>
          <label>
            Solution explanation
            <textarea
              required
              rows={7}
              value={draft.explanation}
              maxLength={8192}
              onChange={(event) =>
                onChange({ ...draft, explanation: event.target.value })
              }
            />
          </label>
          <p>
            Use up to 512 UTF-8 bytes for the title and 8 KiB for the
            explanation.
          </p>
          <label className="solution-check">
            <input
              type="checkbox"
              checked={draft.local_status === "resolved"}
              onChange={(event) =>
                onChange({
                  ...draft,
                  local_status: event.target.checked ? "resolved" : "finding",
                })
              }
            />{" "}
            Mark as resolved locally
          </label>
          <p>
            This is your private assessment. The result remains unverified and
            no answer is submitted.
          </p>
          {source.type === "revision" ? (
            <p>
              The final selection, candidate and evidence are preserved from
              revision {source.revision}. You can revise the explanation while
              recovering unavailable artifacts.
            </p>
          ) : (
            <>
              <p>
                Selected result:{" "}
                {root?.provenance.operation ?? source.selection.result_id}
              </p>
              {names.length > 0 && (
                <label>
                  Final output port
                  <select
                    value={source.selection.port ?? ""}
                    onChange={(event) =>
                      onChange({
                        ...draft,
                        source: {
                          ...source,
                          selection: {
                            ...source.selection,
                            port: event.target.value,
                            field: null,
                          },
                          candidate: null,
                        },
                      })
                    }
                  >
                    {names.map((name) => (
                      <option key={name} value={name}>
                        {name}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              {root?.representation === "structured" && (
                <>
                  <label className="solution-check">
                    <input
                      type="checkbox"
                      checked={source.selection.field !== null}
                      onChange={(event) =>
                        onChange({
                          ...draft,
                          source: {
                            ...source,
                            selection: {
                              ...source.selection,
                              field: event.target.checked
                                ? {
                                    pointer: source.selection.port ? "" : "/",
                                    conversion: "text",
                                  }
                                : null,
                            },
                            candidate: null,
                          },
                        })
                      }
                    />{" "}
                    Select or convert a string field explicitly
                  </label>
                  {source.selection.field && (
                    <>
                      <label>
                        Final field JSON pointer
                        <input
                          value={source.selection.field.pointer}
                          maxLength={512}
                          onChange={(event) =>
                            onChange({
                              ...draft,
                              source: {
                                ...source,
                                selection: {
                                  ...source.selection,
                                  field: {
                                    pointer: event.target.value,
                                    conversion:
                                      source.selection.field?.conversion ??
                                      "text",
                                  },
                                },
                                candidate: null,
                              },
                            })
                          }
                        />
                      </label>
                      <p>
                        Use /field/0 for a nested field. An empty pointer
                        selects the named output itself. Numeric and other
                        implicit conversions are rejected.
                      </p>
                      <label>
                        Final field conversion
                        <select
                          value={source.selection.field.conversion}
                          onChange={(event) =>
                            onChange({
                              ...draft,
                              source: {
                                ...source,
                                selection: {
                                  ...source.selection,
                                  field: {
                                    pointer:
                                      source.selection.field?.pointer ?? "",
                                    conversion:
                                      event.target.value === "base64_bytes"
                                        ? "base64_bytes"
                                        : "text",
                                  },
                                },
                                candidate: null,
                              },
                            })
                          }
                        >
                          <option value="text">Text string</option>
                          <option value="base64_bytes">
                            Base64 string to bytes
                          </option>
                        </select>
                      </label>
                    </>
                  )}
                </>
              )}
              {source.candidate && (
                <p>
                  Local stage {source.candidate.stage} · candidate revision{" "}
                  {source.candidate.revision}{" "}
                  <button
                    type="button"
                    className="button secondary"
                    onClick={() =>
                      onChange({
                        ...draft,
                        source: { ...source, candidate: null },
                      })
                    }
                  >
                    Remove stage association
                  </button>
                </p>
              )}
            </>
          )}
          <button type="button" className="button secondary" onClick={onBrowse}>
            Choose another retained result
          </button>
        </fieldset>
        <SolutionCandidatePicker disabled={busy} onChoose={onCandidate} />
        {error && (
          <div>
            <p role="alert">
              {error} Your draft and earlier revisions were preserved.
            </p>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={onReload}
            >
              Load latest revision and keep draft
            </button>
          </div>
        )}
        <div className="lab-controls">
          <button type="submit" className="button" disabled={busy || !valid}>
            {saving ? "Saving solution…" : "Save private solution"}
          </button>
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={onCancel}
          >
            Cancel editing and discard draft
          </button>
        </div>
      </form>
    </section>
  );
}
