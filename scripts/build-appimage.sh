#!/usr/bin/env bash
set -euo pipefail

readonly OUTPUT_DIR="${1:-src-tauri/target/release/bundle/appimage}"
readonly IMAGE_TAG="opets-manager-appimage-builder"

if ! command -v docker >/dev/null 2>&1; then
  printf '%s\n' "Docker é necessário para gerar AppImage neste projeto." >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"
docker build \
  --file Dockerfile.appimage \
  --target artifacts \
  --tag "${IMAGE_TAG}" \
  .

container_id="$(docker create "${IMAGE_TAG}")"
trap 'docker rm -f "${container_id}" >/dev/null' EXIT
docker cp "${container_id}:/artifacts/." "${OUTPUT_DIR}"
