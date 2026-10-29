import VscodeTextfield from "@vscode-elements/react-elements/dist/components/VscodeTextfield.js";
import { useRef, useState } from "react";
import limits from "../../desktop/protocol.json";
import type { PipelineDefinition } from "../desktop/workspace-contract";
import { Select } from "./GraphInputs";
import VscodeButton from "./VscodeButton";
export function PipelineInputs({
  definition,
  disabled,
  onChange,
}: {
  definition: PipelineDefinition;
  disabled: boolean;
  onChange: (d: PipelineDefinition) => void;
}) {
  const [name, setName] = useState("");
  const [kind, setKind] = useState("text");
  const field = useRef<HTMLElementTagNameMap["vscode-textfield"]>(null);
  const valid =
    /^[a-z][a-z0-9_]{0,31}$/.test(name) &&
    !definition.inputs.some((s) => s.name === name);
  return (
    <fieldset className="pipeline-inputs">
      <legend>Pipeline inputs</legend>
      <ul>
        {definition.inputs.map((s) => (
          <li key={s.name}>
            <code>{s.name}</code> · {s.semanticType}
            {s.name === "source" ? (
              <small> · Trail source</small>
            ) : (
              <VscodeButton
                secondary
                disabled={
                  disabled ||
                  definition.nodes.some((n) =>
                    Object.values(n.inputs).some(
                      (i) => i.kind === "input" && i.name === s.name,
                    ),
                  )
                }
                onClick={() =>
                  onChange({
                    ...definition,
                    inputs: definition.inputs.filter((i) => i.name !== s.name),
                  })
                }
              >
                Remove {s.name}
              </VscodeButton>
            )}
          </li>
        ))}
      </ul>
      <label htmlFor="input-name">Input name</label>
      <VscodeTextfield
        id="input-name"
        label="Input name"
        ref={field}
        value={name}
        disabled={disabled}
        onInput={() => setName(field.current?.value ?? "")}
      />
      <Select
        id="input-type"
        label="Input type"
        value={kind}
        disabled={disabled}
        options={[
          { value: "text", label: "Text" },
          { value: "image.samples", label: "Image" },
        ]}
        onChange={setKind}
      />
      <VscodeButton
        secondary
        disabled={
          disabled ||
          !valid ||
          definition.inputs.length >= limits.pipeline_input_limit
        }
        onClick={() => {
          onChange({
            ...definition,
            inputs: [...definition.inputs, { name, semanticType: kind }],
          });
          setName("");
        }}
      >
        Add input
      </VscodeButton>
      <small>
        Up to {limits.pipeline_input_limit} named inputs. Use lowercase letters,
        numbers and underscores, starting with a letter. Disconnect an input
        before removing it.
      </small>
    </fieldset>
  );
}
