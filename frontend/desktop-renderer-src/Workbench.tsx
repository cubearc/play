import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeOption from "@vscode-elements/react-elements/dist/components/VscodeOption.js";
import VscodeSingleSelect from "@vscode-elements/react-elements/dist/components/VscodeSingleSelect.js";
import VscodeSplitLayout from "@vscode-elements/react-elements/dist/components/VscodeSplitLayout.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeToolbarButton from "@vscode-elements/react-elements/dist/components/VscodeToolbarButton.js";
import { useCallback, useEffect, useRef, useState } from "react";
import { CommandPalette, type WorkbenchCommand } from "./CommandPalette";
import { Explorer } from "./Explorer";
import { OutputPanel } from "./OutputPanel";
import { useProbe } from "./useProbe";

const mac = navigator.userAgent.includes("Mac");
const modifier = mac ? "⌘" : "Ctrl+";

export function Workbench() {
  const probe = useProbe();
  const [sidebar, setSidebar] = useState(true);
  const [output, setOutput] = useState(true);
  const [palette, setPalette] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(240);
  const [editorPercent, setEditorPercent] = useState(65);
  const [cursor, setCursor] = useState({ line: 1, column: 1 });
  const editor = useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const selector = useRef<HTMLElementTagNameMap["vscode-single-select"]>(null);
  const gutter = useRef<HTMLPreElement>(null);
  const sidebarSplit =
    useRef<HTMLElementTagNameMap["vscode-split-layout"]>(null);
  const editorSplit =
    useRef<HTMLElementTagNameMap["vscode-split-layout"]>(null);
  const focusEditor = useCallback(
    () => editor.current?.wrappedElement.focus(),
    [],
  );
  const run = useCallback(() => {
    setOutput(true);
    void probe.run();
  }, [probe.run]);
  const toggleSidebar = useCallback(() => setSidebar((value) => !value), []);
  const toggleOutput = useCallback(() => setOutput((value) => !value), []);
  const updateCursor = () => {
    const field = editor.current?.wrappedElement;
    if (!field) return;
    const position =
      field.selectionDirection === "backward"
        ? field.selectionStart
        : field.selectionEnd;
    const lines = field.value.slice(0, position).split("\n");
    setCursor({ line: lines.length, column: (lines.at(-1)?.length ?? 0) + 1 });
  };

  useEffect(() => {
    let disposed = false;
    let cleanup = () => {};
    const control = editor.current;
    void control?.updateComplete.then(() => {
      if (disposed) return;
      const field = control.wrappedElement;
      field.wrap = "off";
      field.spellcheck = false;
      field.style.lineHeight = "20px";
      field.style.padding = "8px 0";
      field.style.border = "0";
      const scroll = () => {
        if (gutter.current) gutter.current.scrollTop = field.scrollTop;
      };
      field.addEventListener("scroll", scroll);
      cleanup = () => field.removeEventListener("scroll", scroll);
    });
    return () => {
      disposed = true;
      cleanup();
    };
  }, []);

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
      } else if (key === "j" && !event.shiftKey) {
        event.preventDefault();
        toggleOutput();
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
  }, [toggleSidebar, toggleOutput, run, palette]);

  const commands: WorkbenchCommand[] = [
    {
      id: "run",
      label: "Run active operation",
      shortcut: `${modifier}Enter`,
      disabled: !probe.canRun,
      execute: run,
    },
    {
      id: "sidebar",
      label: "Toggle Primary Side Bar",
      shortcut: `${modifier}B`,
      execute: toggleSidebar,
    },
    {
      id: "output",
      label: "Toggle Output Panel",
      shortcut: `${modifier}J`,
      execute: toggleOutput,
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
          <span>Scratch — Cube Play</span>
          <kbd>{mac ? "⇧⌘P" : "Ctrl+Shift+P"}</kbd>
        </button>
        <div className="title-actions">
          <VscodeToolbarButton
            icon="layout-sidebar-left"
            label="Toggle Primary Side Bar"
            onClick={toggleSidebar}
          />
          <VscodeToolbarButton
            icon="layout-panel"
            label="Toggle Output Panel"
            onClick={toggleOutput}
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
        <button
          type="button"
          className={output ? "activity active" : "activity"}
          aria-label="Output"
          aria-pressed={output}
          title={`Output (${modifier}J)`}
          onClick={toggleOutput}
        >
          <VscodeIcon name="output" size={24} />
        </button>
      </nav>
      <VscodeSplitLayout
        ref={sidebarSplit}
        className="workspace-split"
        split="vertical"
        fixedPane="start"
        initialHandlePosition="240px"
        handlePosition={sidebar ? `${sidebarWidth}px` : "0px"}
        minStart={sidebar ? "180px" : "0px"}
        minEnd="400px"
        resetOnDblClick
        onVscSplitLayoutChange={(event) => {
          if (
            event.target === sidebarSplit.current &&
            sidebar &&
            event.detail.position >= 180
          )
            setSidebarWidth(event.detail.position);
        }}
      >
        <div slot="start" className="sidebar-slot">
          <Explorer
            hidden={!sidebar}
            focusEditor={focusEditor}
            selected={probe.selected}
          />
        </div>
        <VscodeSplitLayout
          ref={editorSplit}
          slot="end"
          className="editor-split"
          split="horizontal"
          handlePosition={output ? `${editorPercent}%` : "100%"}
          initialHandlePosition="65%"
          minStart="200px"
          minEnd={output ? "100px" : "0px"}
          resetOnDblClick
          onVscSplitLayoutChange={(event) => {
            if (
              event.target === editorSplit.current &&
              output &&
              event.detail.positionInPercentage < 95
            )
              setEditorPercent(event.detail.positionInPercentage);
          }}
        >
          <main slot="start" className="editor" aria-label="Editor">
            <div className="editor-tabbar">
              <button
                type="button"
                className="editor-tab"
                aria-label="Untitled-1, unsaved"
                onClick={focusEditor}
              >
                <VscodeIcon name="file-text" />
                Untitled-1
                <span className="unsaved" aria-hidden="true">
                  ●
                </span>
              </button>
            </div>
            <div className="breadcrumbs">
              <span>Scratch</span>
              <VscodeIcon name="chevron-right" />
              <span>Untitled-1</span>
            </div>
            <div className="run-toolbar">
              <label className="operation-label" htmlFor="operation">
                Operation
              </label>
              <VscodeSingleSelect
                id="operation"
                ref={selector}
                label="Operation"
                value={probe.operation}
                disabled={probe.running}
                onChange={() =>
                  probe.setOperation(selector.current?.value ?? "")
                }
              >
                {probe.operations.map((entry) => (
                  <VscodeOption key={entry.operation} value={entry.operation}>
                    {entry.operation}
                  </VscodeOption>
                ))}
              </VscodeSingleSelect>
              <VscodeButton icon="play" disabled={!probe.canRun} onClick={run}>
                Run
              </VscodeButton>
              <VscodeButton
                icon="debug-stop"
                secondary
                disabled={!probe.running}
                onClick={() => void window.play.cancel().catch(() => {})}
              >
                Cancel
              </VscodeButton>
            </div>
            <div className="editor-buffer">
              <pre ref={gutter} className="line-numbers" aria-hidden="true">
                {Array.from(
                  { length: probe.input.split("\n").length },
                  (_, index) => index + 1,
                ).join("\n")}
              </pre>
              <VscodeTextarea
                ref={editor}
                className="source-input"
                label="Input"
                monospace
                resize="none"
                value={probe.input}
                disabled={probe.running}
                spellCheck={false}
                onInput={() => {
                  probe.setInput(editor.current?.value ?? "");
                  updateCursor();
                }}
                onClick={updateCursor}
                onKeyUp={updateCursor}
              />
            </div>
            {probe.bytes > probe.limit ? (
              <div className="input-error" role="alert">
                Input exceeds {probe.limit.toLocaleString()} UTF-8 bytes.
              </div>
            ) : null}
          </main>
          <div slot="end" className="panel-slot">
            <OutputPanel
              hidden={!output}
              result={probe.result}
              running={probe.running}
              error={probe.error}
              elapsed={probe.elapsed}
              onClose={() => setOutput(false)}
            />
          </div>
        </VscodeSplitLayout>
      </VscodeSplitLayout>
      <footer className="statusbar">
        <span className="status-remote">
          <VscodeIcon name="remote" />
        </span>
        <span className="connection">
          <VscodeIcon name={probe.running ? "sync" : "pass"} />
          {probe.running
            ? "Running"
            : probe.operations.length
              ? "Engine ready"
              : "Connecting…"}
        </span>
        <span className="status-spacer" />
        <span>
          Ln {cursor.line}, Col {cursor.column}
        </span>
        <span>{probe.bytes.toLocaleString()} bytes</span>
        <span>UTF-8</span>
        <span>Plain Text</span>
      </footer>
      {palette ? (
        <CommandPalette
          commands={commands}
          onClose={() => {
            setPalette(false);
            focusEditor();
          }}
        />
      ) : null}
    </div>
  );
}
