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
  CandidateSelection,
  CandidateView,
  Context,
  WorkspaceRequest,
} from "../desktop/workspace-contract";
import { AssetImage } from "./AssetImage";
import { FilePreview } from "./FilePreview";
import { formatResultValue } from "./ResultValue";
import { errorMessage } from "./useWorkspace";
import VscodeButton from "./VscodeButton";

function sameSelection(
  left: CandidateSelection | undefined,
  right: CandidateSelection,
): boolean {
  if (left?.kind === "asset" && right.kind === "asset")
    return left.asset_id === right.asset_id && left.revision === right.revision;
  if (left?.kind === "run_output" && right.kind === "run_output")
    return (
      left.run_id === right.run_id &&
      left.attempt_id === right.attempt_id &&
      left.port === right.port
    );
  return false;
}

type Intent = Extract<WorkspaceRequest, { kind: "select_candidate" }>;
interface Candidates {
  context: Context | null;
  value: CandidateView | null | undefined;
  busy: boolean;
  error: string;
  read: (revision: number | null) => Promise<void>;
  select: (selection: CandidateSelection) => Promise<string>;
  openRun: (origin: { trailId: string; runId: string }) => void;
}
const CandidateContext = createContext<Candidates | null>(null);
export function CandidateProvider({
  context,
  disabled,
  openRun,
  children,
}: {
  context: Context | null;
  disabled: boolean;
  openRun: Candidates["openRun"];
  children: ReactNode;
}) {
  const owner = useRef(context).current;
  const [value, setValue] = useState<CandidateView | null>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const active = useRef(true);
  const working = useRef(false);
  const intent = useRef<Intent | null>(null);
  const read = useCallback(
    async (revision: number | null) => {
      if (!owner || working.current) return;
      working.current = true;
      setBusy(true);
      try {
        const next = await window.play.workspace({
          kind: "get_candidate",
          context: owner,
          revision,
        });
        if (active.current) {
          setValue(next.candidate);
          setError("");
        }
      } catch (e) {
        if (active.current) setError(errorMessage(e));
      } finally {
        working.current = false;
        if (active.current) setBusy(false);
      }
    },
    [owner],
  );
  useEffect(() => {
    active.current = true;
    void read(null);
    return () => {
      active.current = false;
    };
  }, [read]);
  const select = async (selection: CandidateSelection): Promise<string> => {
    if (!context || working.current || disabled || value === undefined)
      return "Wait for the Stage candidate to load.";
    working.current = true;
    setBusy(true);
    setError("");
    const previous = intent.current;
    const command: Intent =
      previous && sameSelection(previous.selection, selection)
        ? previous
        : {
            kind: "select_candidate",
            context,
            key: crypto.randomUUID(),
            expected_revision: value?.revision ?? 0,
            selection,
          };
    intent.current = command;
    try {
      await window.play.workspace(command);
      const next = await window.play.workspace({
        kind: "get_candidate",
        context,
        revision: null,
      });
      intent.current = null;
      if (active.current) setValue(next.candidate);
      return "";
    } catch (e) {
      let message = errorMessage(e);
      if (e instanceof Error && e.message.includes("state_conflict")) {
        intent.current = null;
        const next = await window.play
          .workspace({ kind: "get_candidate", context, revision: null })
          .catch(() => undefined);
        if (active.current) setValue(next?.candidate);
        message =
          "The Stage candidate changed. Review its current revision, then choose again.";
      }
      if (e instanceof Error && e.message.includes("candidate_revision_limit"))
        message =
          "This Stage has reached its 32 Candidate revisions. Earlier selections remain available.";
      if (e instanceof Error && e.message.includes("result_not_available"))
        message =
          "Choose an output from a completed Run that retained its result.";
      if (
        e instanceof Error &&
        e.message.includes("retention_policy_unavailable")
      )
        message =
          "This operation's privacy policy does not allow retaining a Candidate output.";
      if (active.current) setError(message);
      return message;
    } finally {
      working.current = false;
      if (active.current) setBusy(false);
    }
  };
  return (
    <CandidateContext.Provider
      value={{
        context,
        value,
        busy: busy || disabled,
        error,
        read,
        select,
        openRun,
      }}
    >
      {children}
    </CandidateContext.Provider>
  );
}

