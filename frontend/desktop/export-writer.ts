//! Native output grant: bounded exact bytes, private staging and exclusive publication.
import { createHash, randomUUID } from "node:crypto";
import { constants } from "node:fs";
import {
  type FileHandle,
  link,
  lstat,
  open,
  realpath,
  stat,
  unlink,
} from "node:fs/promises";
import { basename, dirname, isAbsolute, join } from "node:path";
import { contract } from "./contract.ts";

export interface ExportMetadata {
  size: number;
  sha256: string;
}
export interface ExportChunk {
  offset: number;
  base64: string;
  done: boolean;
}
function code(error: unknown): string | undefined {
  return (error as NodeJS.ErrnoException)?.code;
}
async function unused(path: string) {
  try {
    await lstat(path);
  } catch (error) {
    if (code(error) === "ENOENT") return;
    throw error;
  }
  throw new Error("export_target_exists");
}
async function allowedParent(parent: string, roots: readonly string[]) {
  const blocked = await Promise.all(roots.map((root) => stat(root)));
  let current = parent;
  for (let depth = 0; depth < 128; depth++) {
    const identity = await stat(current);
    if (
      blocked.some(
        (root) => root.dev === identity.dev && root.ino === identity.ino,
      )
    )
      throw new Error("export_target_protected");
    const next = dirname(current);
    if (next === current) return;
    current = next;
  }
  throw new Error("export_target_unavailable");
}
async function writeAll(file: FileHandle, bytes: Buffer) {
  let offset = 0;
  while (offset < bytes.length) {
    const { bytesWritten } = await file.write(
      bytes,
      offset,
      bytes.length - offset,
      null,
    );
    if (!bytesWritten) throw new Error("export_write_failed");
    offset += bytesWritten;
  }
}
function chunkBytes(chunk: ExportChunk, offset: number, size: number): Buffer {
  if (
    chunk.offset !== offset ||
    typeof chunk.base64 !== "string" ||
    chunk.base64.length > Math.ceil(contract.export_chunk_bytes / 3) * 4
  )
    throw new Error("invalid_response");
  const bytes = Buffer.from(chunk.base64, "base64");
  if (
    !bytes.length ||
    bytes.length > contract.export_chunk_bytes ||
    bytes.toString("base64") !== chunk.base64 ||
    offset + bytes.length > size ||
    chunk.done !== (offset + bytes.length === size)
  )
    throw new Error("invalid_response");
  return bytes;
}
export async function writeExport(
  destination: string,
  metadata: ExportMetadata,
  read: (offset: number) => Promise<ExportChunk>,
  current: () => boolean,
  protectedRoots: readonly string[] = [],
): Promise<void> {
  if (
    !isAbsolute(destination) ||
    !Number.isSafeInteger(metadata.size) ||
    metadata.size < 0 ||
    metadata.size > contract.export_bytes ||
    !/^[a-f0-9]{64}$/.test(metadata.sha256)
  )
    throw new Error("invalid_export");
  const parent = await realpath(dirname(destination));
  const target = join(parent, basename(destination));
  await allowedParent(parent, protectedRoots);
  await unused(target);
  if (!current()) throw new Error("stale_context");
  const temporary = join(parent, `.cube-play-export-${randomUUID()}.partial`);
  let file: FileHandle | undefined = await open(
    temporary,
    constants.O_WRONLY |
      constants.O_CREAT |
      constants.O_EXCL |
      constants.O_NOFOLLOW,
    0o600,
  );
  try {
    const hash = createHash("sha256");
    let offset = 0;
    while (offset < metadata.size) {
      if (!current()) throw new Error("stale_context");
      const bytes = chunkBytes(await read(offset), offset, metadata.size);
      await writeAll(file, bytes);
      hash.update(bytes);
      offset += bytes.length;
    }
    if (hash.digest("hex") !== metadata.sha256)
      throw new Error("export_integrity_failed");
    await file.sync();
    await file.close();
    file = undefined;
    if (!current()) throw new Error("stale_context");
    try {
      await link(temporary, target);
    } catch (error) {
      if (code(error) === "EEXIST") throw new Error("export_target_exists");
      throw new Error("export_target_unavailable", { cause: error });
    }
  } finally {
    try {
      await file?.close();
    } finally {
      await unlink(temporary);
    }
  }
}
