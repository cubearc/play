import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeOption from "@vscode-elements/react-elements/dist/components/VscodeOption.js";
import VscodeSingleSelect from "@vscode-elements/react-elements/dist/components/VscodeSingleSelect.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { Operation } from "../desktop/contract";
import type { PipelineView } from "../desktop/workspace-contract";
export type PipelineMode = "create" | "revise" | "fork";
export interface PipelineDraft {
  title: string;
  operation: string;
  operationVersion: number;
  parameters: Record<string, unknown>;
  key: string;
  bindingKey: string;
}
export function PipelineEditor({
  mode,
  pipeline,
  operations,
  error,
  onSave,
  onClose,
  onDirty,
}: {
  mode: PipelineMode;
  pipeline: PipelineView | null;
  operations: Operation[];
  error: string;
  onSave: (draft: PipelineDraft) => Promise<boolean>;
  onClose: () => void;
  onDirty: (dirty: boolean) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleField = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const operationField =
    useRef<HTMLElementTagNameMap["vscode-single-select"]>(null);
  const parametersField =
    useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const [title, setTitle] = useState(
    mode === "fork"
      ? `${pipeline?.title} (copy)`
      : mode === "revise"
        ? (pipeline?.title ?? "")
        : "New Pipeline",
  );
  const [operation, setOperation] = useState(
    pipeline?.initialOperation ?? operations[0]?.operation ?? "",
  );
  const initialParameters = JSON.stringify(
    pipeline?.parameters ?? operations[0]?.parameters ?? {},
    null,
    2,
  );
  const [parameters, setParameters] = useState(initialParameters);
  const [saving, setSaving] = useState(false);
  const key = useRef(crypto.randomUUID());
  const bindingKey = useRef(crypto.randomUUID());
  const selected = operations.find((o) => o.operation === operation);
  let parsed: Record<string, unknown> | null = null;
  try {
    const value: unknown = JSON.parse(parameters);
    if (value && typeof value === "object" && !Array.isArray(value))
      parsed = value as Record<string, unknown>;
  } catch {
    /* Keep the draft editable until it is valid JSON. */
  }
  const valid = Boolean(
    selected &&
      parsed &&
      new TextEncoder().encode(JSON.stringify(parsed)).byteLength <=
        limits.pipeline_parameter_bytes &&
      title.trim() &&
      new TextEncoder().encode(title).byteLength <= 120,
  );
  const changed =
    operation !== pipeline?.initialOperation ||
    parameters !== initialParameters ||
    mode !== "revise";
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  useEffect(() => onDirty(changed), [changed, onDirty]);
  const change = () => {
    key.current = crypto.randomUUID();
    bindingKey.current = crypto.randomUUID();
  };
  const save = async () => {
    if (!valid || saving || !selected || !parsed) return;
    setSaving(true);
    try {
      if (
        await onSave({
          title,
          operation,
          operationVersion: selected.operation_version,
          parameters: parsed,
          key: key.current,
          bindingKey: bindingKey.current,
        })
      )
        onClose();
    } finally {
      setSaving(false);
    }
  };
  const heading =
    mode === "create"
      ? "New Pipeline"
      : mode === "fork"
        ? "Fork Pipeline"
        : "Edit Pipeline";
  return (
    <dialog
      ref={dialog}
      className="content-dialog text-editor"
      aria-label={heading}
      onCancel={(e) => {
        e.preventDefault();
        if (!saving) onClose();
      }}
    >
      <h2>{heading}</h2>
      {error ? <p role="alert">{error}</p> : null}
      {mode === "revise" ? (
        <p>
          {pipeline?.title} · based on revision {pipeline?.revision}
        </p>
      ) : (
        <>
          <label htmlFor="pipeline-title">Pipeline name</label>
          <VscodeTextfield
            id="pipeline-title"
            label="Pipeline name"
            ref={titleField}
            value={title}
            disabled={saving}
            onInput={() => {
              change();
              setTitle(titleField.current?.value ?? "");
            }}
          />
        </>
      )}
      <label htmlFor="pipeline-operation">Operation</label>
      <VscodeSingleSelect
        id="pipeline-operation"
        label="Operation"
        ref={operationField}
        value={operation}
        disabled={saving || mode === "fork"}
        onChange={() => {
          change();
          const id = operationField.current?.value ?? "";
          setOperation(id);
          setParameters(
            JSON.stringify(
              operations.find((o) => o.operation === id)?.parameters ?? {},
              null,
              2,
            ),
          );
        }}
      >
        {operations.map((o) => (
          <VscodeOption key={o.operation} value={o.operation}>
            {o.name}
          </VscodeOption>
        ))}
      </VscodeSingleSelect>
      <p>{selected?.description}</p>
      <label htmlFor="pipeline-parameters">Parameters (JSON)</label>
      <VscodeTextarea
        id="pipeline-parameters"
        label="Parameters (JSON)"
        ref={parametersField}
        value={parameters}
        rows={6}
        disabled={saving || mode === "fork"}
        onInput={() => {
          change();
          setParameters(parametersField.current?.value ?? "");
        }}
      />
      {!parsed ? (
        <p role="alert">Enter a JSON object for the parameters.</p>
      ) : null}
      <small>
        {mode === "fork"
          ? "The fork preserves this exact definition and records its source revision."
          : "Saving freezes the operation, version and parameters. Existing Runs remain immutable."}{" "}
        Only this Trail will select the saved revision.
      </small>
      <div className="dialog-actions">
        <VscodeButton secondary disabled={saving} onClick={onClose}>
          Cancel
        </VscodeButton>
        <VscodeButton
          icon="save"
          disabled={saving || !valid}
          onClick={() => void save()}
        >
          {mode === "create"
            ? "Create Pipeline"
            : mode === "fork"
              ? "Fork Pipeline"
              : "Save revision"}
        </VscodeButton>
      </div>
    </dialog>
  );
}
