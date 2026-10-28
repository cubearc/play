import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeSplitLayout from "@vscode-elements/react-elements/dist/components/VscodeSplitLayout.js";
import VscodeToolbarButton from "@vscode-elements/react-elements/dist/components/VscodeToolbarButton.js";
import { useCallback, useEffect, useRef, useState } from "react";
import { AssetCanvas } from "./AssetCanvas";
import { CommandPalette, type WorkbenchCommand } from "./CommandPalette";
import { Explorer } from "./Explorer";
import {
  type ExplorationView,
  exampleWorkspace,
  type TrailView,
} from "./exampleWorkspace";
import { Notebook } from "./Notebook";
import { useExecutions } from "./useExecutions";

const mac = navigator.userAgent.includes("Mac");
const modifier = mac ? "⌘" : "Ctrl+";

export function Workbench() {
  const executions = useExecutions();
  const [exploration, setExploration] = useState<ExplorationView>(
    exampleWorkspace.explorations[0],
  );
  const [tabs, setTabs] = useState<Record<string, readonly string[]>>({});
  const [selection, setSelection] = useState<Record<string, string>>({});
  const [notes, setNotes] = useState<Record<string, string>>({});
  const [sidebar, setSidebar] = useState(true);
  const [palette, setPalette] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(258);
  const sidebarSplit =
    useRef<HTMLElementTagNameMap["vscode-split-layout"]>(null);
  const activeTab = selection[exploration.id] ?? "notebook";
  const activeTrail = exploration.trails.find(
    (entry) => entry.id === activeTab,
  );
  const captionTrail = exploration.trails.find(
    (entry) => entry.source.content.kind === "text",
  );
  const runnableTrail = activeTrail ?? captionTrail;
  const openTabs = exploration.trails.filter((entry) =>
    tabs[exploration.id]?.includes(entry.id),
  );
  const toggleSidebar = useCallback(() => setSidebar((value) => !value), []);
  const openNotebook = () =>
    setSelection((previous) => ({ ...previous, [exploration.id]: "notebook" }));
  const openTrail = (trail: TrailView) => {
    setTabs((previous) => ({
      ...previous,
      [exploration.id]: [
        ...new Set([...(previous[exploration.id] ?? []), trail.id]),
      ],
    }));
    setSelection((previous) => ({ ...previous, [exploration.id]: trail.id }));
  };
  const run = () => {
    if (runnableTrail?.source.content.kind !== "text") return;
    const operation = executions.operationFor(runnableTrail);
    if (operation) void executions.run(runnableTrail, operation);
  };

  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (
        !(mac ? event.metaKey : event.ctrlKey) ||
        event.altKey ||
        event.repeat
      )
        return;
      const key = event.key.toLowerCase();
      if (key === "b" && !event.shiftKey) {
        event.preventDefault();
        toggleSidebar();
      } else if (key === "p" && event.shiftKey) {
        event.preventDefault();
        setPalette(true);
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
          <span>{exampleWorkspace.title} — Cube Play</span>
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
            exploration={exploration}
            activeTab={activeTab}
            selectExploration={setExploration}
            openNotebook={openNotebook}
            openTrail={openTrail}
          />
        </div>
        <main slot="end" className="editor" aria-label="Exploration editors">
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
                      [exploration.id]: (previous[exploration.id] ?? []).filter(
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
            <span>{exampleWorkspace.title}</span>
            <VscodeIcon name="chevron-right" />
            <span>{exploration.title}</span>
            <VscodeIcon name="chevron-right" />
            <span>{exploration.stage.title}</span>
            {activeTrail ? (
              <>
                <VscodeIcon name="chevron-right" />
                <span>{activeTrail.title}</span>
              </>
            ) : null}
          </div>
          <div className="preview-banner">
            <VscodeIcon name="info" />
            Example workspace · changes stay in this session
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
              note={notes[exploration.stage.id] ?? ""}
              onNote={(value) =>
                setNotes((previous) => ({
                  ...previous,
                  [exploration.stage.id]: value,
                }))
              }
              openTrail={openTrail}
            />
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
        <span>{exploration.title}</span>
        <span className="status-spacer" />
        <span>{activeTrail ? "Trail canvas" : "Stage notebook"}</span>
        <span>Local · session only</span>
      </footer>
      {palette ? (
        <CommandPalette commands={commands} onClose={() => setPalette(false)} />
      ) : null}
    </div>
  );
}
