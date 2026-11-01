import type { WorkspaceView } from "../desktop/workspace-contract";

const uuid = /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/;
const key = (workspace: string) => `play.location.${workspace}`;
/** Browser session storage belongs to this window, never to another renderer. */
export function locationFor(
  workspace: WorkspaceView,
): { explorationId: string; stageId: string | null } | null {
  try {
    const text = sessionStorage.getItem(key(workspace.id));
    if (!text || text.length > 256) return null;
    const value = JSON.parse(text);
    if (
      !value ||
      Object.keys(value).sort().join() !== "explorationId,stageId" ||
      typeof value.explorationId !== "string" ||
      !workspace.explorations.some((e) => e.id === value.explorationId) ||
      (value.stageId !== null &&
        (typeof value.stageId !== "string" || !uuid.test(value.stageId)))
    )
      return null;
    return value;
  } catch {
    return null;
  }
}
export function saveLocation(
  workspace: string,
  explorationId: string,
  stageId: string,
): void {
  try {
    sessionStorage.setItem(
      key(workspace),
      JSON.stringify({ explorationId, stageId }),
    );
  } catch {
    /* Storage denial must not prevent navigation. */
  }
}
