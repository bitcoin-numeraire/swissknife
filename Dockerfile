FROM lukemathwalker/cargo-chef:0.1.73-rust-1.90 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 

# Install required packages
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
COPY ./migration ./migration

# Fetch dependencies with cargo cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

# Copy the source code
COPY . .

# Build the project with cargo cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp /app/target/release/swissknife /tmp/swissknife

# Node.js stage for building the dashboard
FROM node:24-alpine AS dashboard-builder
WORKDIR /app/dashboard

# Copy package files first for better caching
COPY ./dashboard/package.json ./dashboard/yarn.lock* ./dashboard/.yarnrc.yml* ./

# Next.js collects completely anonymous telemetry data about general usage.
# Learn more here: https://nextjs.org/telemetry
# Uncomment the following line in case you want to disable telemetry during the build.
ENV NEXT_TELEMETRY_DISABLED=1

# Build in static export mode for bundling with backend
ENV BUILD_STATIC_EXPORT=true

# Install dependencies with yarn cache mount
RUN --mount=type=cache,target=/root/.yarn \
    corepack enable && yarn --frozen-lockfile

# Copy the dashboard source code
COPY ./dashboard .

# Build the dashboard with Next.js cache mount
# This will create /app/dashboard/out directory (static export)
RUN --mount=type=cache,target=/app/dashboard/.next/cache \
    yarn build

# Use same Debian base as builder for GLIBC compatibility
# The cargo-chef:rust-1.90 image is based on Debian trixie (testing)
FROM debian:trixie-slim AS runtime-base

# Install minimal runtime dependencies  
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

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
