import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import type {
  AssetView,
  BlockView,
  Context,
} from "../desktop/workspace-contract";
import { AssetImage } from "./AssetImage";

export function NotebookAsset({
  block,
  openAsset,
  context,
}: {
  block: BlockView;
  context: Context;
  openAsset: (asset: AssetView) => void;
}) {
  return (
    <figure className="notebook-figure">
      <button
        className={`asset-content ${block.asset.content.kind}`}
        type="button"
        aria-label={`Explore ${block.asset.title}`}
        onClick={() => openAsset(block.asset)}
      >
        {block.asset.content.kind === "image" ? (
          <AssetImage asset={block.asset} context={context} />
        ) : (
          <code>{block.asset.content.text}</code>
        )}
        <span className="asset-open">
          <VscodeIcon name="open-preview" /> Open exploration canvas
        </span>
      </button>
      {block.caption ? (
        <figcaption>
          <button
            className="caption-asset"
            type="button"
            aria-label={`Explore ${block.caption.title}`}
            onClick={() => {
              if (block.caption) openAsset(block.caption);
            }}
          >
            <span>{block.caption.content.text}</span>
            <VscodeIcon name="open-preview" />
          </button>
          <span className="caption-meta">
            Caption · r{block.caption.revision}
          </span>
        </figcaption>
      ) : null}
    </figure>
  );
}
