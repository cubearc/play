import {
  type AssetRead,
  type AssetView,
  assetRead,
  type CancelRun,
  cancelRun,
  type PipelineView,
  type RunDetail,
  type RunRequest,
  runRequest,
} from "./workspace-contract.ts";
export type CliIntent =
  | ({ kind: "asset" } & AssetRead)
  | {
      kind: "pipeline";
      context: AssetRead["context"];
      pipeline_id: string;
      revision: number;
    }
  | { kind: "start"; request: RunRequest }
  | ({ kind: "status" } & CancelRun)
  | ({ kind: "cancel" } & CancelRun);
export interface CliResponses {
  asset: AssetView;
  pipeline: PipelineView;
  start: RunDetail;
  status: RunDetail;
  cancel: RunDetail;
}
export function cliIntent(value: unknown): CliIntent {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("invalid_command");
  const { kind, ...rest } = value as Record<string, unknown>;
  switch (kind) {
    case "asset":
      return { kind, ...assetRead(rest) };
    case "pipeline": {
      if (Object.keys(rest).sort().join() !== "context,pipeline_id,revision")
        throw new Error("invalid_command");
      const { pipeline_id, ...fields } = rest;
      const asset = assetRead({ ...fields, asset_id: pipeline_id });
      return {
        kind,
        context: asset.context,
        pipeline_id: asset.asset_id,
        revision: asset.revision,
      };
    }
    case "start":
      if (Object.keys(rest).join() !== "request")
        throw new Error("invalid_command");
      return { kind, request: runRequest(rest.request) };
    case "status":
    case "cancel":
      return { kind, ...cancelRun(rest) };
    default:
      throw new Error("invalid_command");
  }
}
