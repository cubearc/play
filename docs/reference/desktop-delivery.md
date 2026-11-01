# Qualify a macOS preview package

The Mac packager produces a standalone application, a read-only DMG with an
Applications shortcut and a `.dmg.proof.json` receipt. The receipt records SHA-256,
size, architecture, clean/dirty source identity, component versions, protocol,
storage generation, schema and native binary digests. An occupied output is never
replaced. Preview packaging does not select an Apple certificate or notarize code.

## Verify an offline test update

The build-only delivery tool signs the receipt with an explicitly supplied
Ed25519 key. This is a **local-test** channel, separate from Apple publisher
signing. It has no network feed, background check, stored trust key or automatic
installation. Obtain the public key through a trusted channel and compare its
fingerprint independently. A key delivered by the same untrusted source as the
package does not establish trust.

From `frontend`, after a clean reviewed build and package:

```sh
npm run desktop:delivery -- sign /path/to/package.dmg.proof.json /private/path/test-private.pem /path/to/test-update.json
npm run desktop:delivery -- verify /path/to/test-update.json /trusted/path/test-public.pem /path/to/package.dmg /path/to/new-verified.dmg 0.2.0
```

The final argument is the **installed app version**, read from its package proof.
Signing refuses dirty source receipts. Verification checks Ed25519 before
interpreting package metadata, enforces a newer version and matching architecture,
requires the current storage generation/schema, and rejects expired metadata.
The signature binds the exact artifact hash and complete bundle identity.

The verifier copies at most 512 MiB into private temporary storage, checks exact
size/hash, mounts that same copy read-only, checks its actual package/protocol
identity and all native digests, and verifies the sandbox helper signatures.
Only then does it publish a verified DMG at a new filename. Occupied outputs are
preserved. Invalid signatures, interrupted copies, corrupt bytes and mismatches
leave no verified output. It sends no source files or credentials anywhere.

Open the verified copy and follow [manual installation](../installation.md#install-replace-or-remove-the-mac-preview).
Quit and save drafts first; accepted active Runs are cancelled and reaped by the
normal Quit path, leaving truthful history. Keep the prior application for manual
recovery. Verification does not replace a running application or rewrite stores.

## Run the actual package checks

```sh
npm run desktop:delivery-test
```

Run this from `frontend` after `desktop:package`. `PLAY_DMG_FIXTURE` can point at
another real host DMG with its identity receipt. The test creates ephemeral
signing keys and verifies the real package, occupied output, a wrong key,
truncated/corrupt artifacts, equal-version rejection and incompatible storage.
It does not fake an older installed app or establish automatic handover.
The macOS CI runs this check against its exact built package.

The supported automatic macOS updater requires an appropriately signed app.
Publisher signing, notarization, production feeds and automatic handover remain
unavailable without the delivery credentials. See the official
[Electron update contract](https://www.electronjs.org/docs/latest/api/auto-updater).
