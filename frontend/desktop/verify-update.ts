//! Verify an offline test update and publish an exclusive verified copy; never replace a running app.
import { createHash } from "node:crypto";
import { constants } from "node:fs";
import { link, mkdtemp, open, readFile, rm } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { contract } from "./contract.ts";
import { inspectDiskImage } from "./disk-image.ts";
import { verifyNative } from "./integrity.ts";
import { type PreviewManifest, verifyManifest } from "./update-metadata.ts";

async function inspect(application: string, manifest: PreviewManifest) {
  const app = join(application, "Contents/Resources/app");
  const proof = JSON.parse(
    await readFile(join(app, "package-proof.json"), "utf8"),
  );
  const pkg = JSON.parse(await readFile(join(app, "package.json"), "utf8"));
  const protocol = JSON.parse(
    await readFile(join(app, "desktop/protocol.json"), "utf8"),
  );
  if (
    !isDeepStrictEqual(proof, manifest.bundle) ||
    pkg.version !== proof.appVersion ||
    protocol.engine !== proof.engineVersion ||
    protocol.protocol !== proof.protocol ||
    protocol.storage_generation !== proof.storageGeneration ||
    protocol.schema_version !== proof.schemaVersion
  )
    throw new Error("update_bundle_mismatch");
  const native = join(app, "frontend/desktop-native");
  if (
    !isDeepStrictEqual(
      JSON.parse(await readFile(join(native, "integrity.json"), "utf8")),
      proof.native,
    )
  )
    throw new Error("update_bundle_mismatch");
  await verifyNative(pathToFileURL(`${native}/`));
}
export async function verifyLocalUpdate(
  document: string,
  publicKey: string,
  artifact: string,
  destination: string,
  currentVersion: string,
): Promise<{ version: string; sha256: string; fingerprint: string }> {
  if (process.platform !== "darwin")
    throw new Error("update_platform_unavailable");
  const checked = verifyManifest(document, publicKey, currentVersion);
  const manifest = checked.manifest;
  if (
    manifest.bundle.storageGeneration !== contract.storage_generation ||
    manifest.bundle.schemaVersion !== contract.schema_version
  )
    throw new Error("update_storage_incompatible");
  const temporary = await mkdtemp(
    join(dirname(destination), ".cube-play-verified-"),
  );
  const copied = join(temporary, "package.dmg");
  try {
    const source = await open(
      artifact,
      constants.O_RDONLY | constants.O_NOFOLLOW,
    );
    try {
      const stat = await source.stat();
      if (!stat.isFile() || stat.size !== manifest.size)
        throw new Error("update_size_mismatch");
      const output = await open(copied, "wx", 0o600);
      try {
        const hash = createHash("sha256");
        let offset = 0;
        const buffer = Buffer.alloc(65536);
        while (offset < manifest.size) {
          const { bytesRead } = await source.read(
            buffer,
            0,
            Math.min(buffer.length, manifest.size - offset),
            offset,
          );
          if (!bytesRead) throw new Error("update_interrupted");
          const bytes = buffer.subarray(0, bytesRead);
          hash.update(bytes);
          let written = 0;
          while (written < bytesRead) {
            const part = await output.write(
              bytes,
              written,
              bytesRead - written,
              null,
            );
            if (!part.bytesWritten) throw new Error("update_write_failed");
            written += part.bytesWritten;
          }
          offset += bytesRead;
        }
        if (hash.digest("hex") !== manifest.sha256)
          throw new Error("update_integrity_failed");
        await output.sync();
      } finally {
        await output.close();
      }
    } finally {
      await source.close();
    }
    await inspectDiskImage(copied, (app) => inspect(app, manifest));
    await link(copied, destination);
    return {
      version: manifest.bundle.appVersion,
      sha256: manifest.sha256,
      fingerprint: checked.fingerprint,
    };
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
