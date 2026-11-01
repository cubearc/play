//! Build-only macOS preview installer. No publisher identity is invented.
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import {
  cp,
  link,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { promisify } from "node:util";

export const command = promisify(execFile);
export const artifactLimit = 512 * 1024 * 1024;
export async function digestFile(
  path: string,
): Promise<{ size: number; sha256: string }> {
  const hash = createHash("sha256");
  let size = 0;
  for await (const part of createReadStream(path)) {
    size += part.length;
    if (size > artifactLimit) throw new Error("package_size_limit");
    hash.update(part);
  }
  return { size, sha256: hash.digest("hex") };
}
export async function createDiskImage(
  application: string,
  output: string,
  version: string,
): Promise<string> {
  if (process.platform !== "darwin")
    throw new Error("installer_platform_unavailable");
  if (!/^\d+\.\d+\.\d+$/.test(version))
    throw new Error("invalid_package_version");
  await mkdir(output, { recursive: true });
  const temporary = await mkdtemp(join(output, ".cube-play-installer-"));
  const fileName = `Cube-Play-${version}-darwin-${process.arch}.dmg`;
  try {
    const folder = join(temporary, "contents");
    await mkdir(folder);
    await cp(application, join(folder, "Cube Play.app"), {
      recursive: true,
      verbatimSymlinks: true,
    });
    await symlink("/Applications", join(folder, "Applications"));
    await writeFile(
      join(folder, "Install Cube Play.txt"),
      "Cube Play local preview\n\nQuit Cube Play before replacing an installed copy. Drag Cube Play.app to Applications.\nThis preview has no Apple publisher signature or notarization.\nDo not remove its private app profile when replacing or deleting the application.\nSee https://cubearc.github.io/docs/play/installation.html for exact boundaries.\n",
    );
    const staged = join(temporary, fileName);
    await command(
      "/usr/bin/hdiutil",
      [
        "create",
        "-srcfolder",
        folder,
        "-volname",
        "Cube Play Preview",
        "-format",
        "UDZO",
        "-fs",
        "HFS+",
        "-o",
        staged,
      ],
      { timeout: 120000, maxBuffer: 65536 },
    );
    const bundle = JSON.parse(
      await readFile(
        join(application, "Contents/Resources/app/package-proof.json"),
        "utf8",
      ),
    );
    const metadata = {
      format: "cube-play.preview-package",
      formatVersion: 1,
      channel: "local-test",
      artifact: fileName,
      ...(await digestFile(staged)),
      bundle,
    };
    // Exclusive publication retains an earlier installer if this version already exists.
    const target = join(output, fileName);
    await link(staged, target);
    await writeFile(
      `${target}.proof.json`,
      `${JSON.stringify(metadata, null, 2)}\n`,
      { flag: "wx", mode: 0o600 },
    );
    return target;
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
export async function inspectDiskImage(
  image: string,
  inspect: (application: string) => Promise<void>,
): Promise<void> {
  if (process.platform !== "darwin")
    throw new Error("installer_platform_unavailable");
  const root = await mkdtemp(join(tmpdir(), "cube-play-mount-"));
  const mount = join(root, "volume");
  await mkdir(mount);
  let mounted = false;
  try {
    await command(
      "/usr/bin/hdiutil",
      ["attach", image, "-readonly", "-nobrowse", "-mountpoint", mount],
      { timeout: 30000, maxBuffer: 65536 },
    );
    mounted = true;
    await inspect(join(mount, "Cube Play.app"));
  } finally {
    if (mounted)
      await command("/usr/bin/hdiutil", ["detach", mount], {
        timeout: 30000,
        maxBuffer: 65536,
      });
    // Keep an undetached mount intact for explicit recovery if the OS refuses detachment.
    await rm(root, { recursive: true, force: true });
  }
}
export function packageName(path: string): string {
  const name = basename(path);
  if (!/^[A-Za-z0-9._-]+\.dmg$/.test(name))
    throw new Error("invalid_package_name");
  return name;
}
