ARG RUST_VERSION=1.99.1
FROM node:24-bookworm-slim@sha256:55fb5f5a527ea2c5c4a95f963e48ef90aa38c7b346054fdc8ee9957a2b55d205 AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:${RUST_VERSION}-bookworm@sha256:48b7f435b8fc6378ce43b44f15e3a47fd8919b2825f835c39bd4bd3da50af5b8 AS backend
ARG RUST_VERSION
WORKDIR /build
COPY --from=cube_engine /Cargo.toml /cube/Cargo.toml
COPY --from=cube_engine /Cargo.lock /cube/Cargo.lock
COPY --from=cube_engine /crates/cube-cli/ /cube/crates/cube-cli/
COPY --from=cube_engine /bindings/python/Cargo.toml /cube/bindings/python/Cargo.toml
COPY --from=cube_engine /bindings/python/src/lib.rs /cube/bindings/python/src/lib.rs
COPY --from=cube_engine /crates/cube-core/Cargo.toml /cube/crates/cube-core/Cargo.toml
COPY --from=cube_engine /crates/cube-core/src/ /cube/crates/cube-core/src/
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
# COPY snapshots can have older mtimes than cached outputs; refresh local sources.
RUN --mount=type=cache,id=cube-play-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cube-play-target-r4,sharing=locked,target=/build/target \
    find /cube/crates/cube-cli /cube/crates/cube-core crates -type f -name '*.rs' -exec touch {} + && \
    cargo +${RUST_VERSION} build --release --locked --manifest-path /cube/Cargo.toml -p cube-cli --target-dir /build/target && \
    cargo +${RUST_VERSION} build --release --locked -p play-local -p play-runner && \
    mkdir /binaries && cp /build/target/release/play-local /build/target/release/cube-runner /build/target/release/cube /binaries/

FROM backend AS runner-tests
RUN --mount=type=cache,id=cube-play-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cube-play-target-r4,sharing=locked,target=/build/target \
    find crates/play-runner -type f -name '*.rs' -exec touch {} + && \
    CUBE_RUNNER_CUBE=/binaries/cube cargo +${RUST_VERSION} test --locked -p play-runner

FROM backend AS lab-tests
COPY tests/fixtures/ ./tests/fixtures/
RUN --mount=type=cache,id=cube-play-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cube-play-target-r4,sharing=locked,target=/build/target \
    find crates/play-local -type f -name '*.rs' -exec touch {} + && \
    cargo +${RUST_VERSION} test --locked -p play-local --lib --tests

FROM debian:bookworm-slim@sha256:7ff6ba31527b9e336162fa061746c366c4b2032f4d2cff2078bea43f78efe3f8
ARG CUBE_PLAY_UID=65532
ARG CUBE_PLAY_GID=65532
LABEL org.cubearc.play.protocol="1" org.cubearc.play.version="0.1.0" org.cubearc.play.runtime-revision="12"
RUN test "${CUBE_PLAY_UID}" -gt 0 && \
    mkdir -p /var/lib/cube-play/artifacts /var/lib/cube-play/imports /workspace/data/imports /workspace/output /run/cube-play/terminal /workspace/source && \
    chown -R "${CUBE_PLAY_UID}:${CUBE_PLAY_GID}" /var/lib/cube-play /workspace/data /workspace/output /run/cube-play/terminal && \
    chmod 700 /var/lib/cube-play/imports /workspace/data/imports /run/cube-play/terminal
WORKDIR /app
COPY --from=backend /binaries/ /usr/local/bin/
COPY --from=frontend /build/frontend/dist ./frontend
ENV CUBE_PLAY_BIND=0.0.0.0:7331 \
    CUBE_PLAY_ORIGIN=http://127.0.0.1:7331 \
    CUBE_PLAY_FRONTEND_DIR=/app/frontend \
    CUBE_PLAY_CONTAINER=1 \
    CUBE_PLAY_DATA=/var/lib/cube-play/imports
USER ${CUBE_PLAY_UID}:${CUBE_PLAY_GID}
EXPOSE 7331
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=6 CMD ["play-local", "--healthcheck"]
ENTRYPOINT ["play-local"]
