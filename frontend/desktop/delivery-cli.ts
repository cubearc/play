//! Offline preview-delivery tooling; excluded from packaged runtime files.
import { constants } from "node:fs";
import { open, writeFile } from "node:fs/promises";
import { basename, resolve } from "node:path";
import { packageName } from "./disk-image.ts";
import { signManifest } from "./update-metadata.ts";
import { verifyLocalUpdate } from "./verify-update.ts";

async function bounded(path: string, limit: number): Promise<string> {
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.size > limit)
      throw new Error("invalid_delivery_file");
    const bytes = Buffer.alloc(limit + 1);
    const { bytesRead } = await file.read(bytes, 0, bytes.length, 0);
    if (bytesRead > limit) throw new Error("invalid_delivery_file");
    return bytes.subarray(0, bytesRead).toString("utf8");
  } finally {
    await file.close();
  }
}
const [action, ...args] = process.argv.slice(2);
if (action === "sign" && args.length === 3) {
  const [proofPath, keyPath, output] = args as [string, string, string];
  const proof = JSON.parse(await bounded(resolve(proofPath), 16384));
  packageName(proof.artifact);
  const now = Date.now();
  const signed = signManifest(
    { ...proof, issuedAt: now, expiresAt: now + 86400000 },
    await bounded(resolve(keyPath), 4096),
    now,
  );
  await writeFile(resolve(output), signed, { flag: "wx", mode: 0o600 });
  console.log(
    JSON.stringify({
      manifest: basename(output),
      channel: "local-test",
      publisherSigning: "absent",
    }),
  );
} else if (action === "verify" && args.length === 5) {
  const [document, key, artifact, output, currentVersion] = args as [
    string,
    string,
    string,
    string,
    string,
  ];
  const result = await verifyLocalUpdate(
    await bounded(resolve(document), 16384),
    await bounded(resolve(key), 4096),
    resolve(artifact),
    resolve(output),
    currentVersion,
  );
  console.log(
    JSON.stringify(
      {
        ...result,
        verifiedCopy: resolve(output),
        installation: "manual_after_quit",
        publisherSigning: "absent",
      },
      null,
      2,
    ),
  );
} else {
  throw new Error(
    "usage: delivery-cli sign PROOF PRIVATE_KEY NEW_MANIFEST | verify MANIFEST TRUSTED_PUBLIC_KEY DMG NEW_VERIFIED_DMG CURRENT_VERSION",
  );
}
