import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import { useEffect, useRef, useState } from "react";
import contract from "../../desktop/protocol.json" with { type: "json" };
import type {
  AssetBlockView,
  AssetView,
  BlockView,
  Context,
  EditableAssetView,
  ExplorationView,
  RunOutputOrigin,
  TrailView,
} from "../desktop/workspace-contract";
import { blockTitle, contextOf } from "../desktop/workspace-contract";
import { ArchivedBlocks } from "./ArchivedBlocks";
import { ChallengeOrigin } from "./ChallengeOrigin";
import { NotebookAsset } from "./NotebookAsset";
import { NoteCell, type NoteEditing } from "./NoteCell";
import { TrailCell } from "./TrailCell";
import type { Executions } from "./useExecutions";
import VscodeButton from "./VscodeButton";

export function Notebook({
  exploration,
  executions,
  notes,
  openTrail,
  addText,
  addImage,
  addFile,
  addPage,
  addAnnotation,
  busy,
  editText,
  reorderBlocks,
  duplicateBlock,
  setArchived,
  openSourceRun,
  focusBlockId,
  onFocusHandled,
}: {
  exploration: ExplorationView;
  executions: Executions;
  notes: NoteEditing;
  busy: boolean;
  editText: (block: AssetBlockView, asset: EditableAssetView) => void;
  addText: () => void;
  addImage: () => void;
  addFile: () => void;
  addPage: () => void;
  addAnnotation: () => void;
  openSourceRun: (origin: RunOutputOrigin) => void;
  focusBlockId: string | null;
  onFocusHandled: () => void;
  openTrail: (trail: TrailView) => void;
  reorderBlocks: (
    context: Context,
    revision: number,
    ids: string[],
  ) => Promise<boolean>;
  duplicateBlock: (
    context: Context,
    revision: number,
    block: BlockView,
  ) => Promise<string | null>;
  setArchived: (
    context: Context,
    revision: number,
    block: BlockView,
    archived: boolean,
  ) => Promise<boolean>;
}) {
  const cells = useRef(new Map<string, HTMLElement>());
  const reasoningBlockId =
    exploration.stage.blocks.find(
      (block) => block.kind === "note" && block.note.kind === "reasoning",
    )?.id ?? "";
  const archivePanel = useRef<HTMLElement>(null);
  useEffect(() => {
    if (focusBlockId) {
      (
        cells.current.get(focusBlockId) ?? cells.current.get(reasoningBlockId)
      )?.focus();
      onFocusHandled();
    }
  }, [focusBlockId, reasoningBlockId, onFocusHandled]);
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
  const duplicate = async (block: BlockView) => {
    const id = await duplicateBlock(
      contextOf(exploration),
      exploration.stage.revision,
      block,
    );
    if (id)
      setMoved({
        stageId: exploration.stage.id,
        blockId: id,
        message: `Copied ${blockTitle(block)}. The copy was inserted after its source.`,
      });
  };
  const changeArchive = async (block: BlockView, archived: boolean) => {
    if (archived && !exploration.stage.archivedBlocks.length)
      setArchiveStage(null);
    const blocks = exploration.stage.blocks;
    const index = blocks.findIndex((entry) => entry.id === block.id);
    const next =
      blocks[index + 1] ??
      blocks[index - 1] ??
      blocks.find(
        (entry) => entry.kind === "note" && entry.note.kind === "reasoning",
      );
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
          ? `Archived ${blockTitle(block)}. Saved content and evidence are kept in Archived blocks.`
          : `Restored ${blockTitle(block)}. Its original identity and saved content are available.`,
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
        <ChallengeOrigin origin={exploration.origin} />
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
            icon="file-code"
            disabled={busy || !canAdd}
            onClick={addPage}
          >
            Add page
          </VscodeButton>
          <VscodeButton
            secondary
            icon="note"
            disabled={busy || !canAdd}
            aria-disabled={busy || !canAdd ? "true" : "false"}
            title={canAdd ? undefined : limitMessage}
            onClick={addAnnotation}
          >
            Add annotation
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
        const title = blockTitle(block);
        const draft = block.kind === "note" ? notes.state(block.note) : null;
        const protectedReasoning =
          block.kind === "note" && block.note.kind === "reasoning";
        const unsavedNote = Boolean(draft?.dirty || draft?.saving);
        const textAsset =
          block.kind === "asset"
            ? (block.caption ??
              (["text", "page"].includes(block.asset.content.kind) &&
              "text" in block.asset.content
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
                block.kind === "note" &&
                event.key.toLowerCase() === "s" &&
                (event.metaKey || event.ctrlKey) &&
                !event.altKey &&
                !event.shiftKey &&
                !event.repeat
              ) {
                event.preventDefault();
                event.stopPropagation();
                void notes.save(block.note.id);
                return;
              }
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
                    ? draft?.saving
                      ? "Saving…"
                      : draft?.dirty
                        ? "Unsaved draft"
                        : `Saved · r${block.note.revision}`
                    : `${block.asset.origin?.kind === "run_output" ? "Result" : "Asset"} · r${block.asset.revision}`}
                </span>
                {block.kind === "asset" &&
                textAsset &&
                textAsset.origin?.kind !== "run_output" ? (
                  <VscodeButton
                    secondary
                    icon="edit"
                    disabled={busy}
                    onClick={() => editText(block, textAsset)}
                  >
                    {block.caption
                      ? "Edit caption"
                      : block.asset.content.kind === "page"
                        ? "Edit page"
                        : "Edit text"}
                  </VscodeButton>
                ) : null}
                <div className="block-order-controls">
                  {!protectedReasoning ? (
                    <VscodeButton
                      secondary
                      iconOnly
                      icon="copy"
                      aria-label={`Duplicate ${title}`}
                      title={
                        unsavedNote
                          ? "Save or discard this note draft first"
                          : !canAdd
                            ? limitMessage
                            : "Duplicate this saved block"
                      }
                      disabled={busy || !canAdd || unsavedNote}
                      aria-disabled={
                        busy || !canAdd || unsavedNote ? "true" : "false"
                      }
                      onClick={() => void duplicate(block)}
                    />
                  ) : null}
                  {!protectedReasoning ? (
                    <VscodeButton
                      secondary
                      iconOnly
                      icon="archive"
                      aria-label={`Archive ${title}`}
                      title={
                        unsavedNote
                          ? "Save or discard this note draft first"
                          : "Archive this saved block and keep its evidence"
                      }
                      disabled={busy || unsavedNote}
                      aria-disabled={busy || unsavedNote ? "true" : "false"}
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
                <NoteCell note={block.note} editing={notes} />
              ) : (
                <>
                  <NotebookAsset
                    block={block}
                    context={contextOf(exploration)}
                    openAsset={openAsset}
                    openSourceRun={openSourceRun}
                  />
                  {trail ? (
                    <TrailCell trail={trail} executions={executions} />
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
            openSourceRun={openSourceRun}
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
