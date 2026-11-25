#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
runtime="${root}/runtime/cln"
mkdir -p "${runtime}"
# Write the bind-mounted file in place so an existing container sees new settings.
cat "${root}/config/cln/config" > "${runtime}/node.conf"
if [[ -s "${runtime}/fronting-node" ]]; then
  printf '\npayment-fronting-node=%s\n' "$(cat "${runtime}/fronting-node")" >> "${runtime}/node.conf"
fi
