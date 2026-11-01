import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import test from "node:test";
import { signManifest, verifyManifest } from "./update-metadata.ts";

test("offline update metadata requires the chosen signing key, current compatible identity and a newer version", () => {
  const pair = generateKeyPairSync("ed25519"),
    foreign = generateKeyPairSync("ed25519");
  const privateKey = pair.privateKey
      .export({ type: "pkcs8", format: "pem" })
      .toString(),
    publicKey = pair.publicKey
      .export({ type: "spki", format: "pem" })
      .toString();
  const now = Date.now();
  const manifest = {
    format: "cube-play.preview-package",
    formatVersion: 1,
    channel: "local-test",
    artifact: "Cube-Play-0.3.0.dmg",
    size: 123,
    sha256: "a".repeat(64),
    issuedAt: now,
    expiresAt: now + 86400000,
    bundle: {
      distribution: "local_preview",
      publisherSigning: "absent",
      platform: "darwin",
      architecture: process.arch,
      appVersion: "0.3.0",
      engineVersion: "0.3.0",
      protocol: 29,
      storageGeneration: 2,
      schemaVersion: 18,
      sourceRevision: "a".repeat(40),
      sourceDirty: false,
      native: { engine: "b".repeat(64) },
    },
  };
  const document = signManifest(manifest, privateKey, now);
  const checked = verifyManifest(
    document,
    publicKey,
    "0.2.0",
    process.arch,
    now,
  );
  assert.equal(checked.manifest.bundle.appVersion, "0.3.0");
  assert.match(checked.fingerprint, /^[a-f0-9]{64}$/);
  assert.throws(
    () =>
      verifyManifest(
        document,
        foreign.publicKey.export({ type: "spki", format: "pem" }).toString(),
        "0.2.0",
        process.arch,
        now,
      ),
    /invalid_update_signature/,
  );
  const changed = JSON.parse(document);
  const data = JSON.parse(Buffer.from(changed.payload, "base64").toString());
  data.sha256 = "c".repeat(64);
  changed.payload = Buffer.from(JSON.stringify(data)).toString("base64");
  assert.throws(
    () =>
      verifyManifest(
        JSON.stringify(changed),
        publicKey,
        "0.2.0",
        process.arch,
        now,
      ),
    /invalid_update_signature/,
  );
  for (const current of ["0.3.0", "0.4.0"]) {
    assert.throws(
      () => verifyManifest(document, publicKey, current, process.arch, now),
      /update_not_newer/,
    );
  }
  assert.throws(
    () =>
      verifyManifest(
        document,
        publicKey,
        "0.2.0",
        process.arch === "x64" ? "arm64" : "x64",
        now,
      ),
    /update_architecture_mismatch/,
  );
  assert.throws(
    () =>
      verifyManifest(
        document,
        publicKey,
        "0.2.0",
        process.arch,
        now + 86400000,
      ),
    /invalid_update_metadata/,
  );
  assert.throws(
    () =>
      signManifest(
        { ...manifest, bundle: { ...manifest.bundle, sourceDirty: true } },
        privateKey,
        now,
      ),
    /invalid_update_metadata/,
  );
});
