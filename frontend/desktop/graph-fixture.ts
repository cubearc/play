import { randomUUID } from "node:crypto";
import type { Operation } from "./contract.js";
import type { PipelineDefinition } from "./workspace-contract.js";
export function singleGraph(
  operation: Pick<
    Operation,
    "operation" | "operation_version" | "parameters" | "input_type" | "contract"
  >,
): PipelineDefinition {
  const id = randomUUID();
  return {
    formatVersion: 4,
    inputs: [{ name: "source", semanticType: operation.input_type }],
    nodes: [
      {
        id,
        operation: operation.operation,
        operationVersion: operation.operation_version,
        parameters: operation.parameters,
        position: { x: 80, y: 120 },
        inputs: Object.fromEntries(
          operation.contract.inputs.map((p) => [
            p.name,
            p.semantic_type === operation.input_type
              ? { kind: "input", name: "source" }
              : { kind: "literal", value: [] },
          ]),
        ),
      },
    ],
    output: id,
  };
}
