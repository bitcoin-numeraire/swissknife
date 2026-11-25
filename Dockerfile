FROM rust:1.100.0-trixie@sha256:223cc32ddca8f3ee90c436dbc94b6b5f052ea7f9653c5201e9612f08ad7d21b8 AS chef
# The published cargo-chef lockfile pins vulnerable crossbeam-epoch.
# Keep the tool version fixed while resolving compatible security fixes.
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
    rm -rf /var/lib/apt/lists/* && \
    cargo install cargo-chef --version 0.1.78 --message-format=json
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 

# Install required packages
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
COPY ./crates ./crates

# Fetch dependencies with cargo cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --locked --release --recipe-path recipe.json

# Copy the source code
COPY . .

# Build the project with cargo cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --locked --release && \
    cp /app/target/release/swissknife /tmp/swissknife

# Node.js stage for building the dashboard
FROM node:24.22.3-alpine3.24@sha256:116b800c3754b440ee0aa3ed2f20f5bf3d99a554bfd9adeea09ea8017f6e85f5 AS dashboard-builder
WORKDIR /app/dashboard

# Copy package files first for better caching
COPY ./dashboard/package.json ./dashboard/yarn.lock* ./dashboard/.yarnrc.yml* ./

# Next.js collects completely anonymous telemetry data about general usage.
# Learn more here: https://nextjs.org/telemetry
# Uncomment the following line in case you want to disable telemetry during the build.
ENV NEXT_TELEMETRY_DISABLED=1
ENV NODE_OPTIONS=--max-old-space-size=4096

# The bundled UI connects to its own backend and uses the local account flow.
# Explicit build arguments take precedence over dashboard/.env.production.
ARG NEXT_PUBLIC_SERVER_URL=""
ARG NEXT_PUBLIC_AUTH_METHOD=jwt
ARG NEXT_PUBLIC_DOMAIN=""
ENV NEXT_PUBLIC_SERVER_URL=$NEXT_PUBLIC_SERVER_URL
ENV NEXT_PUBLIC_AUTH_METHOD=$NEXT_PUBLIC_AUTH_METHOD
ENV NEXT_PUBLIC_DOMAIN=$NEXT_PUBLIC_DOMAIN
ENV BUILD_STATIC_EXPORT=true

# Install dependencies with yarn cache mount
RUN --mount=type=cache,target=/root/.yarn \
    corepack enable && yarn install --immutable

# Copy the dashboard source code
COPY ./dashboard .

# Build the dashboard with Next.js cache mount
# This will create /app/dashboard/out directory (static export)
RUN --mount=type=cache,target=/app/dashboard/.next/cache \
    yarn build

# Use debian:trixie-slim as minimal runtime base
# Trixie = Debian 13
# Match the builder's Debian release for GLIBC and OpenSSL compatibility.
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2 AS runtime-base

# Install minimal runtime dependencies
# ca-certificates: Required for HTTPS connections
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates tzdata && \
    rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder /tmp/swissknife /usr/local/bin/swissknife
COPY ./config/default.toml /config/default.toml

# Set the environment variable for production
ENV RUN_MODE=production

# Set the entrypoint
ENTRYPOINT ["swissknife"]

FROM runtime-base AS swissknife-server

ENV RUN_MODE=production
ENV SWISSKNIFE_DASHBOARD_DIR=""

FROM runtime-base AS swissknife

COPY --from=dashboard-builder /app/dashboard/out /var/www/swissknife-dashboard
ENV SWISSKNIFE_DASHBOARD_DIR="/var/www/swissknife-dashboard"
