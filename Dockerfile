# syntax=docker/dockerfile:1
#
# Multi-stage build: SPA (Node) -> server (Rust) -> slim runtime image.
# Built and run by docker-compose.yml; see README "Self-hosting".
#
# Final layout, which the STATIC_DIR / SEED_DIR defaults below must match:
#   /app/blackout        the server binary
#   /app/dist            the built SPA          -> STATIC_DIR=/app/dist
#   /app/data/seed       committed seed files   -> SEED_DIR=/app/data/seed
# The SQLite database lives on a mounted volume, NOT in the image:
#   /data/mena_ai.db                            -> DATABASE_PATH=/data/mena_ai.db
#
# /data must be a real volume (docker-compose.yml mounts `blackout-data`).
# Without one, every container re-creation starts from an empty database —
# and three fetchers keep history upstream will never serve again, on top of
# the satellite catalog. The server prints a WARNING block at boot when it
# opens a database with no previous state.


# ---------------------------------------------------------------------------
# Stage 1 — build the SPA.
#
# Vite 5 needs Node 18+; 22 is the current LTS. `npm ci` (not `install`) so the
# committed package-lock.json is authoritative.
# ---------------------------------------------------------------------------
FROM node:22-bookworm-slim AS frontend

WORKDIR /build

# Dependencies first: this layer is cached until the lockfile actually changes,
# which is what keeps iteration on source cheap.
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./

# Baked into the bundle at build time (see frontend/.env.example). Both are
# optional and public by nature: empty VITE_API_BASE_URL means the SPA calls
# the API on its own origin, which is what this image serves.
ARG VITE_API_BASE_URL=
ARG VITE_CESIUM_ION_TOKEN=
ENV VITE_API_BASE_URL=$VITE_API_BASE_URL \
    VITE_CESIUM_ION_TOKEN=$VITE_CESIUM_ION_TOKEN

# Output is ~16 MB, most of it the Cesium assets that vite-plugin-cesium copies
# in; `npm run build` also writes a `.gz` beside each asset, which the server
# hands out as-is.
RUN npm run build


# ---------------------------------------------------------------------------
# Stage 2 — build the server.
#
# backend/Cargo.toml declares `edition = "2024"`, which needs Rust >= 1.85.
# 1.91 is the version the crate is verified against (`--locked`, clean).
# ---------------------------------------------------------------------------
FROM rust:1.91-slim-bookworm AS backend

# Two native dependencies, both confirmed from Cargo.lock:
#   - libsqlite3-sys is used via rusqlite's `bundled` feature, so SQLite is
#     compiled from C source and needs a C toolchain.
#   - reqwest 0.12 with default features resolves to native-tls -> openssl-sys,
#     so OpenSSL headers are needed here (and libssl3 at runtime, below).
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# The crate lives at /build/backend because src/api/methodology.rs embeds
# ../../../methodology/*.md with include_str! — i.e. /build/methodology.
WORKDIR /build/backend

# Dependencies are compiled against a stub binary first, so the ~200-crate
# release build lands in its own layer keyed only on Cargo.toml/Cargo.lock.
# Without this split, editing one line of Rust would rebuild every dependency.
#
# --locked: Cargo.lock is committed, so the dependency set is reproducible and
# a drifted lockfile fails loudly instead of resolving something newer.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY methodology /build/methodology
COPY backend/src ./src
COPY backend/data ./data
# `touch` so cargo does not mistake the real main.rs for the already-compiled
# stub — mtime is what invalidates its fingerprint.
RUN touch src/main.rs && cargo build --release --locked


# ---------------------------------------------------------------------------
# Stage 3 — runtime.
#
# Neither toolchain ships in the final image; only the artefacts do.
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# ca-certificates is not optional: every fetcher talks HTTPS, and without a
# trust store all of them fail at runtime while the process itself looks
# healthy. libssl3 is the runtime half of the openssl-sys link above — the
# binary will not start without it. curl is for the container healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend  /build/backend/target/release/backend  /app/blackout
COPY --from=backend  /build/backend/data/seed               /app/data/seed
COPY --from=frontend /build/dist                            /app/dist

# The server runs unprivileged, but the container must *start* as root.
#
# Chowning /data at build time only covers a fresh Docker named volume, which
# inherits the image's ownership. A bind mount, or a volume created by an
# earlier image, can be root-owned — and the unprivileged process then cannot
# create the database, which is a crash loop, not a degraded start.
#
# So: fix the mount's ownership at boot, then drop to the unprivileged user for
# the server itself. `setpriv` ships in the base image (util-linux), and `exec`
# hands the process over so signals reach the server directly.
RUN useradd --system --uid 10001 --create-home blackout \
    && mkdir -p /data \
    && chown -R blackout:blackout /data /app

RUN printf '%s\n' \
    '#!/bin/sh' \
    'set -e' \
    '# Only the mount point and its direct contents — a handful of SQLite files,' \
    '# so this stays instant even with a large database.' \
    'chown blackout:blackout /data 2>/dev/null || true' \
    'chown blackout:blackout /data/* 2>/dev/null || true' \
    'exec setpriv --reuid=10001 --regid=10001 --clear-groups /app/blackout' \
    > /usr/local/bin/entrypoint.sh \
    && chmod +x /usr/local/bin/entrypoint.sh

# Defaults baked in so the image is correct on its own; every one is still
# overridable at runtime. SEED_DIR / STATIC_DIR are absolute here on purpose:
# the code defaults are relative to the working directory, which is a
# laptop-shaped assumption, and a wrong SEED_DIR is fatal (the container
# crash-loops rather than starting degraded).
ENV BIND_ADDR=0.0.0.0 \
    PORT=3001 \
    SEED_DIR=/app/data/seed \
    STATIC_DIR=/app/dist \
    DATABASE_PATH=/data/mena_ai.db

VOLUME ["/data"]
EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/health/live" >/dev/null || exit 1

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
