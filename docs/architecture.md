# Architecture

Cube Play is a local application with one Rust API and one React interface.
The Rust workspace contains `crates/play-local`, the host-side `crates/play-launcher`
and `crates/play-runner`, alongside the React `frontend`. Local state uses SQLx/SQLite and private filesystem storage; there is
no second operation registry or idle execution service.

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
Session bodies are limited to 1 KiB. Streaming imports have a 32 MiB/30-second limit;
URLs/queries are limited to 16 KiB. Static serving is limited to the
trusted frontend build directory; user workspaces must never be served from it.
Diagnostics log process events without request URLs, tokens or payloads.

The [workspace boundary](workspace.md) opens directory capabilities once and
rejects symlinks/special files. Two bounded workers handle shallow discovery and
Core reader APIs. Streaming responses retain their permit until closed. Imports
commit atomically into separate private storage; source files are never overwritten.

## Runtime ownership

The complete local runtime has two containers:

| Component | Responsibility |
| --- | --- |
| Play Local | Frontend, Axum API, workspace authorization, typed Core calls, SQLite, notes/artifacts and terminal coordination |
| Cube Runner | Real Cube CLI execution, PTYs, restricted workspace access and process/resource boundaries |

The local runtime runs both Play Local and the [isolated Cube terminal](terminal.md). The trusted host-side Play launcher owns Docker;
the Cube CLI delegates `cube play [path]` to it. Neither container receives a Docker
socket or equivalent host control. Runner networking must be disabled by default;
its private session transport must work within that constraint, the constrained
Unix socket described in the terminal contract. No runner port is published.

The Compose development default uses UID/GID 65532; the host launcher builds Unix
runtime images for the calling user's UID/GID so private source files remain readable
without chmod or root. Windows uses an unprivileged UID with Desktop sharing. Generated
Compose configuration publishes Play Local only on `127.0.0.1`
with dropped capabilities/no-new-privileges, a read-only root filesystem, bounded
CPU/memory/processes and a Compose bridge network. Play Local needs a reachable
loopback-published HTTP port; Docker Desktop versions can make internal-only bridges
unreachable through host port forwarding. This bridge does not enforce an egress
firewall. The application has no outgoing network APIs or automatic reporting;
Runner uses its separate network-disabled boundary and Landlock file restrictions. Container wildcard binding requires
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

The prerelease consumes the sibling Cube 0.18.0 library directly, with a pinned CI
checkout. Docker's `cube_engine` context contains workspace metadata, Core/CLI
sources and the minimal Python member manifest/entry point needed by Cargo.
Credentials, history, fixtures and build caches are excluded. Only compiled Rust
binaries enter the runtime image.
A maintainer must configure read-only source access for hosted CI while Core is private.

The [typed Lab API](lab-api.md) derives discovery from Core, resolves authorized
inputs and executes one bounded operation in a short-lived backend worker. It
terminates and reaps workers on timeout/cancellation; generated result bundles
use the existing artifact volume, separate from source and private SQLite state.

Use the versioned direct `cube-core` dependency for algorithms, semantic types,
validation and operation metadata. Descriptors describe functions, not universal
dispatch. Existing pipeline execution applies only to its bounded adapter allowlist;
image/media operations can use direct typed APIs. Preserve explicit binary/text
representations, stable IDs and engine/adapter compatibility versions. Do not parse
human terminal output into Lab results or copy algorithms into TypeScript.

The [persistence foundation](persistence.md) uses SQLx/SQLite with versioned migrations
for local metadata, notes, sessions, progress, saved navigation and imported-file
references. Large binary artifacts live in
separate filesystem storage on persistent volumes. Authorize source roots explicitly
and mount them read-only by default. Output storage is separate; exports require
explicit non-destructive destination authorization. Resolve symlinks, reject traversal
and bound discovery, parsing and operations before processing untrusted files.

[Local challenge definitions](challenges.md) are bounded non-executable JSON data,
separate from private progress/answers. Current Core JSON recipes are not stage
manifests or a full `.cube` DSL. Import never invokes an executor or fetches assets. Free Lab
must work independently of challenges. Cloud services, remote execution, collaboration
and speculative synchronization frameworks are outside the local MVP.

The [visual Lab](visual-lab.md) consumes typed Core responses. Canvas 2D only
draws a bounded viewport; ECharts plots returned counts. Transient pixel/histogram/
comparison probes use the existing worker without allocating permanent result bundles.

The [notebook](notebook.md) names immutable Core result bundles in SQLite and
serializes note autosave with revision conflicts. Exports are authenticated browser
downloads; containers never receive a host write destination.

Private stage candidates append typed selection/conversion revisions in the same
SQLite database. They reference named immutable Core bundles and preserve lineage;
Core decodes explicitly selected Base64 fields. No verification or submission occurs.

[Typed explorations](explorations.md) separate immutable configuration revisions
from actual execution attempts. Named-port schemas come from versioned Core
contracts; Play adds tested interface adapters, capability resolution and UI
controls. Execution evidence points to existing immutable artifact bundles instead
of duplicating result bytes or private parameters in SQLite.
