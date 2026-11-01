/** Source locations only orient native dialogs; they never become private stores. */

import { randomUUID } from "node:crypto";
import { constants } from "node:fs";
import {
  type FileHandle,
  open,
  realpath,
  rename,
  stat,
  unlink,
} from "node:fs/promises";
import { basename, isAbsolute, join } from "node:path";
import { contract } from "./contract.ts";
import { readHandle } from "./imports.ts";

interface Entry {
  path: string;
  id: string | null;
  key: string;
  title: string;
}
interface Host {
  create: (key: string, title: string) => Promise<string>;
  open: (id: string) => Promise<void>;
}
const uuid = /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/;
export class SourceRoots {
  private readonly file: string;
  private readonly profile: string;
  private pending = false;
  constructor(profile: string) {
    this.profile = profile;
    this.file = join(profile, "sources.json");
  }
  private async read(): Promise<Entry[]> {
    let handle: FileHandle;
    try {
      handle = await open(this.file, constants.O_RDONLY | constants.O_NOFOLLOW);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
      throw new Error("unsafe_source_registry");
    }
    try {
      const meta = await handle.stat();
      if (
        !meta.isFile() ||
        meta.nlink !== 1 ||
        meta.size > 32768 ||
        (process.platform !== "win32" &&
          ((meta.mode & 0o077) !== 0 || meta.uid !== process.getuid?.()))
      )
        throw new Error("unsafe_source_registry");
      const value = JSON.parse(
        (await readHandle(handle, 32768, "unsafe_source_registry")).toString(
          "utf8",
        ),
      );
      if (
        !value ||
        Object.keys(value).sort().join() !== "roots,version" ||
        value.version !== 1 ||
        !Array.isArray(value.roots) ||
        value.roots.length > contract.workspace_limit
      )
        throw new Error("unsafe_source_registry");
      const paths = new Set();
      const keys = new Set();
      for (const row of value.roots) {
        if (
          !row ||
          Object.keys(row).sort().join() !== "id,key,path,title" ||
          typeof row.path !== "string" ||
          !isAbsolute(row.path) ||
          typeof row.key !== "string" ||
          !uuid.test(row.key) ||
          (row.id !== null &&
            (typeof row.id !== "string" || !uuid.test(row.id))) ||
          typeof row.title !== "string" ||
          !row.title.trim() ||
          Buffer.byteLength(row.title) > 120 ||
          paths.has(row.path) ||
          keys.has(row.key)
        )
          throw new Error("unsafe_source_registry");
        paths.add(row.path);
        keys.add(row.key);
      }
      return value.roots;
    } catch {
      throw new Error("unsafe_source_registry");
    } finally {
      await handle.close();
    }
  }
  private async write(roots: Entry[]): Promise<void> {
    const data = `${JSON.stringify({ version: 1, roots })}\n`;
    if (Buffer.byteLength(data) > 32768) throw new Error("storage_quota");
    const temporary = join(this.profile, `.sources-${randomUUID()}`);
    const handle = await open(temporary, "wx", 0o600);
    try {
      await handle.writeFile(data);
      await handle.sync();
    } finally {
      await handle.close();
    }
    try {
      await rename(temporary, this.file);
      if (process.platform !== "win32") {
        const directory = await open(this.profile, "r");
        try {
          await directory.sync();
        } finally {
          await directory.close();
        }
      }
    } catch (error) {
      await unlink(temporary).catch(() => {});
      throw error;
    }
  }
  async source(id: string): Promise<string | undefined> {
    return (await this.read()).find((row) => row.id === id)?.path;
  }
  async open(path: string, host: Host): Promise<string> {
    if (this.pending) throw new Error("source_open_busy");
    this.pending = true;
    try {
      const canonical = await realpath(path);
      if (!(await stat(canonical)).isDirectory())
        throw new Error("invalid_source_root");
      const roots = await this.read();
      let entry = roots.find((row) => row.path === canonical);
      if (entry?.id) {
        await host.open(entry.id);
        return entry.id;
      }
      if (!entry) {
        if (roots.length >= contract.workspace_limit)
          throw new Error("storage_quota");
        let title = "";
        for (const c of basename(canonical).trim() || "Source folder") {
          if (Buffer.byteLength(title + c) > 120) break;
          title += c;
        }
        entry = { path: canonical, id: null, key: randomUUID(), title };
        roots.push(entry);
        await this.write(roots);
      }
      const id = await host.create(entry.key, entry.title);
      if (!uuid.test(id)) throw new Error("invalid_response");
      entry.id = id;
      await this.write(roots);
      return id;
    } finally {
      this.pending = false;
    }
  }
}
