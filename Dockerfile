FROM lukemathwalker/cargo-chef:0.1.67-rust-1.79 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 

# Install required packages
RUN apt-get update && apt-get install -y protobuf-compiler

COPY --from=planner /app/recipe.json recipe.json
COPY ./migration ./migration

# Fetch dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Copy the source code
COPY . .

# Build the project
RUN cargo build --release

# Use a minimal base image for the final stage
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder /app/target/release/swissknife /usr/local/bin
COPY ./config/default.toml /config/default.toml

# Set the environment variable for production
ENV RUN_MODE=production

# Set the entrypoint
ENTRYPOINT ["swissknife"]
