import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeSplitLayout from "@vscode-elements/react-elements/dist/components/VscodeSplitLayout.js";
import VscodeToolbarButton from "@vscode-elements/react-elements/dist/components/VscodeToolbarButton.js";
import { useCallback, useEffect, useRef, useState } from "react";
import type { PipelineView } from "../desktop/workspace-contract";
import {
  type AssetBlockView,
  type Context,
  contextOf,
  type EditableAssetView,
  type RunOutputOrigin,
  type TrailView,
} from "../desktop/workspace-contract";
import { AssetCanvas } from "./AssetCanvas";
import { CommandPalette, type WorkbenchCommand } from "./CommandPalette";
import { ContentDialog, type DialogKind } from "./ContentDialog";
import { ContextTerminal } from "./ContextTerminal";
import { Explorer } from "./Explorer";
import { Notebook } from "./Notebook";
import { PipelineEditor, type PipelineMode } from "./PipelineEditor";
import { runSelection } from "./runSelection";
import { TextAssetEditor } from "./TextAssetEditor";
import { TrailEditor } from "./TrailEditor";
import { useExecutions } from "./useExecutions";
import { useWorkspace } from "./useWorkspace";
import VscodeButton from "./VscodeButton";
import { WorkspaceWelcome } from "./WorkspaceWelcome";

const mac = navigator.userAgent.includes("Mac");
const modifier = mac ? "⌘" : "Ctrl+";
interface TextEdit {
  context: Context;
  blockId: string;
  asset: EditableAssetView;
  caption: boolean;
}

