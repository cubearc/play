# Security policy

## Report privately

Do not post exploit details, credentials or private investigation files in public
issues. This repository is public. Use a private advisory only if its Security
tab offers a private reporting form. Otherwise, open a minimal issue asking
maintainers to establish a confidential channel, without including sensitive
material. There is no security email address documented by this project; do not
assume an unverified address or public issue is a confidential reporting channel.

Include the affected commit/build, operating system and scope after a private
channel is established. Security fixes currently target `main`.

## Local trust boundary

The Electron preview is a private local application. Its renderer has sandboxing
and context isolation, no Node integration and no unrestricted shell, path or
SQLite API. Main accepts closed commands only from the registered trusted top
frame. It launches the exact local Rust engine over private pipes; there is no
session URL or localhost server in this invocation path.

The engine validates Workspace/Exploration ownership and exact Asset revisions.
Private stores have an OS-held owner lock and explicit schema refusal; unsupported
stores are preserved, not reset. Native PNG selection copies only the selected
bounded input into private storage and does not overwrite the source or grant
access to sibling files. Remote content must not gain application preload or
native authority, and Challenge content never executes automatically.

macOS execution uses a locally ad-hoc-signed App Sandbox helper with bounded
inputs/outputs and enforced file/network/process/time controls. Windows/Linux
workers refuse execution while native enforcement is unavailable. These are
explicit development boundaries, not a complete signed release or aggregate
process-tree/PTY guarantee. See [native execution](docs/reference/desktop-runtime.md)
for precise accounting, limits and unimplemented release requirements.

Local results and reasoning stay private. Retaining a Run output is an explicit
choice; a result is not an official Challenge evaluation. The preview has no
Cloud credential store, submission or automatic reporting. Dependency downloads
occur during setup/builds. Diagnostics must omit credentials, private payloads,
command contents and unnecessary source paths. Protect the host account: the
application does not defend against an attacker controlling that account.

## Repository transition

The served-web, host launcher and container Runner still exist pending
[required retirement](https://github.com/cubearc/play/issues/56). Their session,
Host/Origin, source, network and resource checks remain required until replaced;
they are not fallback policies for native execution. Do not add container-control
privileges to application workers or weaken tests while retiring those paths.

Follow the [native architecture](docs/architecture.md) and
[contributor verification](docs/reference/validation.md) when changing a boundary.
A passing protocol test or local signature does not establish packaged platform,
update or hosted-service security.
