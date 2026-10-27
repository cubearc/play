import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { InterfaceIcon } from "./InterfaceIcon";
import type { Panel } from "./investigation-api";

const panels: { id: Panel; label: string; description: string }[] = [
  {
    id: "files",
    label: "Files",
    description:
      "Browse and inspect authorized source files and private imports.",
  },
  {
    id: "explorations",
    label: "Explorations",
    description:
      "Connect typed operations, investigate hypotheses and revisit actual executions.",
  },
  {
    id: "notes",
    label: "Notes",
    description:
      "Keep private observations alongside the investigation and selected file.",
  },
  {
    id: "artifacts",
    label: "Artifacts",
    description:
      "Reopen named results, inspect provenance and export explicitly.",
  },
  {
    id: "solutions",
    label: "Solutions",
    description:
      "Preserve private findings and curated execution paths in immutable revisions.",
  },
  {
    id: "challenges",
    label: "Local challenges",
    description:
      "Import inert local instructions and maintain private stage progress.",
  },
  {
    id: "terminal",
    label: "Terminal",
    description: "Run real Cube commands in the isolated local Runner.",
  },
];
export function WorkspacePanels({
  active,
  onChange,
  children,
}: {
  active: Panel;
  onChange: (panel: Panel) => void;
  children: Record<Panel, ReactNode>;
}) {
  const id = useId();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const previous = useRef(active),
    focusedPanel = useRef<string | null>(null);
  useEffect(() => {
    if (
      previous.current !== active &&
      focusedPanel.current === previous.current
    )
      buttons.current[panels.findIndex((p) => p.id === active)]?.focus();
    previous.current = active;
  }, [active]);
  const [vertical, setVertical] = useState(() => window.innerWidth >= 960);
  useEffect(() => {
    const resize = () => setVertical(window.innerWidth >= 960);
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  return (
    <div
      className="workspace-layout"
      onFocusCapture={(event) => {
        focusedPanel.current =
          event.target instanceof HTMLElement
            ? (event.target
                .closest("[data-workspace-panel]")
                ?.getAttribute("data-workspace-panel") ?? null)
            : null;
      }}
    >
      <div
        className="workspace-navigation"
        role="tablist"
        aria-label="Workspace sections"
        aria-orientation={vertical ? "vertical" : "horizontal"}
      >
        {panels.map((panel, index) => (
          <div className="workspace-nav-item" key={panel.id}>
            {(index === 0 ||
              panel.id === "artifacts" ||
              panel.id === "challenges") && (
              <span className="workspace-nav-group" aria-hidden="true">
                {index === 0
                  ? "Investigate"
                  : panel.id === "artifacts"
                    ? "Library"
                    : "Tools"}
              </span>
            )}
            <button
              type="button"
              role="tab"
              id={`${id}-tab-${panel.id}`}
              aria-controls={`${id}-panel-${panel.id}`}
              aria-selected={panel.id === active}
              tabIndex={panel.id === active ? 0 : -1}
              ref={(button) => {
                buttons.current[index] = button;
              }}
              onClick={() => onChange(panel.id)}
              onKeyDown={(event) => {
                const next = vertical ? "ArrowDown" : "ArrowRight";
                const previous = vertical ? "ArrowUp" : "ArrowLeft";
                let target: number;
                if (event.key === next) target = (index + 1) % panels.length;
                else if (event.key === previous)
                  target = (index + panels.length - 1) % panels.length;
                else if (event.key === "Home") target = 0;
                else if (event.key === "End") target = panels.length - 1;
                else return;
                event.preventDefault();
                const panel = panels[target];
                if (panel) {
                  onChange(panel.id);
                  buttons.current[target]?.focus();
                }
              }}
            >
              <InterfaceIcon name={panel.id} />
              <span>{panel.label}</span>
            </button>
          </div>
        ))}
      </div>
      <div className="workspace-panels">
        {panels.map((panel) => (
          <div
            key={panel.id}
            role="tabpanel"
            data-workspace-panel={panel.id}
            id={`${id}-panel-${panel.id}`}
            aria-labelledby={`${id}-tab-${panel.id}`}
            hidden={active !== panel.id}
          >
            <header className="workspace-panel-heading">
              <h2>{panel.label}</h2>
              <p>{panel.description}</p>
            </header>
            {children[panel.id]}
          </div>
        ))}
      </div>
    </div>
  );
}