export function CandidateButton({
  selection,
  label,
}: {
  selection: CandidateSelection;
  label: string;
}) {
  const state = useContext(CandidateContext);
  const [message, setMessage] = useState("");
  if (!state) return null;
  const current = state.value?.history[0]?.selection;
  const selected = sameSelection(current, selection);
  const disabled =
    state.busy ||
    state.value === undefined ||
    selected ||
    (state.value?.revision ?? 0) >= limits.candidate_revision_limit;
  return (
    <div className="candidate-action">
      <VscodeButton
        secondary
        icon={selected ? "check" : "bookmark"}
        disabled={disabled}
        aria-disabled={disabled ? "true" : "false"}
        title={
          (state.value?.revision ?? 0) >= limits.candidate_revision_limit
            ? "This Stage has reached its Candidate revision limit."
            : undefined
        }
        aria-label={`${selected ? "Selected" : "Choose"} ${label} as candidate`}
        onClick={async () => {
          setMessage(await state.select(selection));
        }}
      >
        {selected
          ? `Current candidate${selection.kind === "run_output" ? ` · ${label}` : ""}`
          : selection.kind === "run_output"
            ? `Choose ${label} as candidate`
            : "Choose as candidate"}
      </VscodeButton>
      {message ? <p role="alert">{message}</p> : null}
    </div>
  );
}

export function CandidatePanel() {
  const state = useContext(CandidateContext);
  if (!state?.context) return null;
  const candidate = state.value;
  const evidence = candidate?.evidence;
  return (
    <section className="candidate-panel" aria-label="Stage candidate">
      <div className="candidate-heading">
        <h2>Candidate</h2>
        <span className="muted">Private · Unverified</span>
      </div>
      {state.error ? <p role="alert">{state.error}</p> : null}
      {candidate === undefined ? (
        <p role="status">
          {state.error ? "Candidate unavailable." : "Loading candidate…"}
        </p>
      ) : !candidate ? (
        <p className="muted">
          Choose an asset in its canvas or a retained Run output as this Stage’s
          answer. Each choice preserves its exact revision.
        </p>
      ) : (
        <>
          <label className="candidate-history">
            Selection{" "}
            <select
              aria-label="Candidate revision"
              value={candidate.selectedRevision}
              disabled={state.busy}
              onChange={(event) => void state.read(Number(event.target.value))}
            >
              {candidate.history.map((entry) => (
                <option key={entry.revision} value={entry.revision}>
                  Revision {entry.revision}
                  {entry.revision === candidate.revision
                    ? " · current"
                    : " · earlier"}
                </option>
              ))}
            </select>
          </label>
          {evidence?.kind === "asset" ? (
            <>
              <p>
                {evidence.asset.title} · Asset r{evidence.asset.revision}
              </p>
              {evidence.asset.content.kind === "image" ? (
                <AssetImage asset={evidence.asset} context={state.context} />
              ) : evidence.asset.content.kind === "file" ? (
                <FilePreview content={evidence.asset.content} details />
              ) : (
                <pre>{evidence.asset.content.text || "(empty text)"}</pre>
              )}
            </>
          ) : evidence?.kind === "run_output" ? (
            <>
              <p>
                {evidence.origin.operationName} · {evidence.origin.port} · Run{" "}
                {evidence.origin.sequence} · {evidence.semanticType}
              </p>
              <pre>
                {formatResultValue(evidence.value, evidence.semanticType)}
              </pre>
              <VscodeButton
                secondary
                onClick={() => state.openRun(evidence.origin)}
              >
                Open source Run
              </VscodeButton>
            </>
          ) : evidence?.kind === "unavailable" ? (
            <p role="alert">
              Evidence unavailable ({evidence.code}). The selected reference is
              preserved; availability does not verify an answer.
            </p>
          ) : null}
          <details>
            <summary>Selected evidence reference</summary>
            <pre>{JSON.stringify(candidate.selection, null, 2)}</pre>
          </details>
        </>
      )}
      {state.error ? (
        <VscodeButton
          secondary
          disabled={state.busy}
          onClick={() => void state.read(null)}
        >
          Reload candidate
        </VscodeButton>
      ) : null}
    </section>
  );
}

export function CandidateSummary() {
  const state = useContext(CandidateContext);
  return (
    <span>
      {state?.value
        ? `Stage candidate · revision ${state.value.revision} · private`
        : state?.error
          ? "Candidate unavailable. Reopen the Stage notebook to retry."
          : state?.value === undefined
            ? "Loading Stage candidate…"
            : "Choose an answer in the Stage notebook or asset canvas."}
    </span>
  );
}
