FROM rust:1.99.1-trixie@sha256:931c2c0531a8b143e044f0f3914dfe802f15bb3d1955f0b657e9359327ac2ffc AS chef
RUN cargo install --locked cargo-chef --version 0.1.78
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
FROM node:24.22.1-alpine3.24@sha256:06d7adf3e0ef61a13f1d28682726366f97a15aa1888c934d3ecec86c33918b53 AS dashboard-builder
WORKDIR /app/dashboard

# Copy package files first for better caching
COPY ./dashboard/package.json ./dashboard/yarn.lock* ./dashboard/.yarnrc.yml* ./

# Next.js collects completely anonymous telemetry data about general usage.
# Learn more here: https://nextjs.org/telemetry
# Uncomment the following line in case you want to disable telemetry during the build.
ENV NEXT_TELEMETRY_DISABLED=1
ENV NODE_OPTIONS=--max-old-space-size=4096

# Build in static export mode for bundling with backend
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
FROM debian:trixie-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad AS runtime-base

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
