import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import type {
  ArchivedBlockView,
  AssetBlockData,
  AssetView,
  Context,
} from "../desktop/workspace-contract";
import { NotebookAsset } from "./NotebookAsset";

export function ArchivedBlocks({
  items,
  context,
  busy,
  full,
  openAsset,
  restore,
}: {
  items: ArchivedBlockView[];
  context: Context;
  busy: boolean;
  full: boolean;
  openAsset: (asset: AssetView) => void;
  restore: (block: AssetBlockData) => void;
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
          aria-label={`Archived ${block.asset.title}`}
        >
          <header className="block-heading">
            <strong>{block.asset.title}</strong>
            <span className="badge">Archived · r{block.asset.revision}</span>
          </header>
          <NotebookAsset
            block={block}
            context={context}
            openAsset={openAsset}
          />
          <div className="notebook-actions">
            <VscodeButton
              secondary
              icon="history"
              onClick={() => openAsset(block.caption ?? block.asset)}
              aria-label={`Open evidence ${block.asset.title}`}
            >
              Open evidence
            </VscodeButton>
            <VscodeButton
              icon="discard"
              disabled={busy || full}
              aria-disabled={busy || full ? "true" : "false"}
              title={
                full
                  ? "Archive another block to make room"
                  : "Restore this block"
              }
              aria-label={`Restore ${block.asset.title}`}
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
