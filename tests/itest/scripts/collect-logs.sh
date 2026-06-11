#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ITEST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${ITEST_DIR}/../.." && pwd)"
COMPOSE_FILE="${SWISSKNIFE_ITEST_COMPOSE_FILE:-${ITEST_DIR}/docker-compose.yml}"
PROJECT="${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-itest}"
ARTIFACT_DIR="${1:-${REPO_ROOT}/target/itest/dependency-logs-$(date +%s)}"

COMPOSE=(docker compose -p "${PROJECT}" -f "${COMPOSE_FILE}")
mkdir -p "${ARTIFACT_DIR}"

"${COMPOSE[@]}" ps >"${ARTIFACT_DIR}/docker-compose.ps.txt" 2>&1 || true
"${COMPOSE[@]}" logs --no-color >"${ARTIFACT_DIR}/docker-compose.log" 2>&1 || true

for service in postgres bitcoind lnd cln; do
  "${COMPOSE[@]}" logs --no-color "${service}" >"${ARTIFACT_DIR}/${service}.log" 2>&1 || true
done

find "${ITEST_DIR}/runtime" -maxdepth 4 -type f \
  \( -name '*.log' -o -name 'debug.log' -o -name 'config' \) \
  -print >"${ARTIFACT_DIR}/runtime-files.txt" 2>/dev/null || true

echo "Integration dependency logs collected in ${ARTIFACT_DIR}"
