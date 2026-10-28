import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useRef } from "react";
import type { ExplorationView, TrailView } from "./exampleWorkspace";
import { OperationCell } from "./OperationCell";
import { SampleImage } from "./SampleImage";
import type { Executions } from "./useExecutions";

export function Notebook({
  exploration,
  executions,
  note,
  onNote,
  openTrail,
}: {
  exploration: ExplorationView;
  executions: Executions;
  note: string;
  onNote: (value: string) => void;
  openTrail: (trail: TrailView) => void;
}) {
  const field = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  return (
    <section className="notebook scroll-surface" aria-label="Stage notebook">
      <header className="document-heading">
        <div className="eyebrow">Stage 01 · Local notebook</div>
        <h1>{exploration.stage.title}</h1>
        <p>{exploration.stage.introduction}</p>
      </header>
      {exploration.stage.blocks.map((block, index) => {
        const trail = exploration.trails.find(
          (entry) => entry.source.id === block.asset.id,
        );
        return (
          <article
            className="notebook-cell"
            key={block.id}
            aria-label={`${block.asset.title} block`}
          >
            <span className="cell-number">
              {String(index + 1).padStart(2, "0")}
            </span>
            <div className="block-body">
              <div className="block-heading">
                <span>
                  <VscodeIcon
                    name={
                      block.asset.content.kind === "image"
                        ? "file-media"
                        : "symbol-string"
                    }
                  />{" "}
                  {block.asset.title}
                </span>
                <span className="badge">Asset · r{block.asset.revision}</span>
              </div>
              <button
                className={`asset-content ${block.asset.content.kind}`}
                type="button"
                aria-label={`Explore ${block.asset.title}`}
                onClick={() => {
                  if (trail) openTrail(trail);
                }}
              >
                {block.asset.content.kind === "image" ? (
                  <SampleImage />
                ) : (
                  <code>{block.asset.content.text}</code>
                )}
                <span className="asset-open">
                  <VscodeIcon name="open-preview" /> Open exploration canvas
                </span>
              </button>
              {block.asset.content.kind === "image" ? (
                <small className="image-caption">
                  Illustrative source image · click to inspect in its own trail
                </small>
              ) : null}
              {trail?.pipelineId ? (
                <OperationCell trail={trail} executions={executions} />
              ) : null}
            </div>
          </article>
        );
      })}
      <article className="notebook-cell note-cell" aria-label="Reasoning block">
        <span className="cell-number">03</span>
        <div className="block-body">
          <div className="block-heading">
            <span>
              <VscodeIcon name="note" /> Reasoning
            </span>
            <span className="badge">Session draft</span>
          </div>
          <VscodeTextarea
            ref={field}
            label="Stage note"
            value={note}
            rows={3}
            resize="vertical"
            onInput={() => onNote(field.current?.value ?? "")}
          />
        </div>
      </article>
    </section>
  );
}
