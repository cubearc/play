import { type ReactNode, useEffect, useId, useRef, useState } from "react";
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
  const [vertical, setVertical] = useState(() => window.innerWidth >= 960);
  useEffect(() => {
    const resize = () => setVertical(window.innerWidth >= 960);
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  return (
    <div className="workspace-layout">
      <div
        className="workspace-navigation"
        role="tablist"
        aria-label="Workspace sections"
        aria-orientation={vertical ? "vertical" : "horizontal"}
      >
        {panels.map((panel, index) => (
          <button
            key={panel.id}
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
            {panel.label}
          </button>
        ))}
      </div>
      <div className="workspace-panels">
        {panels.map((panel) => (
          <div
            key={panel.id}
            role="tabpanel"
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
