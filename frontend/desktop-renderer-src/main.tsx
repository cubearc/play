import { createRoot } from "react-dom/client";
import type {} from "../desktop/bridge";
import { Workbench } from "./Workbench";
import "@vscode/codicons/dist/codicon.css";
import "./theme.css";
import "./style.css";

const root = document.getElementById("root");
const stylesheet = document.querySelector<HTMLLinkElement>(
  'link[rel="stylesheet"]',
);
if (stylesheet) stylesheet.id = "vscode-codicon-stylesheet";
if (root) createRoot(root).render(<Workbench />);
