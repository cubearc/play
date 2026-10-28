import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useRef } from "react";
import type { AssetView, ExplorationView, TrailView } from "./exampleWorkspace";
import { NotebookAsset } from "./NotebookAsset";
import { OperationCell } from "./OperationCell";
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
  const openAsset = (asset: AssetView) => {
    const trail = exploration.trails.find(
      (entry) => entry.source.id === asset.id,
    );
    if (trail) openTrail(trail);
  };
  return (
    <section className="notebook scroll-surface" aria-label="Stage notebook">
      <header className="document-heading">
        <div className="eyebrow">Stage 01 · Local notebook</div>
        <h1>{exploration.stage.title}</h1>
        <p>{exploration.stage.introduction}</p>
      </header>
      {exploration.stage.blocks.map((block, index) => {
        const trail = exploration.trails.find(
          (entry) => entry.source.id === (block.caption ?? block.asset).id,
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
              <NotebookAsset block={block} openAsset={openAsset} />
              {trail?.pipelineId ? (
                <OperationCell trail={trail} executions={executions} />
              ) : null}
            </div>
          </article>
        );
      })}
      <article className="notebook-cell note-cell" aria-label="Reasoning block">
        <span className="cell-number">
          {String(exploration.stage.blocks.length + 1).padStart(2, "0")}
        </span>
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
