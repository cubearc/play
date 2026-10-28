import VscodeTabHeader from "@vscode-elements/react-elements/dist/components/VscodeTabHeader.js";
import VscodeTabPanel from "@vscode-elements/react-elements/dist/components/VscodeTabPanel.js";
import VscodeTabs from "@vscode-elements/react-elements/dist/components/VscodeTabs.js";
import VscodeToolbarButton from "@vscode-elements/react-elements/dist/components/VscodeToolbarButton.js";
import type { RunResult } from "../desktop/contract";

function outputText(
  result: RunResult | undefined,
  running: boolean,
  error: string,
): string {
  if (running) return "Running…";
  if (error) return error;
  if (!result) return "Run an operation to inspect its output.";
  if (result.status === "cancelled") return "Cancelled";
  if (result.status === "failed") return `Execution failed: ${result.code}`;
  return "text" in result.output
    ? result.output.text
    : JSON.stringify(result.output.bytes);
}

export function OutputPanel({
  hidden,
  result,
  running,
  error,
  elapsed,
  onClose,
}: {
  hidden: boolean;
  result: RunResult | undefined;
  running: boolean;
  error: string;
  elapsed: number;
  onClose: () => void;
}) {
  return (
    <section className="output-panel" aria-label="Output" hidden={hidden}>
      <VscodeTabs className="output-tabs" panel>
        <VscodeTabHeader slot="header">Output</VscodeTabHeader>
        <VscodeTabHeader slot="header">Details</VscodeTabHeader>
        <VscodeToolbarButton
          slot="addons"
          icon="close"
          label="Close Output"
          onClick={onClose}
        />
        <VscodeTabPanel className="panel-content">
          <div className="output-caption">
            <span>
              {running
                ? "Running"
                : (result?.status ?? (error ? "Rejected" : "Ready"))}
            </span>
            {(result || error) && !running ? <span>{elapsed} ms</span> : null}
          </div>
          <pre className="output-value" role="status">
            {outputText(result, running, error)}
          </pre>
        </VscodeTabPanel>
        <VscodeTabPanel className="panel-content">
          <pre className="output-value">
            {result ? JSON.stringify(result, null, 2) : "No response yet."}
          </pre>
        </VscodeTabPanel>
      </VscodeTabs>
    </section>
  );
}
