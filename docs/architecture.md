# Architecture

Cube Play is a local application with one Rust API and one React interface.
The application contains `crates/play-local`, the host-side `crates/play-launcher`
and `frontend`; it has no database,
operation registry, challenge parser or idle execution service.

## Request boundary

Native serving defaults to `127.0.0.1:7331`. The production backend serves the Vite
build, public readiness at `/healthz`, session exchange at `POST /api/session` and
authenticated health at `/api/health`. All surfaces validate the exact configured
Host and any Origin; mutations require the exact Origin. Cross-site fetches and
forwarded-host headers are rejected. There is no CORS allowance.

A random per-process capability appears only in the explicit startup URL fragment.
The frontend removes it before sending requests, exchanges it in an Authorization
header and receives an HttpOnly, SameSite=Strict session cookie scoped to `/api`.
Cookie names include the local port so simultaneous workspaces do not overwrite
each other's sessions. Authenticated health includes bounded UUID/name context;
public readiness never includes workspace metadata or host paths.
Credential comparison is constant-time. No token is stored in browser persistent
storage; refresh uses the cookie, and process restart invalidates it. This protects
the local browser boundary, not an attacker controlling the host account.

Responses use a restrictive Content Security Policy, no-referrer and no-store.
API bodies are limited to 1 KiB at this stage. Static serving is limited to the
trusted frontend build directory; user workspaces must never be served from it.
Diagnostics log process events without request URLs, tokens or payloads.

## Runtime ownership

The complete local runtime has two containers:

| Component | Responsibility |
| --- | --- |
| Play Local | Frontend, Axum API, workspace authorization, typed Core calls, SQLite, notes/artifacts and terminal coordination |
| Cube Runner | Real Cube CLI execution, PTYs, restricted workspace access and process/resource boundaries |

The foundation runs Play Local. Runner is introduced with actual terminal sessions,
not as an unused placeholder. The trusted host-side Play launcher owns Docker;
the Cube CLI delegates `cube play [path]` to it. Neither container receives a Docker
socket or equivalent host control. Runner networking must be disabled by default;
its private session transport must work within that constraint, such as a constrained
Unix socket. No runner port is published.

The Compose development default uses UID/GID 65532; the host launcher builds Unix
runtime images for the calling user's UID/GID so private source files remain readable
without chmod or root. Windows uses an unprivileged UID with Desktop sharing. Generated
Compose configuration publishes Play Local only on `127.0.0.1`
with dropped capabilities/no-new-privileges, a read-only root filesystem, bounded
CPU/memory/processes and a Compose bridge network. Play Local needs a reachable
loopback-published HTTP port; Docker Desktop versions can make internal-only bridges
unreachable through host port forwarding. This bridge does not enforce an egress
firewall. The application has no outgoing network APIs or automatic reporting;
Runner will use its separate network-disabled boundary. Container wildcard binding requires
explicit process configuration; never pair it with non-loopback host publication.
Images and build dependencies are pinned by verified digest/lockfile. Docker is
defense in depth, not an absolute sandbox.

The [launcher contract](launcher.md) defines installation, path resolution, stable
workspace IDs, bounded host commands, locking, reuse, cancellation and platform limits.
Local host JSON configuration authorizes roots and identifies runtime resources;
investigation records remain the responsibility of SQLite. The source is read-only,
data/artifacts use separate named volumes, and no shutdown removes them. Ownership
labels and runtime configuration are checked before reuse. Remote Docker contexts
and source roots containing Docker control sockets are rejected.

## Core and storage boundaries

Use a versioned direct `cube-core` dependency for algorithms, semantic types,
validation and operation metadata. Descriptors describe functions, not universal
dispatch. Existing pipeline execution applies only to its bounded adapter allowlist;
image/media operations can use direct typed APIs. Preserve explicit binary/text
representations, stable IDs and engine/adapter compatibility versions. Do not parse
human terminal output into Lab results or copy algorithms into TypeScript.

The persistence capability will add SQLx/SQLite with versioned migrations for local
metadata, notes, history, progress and references. Large binary artifacts live in
separate filesystem storage on persistent volumes. Authorize source roots explicitly
and mount them read-only by default. Output storage is separate; exports require
explicit non-destructive destination authorization. Resolve symlinks, reject traversal
and bound discovery, parsing and operations before processing untrusted files.

Challenge definitions are non-executable data, separate from private progress/answers.
Current Core JSON recipes are not stage manifests or a full `.cube` DSL. Finalize a
minimal versioned compatibility/security contract before challenge import. Free Lab
must work independently of challenges. Cloud services, remote execution, collaboration
and speculative synchronization frameworks are outside the local MVP.
