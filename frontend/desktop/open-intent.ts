/** An OS link selects existing local content; it never grants paths or execution. */
export interface OpenIntent {
  workspace_id: string;
  exploration_id: string | null;
  stage_id: string | null;
}
const uuid = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
export function openIntent(input: unknown): OpenIntent {
  if (typeof input !== "string" || input.length > 512)
    throw new Error("invalid_open_intent");
  const matched = new RegExp(
    `^cube-play-open://workspace/(${uuid})(?:\\?exploration=(${uuid})(?:&stage=(${uuid}))?)?$`,
  ).exec(input);
  if (!matched?.[1]) throw new Error("invalid_open_intent");
  return {
    workspace_id: matched[1],
    exploration_id: matched[2] ?? null,
    stage_id: matched[3] ?? null,
  };
}

export function workspaceLink(location: OpenIntent): string {
  const link = `cube-play-open://workspace/${location.workspace_id}${location.exploration_id ? `?exploration=${location.exploration_id}${location.stage_id ? `&stage=${location.stage_id}` : ""}` : ""}`;
  openIntent(link);
  return link;
}