export function Workbench() {
  const state = useWorkspace();
  const { workspace, exploration } = state;
  const [terminal, setTerminal] = useState(false);
  const [terminalEpoch, setTerminalEpoch] = useState(0);
  const [welcome, setWelcome] = useState(false);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [editing, setEditing] = useState<TextEdit | null>(null);
  const [editorDirty, setEditorDirty] = useState(false);
  const [editingTrail, setEditingTrail] = useState<TrailView | null>(null);
  const [editingPipeline, setEditingPipeline] = useState<{
    trail: TrailView;
    mode: PipelineMode;
    pipeline: PipelineView | null;
  } | null>(null);
  const closeConfiguration = () => {
    setEditingTrail(null);
    setEditingPipeline(null);
    setEditorDirty(false);
    state.clearError();
  };
  useEffect(() => {
    void window.play
      .dirty(state.hasUnsavedNotes || editorDirty)
      .catch(() => {});
  }, [state.hasUnsavedNotes, editorDirty]);
  const [tabs, setTabs] = useState<Record<string, readonly string[]>>({});
  const [selection, setSelection] = useState<Record<string, string>>({});
  const [pendingFocus, setPendingFocus] = useState<{
    stageId: string;
    blockId: string;
  } | null>(null);
  const focusHandled = useCallback(() => setPendingFocus(null), []);
  const executions = useExecutions(state.exploration, state.refreshSummary, {
    busy: state.busy,
    save: async (trail, run, port) => {
      const attempt = run.attempts.find((a) =>
        run.plan.targetNodes.includes(a.nodeId),
      );
      if (
        !exploration ||
        exploration.id !== trail.explorationId ||
        exploration.stage.id !== trail.stageId ||
        !attempt
      )
        return;
      const block = await state.pinRunOutput(
        contextOf(exploration),
        exploration.stage.revision,
        run.id,
        attempt.id,
        port,
      );
      if (block) {
        const key = `${trail.explorationId}/${trail.stageId}`;
        setSelection((previous) => ({ ...previous, [key]: "notebook" }));
        setPendingFocus({ stageId: trail.stageId, blockId: block });
      }
    },
  });
  const [sidebar, setSidebar] = useState(true);
  const [palette, setPalette] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(258);
  const sidebarSplit =
    useRef<HTMLElementTagNameMap["vscode-split-layout"]>(null);
  const contextKey = exploration
    ? `${exploration.id}/${exploration.stage.id}`
    : "";
  const activeTab = selection[contextKey] ?? "notebook";
  const activeTrail = exploration?.trails.find(
    (entry) => entry.id === activeTab,
  );
  const firstText = exploration?.stage.blocks.find(
    (block) =>
      block.kind === "asset" &&
      (block.caption || block.asset.content.kind === "text"),
  );
  const firstSource =
    firstText?.kind === "asset" ? (firstText.caption ?? firstText.asset) : null;
  const captionTrail = exploration?.trails.find(
    (entry) => entry.source.id === firstSource?.id,
  );
  const runnableTrail = activeTrail ?? captionTrail;
  const openTabs =
    exploration?.trails.filter((entry) =>
      tabs[contextKey]?.includes(entry.id),
    ) ?? [];
  const toggleSidebar = useCallback(() => setSidebar((value) => !value), []);
  const openNotebook = () =>
    setSelection((previous) => ({ ...previous, [contextKey]: "notebook" }));
  const openTrail = (trail: TrailView) => {
    setTabs((previous) => ({
      ...previous,
      [contextKey]: [...new Set([...(previous[contextKey] ?? []), trail.id])],
    }));
    setSelection((previous) => ({ ...previous, [contextKey]: trail.id }));
  };
  const openSourceRun = (origin: RunOutputOrigin) => {
    const trail = exploration?.trails.find(
      (trail) => trail.id === origin.trailId,
    );
    if (trail) {
      openTrail(trail);
      void executions.show(trail, origin.runId);
    }
  };
  const run = () => {
    if (!runnableTrail?.pipelineId || runnableTrail.archived) return;
    const operation = executions.operationFor(runnableTrail);
    if (operation) void executions.run(runnableTrail, operation);
  };
  const editText = (block: AssetBlockView, asset: EditableAssetView) => {
    if (!exploration) return;
    state.clearError();
    setEditing({
      context: contextOf(exploration),
      blockId: block.id,
      asset,
      caption: block.caption?.id === asset.id,
    });
  };
  const closeEditor = () => {
    setEditing(null);
    setEditorDirty(false);
    state.clearError();
  };

  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.code === "Backquote" && !event.repeat) {
        event.preventDefault();
        setTerminal((value) => !value);
        return;
      }
      if (
        event.target instanceof Element &&
        event.target.closest(".context-terminal")
      )
        return;
      if (
        !(mac ? event.metaKey : event.ctrlKey) ||
        event.altKey ||
        event.repeat
      )
        return;
      if (dialog || editing || editingTrail || editingPipeline) return;
      const key = event.key.toLowerCase();
      if (key === "b" && !event.shiftKey) {
        event.preventDefault();
        toggleSidebar();
      } else if (key === "p" && event.shiftKey) {
        event.preventDefault();
        setPalette(true);
      } else if (key === "s" && !event.shiftKey && !palette) {
        event.preventDefault();
        void state.saveNote();
      } else if (key === "enter" && !event.shiftKey && !palette) {
        event.preventDefault();
        run();
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });

  const terminalOperation = activeTrail
    ? executions.operationFor(activeTrail)
    : undefined;
  const commands: WorkbenchCommand[] = [
    {
      id: "terminal",
      label: "Toggle Cube terminal",
      shortcut: "Ctrl+`",
      disabled: !exploration,
      execute: () => setTerminal((value) => !value),
    },
    {
      id: "run",
      label: "Run notebook cell",
      shortcut: `${modifier}Enter`,
      disabled:
        !runnableTrail ||
        executions.isBusy(runnableTrail) ||
        runnableTrail.archived ||
        !runnableTrail.pipelineId ||
        !executions.operationFor(runnableTrail),
      execute: run,
    },
    {
      id: "notebook",
      label: "Open Stage notebook",
      shortcut: "",
      execute: openNotebook,
    },
    {
      id: "sidebar",
      label: "Toggle Primary Side Bar",
      shortcut: `${modifier}B`,
      execute: toggleSidebar,
    },
  ];

  return (
    <div className={`workbench ${mac ? "mac" : ""}`}>
      <header className="titlebar">
        <span className="app-name">
          <VscodeIcon name="symbol-misc" />
          Cube Play
        </span>
        <button
          className="command-center"
          type="button"
          aria-label="Command Palette"
          onClick={() => setPalette(true)}
        >
          <VscodeIcon name="search" />
          <span>{workspace?.title ?? "Cube Play"} — Cube Play</span>
          <kbd>{mac ? "⇧⌘P" : "Ctrl+Shift+P"}</kbd>
        </button>
        <div className="title-actions">
          <VscodeToolbarButton
            icon="terminal"
            label="Toggle Cube terminal"
            aria-disabled={!exploration}
            onClick={() => {
              if (exploration) setTerminal((value) => !value);
            }}
          />
          <VscodeToolbarButton
            icon="layout-sidebar-left"
            label="Toggle Primary Side Bar"
            onClick={toggleSidebar}
          />
        </div>
      </header>
      <nav className="activitybar" aria-label="Activity Bar">
        <button
          type="button"
          className={sidebar ? "activity active" : "activity"}
          aria-label="Explorer"
          aria-pressed={sidebar}
          title={`Explorer (${modifier}B)`}
          onClick={toggleSidebar}
        >
          <VscodeIcon name="files" size={24} />
        </button>
      </nav>
      {/* The element clamps positions against its current minimum sizes. */}
      <VscodeSplitLayout
        ref={sidebarSplit}
        className="workspace-split"
        split="vertical"
        fixedPane="start"
        initialHandlePosition="258px"
        minStart={sidebar ? "210px" : "0px"}
        minEnd="400px"
        handlePosition={sidebar ? `${sidebarWidth}px` : "0px"}
        resetOnDblClick
        onVscSplitLayoutChange={(event) => {
          if (
            event.target === sidebarSplit.current &&
            sidebar &&
            event.detail.position >= 210
          )
            setSidebarWidth(event.detail.position);
        }}
      >
        <div slot="start" className="sidebar-slot">
          <Explorer
            hidden={!sidebar}
            workspace={workspace}
            exploration={exploration}
            activeTab={activeTab}
            busy={state.busy}
            selectExploration={(id) => {
              setWelcome(false);
              void state.selectExploration(id);
            }}
            selectStage={(id) => void state.selectStage(id)}
            newExploration={() => setDialog("exploration")}
            newStage={() => setDialog("stage")}
            chooseWorkspace={() => setWelcome(true)}
            openNotebook={openNotebook}
            openTrail={openTrail}
          />
        </div>
        <main slot="end" className="editor" aria-label="Exploration editors">
          {state.error ? (
            <div className="workspace-error" role="alert">
              {state.error}
              <button
                type="button"
                aria-label="Dismiss error"
                onClick={state.clearError}
              >
                ×
              </button>
            </div>
          ) : null}
          {!workspace || welcome ? (
            <WorkspaceWelcome
              state={state}
              onOpened={() => setWelcome(false)}
            />
          ) : !exploration ? (
            <section className="workspace-welcome" aria-label="Empty workspace">
              <VscodeIcon name="beaker" size={40} />
              <h1>{workspace.title}</h1>
              <p>Create an Exploration to start a local notebook.</p>
              <VscodeButton
                icon="add"
                disabled={state.busy}
                onClick={() => setDialog("exploration")}
              >
                New Exploration
              </VscodeButton>
            </section>
          ) : (
            <>
              <div className="editor-tabbar">
                <div
                  role="tablist"
                  aria-label="Exploration tabs"
                  className="editor-tabs"
                  onKeyDown={(event) => {
                    const tabs = Array.from(
                      event.currentTarget.querySelectorAll<HTMLButtonElement>(
                        '[role="tab"]',
                      ),
                    );
                    const index = tabs.indexOf(
                      event.target as HTMLButtonElement,
                    );
                    const next =
                      event.key === "ArrowRight"
                        ? (index + 1) % tabs.length
                        : event.key === "ArrowLeft"
                          ? (index + tabs.length - 1) % tabs.length
                          : event.key === "Home"
                            ? 0
                            : event.key === "End"
                              ? tabs.length - 1
                              : null;
                    if (next !== null) {
                      event.preventDefault();
                      tabs[next]?.focus();
                      tabs[next]?.click();
                    }
                  }}
                >
                  <button
                    type="button"
                    role="tab"
                    aria-selected={activeTab === "notebook"}
                    className={`editor-tab ${activeTab === "notebook" ? "selected" : ""}`}
                    onClick={openNotebook}
                  >
                    <VscodeIcon name="notebook" />
                    {exploration.stage.title}
                  </button>
                  {openTabs.map((trail) => (
                    <button
                      key={trail.id}
                      type="button"
                      role="tab"
                      aria-selected={activeTab === trail.id}
                      className={`editor-tab ${activeTab === trail.id ? "selected" : ""}`}
                      onClick={() => openTrail(trail)}
                    >
                      <VscodeIcon name="git-branch" />
                      {trail.source.title}
                    </button>
                  ))}
                </div>
                {activeTrail ? (
                  <button
                    type="button"
                    className="close-tab"
                    aria-label={`Close ${activeTrail.source.title} canvas`}
                    onClick={() => {
                      setTabs((previous) => ({
                        ...previous,
                        [contextKey]: (previous[contextKey] ?? []).filter(
                          (id) => id !== activeTrail.id,
                        ),
                      }));
                      openNotebook();
                    }}
                  >
                    <VscodeIcon name="close" />
                  </button>
                ) : null}
              </div>
              <div className="breadcrumbs">
                <span>{workspace?.title ?? "Cube Play"}</span>
                <VscodeIcon name="chevron-right" />
                <span
                  className="status-exploration"
                  title={exploration?.title ?? ""}
                >
                  {exploration?.title ?? ""}
                </span>
                <VscodeIcon name="chevron-right" />
                <span>{exploration.stage.title}</span>
                {activeTrail ? (
                  <>
                    <VscodeIcon name="chevron-right" />
                    <span>{activeTrail.title}</span>
                  </>
                ) : null}
              </div>
              <div className="workspace-context">
                <VscodeIcon name="lock" />
                Private workspace · content and saved reasoning stay on this
                machine
              </div>
              {activeTrail ? (
                <AssetCanvas
                  key={activeTrail.id}
                  trail={activeTrail}
                  editTrail={() => {
                    state.clearError();
                    setEditingTrail(activeTrail);
                  }}
                  editPipeline={(mode) => {
                    state.clearError();
                    setEditingPipeline({
                      trail: activeTrail,
                      mode,
                      pipeline:
                        mode === "revise"
                          ? (workspace?.pipelines.find(
                              (p) => p.id === activeTrail.pipelineId,
                            ) ?? activeTrail.pipeline)
                          : activeTrail.pipeline,
                    });
                  }}
                  executions={executions}
                  openSourceRun={openSourceRun}
                />
              ) : (
                <Notebook
                  exploration={exploration}
                  executions={executions}
                  notes={{
                    state: state.noteState,
                    edit: state.editNote,
                    save: state.saveNote,
                    discard: state.discardNote,
                  }}
                  busy={state.busy}
                  addText={() => setDialog("text")}
                  addImage={() => setDialog("image")}
                  addFile={() => void state.importFile()}
                  addAnnotation={() => setDialog("annotation")}
                  addPage={() => setDialog("page")}
                  openTrail={openTrail}
                  editText={editText}
                  reorderBlocks={state.reorderBlocks}
                  duplicateBlock={state.duplicateBlock}
                  setArchived={state.setArchived}
                  openSourceRun={openSourceRun}
                  focusBlockId={
                    pendingFocus?.stageId === exploration.stage.id
                      ? pendingFocus.blockId
                      : null
                  }
                  onFocusHandled={focusHandled}
                />
              )}
            </>
          )}
          {terminal && exploration && !welcome ? (
            <ContextTerminal
              key={`${contextKey}:${terminalEpoch}`}
              context={contextOf(exploration)}
              selection={
                activeTrail && terminalOperation
                  ? runSelection(
                      activeTrail,
                      terminalOperation,
                      executions.keepOutput(activeTrail),
                    )
                  : null
              }
              title={`${exploration.title} / ${exploration.stage.title}${activeTrail?.pipeline ? ` · ${activeTrail.title} · Pipeline r${activeTrail.pipeline.revision} · Asset r${activeTrail.source.revision}` : " · No Pipeline selected"}`}
              assetCommand={
                activeTrail
                  ? `asset show ${activeTrail.source.id} --revision ${activeTrail.source.revision}`
                  : null
              }
              onClose={() => setTerminal(false)}
              onReconnect={() => setTerminalEpoch((value) => value + 1)}
              onRun={(id) => {
                const trail = exploration.trails.find((t) => t.id === id);
                if (trail) void executions.load(trail);
              }}
            />
          ) : null}
        </main>
      </VscodeSplitLayout>
      <footer className="statusbar">
        <span className="status-remote">
          <VscodeIcon name="remote" />
        </span>
        <span className="connection">
          <VscodeIcon name={executions.busy ? "sync" : "pass"} />
          {executions.busy
            ? "Running"
            : executions.operations.length
              ? "Engine ready"
              : "Connecting…"}
        </span>
        <span className="status-exploration" title={exploration?.title ?? ""}>
          {exploration?.title ?? ""}
        </span>
        <span className="status-spacer" />
        <span className="status-view">
          {activeTrail ? "Trail canvas" : "Stage notebook"}
        </span>
        <span className="status-storage">
          {state.noteSaving
            ? "Saving…"
            : state.noteDirty || editorDirty
              ? "Unsaved content"
              : workspace
                ? "Local storage"
                : "Local"}
        </span>
      </footer>
      {editingTrail && workspace ? (
        <TrailEditor
          trail={editingTrail}
          pipelines={workspace.pipelines}
          assets={
            exploration?.stage.blocks.flatMap((b) =>
              b.kind === "asset"
                ? [b.asset, ...(b.caption ? [b.caption] : [])]
                : [],
            ) ?? []
          }
          error={state.error}
          onClose={closeConfiguration}
          onDirty={setEditorDirty}
          onSave={(hypothesis, pipeline, key, bindings) =>
            state.saveTrail(editingTrail, hypothesis, pipeline, key, bindings)
          }
        />
      ) : null}
      {editingPipeline ? (
        <PipelineEditor
          mode={editingPipeline.mode}
          trail={editingPipeline.trail}
          assets={
            exploration?.stage.blocks.flatMap((b) =>
              b.kind === "asset"
                ? [b.asset, ...(b.caption ? [b.caption] : [])]
                : [],
            ) ?? []
          }
          pipeline={editingPipeline.pipeline}
          operations={executions.operations}
          inputType={
            editingPipeline.trail.source.content.kind === "image"
              ? "image.samples"
              : "text"
          }
          error={state.error}
          onClose={closeConfiguration}
          onDirty={setEditorDirty}
          onSave={(draft) =>
            state.savePipeline(
              editingPipeline.trail,
              editingPipeline.mode,
              editingPipeline.pipeline,
              draft,
            )
          }
        />
      ) : null}
      {dialog ? (
        <ContentDialog
          kind={dialog}
          error={state.error}
          onClose={() => setDialog(null)}
          onSubmit={async (title, text, key) => {
            if (dialog === "exploration")
              return state.createExploration(title, key);
            if (dialog === "stage") return state.createStage(title, key);
            if (dialog === "text") return state.addText(title, text, key);
            if (dialog === "page") return state.addPage(title, text, key);
            if (dialog === "annotation")
              return state.addAnnotation(title, text, key);
            return state.importImage(text, key);
          }}
        />
      ) : null}
      {editing ? (
        <TextAssetEditor
          asset={editing.asset}
          caption={editing.caption}
          error={state.error}
          onClose={closeEditor}
          onDirty={setEditorDirty}
          onSave={(text, key) =>
            state.reviseText(
              editing.context,
              editing.blockId,
              editing.asset.id,
              editing.asset.revision,
              text,
              key,
            )
          }
        />
      ) : null}
      {palette ? (
        <CommandPalette commands={commands} onClose={() => setPalette(false)} />
      ) : null}
    </div>
  );
}
