/** One owner for admitted and in-flight external connections. */
import { randomUUID } from "node:crypto";
import { Attachment } from "./attachment.ts";
import type { CliIntent } from "./cli-contract.ts";
import type { Context, RunRequest } from "./workspace-contract.ts";

interface Connection {
  id: string;
  owner: number;
  attachment: Attachment;
}
export class Attachments {
  private readonly connections = new Map<string, Connection>();
  private readonly pending = new Set<Promise<Connection>>();
  private epoch = 0;

  async open(
    owner: number,
    context: Context,
    selection: RunRequest | null,
    dispatch: (intent: CliIntent) => Promise<unknown>,
  ): Promise<Connection> {
    for (const [id, entry] of this.connections)
      if (entry.attachment.isClosed) this.connections.delete(id);
    if (this.connections.size + this.pending.size >= 8)
      throw new Error("attachment_limit");
    const epoch = this.epoch;
    const pending = Attachment.create(context, selection, dispatch).then(
      async (attachment) => {
        if (epoch !== this.epoch) {
          await attachment.close();
          throw new Error("attachment_revoked");
        }
        const entry = { id: randomUUID(), owner, attachment };
        this.connections.set(entry.id, entry);
        return entry;
      },
    );
    this.pending.add(pending);
    try {
      return await pending;
    } finally {
      this.pending.delete(pending);
    }
  }

  async close(owner: number, id: string): Promise<void> {
    const entry = this.connections.get(id);
    if (!entry || entry.owner !== owner)
      throw new Error("unauthorized_attachment");
    await entry.attachment.close();
    this.connections.delete(id);
  }

  async clear(): Promise<void> {
    this.epoch++;
    const current = [...this.connections.values()];
    this.connections.clear();
    await Promise.all([
      ...current.map((entry) => entry.attachment.close()),
      // Admission checks the epoch before publishing its credential. Wait for
      // already reserved allocations to close, even when issuance was revoked.
      ...[...this.pending].map((entry) =>
        entry.then(
          () => {},
          () => {},
        ),
      ),
    ]);
  }
}
