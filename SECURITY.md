# Security policy

## Report privately

This repository is currently private. Authorized collaborators can use its
[private issue tracker](https://github.com/cubearc/play/issues/new) to ask maintainers
for a confidential reporting channel. Include the affected version and a request
for coordination; withhold exploit details, credentials and private investigation
files until maintainers establish the channel. There is no configured security
email address or verified external-reporter channel.

GitHub private vulnerability reporting is a public-repository feature. Do not assume
the advisory reporting form is available here. If maintainers later make the project
public, they must enable and verify private reporting, update this policy and monitor
reports before directing reporters to that form. Repository visibility is a separate
maintainer decision. Never post sensitive details in public issues.

## Local trust boundary

Cube Play is intended for use on the same machine as its browser. Host exposure must
remain loopback-only. Host/Origin validation, same-site HttpOnly cookies and short-lived
process credentials protect against unauthorized browser access and DNS rebinding.
The session link grants access to that local process; do not share it or include it
in diagnostics. A restart rotates the credential. Session cookies use HTTP on loopback;
they are not a remote-access or multi-user authentication system.

The host launcher may control a local Docker daemon. Application containers never
receive its socket or equivalent privileges. The launcher rejects remote Docker
endpoints and source roots containing the active or standard Docker socket paths.
Only the selected source directory and minimal workspace metadata are mounted,
read-only. Generated data and artifacts use separate named volumes.

The runtime is non-root, read-only and resource-bounded, without privileged mode.
On Unix it uses the invoking user's UID/GID so private sources remain readable
without changing their permissions; running the launcher as root is rejected.
Unsupported Docker file sharing or user mappings fail explicitly. Docker isolation
is defense in depth, not an absolute security boundary. File discovery, previews,
terminal execution and challenge validation remain separate implementation work;
passing a challenge path currently selects it without executing or importing it.
Future changes must preserve the [documented boundaries](docs/architecture.md).

There are no implicit telemetry calls, uploads or Cloud services. Dependency/image
downloads occur during setup and builds. Structured diagnostics must omit session
credentials, command contents and private paths; the explicit startup session link
is the only intentional credential output. Security fixes currently target `main`.
