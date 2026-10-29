import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useEffect, useRef, useState } from "react";
import type {
  AssetBlockView,
  AssetView,
  Context,
  ExplorationView,
  TextAssetView,
  TrailView,
} from "../desktop/workspace-contract";
import { contextOf } from "../desktop/workspace-contract";
import { NotebookAsset } from "./NotebookAsset";
import { OperationCell } from "./OperationCell";
import type { Executions } from "./useExecutions";

function Reasoning({
  value,
  dirty,
  saving,
  onChange,
  onSave,
}: {
  value: string;
  dirty: boolean;
  saving: boolean;
  onChange: (value: string) => void;
  onSave: () => void;
}) {
  const field = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  return (
    <>
      <VscodeTextarea
        ref={field}
        label="Stage note"
        value={value}
        rows={3}
        disabled={saving}
        resize="vertical"
        onInput={() => onChange(field.current?.value ?? "")}
      />
      <div className="note-save">
        <VscodeButton icon="save" disabled={!dirty || saving} onClick={onSave}>
          Save reasoning
        </VscodeButton>
        <span role="status" aria-label="Note save status">
          {saving ? "Saving…" : dirty ? "Unsaved changes" : "Saved"}
        </span>
      </div>
    </>
  );
}
export function Notebook({
  exploration,
  executions,
  note,
  onNote,
  openTrail,
  saveNote,
  noteDirty,
  noteSaving,
  addText,
  addImage,
  busy,
  editText,
  reorderBlocks,
}: {
  exploration: ExplorationView;
  executions: Executions;
  note: string;
  noteDirty: boolean;
  noteSaving: boolean;
  busy: boolean;
  editText: (block: AssetBlockView, asset: TextAssetView) => void;
  saveNote: () => void;
  addText: () => void;
  addImage: () => void;
  onNote: (value: string) => void;
  openTrail: (trail: TrailView) => void;
  reorderBlocks: (
    context: Context,
    revision: number,
    ids: string[],
  ) => Promise<boolean>;
}) {
  const cells = useRef(new Map<string, HTMLElement>());
  const [moved, setMoved] = useState<{
    stageId: string;
    blockId: string;
    message: string;
  } | null>(null);
  useEffect(() => {
    if (moved?.stageId === exploration.stage.id)
      cells.current.get(moved.blockId)?.focus();
  }, [moved, exploration.stage.id]);
  const move = async (index: number, direction: number, title: string) => {
    const stage = exploration.stage;
    const block = stage.blocks[index];
    const adjacent = stage.blocks[index + direction];
    if (!block || !adjacent) return;
    const ids = stage.blocks.map((entry) => entry.id);
    ids[index] = adjacent.id;
    ids[index + direction] = block.id;
    if (await reorderBlocks(contextOf(exploration), stage.revision, ids))
      setMoved({
        stageId: stage.id,
        blockId: block.id,
        message: `${title} moved to position ${index + direction + 1} of ${ids.length}.`,
      });
  };
  const openAsset = (asset: AssetView) => {
    const trail = exploration.trails.find(
      (entry) => entry.source.id === asset.id,
    );
    if (trail) openTrail(trail);
  };
  return (
    <section className="notebook scroll-surface" aria-label="Stage notebook">
      <header className="document-heading">
        <div className="eyebrow">
          Stage{" "}
          {String(
            exploration.stages.findIndex(
              (stage) => stage.id === exploration.stage.id,
            ) + 1,
          ).padStart(2, "0")}{" "}
          · Local notebook
        </div>
        <h1>{exploration.stage.title}</h1>
        {exploration.stage.introduction ? (
          <p>{exploration.stage.introduction}</p>
        ) : null}
        <div className="notebook-actions">
          <VscodeButton
            secondary
            icon="symbol-string"
            disabled={busy}
            onClick={addText}
          >
            Add text
          </VscodeButton>
          <VscodeButton
            secondary
            icon="file-media"
            disabled={busy}
            onClick={addImage}
          >
            Add image
          </VscodeButton>
        </div>
        <span
          className="screen-reader-only"
          role="status"
          aria-label="Notebook order"
        >
          {moved?.stageId === exploration.stage.id ? moved.message : ""}
        </span>
      </header>
      {exploration.stage.blocks.map((block, index) => {
        const title = block.kind === "note" ? "Reasoning" : block.asset.title;
        const textAsset =
          block.kind === "asset"
            ? (block.caption ??
              (block.asset.content.kind === "text"
                ? { ...block.asset, content: block.asset.content }
                : null))
            : null;
        const trail =
          block.kind === "asset"
            ? exploration.trails.find(
                (entry) =>
                  entry.source.id === (block.caption ?? block.asset).id,
              )
            : null;
        return (
          <article
            className={`notebook-cell ${block.kind === "note" ? "note-cell" : ""}`}
            key={block.id}
            aria-label={`${title} block`}
            data-block-id={block.id}
            tabIndex={-1}
            ref={(element) => {
              if (element) cells.current.set(block.id, element);
              else cells.current.delete(block.id);
            }}
            onKeyDownCapture={(event) => {
              if (
                event.key !== "Enter" ||
                !(event.metaKey || event.ctrlKey) ||
                event.altKey ||
                event.shiftKey ||
                event.repeat
              )
                return;
              event.preventDefault();
              event.stopPropagation();
              if (trail) {
                const operation = executions.operationFor(trail);
                if (operation) void executions.run(trail, operation);
              }
            }}
          >
            <span className="cell-number">
              {String(index + 1).padStart(2, "0")}
            </span>
            <div className="block-body">
              <div className="block-heading">
                <span>
                  <VscodeIcon
                    name={
                      block.kind === "note"
                        ? "note"
                        : block.asset.content.kind === "image"
                          ? "file-media"
                          : "symbol-string"
                    }
                  />{" "}
                  {title}
                </span>
                <span className="badge">
                  {block.kind === "note"
                    ? noteSaving
                      ? "Saving…"
                      : noteDirty
                        ? "Unsaved draft"
                        : `Saved · r${block.note.revision}`
                    : `Asset · r${block.asset.revision}`}
                </span>
                {block.kind === "asset" && textAsset ? (
                  <VscodeButton
                    secondary
                    icon="edit"
                    disabled={busy}
                    onClick={() => editText(block, textAsset)}
                  >
                    {block.caption ? "Edit caption" : "Edit text"}
                  </VscodeButton>
                ) : null}
                <div className="block-order-controls">
                  <VscodeButton
                    secondary
                    iconOnly
                    icon="arrow-up"
                    aria-label={`Move ${title} up`}
                    title={`Move ${title} up`}
                    disabled={busy || index === 0}
                    aria-disabled={busy || index === 0 ? "true" : "false"}
                    onClick={() => void move(index, -1, title)}
                  />
                  <VscodeButton
                    secondary
                    iconOnly
                    icon="arrow-down"
                    aria-label={`Move ${title} down`}
                    title={`Move ${title} down`}
                    disabled={
                      busy || index === exploration.stage.blocks.length - 1
                    }
                    aria-disabled={
                      busy || index === exploration.stage.blocks.length - 1
                        ? "true"
                        : "false"
                    }
                    onClick={() => void move(index, 1, title)}
                  />
                </div>
              </div>
              {block.kind === "note" ? (
                <Reasoning
                  value={note}
                  dirty={noteDirty}
                  saving={noteSaving}
                  onChange={onNote}
                  onSave={saveNote}
                />
              ) : (
                <>
                  <NotebookAsset
                    block={block}
                    context={contextOf(exploration)}
                    openAsset={openAsset}
                  />
                  {trail?.pipelineId ? (
                    <OperationCell trail={trail} executions={executions} />
                  ) : null}
                </>
              )}
            </div>
          </article>
        );
      })}
    </section>
  );
}
