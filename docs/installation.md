# Run the Electron source preview

<a id="install-cube-play-from-source"></a>

Build the current desktop preview from trusted Cube and Play checkouts side by
side. Operations run on macOS and Linux x64/ARM64. macOS uses a locally signed
App Sandbox helper; Linux requires enabled Landlock ABI 3 and seccomp filtering.
The Windows preview refuses execution with `native_policy_unavailable`. Missing
enforcement has no fallback. The interactive terminal currently requires macOS.
There is no release-supported signed installer or automatic updater yet.

## Prerequisites

- The Rust toolchain in `rust-toolchain.toml`, installed through rustup.
- Node.js from `frontend/.nvmrc` and npm; the package minimum is Node 24.15.
- Cube 0.18.0 sources at `../cube`, matching Play's dependency and the reviewed
  Core revision in [CI](../.github/workflows/ci.yml).
- On macOS, Xcode Command Line Tools, including the local `codesign` tool.
- On Linux, an active graphical session and Electron/Chromium runtime libraries.
  Ubuntu 24.04 x64/ARM64 is tested. Chromium must have a usable OS sandbox;
  never launch with `--no-sandbox`. See [Linux validation](development.md#validate-linux-electron)
  for the tested sandbox-helper setup.

Ensure `cargo` and `rustc` resolve to rustup, and Node matches `frontend/.nvmrc`;
a separate system Rust/Node installation may take precedence in your PATH.

Docker, Compose and WSL are not application prerequisites. Installation downloads
npm dependencies, Electron and Rust dependencies as needed.

## Build and open

From the Play repository root:

```sh
npm ci --prefix frontend
npm run desktop:build --prefix frontend
npm run desktop:proof --prefix frontend
```

The build compiles the renderer, preload and Rust engine, and prepares the local
native helper. The last command opens **Cube Play**. Create a Workspace in the
welcome screen; an empty checkout does not contain a sample investigation.
Continue with [your first notebook](getting-started.md).

![Electron notebook on Ubuntu 24.04 showing an image and its caption in one Block.](assets/desktop/linux-notebook.png)

The actual Linux Electron notebook, captured by the native UI test with disposable
example data. [Open full size](assets/desktop/linux-notebook.png).

Use **Quit** to end the app. Closing a window on macOS can leave the app and its
engine alive; closing a canvas does not cancel its Run. Save reasoning before
quitting. [Resume saved work](persistence.md) explains what survives restart.

These commands run a development build from the checkout, not an installed
release. Local helper signing does not establish publisher signing or
notarization. See [native execution](reference/desktop-runtime.md) for exact
platform boundaries, or [Development](development.md) to validate the build.
