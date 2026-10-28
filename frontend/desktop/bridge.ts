import type { Operation } from "./contract.ts";
import type {
  AssetRead,
  CancelRun,
  ImageImport,
  MutationResult,
  Responses,
  RunDetail,
  RunRequest,
  WorkspaceRequest,
} from "./workspace-contract.ts";

export interface PlayBridge {
  catalog: () => Promise<Operation[]>;
  workspace: <K extends WorkspaceRequest["kind"]>(
    value: Extract<WorkspaceRequest, { kind: K }>,
  ) => Promise<Responses[K]>;
  importImage: (value: ImageImport) => Promise<MutationResult | null>;
  image: (value: AssetRead) => Promise<string>;
  startRun: (value: RunRequest) => Promise<RunDetail>;
  cancelRun: (value: CancelRun) => Promise<RunDetail>;
  dirty: (value: boolean) => Promise<void>;
}
declare global {
  interface Window {
    play: PlayBridge;
  }
}
