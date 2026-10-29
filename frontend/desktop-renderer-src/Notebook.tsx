import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useEffect, useRef, useState } from "react";
import contract from "../../desktop/protocol.json" with { type: "json" };
import type {
  AssetBlockData,
  AssetBlockView,
  AssetView,
  Context,
  ExplorationView,
  TextAssetView,
  TrailView,
} from "../desktop/workspace-contract";
import { contextOf } from "../desktop/workspace-contract";
import { ArchivedBlocks } from "./ArchivedBlocks";
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
  addFile,
  busy,
  editText,
  reorderBlocks,
  duplicateBlock,
  setArchived,
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
  addFile: () => void;
  onNote: (value: string) => void;
  openTrail: (trail: TrailView) => void;
  reorderBlocks: (
    context: Context,
    revision: number,
    ids: string[],
  ) => Promise<boolean>;
  duplicateBlock: (
    context: Context,
    revision: number,
    block: AssetBlockView,
  ) => Promise<string | null>;
  setArchived: (
    context: Context,
    revision: number,
    block: AssetBlockData,
    archived: boolean,
  ) => Promise<boolean>;
}) {
  const cells = useRef(new Map<string, HTMLElement>());
  const reasoningBlockId =
    exploration.stage.blocks.find((block) => block.kind === "note")?.id ?? "";
  const archivePanel = useRef<HTMLElement>(null);
  const [archiveStage, setArchiveStage] = useState<string | null>(null);
  const archiveOpen =
    archiveStage === exploration.stage.id &&
    exploration.stage.archivedBlocks.length > 0;
  const retained =
    exploration.stage.blocks.length + exploration.stage.archivedBlocks.length;
  const full = exploration.stage.blocks.length >= contract.block_limit;
  const canAdd = !full && retained < contract.retained_block_limit;
  const limitMessage =
    retained >= contract.retained_block_limit
      ? "Stage retention limit reached; create another Stage"
      : "Archive a block to make room";
  useEffect(() => {
    if (archiveOpen) archivePanel.current?.focus();
  }, [archiveOpen]);
  const [moved, setMoved] = useState<{
    stageId: string;
    blockId: string;
    message: string;
  } | null>(null);
  useEffect(() => {
    if (moved?.stageId === exploration.stage.id) {
      (
        cells.current.get(moved.blockId) ?? cells.current.get(reasoningBlockId)
      )?.focus();
    }
  }, [moved, exploration.stage.id, reasoningBlockId]);
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
  const duplicate = async (block: AssetBlockView) => {
    const id = await duplicateBlock(
      contextOf(exploration),
      exploration.stage.revision,
      block,
    );
    if (id)
      setMoved({
        stageId: exploration.stage.id,
        blockId: id,
        message: `Copied ${block.asset.title}. The copy was inserted after its source.`,
      });
  };
  const changeArchive = async (block: AssetBlockData, archived: boolean) => {
    if (archived && !exploration.stage.archivedBlocks.length)
      setArchiveStage(null);
    const blocks = exploration.stage.blocks;
    const index = blocks.findIndex((entry) => entry.id === block.id);
    const next =
      blocks[index + 1] ??
      blocks[index - 1] ??
      blocks.find((entry) => entry.kind === "note");
    if (
      await setArchived(
        contextOf(exploration),
        exploration.stage.revision,
        block,
        archived,
      )
    ) {
      if (!archived && exploration.stage.archivedBlocks.length === 1)
        setArchiveStage(null);
      setMoved({
        stageId: exploration.stage.id,
        blockId: archived ? (next?.id ?? "") : block.id,
        message: archived
          ? `Archived ${block.asset.title}. Content and Run history are kept in Archived blocks.`
          : `Restored ${block.asset.title}. Its original Assets and Run history are available.`,
      });
    }
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
            disabled={busy || !canAdd}
            aria-disabled={busy || !canAdd ? "true" : "false"}
            title={canAdd ? undefined : limitMessage}
            onClick={addText}
          >
            Add text
          </VscodeButton>
          <VscodeButton
            secondary
            icon="file-media"
            disabled={busy || !canAdd}
            aria-disabled={busy || !canAdd ? "true" : "false"}
            title={canAdd ? undefined : limitMessage}
            onClick={addImage}
          >
            Add image
          </VscodeButton>
          <VscodeButton
            secondary
            icon="file-add"
            disabled={busy || !canAdd}
            aria-disabled={busy || !canAdd ? "true" : "false"}
            title={canAdd ? undefined : limitMessage}
            onClick={addFile}
          >
            Add file
          </VscodeButton>
          <VscodeButton
            secondary
            icon="archive"
            disabled={!exploration.stage.archivedBlocks.length}
            aria-disabled={
              !exploration.stage.archivedBlocks.length ? "true" : "false"
            }
            aria-expanded={archiveOpen ? "true" : "false"}
            onClick={() =>
              setArchiveStage(archiveOpen ? null : exploration.stage.id)
            }
          >
            Archived blocks ({exploration.stage.archivedBlocks.length})
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
                          : block.asset.content.kind === "text"
                            ? "symbol-string"
                            : "file"
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
                  {block.kind === "asset" ? (
                    <VscodeButton
                      secondary
                      iconOnly
                      icon="copy"
                      aria-label={`Duplicate ${title}`}
                      title={
                        !canAdd ? limitMessage : "Duplicate this saved block"
                      }
                      disabled={busy || !canAdd}
                      aria-disabled={busy || !canAdd ? "true" : "false"}
                      onClick={() => void duplicate(block)}
                    />
                  ) : null}
                  {block.kind === "asset" ? (
                    <VscodeButton
                      secondary
                      iconOnly
                      icon="archive"
                      aria-label={`Archive ${title}`}
                      title="Archive this saved block and keep its evidence"
                      disabled={busy}
                      aria-disabled={busy ? "true" : "false"}
                      onClick={() => void changeArchive(block, true)}
                    />
                  ) : null}
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
      {archiveOpen ? (
        <section
          className="archived-blocks"
          aria-label="Archived blocks"
          tabIndex={-1}
          ref={archivePanel}
          onKeyDownCapture={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
        >
          <ArchivedBlocks
            items={exploration.stage.archivedBlocks}
            context={contextOf(exploration)}
            busy={busy}
            full={full}
            openAsset={openAsset}
            restore={(block) => void changeArchive(block, false)}
          />
          <p className="muted">
            {exploration.stage.blocks.length} / {contract.block_limit} active ·{" "}
            {retained} / {contract.retained_block_limit} retained
          </p>
        </section>
      ) : null}
    </section>
  );
}
