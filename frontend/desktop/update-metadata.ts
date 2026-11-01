//! Offline test-channel verification. This grants no automatic update authority.
import { createHash, createPublicKey, sign, verify } from "node:crypto";
import { artifactLimit } from "./disk-image.ts";

export interface PreviewManifest {
  format: "cube-play.preview-package";
  formatVersion: 1;
  channel: "local-test";
  artifact: string;
  size: number;
  sha256: string;
  issuedAt: number;
  expiresAt: number;
  bundle: {
    distribution: string;
    publisherSigning: string;
    platform: string;
    architecture: string;
    appVersion: string;
    engineVersion: string;
    protocol: number;
    storageGeneration: number;
    schemaVersion: number;
    sourceRevision: string;
    sourceDirty: boolean;
    native: Record<string, string>;
  };
}
export function version(value: string): number[] {
  if (!/^\d{1,5}\.\d{1,5}\.\d{1,5}$/.test(value))
    throw new Error("invalid_package_version");
  return value.split(".").map(Number);
}
export function newer(target: string, current: string): boolean {
  const a = version(target),
    b = version(current);
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return (a[i] ?? 0) > (b[i] ?? 0);
  }
  return false;
}
function manifest(value: unknown, now: number): PreviewManifest {
  if (!value || typeof value !== "object")
    throw new Error("invalid_update_metadata");
  const m = value as PreviewManifest;
  const b = m.bundle;
  if (
    m.format !== "cube-play.preview-package" ||
    m.formatVersion !== 1 ||
    m.channel !== "local-test" ||
    typeof m.artifact !== "string" ||
    !/^[A-Za-z0-9._-]+\.dmg$/.test(m.artifact) ||
    !Number.isSafeInteger(m.size) ||
    m.size < 1 ||
    m.size > artifactLimit ||
    typeof m.sha256 !== "string" ||
    !/^[a-f0-9]{64}$/.test(m.sha256) ||
    !Number.isSafeInteger(m.issuedAt) ||
    !Number.isSafeInteger(m.expiresAt) ||
    m.issuedAt > now ||
    m.expiresAt <= now ||
    m.expiresAt <= m.issuedAt ||
    m.expiresAt - m.issuedAt > 7 * 86400000 ||
    !b ||
    b.distribution !== "local_preview" ||
    b.publisherSigning !== "absent" ||
    b.sourceDirty !== false ||
    b.platform !== "darwin" ||
    !["arm64", "x64"].includes(b.architecture) ||
    typeof b.sourceRevision !== "string" ||
    !/^[a-f0-9]{40}$/.test(b.sourceRevision) ||
    !Number.isSafeInteger(b.protocol) ||
    b.protocol < 1 ||
    !Number.isSafeInteger(b.storageGeneration) ||
    !Number.isSafeInteger(b.schemaVersion) ||
    !b.native ||
    typeof b.native !== "object" ||
    !Object.values(b.native).every(
      (hash) => typeof hash === "string" && /^[a-f0-9]{64}$/.test(hash),
    )
  )
    throw new Error("invalid_update_metadata");
  version(b.appVersion);
  version(b.engineVersion);
  return m;
}
export function signManifest(
  value: unknown,
  privateKey: string,
  now = Date.now(),
): string {
  const checked = manifest(value, now);
  const payload = Buffer.from(JSON.stringify(checked));
  const key = createPublicKey(privateKey);
  if (key.asymmetricKeyType !== "ed25519") throw new Error("update_key_type");
  return `${JSON.stringify(
    {
      format: "cube-play.signed-test-update",
      payload: payload.toString("base64"),
      signature: sign(null, payload, privateKey).toString("base64"),
    },
    null,
    2,
  )}\n`;
}
export function verifyManifest(
  document: string,
  publicKey: string,
  currentVersion: string,
  architecture = process.arch,
  now = Date.now(),
): { manifest: PreviewManifest; fingerprint: string } {
  if (
    Buffer.byteLength(document) > 16384 ||
    Buffer.byteLength(publicKey) > 4096
  )
    throw new Error("update_metadata_limit");
  const envelope = JSON.parse(document) as {
    format: string;
    payload: string;
    signature: string;
  };
  if (
    envelope.format !== "cube-play.signed-test-update" ||
    typeof envelope.payload !== "string" ||
    typeof envelope.signature !== "string"
  )
    throw new Error("invalid_update_metadata");
  const payload = Buffer.from(envelope.payload, "base64"),
    signature = Buffer.from(envelope.signature, "base64");
  if (
    payload.toString("base64") !== envelope.payload ||
    signature.length !== 64 ||
    signature.toString("base64") !== envelope.signature
  )
    throw new Error("invalid_update_signature");
  const key = createPublicKey(publicKey);
  if (
    key.asymmetricKeyType !== "ed25519" ||
    !verify(null, payload, key, signature)
  )
    throw new Error("invalid_update_signature");
  const checked = manifest(JSON.parse(payload.toString("utf8")), now);
  if (checked.bundle.architecture !== architecture)
    throw new Error("update_architecture_mismatch");
  if (!newer(checked.bundle.appVersion, currentVersion))
    throw new Error("update_not_newer");
  return {
    manifest: checked,
    fingerprint: createHash("sha256")
      .update(key.export({ type: "spki", format: "der" }))
      .digest("hex"),
  };
}
