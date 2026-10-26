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

The runtime is non-root, read-only and resource-bounded, with no privileged mode,
host paths or Docker socket. Docker isolation is defense in depth, not an absolute
security boundary. The foundation does not open investigation files or execute
imported content. Future filesystem, terminal and challenge changes must preserve
the [documented boundaries](docs/architecture.md).

There are no implicit telemetry calls, uploads or Cloud services. Dependency/image
downloads occur during setup and builds. Structured diagnostics must omit session
credentials, command contents and private paths; the explicit startup session link
is the only intentional credential output. Security fixes currently target `main`.
