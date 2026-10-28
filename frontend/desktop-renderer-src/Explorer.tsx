import VscodeCollapsible from "@vscode-elements/react-elements/dist/components/VscodeCollapsible.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTree from "@vscode-elements/react-elements/dist/components/VscodeTree.js";
import VscodeTreeItem from "@vscode-elements/react-elements/dist/components/VscodeTreeItem.js";
import type { Operation } from "../desktop/contract";

export function Explorer({
  hidden,
  focusEditor,
  selected,
}: {
  hidden: boolean;
  focusEditor: () => void;
  selected: Operation | undefined;
}) {
  return (
    <aside className="explorer" aria-label="Explorer" hidden={hidden}>
      <h2 className="sidebar-title">Explorer</h2>
      <VscodeCollapsible heading="OPEN EDITORS" open>
        <VscodeTree aria-label="Open editors">
          <VscodeTreeItem selected active onClick={focusEditor}>
            <span className="tree-label">
              <VscodeIcon name="file-text" />
              Untitled-1
            </span>
            <span slot="description">in memory</span>
          </VscodeTreeItem>
        </VscodeTree>
      </VscodeCollapsible>
      <VscodeCollapsible heading="SCRATCH" open>
        <p className="sidebar-note">
          No folder opened.
          <br />
          This buffer stays in memory.
        </p>
      </VscodeCollapsible>
      <VscodeCollapsible className="operation-info" heading="OPERATION" open>
        {selected ? (
          <div className="sidebar-note">
            <div className="operation-id">{selected.operation}</div>
            <p>{selected.description}</p>
            <span className="muted">Version {selected.operation_version}</span>
          </div>
        ) : (
          <p className="sidebar-note">Loading operations…</p>
        )}
      </VscodeCollapsible>
    </aside>
  );
}
