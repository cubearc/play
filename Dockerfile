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
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
RUN cargo +${RUST_VERSION} build --release --locked -p play-local

FROM debian:bookworm-slim@sha256:7ff6ba31527b9e336162fa061746c366c4b2032f4d2cff2078bea43f78efe3f8
WORKDIR /app
COPY --from=backend /build/target/release/play-local /usr/local/bin/play-local
COPY --from=frontend /build/frontend/dist ./frontend
ENV CUBE_PLAY_BIND=0.0.0.0:7331 \
    CUBE_PLAY_ORIGIN=http://127.0.0.1:7331 \
    CUBE_PLAY_FRONTEND_DIR=/app/frontend \
    CUBE_PLAY_CONTAINER=1
USER 65532:65532
EXPOSE 7331
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=6 CMD ["play-local", "--healthcheck"]
ENTRYPOINT ["play-local"]
