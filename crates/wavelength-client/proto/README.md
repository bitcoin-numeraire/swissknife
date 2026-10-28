`wallet.proto` is copied verbatim from
`lightninglabs/wavelength/rpc/wavewalletrpc/wallet.proto` at
`b3ad398d6a9c31c513d62583bd9b3166bbbcddc0` (MIT; see LICENSE).

To update, fetch an explicitly selected upstream revision, replace the file,
record that revision here, and run the crate tests. `build.rs` generates the
Rust types and gRPC clients/servers using `protoc`; generated files stay in
Cargo's build directory. Pin the matching daemon revision in Docker tests.
