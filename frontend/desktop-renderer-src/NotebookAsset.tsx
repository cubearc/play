import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import type {
  AssetBlockData,
  AssetView,
  Context,
  RunOutputOrigin,
} from "../desktop/workspace-contract";
import { AssetImage } from "./AssetImage";
import { AssetOrigin } from "./AssetOrigin";
import { FilePreview } from "./FilePreview";

export function NotebookAsset({
  block,
  openAsset,
  context,
  openSourceRun,
}: {
  block: AssetBlockData;
  context: Context;
  openAsset: (asset: AssetView) => void;
  openSourceRun: (origin: RunOutputOrigin) => void;
}) {
  return (
    <figure className="notebook-figure">
      <AssetOrigin asset={block.asset} openRun={openSourceRun} />
      <button
        className={`asset-content ${block.asset.content.kind}`}
        type="button"
        aria-label={`Explore ${block.asset.title}`}
        onClick={() => openAsset(block.asset)}
      >
        {block.asset.content.kind === "image" ? (
          <AssetImage asset={block.asset} context={context} />
        ) : block.asset.content.kind === "file" ? (
          <FilePreview content={block.asset.content} />
        ) : block.asset.content.kind === "page" ? (
          <div className="page-source">
            <span>Page source · inert</span>
            <pre>{block.asset.content.text}</pre>
          </div>
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
            {block.caption.origin?.kind === "asset"
              ? ` · Copied from r${block.caption.origin.revision}`
              : ""}
          </span>
        </figcaption>
      ) : null}
    </figure>
  );
}
