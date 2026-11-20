import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { Operation } from "../../desktop/contract";
import type {
  GraphNode,
  PipelineDefinition,
} from "../../desktop/workspace-contract";
import { GraphInputs, initialInputs, secretInputsBound } from "../GraphInputs";

it("requires an explicit local Asset slot for a secret port", () => {
  const operation: Operation = {
    operation: "hash.siphash24",
    name: "SipHash-2-4",
    availability: { state: "available" },
    operation_version: 1,
    description: "Compute a keyed digest",
    input_type: "bytes",
    parameters: {},
    contract: {
      inputs: [
        {
          name: "key",
          semantic_type: "bytes",
          schema: { "x-cube-secret": true },
        },
      ],
      outputs: [{ name: "output", semantic_type: "bytes", schema: {} }],
      parameters: {},
    },
    execution: {
      effects: "pure",
      inputs: { key: "bytes" },
      outputs: { output: "bytes" },
    },
  };
  const inputs = [
    { name: "source", semanticType: "text" },
    { name: "key_asset", semanticType: "bytes" },
  ];
  const node: GraphNode = {
    id: "siphash",
    operation: operation.operation,
    operationVersion: 1,
    position: { x: 0, y: 0 },
    parameters: {},
    inputs: initialInputs(operation, inputs),
  };
  expect(node.inputs.key).toEqual({ kind: "input", name: "" });
  expect(secretInputsBound(operation, node, inputs)).toBe(false);
  node.inputs.key = { kind: "input", name: "source" };
  expect(secretInputsBound(operation, node, inputs)).toBe(false);
  node.inputs.key = { kind: "input", name: "key_asset" };
  expect(secretInputsBound(operation, node, inputs)).toBe(true);

  const definition: PipelineDefinition = {
    formatVersion: 4,
    inputs,
    nodes: [{ ...node, id: "another-node" }, node],
    output: node.id,
  };
  const markup = renderToStaticMarkup(
    <GraphInputs
      node={node}
      definition={definition}
      operations={[operation]}
      disabled={false}
      onChange={vi.fn()}
      onInvalidChange={vi.fn()}
    />,
  );
  expect(markup).toContain("key_asset · bytes");
  expect(markup).not.toContain("Literal");
  expect(markup).not.toContain("SipHash-2-4 · output");
});
