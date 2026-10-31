import {
  createContext,
  type ReactNode,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import type {
  Context,
  NoteLink,
  NoteTarget,
  NoteTargetDetail,
} from "../desktop/workspace-contract";
import { CandidateEvidence } from "./Candidates";
import { ContentDialog } from "./ContentDialog";
import { errorMessage } from "./useWorkspace";
import VscodeButton from "./VscodeButton";

interface Actions {
  context: Context | null;
  disabled: boolean;
  begin: (target: NoteTarget, label: string) => void;
  openRun: (origin: { runId: string; trailId: string }) => void;
}
const EvidenceNotes = createContext<Actions | null>(null);
export function EvidenceNotesProvider({
  context,
  disabled,
  save,
  onSaved,
  openRun,
  onDirty,
  children,
}: {
  context: Context | null;
  disabled: boolean;
  save: (
    context: Context,
    target: NoteTarget,
    title: string,
    text: string,
    key: string,
  ) => Promise<string | null>;
  onSaved: (block: string) => void;
  openRun: Actions["openRun"];
  onDirty: (dirty: boolean) => void;
  children: ReactNode;
}) {
  const [pending, setPending] = useState<{
    context: Context;
    target: NoteTarget;
    label: string;
  } | null>(null);
  const [error, setError] = useState("");
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const close = () => {
    setPending(null);
    setError("");
    onDirty(false);
  };
  return (
    <EvidenceNotes.Provider
      value={{
        context,
        disabled,
        openRun,
        begin: (target, label) => {
          if (context && !disabled) {
            setPending({ context, target, label });
            setError("");
          }
        },
      }}
    >
      {children}
      {pending ? (
        <ContentDialog
          kind="annotation"
          description={`Linked to ${pending.label}. The target is fixed when this annotation is saved.`}
          error={error}
          onDirty={onDirty}
          onClose={close}
          onSubmit={async (title, text, key) => {
            const block = await save(
              pending.context,
              pending.target,
              title,
              text,
              key,
            );
            if (!active.current) return false;
            if (!block) {
              setError(
                "The annotation could not be saved. Your draft is preserved. Check the workspace message and retry.",
              );
              return false;
            }
            onSaved(block);
            return true;
          }}
        />
      ) : null}
    </EvidenceNotes.Provider>
  );
}
export function AnnotateButton({
  target,
  label,
  compact = false,
}: {
  target: NoteTarget;
  label: string;
  compact?: boolean;
}) {
  const state = useContext(EvidenceNotes);
  if (!state?.context) return null;
  return (
    <VscodeButton
      secondary
      icon="note"
      iconOnly={compact}
      disabled={state.disabled}
      aria-label={`Annotate ${label}`}
      title={`Annotate ${label}`}
      onClick={() => state.begin(target, label)}
    >
      {compact
        ? null
        : target.kind === "run_output"
          ? `Annotate ${target.port}`
          : target.kind === "attempt"
            ? "Annotate attempt"
            : "Annotate asset"}
    </VscodeButton>
  );
}
export function LinkedEvidence({
  noteId,
  link,
}: {
  noteId: string;
  link: NoteLink;
}) {
  const state = useContext(EvidenceNotes);
  const [detail, setDetail] = useState<NoteTargetDetail>();
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  if (!state?.context) return null;
  const inspect = async () => {
    if (!state.context || busy) return;
    setBusy(true);
    setError("");
    try {
      const result = await window.play.workspace({
        kind: "inspect_note_target",
        context: state.context,
        note_id: noteId,
      });
      if (active.current) setDetail(result);
    } catch (e) {
      if (active.current) setError(errorMessage(e));
    } finally {
      if (active.current) setBusy(false);
    }
  };
  const evidence = detail?.evidence;
  return (
    <details className="linked-evidence">
      <summary>Linked to {link.label}</summary>
      <VscodeButton secondary disabled={busy} onClick={() => void inspect()}>
        {busy
          ? "Loading evidence…"
          : detail
            ? "Reload linked evidence"
            : "Inspect linked evidence"}
      </VscodeButton>
      {error ? <p role="alert">{error}</p> : null}
      {evidence?.kind === "block" ? (
        <p>
          {evidence.title} · {evidence.archived ? "Archived" : "Active"} Block.
          This link identifies the notebook item; it does not freeze its
          content.
        </p>
      ) : evidence?.kind === "attempt" ? (
        <p>
          Attempt status: {evidence.status}
          {evidence.code ? ` · ${evidence.code}` : ""}
        </p>
      ) : evidence ? (
        <CandidateEvidence
          evidence={evidence}
          context={state.context}
          openRun={state.openRun}
        />
      ) : null}
      {detail?.origin && evidence?.kind !== "run_output" ? (
        <VscodeButton
          secondary
          onClick={() => {
            if (detail.origin) state.openRun(detail.origin);
          }}
        >
          Open source Run
        </VscodeButton>
      ) : null}
      <details>
        <summary>Target reference</summary>
        <pre>{JSON.stringify(link.reference, null, 2)}</pre>
      </details>
    </details>
  );
}
