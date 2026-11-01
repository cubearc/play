import type { CliIntent, CliResponses } from "./cli-contract.ts";
import type { Operation } from "./contract.ts";
import type {
  CancelRun,
  FileImport,
  ImageImport,
  ImageRead,
  MutationResult,
  Responses,
  RunDetail,
  RunRequest,
  WorkspaceRequest,
} from "./workspace-contract.ts";

export interface PlayBridge {
  location: () => Promise<import("./open-intent.ts").OpenIntent | null>;
  attachment: (
    value: import("./attachment-contract.ts").AttachmentIntent,
  ) => Promise<import("./attachment-contract.ts").AttachmentView | null>;
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
  importChallenge: (
    value: import("./workspace-contract.ts").ChallengeImport,
  ) => Promise<import("./workspace-contract.ts").ImportedChallenge | null>;
  importImage: (value: ImageImport) => Promise<MutationResult | null>;
  importFile: (value: FileImport) => Promise<MutationResult | null>;
  image: (value: ImageRead) => Promise<string>;
  startRun: (value: RunRequest) => Promise<RunDetail>;
  cancelRun: (value: CancelRun) => Promise<RunDetail>;
  dirty: (value: boolean) => Promise<void>;
}
declare global {
  interface Window {
    play: PlayBridge;
  }
}
