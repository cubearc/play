import type { CliIntent, CliResponses } from "./cli-contract.ts";
import type { Operation } from "./contract.ts";
import type {
  AssetRead,
  CancelRun,
  FileImport,
  ImageImport,
  MutationResult,
  Responses,
  RunDetail,
  RunRequest,
  WorkspaceRequest,
} from "./workspace-contract.ts";

export interface PlayBridge {
  terminal: (
    value: import("./terminal-contract.ts").TerminalIntent,
  ) => Promise<import("./terminal-contract.ts").TerminalView>;
  cli: <K extends CliIntent["kind"]>(
    value: Extract<CliIntent, { kind: K }>,
  ) => Promise<CliResponses[K]>;
  catalog: () => Promise<Operation[]>;
  workspace: <K extends WorkspaceRequest["kind"]>(
    value: Extract<WorkspaceRequest, { kind: K }>,
  ) => Promise<Responses[K]>;
  importImage: (value: ImageImport) => Promise<MutationResult | null>;
  importFile: (value: FileImport) => Promise<MutationResult | null>;
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
