//! Real installer qualification: use the host package, never substitute a fixture DMG.
import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import {
  copyFile,
  mkdtemp,
  readdir,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { signManifest } from "./update-metadata.ts";
import { verifyLocalUpdate } from "./verify-update.ts";

test("macOS disk images verify real bundle identity and reject corrupt, interrupted and untrusted test updates", {
  skip: process.platform !== "darwin",
  timeout: 120000,
}, async () => {
  const version = JSON.parse(
    await readFile(new URL("../package.json", import.meta.url), "utf8"),
  ).version;
  const image = resolve(
    process.env.PLAY_DMG_FIXTURE ??
      `desktop-packages/Cube-Play-${version}-darwin-${process.arch}.dmg`,
  );
  const proof = JSON.parse(await readFile(`${image}.proof.json`, "utf8"));
  const root = await mkdtemp(join(tmpdir(), "play-delivery-proof-"));
  const pair = generateKeyPairSync("ed25519"),
    other = generateKeyPairSync("ed25519");
  const key = pair.privateKey
      .export({ type: "pkcs8", format: "pem" })
      .toString(),
    publicKey = pair.publicKey
      .export({ type: "spki", format: "pem" })
      .toString();
  const now = Date.now();
  const metadata = { ...proof, issuedAt: now, expiresAt: now + 86400000 };
  const document = signManifest(metadata, key, now);
  try {
    const target = join(root, "verified.dmg");
    const result = await verifyLocalUpdate(
      document,
      publicKey,
      image,
      target,
      "0.0.0",
    );
    assert.equal(result.version, proof.bundle.appVersion);
    assert.equal(result.sha256, proof.sha256);
    await assert.rejects(
      verifyLocalUpdate(document, publicKey, image, target, "0.0.0"),
      /EEXIST/,
    );
    await assert.rejects(
      verifyLocalUpdate(
        document,
        other.publicKey.export({ type: "spki", format: "pem" }).toString(),
        image,
        join(root, "untrusted.dmg"),
        "0.0.0",
      ),
      /invalid_update_signature/,
    );
    await assert.rejects(
      verifyLocalUpdate(
        document,
        publicKey,
        image,
        join(root, "downgrade.dmg"),
        proof.bundle.appVersion,
      ),
      /update_not_newer/,
    );
    const incomplete = join(root, "interrupted.dmg");
    await writeFile(incomplete, Buffer.from("interrupted"));
    await assert.rejects(
      verifyLocalUpdate(
        document,
        publicKey,
        incomplete,
        join(root, "bad.dmg"),
        "0.0.0",
      ),
      /update_size_mismatch/,
    );
    const corrupt = join(root, "corrupt.dmg");
    await copyFile(image, corrupt);
    const { open } = await import("node:fs/promises");
    const file = await open(corrupt, "r+");
    try {
      await file.write(Buffer.from("bad"), 0, 3, 0);
    } finally {
      await file.close();
    }
    await assert.rejects(
      verifyLocalUpdate(
        document,
        publicKey,
        corrupt,
        join(root, "bad.dmg"),
        "0.0.0",
      ),
      /update_integrity_failed/,
    );
    const incompatible = signManifest(
      { ...metadata, bundle: { ...metadata.bundle, schemaVersion: 999 } },
      key,
      now,
    );
    await assert.rejects(
      verifyLocalUpdate(
        incompatible,
        publicKey,
        image,
        join(root, "bad.dmg"),
        "0.0.0",
      ),
      /update_storage_incompatible/,
    );
    assert.deepEqual((await readdir(root)).sort(), [
      "corrupt.dmg",
      "interrupted.dmg",
      "verified.dmg",
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
