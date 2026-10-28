import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import { useState } from "react";
import type { TrailView } from "../desktop/workspace-contract";
import { AssetImage } from "./AssetImage";
import { OperationCell } from "./OperationCell";
import type { Executions } from "./useExecutions";

export function AssetCanvas({
  trail,
  executions,
}: {
  trail: TrailView;
  executions: Executions;
}) {
  const [zoom, setZoom] = useState(100);
  const image = trail.source.content.kind === "image";
  return (
    <section className="asset-canvas scroll-surface" aria-label="Trail canvas">
      <header className="document-heading">
        <div className="eyebrow">Trail · Asset exploration</div>
        <h1>{trail.title}</h1>
        <p>{trail.hypothesis}</p>
        <div className="context-chips">
          <span>
            <VscodeIcon name="link" /> {trail.source.title} @ r
            {trail.source.revision}
          </span>
          <span>
            {image
              ? "Image inspection"
              : `Pipeline · ${trail.pipeline?.title} · r${trail.pipeline?.revision}`}
          </span>
        </div>
      </header>
      <div className="canvas-surface">
        <article
          className="canvas-node source-node"
          aria-label="Bound source asset"
        >
          <div className="node-heading">
            <VscodeIcon name={image ? "file-media" : "symbol-string"} />
            <strong>{trail.source.title}</strong>
            <span className="badge">Source asset</span>
          </div>
          {trail.source.content.kind === "image" ? (
            <div className="image-viewport">
              <div style={{ width: `${zoom}%` }}>
                <AssetImage
                  asset={trail.source}
                  context={{
                    workspace_id: trail.workspaceId,
                    exploration_id: trail.explorationId,
                    stage_id: trail.stageId,
                  }}
                />
              </div>
            </div>
          ) : (
            <pre>{trail.source.content.text}</pre>
          )}
          <div className="node-footer">
            <code>
              {trail.source.id} @ r{trail.source.revision}
            </code>
            <span className="port-dot" />
          </div>
        </article>
        {image ? (
          <div className="image-tools">
            <VscodeButton
              secondary
              icon="zoom-out"
              disabled={zoom <= 50}
              onClick={() => setZoom((value) => value - 25)}
            >
              Zoom out
            </VscodeButton>
            <output aria-label="Image zoom">{zoom}%</output>
            <VscodeButton
              secondary
              icon="zoom-in"
              disabled={zoom >= 200}
              onClick={() => setZoom((value) => value + 25)}
            >
              Zoom in
            </VscodeButton>
          </div>
        ) : (
          <>
            <div className="canvas-connection">
              <span /> <code>text → {trail.pipeline?.input}</code>{" "}
              <VscodeIcon name="arrow-down" />
            </div>
            <article
              className="canvas-node operation-node"
              aria-label="Pipeline node"
            >
              <OperationCell trail={trail} executions={executions} />
            </article>
          </>
        )}
      </div>
      {image ? (
        <p className="canvas-note">
          Image transformations are not available in this preview. You can
          inspect and zoom the source here.
        </p>
      ) : null}
    </section>
  );
}
