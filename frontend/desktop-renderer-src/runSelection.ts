import type { Operation } from "../desktop/contract";
import type { RunRequest, TrailView } from "../desktop/workspace-contract";
export function runSelection(
  trail: TrailView,
  operation: Operation,
  retain: boolean,
  target = trail.pipeline?.outputNodeId,
): RunRequest | null {
  if (!trail.pipeline || !target || trail.archived) return null;
  return {
    context: {
      workspace_id: trail.workspaceId,
      exploration_id: trail.explorationId,
      stage_id: trail.stageId,
    },
    key: crypto.randomUUID(),
    trail_id: trail.id,
    trail_revision: trail.revision,
    target_node_id: target,
    pipeline_id: trail.pipeline.id,
    pipeline_revision: trail.pipeline.revision,
    source_asset_id: trail.source.id,
    source_revision: trail.source.revision,
    operation: operation.operation,
    operation_version: operation.operation_version,
    retain_output: retain,
  };
}
