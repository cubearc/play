//! Privileged export coordination; the renderer supplies references, never bytes or paths.
import { basename } from "node:path";
import { contract } from "./contract.ts";
import type { ExportIntent, ExportReceipt } from "./export-contract.ts";
import {
  type ExportChunk,
  type ExportMetadata,
  writeExport,
} from "./export-writer.ts";
import { isIdentity } from "./workspace-contract.ts";

interface Peer {
  workspace(command: object): Promise<unknown>;
}
interface Prepared extends ExportMetadata {
  exportId: string;
  fileName: string;
  mime: string;
}
function prepared(value: unknown): Prepared {
  if (!value || typeof value !== "object") throw new Error("invalid_response");
  const v = value as Prepared;
  if (
    !isIdentity(v.exportId) ||
    typeof v.fileName !== "string" ||
    !/^[a-zA-Z0-9_.-]{1,200}$/.test(v.fileName) ||
    v.fileName.startsWith(".") ||
    typeof v.mime !== "string" ||
    v.mime.length > 64 ||
    !Number.isSafeInteger(v.size) ||
    v.size < 0 ||
    v.size > contract.export_bytes ||
    !/^[a-f0-9]{64}$/.test(v.sha256)
  )
    throw new Error("invalid_response");
  return v;
}
export class NativeExports {
  private readonly writes = new Set<Promise<void>>();
  constructor(
    private readonly peer: Peer,
    private readonly protectedRoots: readonly string[],
  ) {}
  private async prepare(intent: ExportIntent) {
    return prepared(
      await this.peer.workspace({ kind: "prepare_export", ...intent }),
    );
  }
  private release(intent: ExportIntent, grant: Prepared) {
    return this.peer.workspace({
      kind: "release_export",
      context: intent.context,
      export_id: grant.exportId,
    });
  }
  async save(
    intent: ExportIntent,
    choose: (suggestion: Prepared) => Promise<string | null>,
    current: () => boolean,
  ): Promise<ExportReceipt | null> {
    if (!current()) throw new Error("stale_context");
    const suggestion = await this.prepare(intent);
    // No private payload remains leased while the user considers the native dialog.
    await this.release(intent, suggestion);
    if (!current()) throw new Error("stale_context");
    const destination = await choose(suggestion);
    if (destination === null) return null;
    if (!current()) throw new Error("stale_context");
    const grant = await this.prepare(intent);
    try {
      if (
        grant.sha256 !== suggestion.sha256 ||
        grant.size !== suggestion.size ||
        grant.mime !== suggestion.mime
      )
        throw new Error("export_source_changed");
      const task = writeExport(
        destination,
        grant,
        async (offset) =>
          (await this.peer.workspace({
            kind: "read_export",
            context: intent.context,
            export_id: grant.exportId,
            offset,
          })) as ExportChunk,
        current,
        this.protectedRoots,
      );
      this.writes.add(task);
      try {
        await task;
      } finally {
        this.writes.delete(task);
      }
      return {
        fileName: basename(destination),
        size: grant.size,
        sha256: grant.sha256,
      };
    } finally {
      await this.release(intent, grant);
    }
  }
  async settle(): Promise<void> {
    await Promise.allSettled([...this.writes]);
  }
}
