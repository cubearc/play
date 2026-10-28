import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useRef, useState } from "react";
import type { WorkspaceState } from "./useWorkspace";

export function WorkspaceWelcome({
  state,
  onOpened,
}: {
  state: WorkspaceState;
  onOpened: () => void;
}) {
  const key = useRef(crypto.randomUUID());
  const create = async () => {
    if (title.trim() && (await state.createWorkspace(title, key.current)))
      onOpened();
  };
  const field = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const [title, setTitle] = useState("");
  return (
    <section className="workspace-welcome" aria-label="Workspace welcome">
      <VscodeIcon name="root-folder" size={40} />
      <h1>Open your investigation workspace</h1>
      <p>
        Keep independent Explorations, stage content and reasoning in private
        local storage.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <label htmlFor="workspace-name">Workspace name</label>
        <VscodeTextfield
          id="workspace-name"
          ref={field}
          label="Workspace name"
          value={title}
          maxlength={120}
          disabled={state.busy}
          onInput={() => {
            key.current = crypto.randomUUID();
            setTitle(field.current?.value ?? "");
          }}
        />
        <VscodeButton
          type="button"
          icon="add"
          disabled={state.busy || !title.trim()}
          onClick={() => {
            void create();
          }}
        >
          Create workspace
        </VscodeButton>
      </form>
      {state.list.workspaces.length ? (
        <div className="workspace-recents">
          <h2>Recent workspaces</h2>
          {state.list.workspaces.map((workspace) => (
            <VscodeButton
              key={workspace.id}
              secondary
              disabled={state.busy || !workspace.available}
              onClick={() =>
                void state.open(workspace.id).then((opened) => {
                  if (opened) onOpened();
                })
              }
            >
              {workspace.title}
              {workspace.available
                ? ""
                : ` · ${workspace.error ?? "Unavailable"}`}
            </VscodeButton>
          ))}
        </div>
      ) : null}
      <small>
        Storage is managed by the app. Importing a file copies it; source files
        are read-only.
      </small>
    </section>
  );
}
