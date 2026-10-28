import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useRef } from "react";
import type {
  AssetView,
  ExplorationView,
  TrailView,
} from "../desktop/workspace-contract";
import { contextOf } from "../desktop/workspace-contract";
import { NotebookAsset } from "./NotebookAsset";
import { OperationCell } from "./OperationCell";
import type { Executions } from "./useExecutions";

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
  busy,
}: {
  exploration: ExplorationView;
  executions: Executions;
  note: string;
  noteDirty: boolean;
  noteSaving: boolean;
  busy: boolean;
  saveNote: () => void;
  addText: () => void;
  addImage: () => void;
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
            disabled={busy}
            onClick={addText}
          >
            Add text
          </VscodeButton>
          <VscodeButton
            secondary
            icon="file-media"
            disabled={busy}
            onClick={addImage}
          >
            Add image
          </VscodeButton>
        </div>
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
              <NotebookAsset
                block={block}
                context={contextOf(exploration)}
                openAsset={openAsset}
              />
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
            <span className="badge">
              {noteSaving
                ? "Saving…"
                : noteDirty
                  ? "Unsaved draft"
                  : `Saved · r${exploration.stage.note.revision}`}
            </span>
          </div>
          <VscodeTextarea
            ref={field}
            label="Stage note"
            value={note}
            rows={3}
            disabled={noteSaving}
            resize="vertical"
            onInput={() => onNote(field.current?.value ?? "")}
          />
          <div className="note-save">
            <VscodeButton
              icon="save"
              disabled={!noteDirty || noteSaving}
              onClick={saveNote}
            >
              Save reasoning
            </VscodeButton>
            <span role="status" aria-label="Note save status">
              {noteSaving ? "Saving…" : noteDirty ? "Unsaved changes" : "Saved"}
            </span>
          </div>
        </div>
      </article>
    </section>
  );
}
