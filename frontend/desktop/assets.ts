import { randomBytes } from "node:crypto";
import { basename } from "node:path";
import {
  type BrowserWindow,
  dialog,
  type WebContents,
  type WebFrameMain,
} from "electron";
import { contract } from "./contract.ts";
import type { Engine } from "./engine.ts";
import { readImport } from "./imports.ts";
import type {
  AssetRead,
  ChallengeImport,
  Context,
  FileImport,
  ImageImport,
} from "./workspace-contract.ts";

interface Grant {
  owner: WebContents;
  frame: WebFrameMain;
  expires: number;
  bytes: Buffer;
}
const contextKey = (context: Context) =>
  `${context.workspace_id}/${context.exploration_id}/${context.stage_id}`;
export class AssetGrants {
  private readonly grants = new Map<string, Grant>();
  private context: string | null = null;
  private epoch = 0;
  select(context: Context | null): void {
    const key = context ? contextKey(context) : null;
    if (key !== this.context) {
      this.context = key;
      this.epoch++;
      this.grants.clear();
    }
  }
  revoke(): void {
    this.context = null;
    this.epoch++;
    this.grants.clear();
  }
  allows(url: string, ownerId: number): boolean {
    const grant = this.grants.get(url);
    return Boolean(
      grant &&
        grant.expires > Date.now() &&
        grant.owner.id === ownerId &&
        !grant.owner.isDestroyed() &&
        grant.owner.mainFrame === grant.frame,
    );
  }
  read(url: string): Buffer | null {
    const grant = this.grants.get(url);
    return grant && this.allows(url, grant.owner.id) ? grant.bytes : null;
  }
  async issue(
    engine: Engine,
    owner: WebContents,
    frame: WebFrameMain,
    value: AssetRead,
  ): Promise<string> {
    if (contextKey(value.context) !== this.context)
      throw new Error("stale_context");
    const epoch = this.epoch;
    const result = (await engine.workspace({
      kind: "read_image",
      ...value,
    })) as { mime?: unknown; base64?: unknown };
    if (
      epoch !== this.epoch ||
      owner.isDestroyed() ||
      owner.mainFrame !== frame
    )
      throw new Error("stale_context");
    if (
      result.mime !== "image/png" ||
      typeof result.base64 !== "string" ||
      result.base64.length > Math.ceil(contract.image_bytes / 3) * 4
    )
      throw new Error("invalid_image");
    const bytes = Buffer.from(result.base64, "base64");
    if (
      bytes.length > contract.image_bytes ||
      bytes.toString("base64") !== result.base64
    )
      throw new Error("invalid_image");
    for (const [key, grant] of this.grants)
      if (grant.expires <= Date.now()) this.grants.delete(key);
    if (this.grants.size >= 32) throw new Error("resource_limit");
    const url = `cube-play://app/assets/${randomBytes(16).toString("hex")}`;
    this.grants.set(url, { owner, frame, expires: Date.now() + 60_000, bytes });
    return url;
  }
}

export async function importImage(
  engine: Engine,
  window: BrowserWindow,
  value: ImageImport,
): Promise<unknown> {
  const selection = await dialog.showOpenDialog(window, {
    title: "Import a PNG image",
    properties: ["openFile"],
    filters: [{ name: "PNG image", extensions: ["png"] }],
  });
  if (selection.canceled || selection.filePaths.length !== 1) return null;
  const path = selection.filePaths[0];
  if (!path) throw new Error("invalid_image");
  const bytes = await readImport(path, contract.image_bytes, "image_limit");
  return engine.workspace({
    kind: "add_image",
    ...value,
    title: basename(path),
    png: bytes.toString("base64"),
  });
}

export async function importFile(
  engine: Engine,
  window: BrowserWindow,
  value: FileImport,
): Promise<unknown> {
  const selection = await dialog.showOpenDialog(window, {
    title: "Import a file",
    properties: ["openFile"],
  });
  if (selection.canceled || selection.filePaths.length !== 1) return null;
  const path = selection.filePaths[0];
  if (!path) throw new Error("invalid_file");
  const bytes = await readImport(path, contract.file_bytes, "file_limit");
  return engine.workspace({
    kind: "add_file",
    ...value,
    title: basename(path),
    base64: bytes.toString("base64"),
  });
}

export async function importChallenge(
  engine: Engine,
  window: BrowserWindow,
  value: ChallengeImport,
): Promise<unknown> {
  const selection = await dialog.showOpenDialog(window, {
    title: "Import a Challenge package",
    properties: ["openFile"],
    filters: [{ name: "Cube Play Challenge", extensions: ["json"] }],
  });
  if (selection.canceled || selection.filePaths.length !== 1) return null;
  const path = selection.filePaths[0];
  if (!path) throw new Error("invalid_package");
  const bytes = await readImport(path, contract.package_bytes, "package_limit");
  return engine.workspace({
    kind: "import_challenge",
    ...value,
    package: bytes.toString("base64"),
  });
}
