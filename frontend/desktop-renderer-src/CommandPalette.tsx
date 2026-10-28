import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useEffect, useRef, useState } from "react";

export interface WorkbenchCommand {
  id: string;
  label: string;
  shortcut: string;
  disabled?: boolean;
  execute: () => void;
}

export function CommandPalette({
  commands,
  onClose,
}: {
  commands: WorkbenchCommand[];
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const input = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const [query, setQuery] = useState("");
  const [selection, setSelection] = useState(0);
  const matches = commands.filter((command) =>
    command.label.toLowerCase().includes(query.toLowerCase()),
  );
  const execute = (command: WorkbenchCommand | undefined) => {
    if (!command || command.disabled) return;
    onClose();
    command.execute();
  };
  useEffect(() => {
    dialog.current?.showModal();
    void input.current?.updateComplete.then(() => input.current?.focus());
  }, []);

  return (
    <dialog
      ref={dialog}
      className="command-palette"
      aria-label="Command Palette"
      onCancel={onClose}
      onKeyDown={(event) => {
        if (event.key === "ArrowDown" || event.key === "ArrowUp") {
          event.preventDefault();
          setSelection((index) =>
            Math.max(
              0,
              Math.min(
                matches.length - 1,
                index + (event.key === "ArrowDown" ? 1 : -1),
              ),
            ),
          );
        } else if (event.key === "Enter") {
          event.preventDefault();
          execute(matches[selection]);
        }
      }}
    >
      <VscodeTextfield
        ref={input}
        label="Command"
        placeholder="Type a command…"
        value={query}
        onInput={() => {
          setQuery(input.current?.value ?? "");
          setSelection(0);
        }}
      />
      <div className="command-results">
        {matches.map((command, index) => (
          <button
            key={command.id}
            type="button"
            className={`command-item ${index === selection ? "selected" : ""}`}
            disabled={command.disabled}
            onClick={() => execute(command)}
          >
            <span>{command.label}</span>
            <kbd>{command.shortcut}</kbd>
          </button>
        ))}
        {matches.length === 0 ? (
          <p className="sidebar-note">No matching commands.</p>
        ) : null}
      </div>
    </dialog>
  );
}
