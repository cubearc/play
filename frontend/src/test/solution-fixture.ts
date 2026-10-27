import type { SolutionState } from "../solution-api";
import { id, outputPort, typedWire } from "./lab-fixture";
export const solutionId = "6d434f94-89f3-4fb8-a7d8-c345a0000002";
export function solutionWire(revision = 1): Record<string, unknown> {
  const wire = typedWire();
  return {
    schema_version: 1,
    solution: {
      schema_version: 1,
      id: solutionId,
      revision,
      title: "Digit finding",
      explanation: "Preserve the original digit result.",
      local_status: "finding",
      privacy: "private",
      verification: "unverified",
      final_output: { result_id: id, port: "text", field: null },
      final_metadata: {
        representation: "text",
        semantic_type: "text",
        sha256: "0".repeat(64),
        byte_size: 2,
      },
      evidence_path: [
        {
          ...wire.result,
          workspace_id: solutionId,
          provenance: {
            operation: "cipher.rot5",
            core_version: "0.18.0",
            parameters: [],
            inputs: [],
            created_ms: 1,
          },
        },
      ],
      configuration_scope: "executed_path",
      configurations: [],
      executions: [],
      candidate: null,
      replay_at_save: {
        schema_version: 1,
        artifact_id: id,
        status: "gaps",
        checked_artifacts: 1,
        checked_bytes: 2,
        gaps: [
          {
            artifact_id: id,
            input_port: "text",
            code: "input_unavailable",
            message: "The literal input was not retained.",
          },
        ],
      },
      derived_from_revision: revision > 1 ? revision - 1 : null,
    },
    latest_revision: revision,
    history: Array.from({ length: revision }, (_, i) => ({
      revision: revision - i,
      title: "Digit finding",
      local_status: "finding",
      created_ms: revision - i,
    })),
    replay_current: {
      schema_version: 1,
      artifact_id: id,
      status: "gaps",
      checked_artifacts: 1,
      checked_bytes: 2,
      gaps: [
        {
          artifact_id: id,
          input_port: "text",
          code: "input_unavailable",
          message: "The literal input was not retained.",
        },
      ],
    },
  };
}
export function finalWire(revision = 1) {
  return {
    schema_version: 1,
    solution_id: solutionId,
    revision,
    metadata: {
      representation: "text",
      semantic_type: "text",
      sha256: "0".repeat(64),
      byte_size: 2,
    },
    value: "A6",
  };
}
export const retained = {
  id,
  representation: "structured",
  semantic_type: "operation.outputs",
  metadata: { schema_version: 2, ports: [outputPort] },
  provenance: { operation: "cipher.rot5", created_ms: 1 },
};
export function solutionState(revision = 1): SolutionState {
  const wire = solutionWire(revision);
  return {
    solution: wire.solution,
    latest_revision: wire.latest_revision,
    history: wire.history,
    replay_current: wire.replay_current,
  } as SolutionState;
}
