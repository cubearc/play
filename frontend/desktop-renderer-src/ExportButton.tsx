import { useEffect, useRef, useState } from "react";
import type {
  ExportIntent,
  ExportReceipt,
  ExportSource,
} from "../desktop/export-contract";
import type { Context } from "../desktop/workspace-contract";
import { Select } from "./GraphInputs";
import VscodeButton from "./VscodeButton";

function failure(error: unknown): string {
  const code = error instanceof Error ? error.message : "";
  if (code.includes("export_target_exists"))
    return "This file already exists. Choose a new filename.";
  if (code.includes("export_target_protected"))
    return "Choose a folder outside the application and its private storage.";
  if (code.includes("stale_context"))
    return "The selection changed. Close this dialog and export the selected evidence again.";
  if (code.includes("output_limit"))
    return "This export exceeds the supported size limit.";
  return "The export could not be saved. Check the destination and try again.";
}
type Props = { context: Context; source: ExportSource; label: string };
export function ExportButton(props: Props) {
  return (
    <ExportAction
      key={JSON.stringify([props.context, props.source])}
      {...props}
    />
  );
}
function ExportAction({
  context,
  source,
  label,
}: {
  context: Context;
  source: ExportSource;
  label: string;
}) {
  const [intent, setIntent] = useState<ExportIntent>();
  const [receipt, setReceipt] = useState<ExportReceipt>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const button = useRef<HTMLElementTagNameMap["vscode-button"]>(null);
  const close = () => {
    setIntent(undefined);
    setError("");
    button.current?.focus();
  };
  return (
    <>
      <VscodeButton
        ref={button}
        secondary
        icon="export"
        onClick={() => {
          setReceipt(undefined);
          setError("");
          setIntent({
            context: { ...context },
            source: { ...source },
            format: source.kind === "solution" ? "document" : "data",
          });
        }}
      >
        Export {label}
      </VscodeButton>
      {receipt ? (
        <p role="status">
          Exported {receipt.fileName} · {receipt.size.toLocaleString()} bytes
        </p>
      ) : null}
      {intent ? (
        <ExportDialog
          label={label}
          intent={intent}
          setIntent={setIntent}
          busy={busy}
          error={error}
          onClose={close}
          onSave={async () => {
            if (busy) return;
            setBusy(true);
            setError("");
            try {
              const saved = await window.play.exportSelection(intent);
              if (saved) {
                setReceipt(saved);
                close();
              }
            } catch (error) {
              setError(failure(error));
            } finally {
              setBusy(false);
            }
          }}
        />
      ) : null}
    </>
  );
}
function ExportDialog({
  label,
  intent,
  setIntent,
  busy,
  error,
  onClose,
  onSave,
}: {
  label: string;
  intent: ExportIntent;
  setIntent: (value: ExportIntent) => void;
  busy: boolean;
  error: string;
  onClose: () => void;
  onSave: () => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const revision =
    intent.source.kind === "run_output" ? undefined : intent.source.revision;
  return (
    <dialog
      ref={dialog}
      className="content-dialog"
      aria-label={`Export ${label}`}
      onCancel={(event) => {
        if (busy) event.preventDefault();
        else onClose();
      }}
    >
      <h2>Export {label}</h2>
      <p>
        {revision ? `Selected revision ${revision}. ` : "Selected Run output. "}
        Choose what to include, then choose a new file in the native save
        dialog.
      </p>
      {intent.source.kind === "solution" ? (
        <p>
          Includes this revision’s explanation, selected evidence, saved Notes,
          Pipeline parameters and literal inputs. These can contain private
          values, including keys. Exporting does not submit or verify the
          Solution. This is not a Workspace backup.
        </p>
      ) : (
        <Select
          id="export-format"
          label="File contents"
          value={intent.format}
          disabled={busy}
          options={[
            { value: "data", label: "Selected data" },
            { value: "document", label: "Data and evidence references (JSON)" },
          ]}
          onChange={(format) =>
            setIntent({
              ...intent,
              format: format === "document" ? "document" : "data",
            })
          }
        />
      )}
      {error ? <p role="alert">{error}</p> : null}
      <div className="dialog-actions">
        <VscodeButton secondary disabled={busy} onClick={onClose}>
          Cancel
        </VscodeButton>
        <VscodeButton disabled={busy} onClick={() => void onSave()}>
          {busy ? "Saving…" : "Choose destination"}
        </VscodeButton>
      </div>
    </dialog>
  );
}
