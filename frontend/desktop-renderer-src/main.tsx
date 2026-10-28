import { createRoot } from "react-dom/client";
import type { Operation, RunResult } from "../desktop/contract";
import { Workbench } from "./Workbench";
import "@vscode/codicons/dist/codicon.css";
import "./theme.css";
import "./style.css";

declare global {
  interface Window {
    play: {
      catalog: () => Promise<Operation[]>;
      run: (
        operation: string,
        version: number,
        input: string,
      ) => Promise<RunResult>;
      cancel: () => Promise<void>;
    };
  }
}

const root = document.getElementById("root");
const stylesheet = document.querySelector<HTMLLinkElement>(
  'link[rel="stylesheet"]',
);
if (stylesheet) stylesheet.id = "vscode-codicon-stylesheet";
if (root) createRoot(root).render(<Workbench />);
