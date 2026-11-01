import { useEffect, useRef, useState } from "react";
import type {
  ExplorationView,
  StageProgress,
} from "../desktop/workspace-contract";
import VscodeButton from "./VscodeButton";

export function StageSettings({
  open,
  exploration,
  busy,
  error,
  close,
  edit,
  move,
  select,
  duplicate,
  onDirty,
}: {
  open: boolean;
  exploration: ExplorationView | null;
  busy: boolean;
  error: string;
  close: () => void;
  edit: (
    title: string,
    progress: StageProgress,
    archived: boolean,
  ) => Promise<boolean>;
  move: (id: string, direction: number) => Promise<boolean>;
  select: (id: string) => Promise<boolean>;
  duplicate: () => Promise<boolean>;
  onDirty: (value: boolean) => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [drafts, setDrafts] = useState<
    Record<string, { value: string; original: string }>
  >({});
  const title = exploration
    ? (drafts[exploration.stage.id]?.value ?? exploration.stage.title)
    : "";
  useEffect(
    () => onDirty(Object.values(drafts).some((d) => d.value !== d.original)),
    [drafts, onDirty],
  );
  useEffect(() => {
    if (open) ref.current?.showModal();
    else ref.current?.close();
  }, [open]);

  if (!exploration) return null;
  const stage = exploration.stage;
  return (
    <dialog
      ref={ref}
      className="content-dialog stage-settings"
      aria-label="Organize Stages"
      onCancel={close}
    >
      <h2>Organize Stages</h2>
      <p>Order your local investigation. Completion is a private annotation.</p>
      {error ? <p role="alert">{error}</p> : null}
      <ol className="stage-order">
        {exploration.stages.map((s, i) => (
          <li key={s.id}>
            <button
              type="button"
              aria-current={s.id === stage.id ? "true" : undefined}
              disabled={busy}
              onClick={() => void select(s.id)}
            >
              {s.title}
            </button>
            <VscodeButton
              secondary
              disabled={busy || i === 0}
              onClick={() => void move(s.id, -1)}
              aria-label={`Move ${s.title} up`}
            >
              Up
            </VscodeButton>
            <VscodeButton
              secondary
              disabled={busy || i === exploration.stages.length - 1}
              onClick={() => void move(s.id, 1)}
              aria-label={`Move ${s.title} down`}
            >
              Down
            </VscodeButton>
          </li>
        ))}
      </ol>
      {exploration.archivedStages.length ? (
        <>
          <h3>Archived Stages</h3>
          {exploration.archivedStages.map((s) => (
            <button
              key={s.id}
              type="button"
              disabled={busy}
              onClick={() => void select(s.id)}
            >
              {s.title} (archived)
            </button>
          ))}
        </>
      ) : null}
      <label htmlFor="stage-name">Stage title</label>
      <input
        id="stage-name"
        value={title}
        maxLength={120}
        disabled={busy}
        onChange={(e) =>
          setDrafts((previous) => ({
            ...previous,
            [stage.id]: { value: e.target.value, original: stage.title },
          }))
        }
      />
      <VscodeButton
        secondary
        disabled={busy || !title.trim() || title === stage.title}
        onClick={() =>
          void edit(title, stage.progress, stage.archived).then((saved) => {
            if (saved)
              setDrafts((previous) => {
                const next = { ...previous };
                delete next[stage.id];
                return next;
              });
          })
        }
      >
        Rename Stage
      </VscodeButton>
      <label htmlFor="stage-progress">Local progress</label>
      <select
        id="stage-progress"
        disabled={busy || stage.archived}
        value={stage.progress}
        onChange={(e) =>
          void edit(stage.title, e.target.value as StageProgress, false)
        }
      >
        <option value="not_started">Not started</option>
        <option value="in_progress">In progress</option>
        <option value="completed">Completed locally</option>
      </select>
      <p>
        Archiving preserves Blocks, Trails, Candidates and Solutions. Restore to
        resume editing.
      </p>
      <p>
        Duplicate copies visible notebook content into independent Assets and
        fresh Trails. It keeps the source evidence and leaves Candidates,
        Solutions and completion in the source Stage.
      </p>
      <div className="dialog-actions">
        <VscodeButton
          secondary
          disabled={busy || stage.archived}
          onClick={() => void duplicate()}
        >
          Duplicate Stage
        </VscodeButton>
        <VscodeButton
          secondary
          disabled={busy}
          onClick={() =>
            void edit(stage.title, stage.progress, !stage.archived)
          }
        >
          {stage.archived ? "Restore Stage" : "Archive Stage"}
        </VscodeButton>
        <VscodeButton onClick={close}>Done</VscodeButton>
      </div>
    </dialog>
  );
}
