# The upstream cache stage at this revision omits ldk-server-macaroons.
# Build the complete pinned workspace instead.
FROM rust:1.99.1-trixie@sha256:931c2c0531a8b143e044f0f3914dfe802f15bb3d1955f0b657e9359327ac2ffc AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p ldk-server -p ldk-server-cli

FROM debian:trixie-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/ldk-server /usr/local/bin/
COPY --from=build /src/target/release/ldk-server-cli /usr/local/bin/
ENTRYPOINT ["ldk-server"]
