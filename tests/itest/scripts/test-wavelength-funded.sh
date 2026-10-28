#!/usr/bin/env bash
# Explicitly funded public-test-network run. Never import an existing seed.
# With no argument, create two fresh test wallets; with a state directory,
# resume the same wallets. Wallet volumes and credentials survive exit.
set -euo pipefail
umask 077
ITEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "${ITEST_DIR}/runtime"
if [[ $# -eq 0 ]]; then
  state="$(mktemp -d "${ITEST_DIR}/runtime/wavelength-funded.XXXXXX")"
  network="${WAVELENGTH_PUBLIC_NETWORK:-signet}"
  case "${network}" in signet|testnet) ;; *) echo "Test networks only" >&2; exit 2 ;; esac
  project="swissknife-wave-funded-$(basename "${state}" | tr '[:upper:].' '[:lower:]-')"
  printf '%s\n' "${project}" > "${state}/project"
  printf '%s\n' "${network}" > "${state}/network"
  export WAVELENGTH_FUNDED_PHASE=create
elif [[ $# -eq 1 ]]; then
  state="$(cd "$1" && pwd)"
  project="$(cat "${state}/project")"
  network="$(cat "${state}/network")"
  case "${project}" in swissknife-wave-funded-wavelength-funded-*) ;; *) echo "Not a funded-test project" >&2; exit 2 ;; esac
  case "${network}" in signet|testnet) ;; *) echo "Test networks only" >&2; exit 2 ;; esac
  export WAVELENGTH_FUNDED_PHASE=resume
else
  echo "usage: $0 [existing-test-state-directory]" >&2; exit 2
fi
export WAVELENGTH_PUBLIC_NETWORK="${network}"
export WAVELENGTH_FUNDED_STATE_DIR="${state}"
export WAVELENGTH_FUNDED_PROJECT="${project}"
export WAVELENGTH_FUNDED_PAYER_PORT="${WAVELENGTH_FUNDED_PAYER_PORT:-19949}"
export WAVELENGTH_FUNDED_PROVIDER_PORT="${WAVELENGTH_FUNDED_PROVIDER_PORT:-19959}"
COMPOSE=(docker compose -p "${project}" -f "${ITEST_DIR}/docker-compose.yml" -f "${ITEST_DIR}/docker-compose.wavelength-funded.yml")
cleanup() {
  "${COMPOSE[@]}" stop waved-funded-payer waved-funded-provider >/dev/null || true
  if [[ "${SWISSKNIFE_ITEST_DATABASE:-sqlite}" == postgres ]]; then
    "${COMPOSE[@]}" stop postgres >/dev/null || true
  fi
  printf 'Preserved test state: %s\nResume: %s %s\n' "${state}" "$0" "${state}"
}
trap cleanup EXIT
"${ITEST_DIR}/scripts/build-provider-image.sh" wavelength
"${COMPOSE[@]}" up -d waved-funded-payer waved-funded-provider
if [[ "${WAVELENGTH_FUNDED_PHASE}" == resume ]]; then
  "${COMPOSE[@]}" restart waved-funded-payer waved-funded-provider
fi
for role in payer provider; do
  mkdir -p "${state}/${role}"
  cp "${ITEST_DIR}/../../crates/wavelength-client/tests/fixtures/cert.pem" "${state}/${role}/tls.cert"
  deadline=$((SECONDS + 60))
  until "${COMPOSE[@]}" cp "waved-funded-${role}:/data/data/${network}/admin.macaroon" "${state}/${role}/admin.macaroon" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then echo "waved did not initialize credentials" >&2; exit 1; fi
    sleep 1
  done
done
if [[ "${SWISSKNIFE_ITEST_DATABASE:-sqlite}" == postgres ]]; then
  export SWISSKNIFE_ITEST_POSTGRES_PORT="${SWISSKNIFE_ITEST_POSTGRES_PORT:-55433}"
  "${COMPOSE[@]}" up -d --wait postgres
  export SWISSKNIFE_ITEST_POSTGRES_ADMIN_URL="postgres://postgres:postgres@127.0.0.1:${SWISSKNIFE_ITEST_POSTGRES_PORT}/postgres"
  export SWISSKNIFE_ITEST_POSTGRES_BASE_URL="postgres://postgres:postgres@127.0.0.1:${SWISSKNIFE_ITEST_POSTGRES_PORT}"
fi
printf 'Test network: %s; Docker project: %s\n' "${network}" "${project}"
printf 'Wallets and state are test-only. No seed is imported or printed.\n'
cargo test --locked --features funded-itest --test wavelength_funded -- --nocapture
