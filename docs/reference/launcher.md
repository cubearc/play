# Native app lifecycle

<a id="cube-integration-contract"></a>
<a id="host-chooser-boundary"></a>
<a id="launcher-and-runtime-contracts"></a>
<a id="platform-boundaries"></a>
<a id="runtime-source-and-private-state"></a>
<a id="workspace-identity-and-lifetime"></a>

Electron main starts the local Rust engine over private pipes. The preview has one active Workspace shared by up to eight registered windows. Closing a window
does not stop accepted Runs; application Quit stops the owner engine.

- [Process and IPC boundary](desktop-runtime.md)
- [Identity and owner locks](desktop-storage.md)
- [Create and reopen Workspaces](../launcher.md)
