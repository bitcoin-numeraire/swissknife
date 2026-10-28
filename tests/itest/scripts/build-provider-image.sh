#!/usr/bin/env bash
set -euo pipefail
ITEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "${1:-}" in
  ldk)
    repo="https://github.com/lightningdevkit/ldk-server.git"
    revision="762bc473342c8f945475785ec6743a9b26f56aff"
    image="swissknife-itest-ldk:762bc473"
    ;;
  wavelength)
    repo="https://github.com/lightninglabs/wavelength.git"
    revision="b3ad398d6a9c31c513d62583bd9b3166bbbcddc0"
    image="swissknife-itest-waved:b3ad398d"
    ;;
  *) echo "usage: $0 ldk|wavelength" >&2; exit 2 ;;
esac
if docker image inspect "${image}" >/dev/null 2>&1; then exit 0; fi
source_dir="${ITEST_DIR}/runtime/provider-sources/${1}"
mkdir -p "${source_dir}"
if [[ ! -d "${source_dir}/.git" ]]; then git -C "${source_dir}" init -q; fi
git -C "${source_dir}" fetch -q --depth 1 "${repo}" "${revision}"
git -C "${source_dir}" checkout -q --detach FETCH_HEAD
if [[ "$(git -C "${source_dir}" rev-parse HEAD)" != "${revision}" ]]; then exit 1; fi
if [[ "$1" == "wavelength" ]]; then
  git -C "${source_dir}" submodule update --init --depth 1
  docker build --build-arg GOTAGS="wavewalletrpc swapruntime" -t "${image}" "${source_dir}"
else
  docker build -f "${ITEST_DIR}/docker/ldk.Dockerfile" -t "${image}" "${source_dir}"
fi
