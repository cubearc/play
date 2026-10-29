import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import { useState } from "react";
import type { RunOutputOrigin, TrailView } from "../desktop/workspace-contract";
import { AssetImage } from "./AssetImage";
import { AssetOrigin } from "./AssetOrigin";
import { FilePreview } from "./FilePreview";
import { PipelineGraph } from "./PipelineGraph";
import { TrailCell } from "./TrailCell";
import type { Executions } from "./useExecutions";
import VscodeButton from "./VscodeButton";

export function AssetCanvas({
  trail,
  executions,
  openSourceRun,
  editTrail,
  editPipeline,
}: {
  trail: TrailView;
  editTrail: () => void;
  editPipeline: (mode: "create" | "revise" | "fork") => void;
  executions: Executions;
  openSourceRun: (origin: RunOutputOrigin) => void;
}) {
  const [zoom, setZoom] = useState(100);
  const image = trail.source.content.kind === "image";
  const text = trail.source.content.kind === "text";
  const page = trail.source.content.kind === "page";
  return (
    <section className="asset-canvas scroll-surface" aria-label="Trail canvas">
      <header className="document-heading">
        <div className="eyebrow">Trail · Asset exploration</div>
        <h1>{trail.title}</h1>
        <p>{trail.hypothesis || "Add a hypothesis to guide this Trail."}</p>
        {image || text || page ? (
          <div className="trail-actions">
            <VscodeButton
              secondary
              disabled={trail.archived}
              onClick={editTrail}
            >
              Edit Trail
            </VscodeButton>
            <VscodeButton
              secondary
              disabled={
                trail.archived || !executions.operationsFor(trail).length
              }
              onClick={() => editPipeline("create")}
            >
              New Pipeline
            </VscodeButton>
            {trail.pipeline ? (
              <>
                <VscodeButton
                  secondary
                  disabled={trail.archived || !executions.operations.length}
                  onClick={() => editPipeline("revise")}
                >
                  Edit Pipeline
                </VscodeButton>
                <VscodeButton
                  secondary
                  disabled={trail.archived || !executions.operations.length}
                  onClick={() => editPipeline("fork")}
                >
                  Fork Pipeline
                </VscodeButton>
              </>
            ) : null}
          </div>
        ) : null}
        <div className="context-chips">
          <span>
            <VscodeIcon name="link" /> {trail.source.title} @ r
            {trail.source.revision}
          </span>
          <span>
            {trail.pipeline
              ? `Pipeline · ${trail.pipeline.title} · r${trail.pipeline.revision}`
              : image
                ? "Image inspection"
                : text
                  ? "No Pipeline selected"
                  : page
                    ? "Page source · inert"
                    : "File inspection"}
          </span>
        </div>
      </header>
      {trail.archived ? (
        <p className="archive-notice" role="note">
          <VscodeIcon name="archive" /> Archived block · Content and Run history
          are preserved. Restore it from the Stage notebook to return it to
          active work. Accepted work can still finish or be cancelled.
        </p>
      ) : null}
      <div className="canvas-surface">
        <AssetOrigin asset={trail.source} openRun={openSourceRun} />
        <article
          className="canvas-node source-node"
          aria-label="Bound source asset"
        >
          <div className="node-heading">
            <VscodeIcon
              name={image ? "file-media" : text ? "symbol-string" : "file"}
            />
            <strong>{trail.source.title}</strong>
            <span className="badge">Source asset</span>
          </div>
          {trail.source.content.kind === "image" ? (
            <div className="image-viewport">
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
          ) : trail.source.content.kind === "file" ? (
            <FilePreview content={trail.source.content} details />
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

        {Object.entries(trail.bindings).length ? (
          <section className="bound-inputs" aria-label="Bound Pipeline inputs">
            <h2>Trail inputs</h2>
            {Object.entries(trail.bindings).map(([name, asset]) => (
              <article
                key={name}
                className="canvas-node"
                aria-label={`Bound input ${name}`}
              >
                <div className="node-heading">
                  <strong>{name}</strong>
                  <span>
                    {asset.title} · pinned r{asset.revision}
                  </span>
                </div>
                {asset.content.kind === "image" ? (
                  <AssetImage
                    asset={asset}
                    context={{
                      workspace_id: trail.workspaceId,
                      exploration_id: trail.explorationId,
                      stage_id: trail.stageId,
                    }}
                  />
                ) : asset.content.kind === "text" ||
                  asset.content.kind === "page" ? (
                  <pre>{asset.content.text}</pre>
                ) : null}
              </article>
            ))}
          </section>
        ) : null}
        {trail.pipelineId ? (
          <>
            <div className="canvas-connection">
              <span /> <code>{trail.source.content.kind} → source</code>{" "}
              <VscodeIcon name="arrow-down" />
            </div>
            {trail.pipeline && trail.pipeline.definition.nodes.length > 1 ? (
              <PipelineGraph
                definition={trail.pipeline.definition}
                operations={executions.operations}
                busy={executions.busy || trail.archived}
                onRun={(id) => {
                  const node = trail.pipeline?.definition.nodes.find(
                    (n) => n.id === id,
                  );
                  const operation = executions.operations.find(
                    (o) => o.operation === node?.operation,
                  );
                  if (operation) void executions.run(trail, operation, id);
                }}
              />
            ) : null}
            <article
              className="canvas-node operation-node"
              aria-label="Pipeline node"
            >
              <TrailCell trail={trail} executions={executions} />
            </article>
          </>
        ) : (
          <TrailCell trail={trail} executions={executions} />
        )}
      </div>
      {page ? (
        <p className="canvas-note">
          Page source is plain text. Scripts, forms, links and embedded content
          do not execute or load.
        </p>
      ) : null}
      {trail.source.content.kind === "file" ? (
        <p className="canvas-note">
          This is an inert preview of the imported bytes. File operations are
          not available in this preview.
        </p>
      ) : null}
    </section>
  );
}
