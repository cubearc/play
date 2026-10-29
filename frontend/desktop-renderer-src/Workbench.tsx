import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeSplitLayout from "@vscode-elements/react-elements/dist/components/VscodeSplitLayout.js";
import VscodeToolbarButton from "@vscode-elements/react-elements/dist/components/VscodeToolbarButton.js";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  type AssetBlockView,
  type Context,
  contextOf,
  type TextAssetView,
  type TrailView,
} from "../desktop/workspace-contract";
import { AssetCanvas } from "./AssetCanvas";
import { CommandPalette, type WorkbenchCommand } from "./CommandPalette";
import { ContentDialog, type DialogKind } from "./ContentDialog";
import { Explorer } from "./Explorer";
import { Notebook } from "./Notebook";
import { TextAssetEditor } from "./TextAssetEditor";
import { useExecutions } from "./useExecutions";
import { useWorkspace } from "./useWorkspace";
import { WorkspaceWelcome } from "./WorkspaceWelcome";

const mac = navigator.userAgent.includes("Mac");
const modifier = mac ? "⌘" : "Ctrl+";
interface TextEdit {
  context: Context;
  blockId: string;
  asset: TextAssetView;
  caption: boolean;
}

export function Workbench() {
  const state = useWorkspace();
  const executions = useExecutions(state.exploration, state.refreshSummary);
  const { workspace, exploration } = state;
  const [welcome, setWelcome] = useState(false);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [editing, setEditing] = useState<TextEdit | null>(null);
  const [editorDirty, setEditorDirty] = useState(false);
  useEffect(() => {
    void window.play
      .dirty(state.hasUnsavedNotes || editorDirty)
      .catch(() => {});
  }, [state.hasUnsavedNotes, editorDirty]);
  const [tabs, setTabs] = useState<Record<string, readonly string[]>>({});
  const [selection, setSelection] = useState<Record<string, string>>({});
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
  const run = () => {
    if (runnableTrail?.source.content.kind !== "text" || runnableTrail.archived)
      return;
    const operation = executions.operationFor(runnableTrail);
    if (operation) void executions.run(runnableTrail, operation);
  };
  const editText = (block: AssetBlockView, asset: TextAssetView) => {
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
      if (
        !(mac ? event.metaKey : event.ctrlKey) ||
        event.altKey ||
        event.repeat
      )
        return;
      if (dialog || editing) return;
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

  const commands: WorkbenchCommand[] = [
    {
      id: "run",
      label: "Run notebook cell",
      shortcut: `${modifier}Enter`,
      disabled:
        executions.busy ||
        !runnableTrail ||
        runnableTrail.archived ||
        runnableTrail.source.content.kind !== "text" ||
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
      <VscodeSplitLayout
        ref={sidebarSplit}
        className="workspace-split"
        split="vertical"
        fixedPane="start"
        initialHandlePosition="258px"
        handlePosition={sidebar ? `${sidebarWidth}px` : "0px"}
        minStart={sidebar ? "210px" : "0px"}
        minEnd="400px"
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
              <div
                className="editor-tabbar"
                role="tablist"
                aria-label="Exploration tabs"
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
                  <div
                    className={`tab-with-close ${activeTab === trail.id ? "selected" : ""}`}
                    key={trail.id}
                  >
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activeTab === trail.id}
                      className="editor-tab"
                      onClick={() => openTrail(trail)}
                    >
                      <VscodeIcon name="git-branch" />
                      {trail.source.title}
                    </button>
                    <button
                      type="button"
                      className="close-tab"
                      aria-label={`Close ${trail.source.title} canvas`}
                      onClick={() => {
                        setTabs((previous) => ({
                          ...previous,
                          [contextKey]: (previous[contextKey] ?? []).filter(
                            (id) => id !== trail.id,
                          ),
                        }));
                        if (activeTab === trail.id) openNotebook();
                      }}
                    >
                      <VscodeIcon name="close" />
                    </button>
                  </div>
                ))}
              </div>
              <div className="breadcrumbs">
                <span>{workspace?.title ?? "Cube Play"}</span>
                <VscodeIcon name="chevron-right" />
                <span>{exploration?.title ?? ""}</span>
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
                  executions={executions}
                />
              ) : (
                <Notebook
                  exploration={exploration}
                  executions={executions}
                  note={state.noteValue}
                  noteDirty={state.noteDirty}
                  noteSaving={state.noteSaving}
                  busy={state.busy}
                  onNote={state.editNote}
                  saveNote={() => void state.saveNote()}
                  addText={() => setDialog("text")}
                  addImage={() => setDialog("image")}
                  openTrail={openTrail}
                  editText={editText}
                  reorderBlocks={state.reorderBlocks}
                  duplicateBlock={state.duplicateBlock}
                  setArchived={state.setArchived}
                />
              )}
            </>
          )}
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
        <span>{exploration?.title ?? ""}</span>
        <span className="status-spacer" />
        <span>{activeTrail ? "Trail canvas" : "Stage notebook"}</span>
        <span>
          {state.noteSaving
            ? "Saving…"
            : state.noteDirty || editorDirty
              ? "Unsaved content"
              : workspace
                ? "Local storage"
                : "Local"}
        </span>
      </footer>
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
