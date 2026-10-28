Canonical `api.proto`, `types.proto` and `events.proto` copied from
`lightningdevkit/ldk-server/ldk-server-grpc/src/proto` at
`762bc473342c8f945475785ec6743a9b26f56aff`, under the upstream MIT license.
Trailing whitespace is normalized. The root build.rs generates the tonic client. Update these files together
with the request-binding dependency and Docker image revision.

The upstream Rust client's HTTP client constructor does not explicitly select
Rustls. When its reqwest dependency is unified with SwissKnife's native-TLS
Socket.IO dependencies, its transport changes. This adapter instead uses the
existing tonic/Rustls transport and the upstream request-binding library.
