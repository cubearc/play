import type { Operation, RunResult } from "./contract.ts";
import type {
  AssetRead,
  AssetRun,
  ImageImport,
  MutationResult,
  Responses,
  WorkspaceRequest,
} from "./workspace-contract.ts";

export interface PlayBridge {
  catalog: () => Promise<Operation[]>;
  workspace: <K extends WorkspaceRequest["kind"]>(
    value: Extract<WorkspaceRequest, { kind: K }>,
  ) => Promise<Responses[K]>;
  importImage: (value: ImageImport) => Promise<MutationResult | null>;
  image: (value: AssetRead) => Promise<string>;
  runAsset: (value: AssetRun) => Promise<RunResult>;
  dirty: (value: boolean) => Promise<void>;
  cancel: () => Promise<void>;
}
declare global {
  interface Window {
    play: PlayBridge;
  }
}
