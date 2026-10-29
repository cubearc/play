import type {
  AssetRevisionRef,
  AssetView,
  PipelineDefinition,
  TrailView,
} from "../desktop/workspace-contract";
import { Select } from "./GraphInputs";
export type Bindings = Record<string, AssetRevisionRef>;
export function bindingRefs(trail: TrailView): Bindings {
  return Object.fromEntries(
    Object.entries(trail.bindings).map(([name, a]) => [
      name,
      { asset_id: a.id, revision: a.revision },
    ]),
  );
}
export function selectedBindings(
  inputs: PipelineDefinition["inputs"],
  bindings: Bindings,
): Bindings {
  return Object.fromEntries(
    inputs.slice(1).flatMap((s) => {
      const ref = bindings[s.name];
      return ref ? [[s.name, ref]] : [];
    }),
  );
}
export function completeBindings(
  inputs: PipelineDefinition["inputs"],
  bindings: Bindings,
): boolean {
  return inputs.slice(1).every((s) => bindings[s.name]);
}
export function BindingsEditor({
  inputs,
  assets,
  pinned,
  bindings,
  disabled,
  onChange,
}: {
  inputs: PipelineDefinition["inputs"];
  assets: AssetView[];
  pinned: Record<string, AssetView>;
  bindings: Bindings;
  disabled: boolean;
  onChange: (value: Bindings) => void;
}) {
  if (inputs.length < 2) return null;
  return (
    <fieldset className="input-bindings">
      <legend>Trail inputs</legend>
      <p>
        Choose assets from this Stage. Each input keeps the selected revision
        until you rebind it.
      </p>
      {inputs.slice(1).map((slot) => {
        const candidates = assets.filter((a) =>
          slot.semanticType === "image.samples"
            ? a.content.kind === "image"
            : ["text", "page"].includes(a.content.kind),
        );
        const old = pinned[slot.name];
        const current = old && candidates.find((a) => a.id === old.id);
        const options = [...candidates];
        if (old && current && current.revision !== old.revision)
          options.push(old);
        const ref = bindings[slot.name];
        return (
          <Select
            key={slot.name}
            id={`binding-${slot.name}`}
            label={`Asset for ${slot.name}`}
            disabled={disabled}
            value={ref ? `${ref.asset_id}:${ref.revision}` : ""}
            options={[
              { value: "", label: "Choose an asset" },
              ...options.map((a) => ({
                value: `${a.id}:${a.revision}`,
                label: `${a.title} · r${a.revision}${current?.id === a.id && current.revision !== a.revision ? ` (pinned; current r${current.revision})` : ""}`,
              })),
            ]}
            onChange={(value) => {
              const asset = options.find(
                (a) => `${a.id}:${a.revision}` === value,
              );
              const next = { ...bindings };
              if (asset)
                next[slot.name] = {
                  asset_id: asset.id,
                  revision: asset.revision,
                };
              else delete next[slot.name];
              onChange(next);
            }}
          />
        );
      })}
    </fieldset>
  );
}
