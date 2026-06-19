#!/usr/bin/env bash
#
# Refresh the vendored Lightning gRPC proto definitions, pinned to the deployed
# node versions. Run via `make protos`. After running, review the diff and run
# `make build` to regenerate the tonic bindings.
#
# Override the versions:
#   CLN_VERSION=v26.06.1 LND_VERSION=v0.21.0-beta make protos
#
set -euo pipefail

# Run from the repo root regardless of where the script is invoked.
cd "$(dirname "$0")/.."

CLN_VERSION="${CLN_VERSION:-v26.06.1}"
LND_VERSION="${LND_VERSION:-v0.21.0-beta}"

cln_url="https://raw.githubusercontent.com/ElementsProject/lightning/${CLN_VERSION}/cln-grpc/proto"
lnd_url="https://raw.githubusercontent.com/lightningnetwork/lnd/${LND_VERSION}/lnrpc"
cln_dir="src/infra/lightning/cln/proto"
lnd_dir="src/infra/lightning/lnd/proto"

fetch() { echo "  -> $2"; curl -sSfL "$1" -o "$2"; }

echo ">>> CLN ${CLN_VERSION}"
fetch "${cln_url}/node.proto"       "${cln_dir}/node.proto"
fetch "${cln_url}/primitives.proto" "${cln_dir}/primitives.proto"

echo ">>> LND ${LND_VERSION}"
mkdir -p "${lnd_dir}/signrpc"
fetch "${lnd_url}/lightning.proto"            "${lnd_dir}/lightning.proto"
fetch "${lnd_url}/invoicesrpc/invoices.proto" "${lnd_dir}/invoices.proto"
fetch "${lnd_url}/routerrpc/router.proto"     "${lnd_dir}/router.proto"
fetch "${lnd_url}/walletrpc/walletkit.proto"  "${lnd_dir}/walletkit.proto"
fetch "${lnd_url}/signrpc/signer.proto"       "${lnd_dir}/signrpc/signer.proto"

echo ">>> Done. Review the diff, then run 'make build' to regenerate the bindings."
