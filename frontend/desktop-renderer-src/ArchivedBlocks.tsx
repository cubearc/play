import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import type {
  ArchivedBlockView,
  AssetView,
  BlockView,
  Context,
  RunOutputOrigin,
} from "../desktop/workspace-contract";
import { blockTitle } from "../desktop/workspace-contract";
import { NotebookAsset } from "./NotebookAsset";

export function ArchivedBlocks({
  items,
  context,
  busy,
  full,
  openAsset,
  restore,
  openSourceRun,
}: {
  items: ArchivedBlockView[];
  context: Context;
  busy: boolean;
  full: boolean;
  openAsset: (asset: AssetView) => void;
  restore: (block: BlockView) => void;
  openSourceRun: (origin: RunOutputOrigin) => void;
}) {
  return (
    <>
      <h2>Archived blocks</h2>
      <p className="muted">
        Content, source links and Run history stay in this Stage. Restore a
        block to return it to the active notebook.
      </p>
      {items.map(({ block }) => (
        <article
          className="archived-cell"
          key={block.id}
          aria-label={`Archived ${blockTitle(block)}`}
        >
          <header className="block-heading">
            <strong>{blockTitle(block)}</strong>
            <span className="badge">
              Archived · r
              {block.kind === "note"
                ? block.note.revision
                : block.asset.revision}
            </span>
          </header>
          {block.kind === "note" ? (
            <div className="annotation-preview">
              {block.note.origin ? (
                <p className="copy-origin">
                  Copied from {block.note.origin.title} · r
                  {block.note.origin.revision}
                </p>
              ) : null}
              <pre>{block.note.text || "Empty annotation"}</pre>
            </div>
          ) : (
            <NotebookAsset
              block={block}
              context={context}
              openAsset={openAsset}
              openSourceRun={openSourceRun}
            />
          )}
          <div className="notebook-actions">
            {block.kind === "asset" ? (
              <VscodeButton
                secondary
                icon="history"
                onClick={() => openAsset(block.caption ?? block.asset)}
                aria-label={`Open evidence ${block.asset.title}`}
              >
                Open evidence
              </VscodeButton>
            ) : null}
            <VscodeButton
              icon="discard"
              disabled={busy || full}
              aria-disabled={busy || full ? "true" : "false"}
              title={
                full
                  ? "Archive another block to make room"
                  : "Restore this block"
              }
              aria-label={`Restore ${blockTitle(block)}`}
              onClick={() => restore(block)}
            >
              Restore
            </VscodeButton>
          </div>
        </article>
      ))}
    </>
  );
}
