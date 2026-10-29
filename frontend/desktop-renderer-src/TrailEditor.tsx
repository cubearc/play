import VscodeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import VscodeOption from "@vscode-elements/react-elements/dist/components/VscodeOption.js";
import VscodeSingleSelect from "@vscode-elements/react-elements/dist/components/VscodeSingleSelect.js";
import VscodeTextarea from "@vscode-elements/react-elements/dist/components/VscodeTextarea.js";
import { useEffect, useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type {
  AssetView,
  PipelineRef,
  PipelineView,
  TrailView,
} from "../desktop/workspace-contract";

import {
  type Bindings,
  BindingsEditor,
  bindingRefs,
  completeBindings,
  selectedBindings,
} from "./BindingsEditor";

export function TrailEditor({
  trail,
  pipelines,
  assets,
  error,
  onSave,
  onClose,
  onDirty,
}: {
  trail: TrailView;
  pipelines: PipelineView[];
  assets: AssetView[];
  error: string;
  onSave: (
    hypothesis: string,
    pipeline: PipelineRef | null,
    key: string,
    bindings: Bindings,
  ) => Promise<boolean>;
  onClose: () => void;
  onDirty: (dirty: boolean) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const hypothesisField =
    useRef<HTMLElementTagNameMap["vscode-textarea"]>(null);
  const pipelineField =
    useRef<HTMLElementTagNameMap["vscode-single-select"]>(null);
  const revisionField =
    useRef<HTMLElementTagNameMap["vscode-single-select"]>(null);
  const [hypothesis, setHypothesis] = useState(trail.hypothesis);
  const [selected, setSelected] = useState(trail.pipeline?.id ?? "");
  const [revision, setRevision] = useState(trail.pipeline?.revision ?? 1);
  const [saving, setSaving] = useState(false);
  const [bindings, setBindings] = useState<Bindings>(() => bindingRefs(trail));
  const [loaded, setLoaded] = useState<PipelineView | null>(trail.pipeline);
  const [loadError, setLoadError] = useState("");
  useEffect(() => {
    let live = true;
    setLoadError("");
    if (!selected) {
      setLoaded(null);
      return;
    }
    void window.play
      .workspace({
        kind: "get_pipeline",
        workspace_id: trail.workspaceId,
        pipeline_id: selected,
        revision,
      })
      .then((p) => {
        if (live) setLoaded(p);
      })
      .catch(() => {
        if (live)
          setLoadError(
            "This revision could not be loaded. Reopen the editor to try again.",
          );
      });
    return () => {
      live = false;
    };
  }, [selected, revision, trail.workspaceId]);
  const ready =
    !selected ||
    (loaded?.id === selected && loaded.revision === revision && !loadError);
  const slots = selected && ready && loaded ? loaded.definition.inputs : [];
  const valid = ready && completeBindings(slots, bindings);
  const key = useRef(crypto.randomUUID());
  const input =
    trail.source.content.kind === "image" ? "image.samples" : "text";
  const choices = pipelines.filter(
    (p) => p.inputType === input || p.id === trail.pipelineId,
  );
  const pipeline = choices.find((p) => p.id === selected);
  const length = new TextEncoder().encode(hypothesis).byteLength;
  const dirty =
    JSON.stringify(bindings) !== JSON.stringify(bindingRefs(trail)) ||
    hypothesis !== trail.hypothesis ||
    selected !== (trail.pipeline?.id ?? "") ||
    revision !== (trail.pipeline?.revision ?? 1);
  useEffect(() => {
    dialog.current?.showModal();
    hypothesisField.current?.focus();
  }, []);
  useEffect(() => onDirty(dirty), [dirty, onDirty]);
  const change = () => {
    key.current = crypto.randomUUID();
  };
  const save = async () => {
    if (saving || !valid || length > limits.hypothesis_bytes) return;
    setSaving(true);
    try {
      if (
        await onSave(
          hypothesis,
          pipeline ? { id: pipeline.id, revision } : null,
          key.current,
          selectedBindings(slots, bindings),
        )
      )
        onClose();
    } finally {
      setSaving(false);
    }
  };
  return (
    <dialog
      ref={dialog}
      className="content-dialog text-editor"
      aria-label="Edit Trail"
      onCancel={(e) => {
        e.preventDefault();
        if (!saving) onClose();
      }}
    >
      <h2>Edit Trail</h2>
      <p>
        {trail.title} · r{trail.revision}
      </p>
      {error ? <p role="alert">{error}</p> : null}
      <label htmlFor="trail-hypothesis">Hypothesis</label>
      <VscodeTextarea
        id="trail-hypothesis"
        label="Hypothesis"
        ref={hypothesisField}
        value={hypothesis}
        rows={4}
        disabled={saving}
        onInput={() => {
          change();
          setHypothesis(hypothesisField.current?.value ?? "");
        }}
      />
      <small>
        {length} / {limits.hypothesis_bytes} UTF-8 bytes. This hypothesis
        belongs to this Trail.
      </small>
      <label htmlFor="trail-pipeline">Saved Pipeline</label>
      <VscodeSingleSelect
        id="trail-pipeline"
        label="Saved Pipeline"
        ref={pipelineField}
        value={selected}
        disabled={saving}
        onChange={() => {
          change();
          const id = pipelineField.current?.value ?? "";
          setSelected(id);
          setRevision(choices.find((p) => p.id === id)?.revision ?? 1);
        }}
      >
        <VscodeOption value="">No Pipeline</VscodeOption>
        {choices.map((p) => (
          <VscodeOption key={p.id} value={p.id}>
            {p.title}
          </VscodeOption>
        ))}
      </VscodeSingleSelect>
      {pipeline ? (
        <>
          <label htmlFor="trail-revision">Pipeline revision</label>
          <VscodeSingleSelect
            id="trail-revision"
            label="Pipeline revision"
            ref={revisionField}
            value={String(revision)}
            disabled={saving}
            onChange={() => {
              change();
              setRevision(Number(revisionField.current?.value ?? 1));
            }}
          >
            {pipeline.revisions.map((r) => (
              <VscodeOption key={r} value={String(r)}>
                Revision {r}
              </VscodeOption>
            ))}
          </VscodeSingleSelect>
        </>
      ) : null}
      {loadError ? (
        <p role="alert">{loadError}</p>
      ) : !ready ? (
        <p role="status">Loading Pipeline revision…</p>
      ) : null}
      {ready ? (
        <BindingsEditor
          inputs={slots}
          assets={assets}
          pinned={trail.bindings}
          bindings={bindings}
          disabled={saving}
          onChange={(next) => {
            change();
            setBindings(next);
          }}
        />
      ) : null}
      <p>
        The selected revision stays pinned. Other Trails and accepted Runs keep
        their own inputs and evidence.
      </p>
      <div className="dialog-actions">
        <VscodeButton secondary disabled={saving} onClick={onClose}>
          Cancel
        </VscodeButton>
        <VscodeButton
          icon="save"
          disabled={saving || !valid || length > limits.hypothesis_bytes}
          onClick={() => void save()}
        >
          Save Trail
        </VscodeButton>
      </div>
    </dialog>
  );
}
