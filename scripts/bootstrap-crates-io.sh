#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

VERSION="${1:-}"
if [[ -z "${VERSION}" ]]; then
  VERSION="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
fi
PUBLISH_COOLDOWN_SECONDS="${PUBLISH_COOLDOWN_SECONDS:-620}"

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  if [[ -n "${CARGO_TOKEN:-}" ]]; then
    export CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}"
  else
    echo "Set CARGO_TOKEN or CARGO_REGISTRY_TOKEN before bootstrapping crates.io" >&2
    exit 1
  fi
fi

if [[ "${ALLOW_DIRTY:-false}" != "true" ]]; then
  if ! git diff --quiet || ! git diff --cached --quiet \
    || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    cat >&2 <<'EOF'
Refusing to publish from a dirty tree.

Commit the release source first, or set ALLOW_DIRTY=true if you intentionally
want crates.io to receive source that does not correspond to a clean git tree.
EOF
    exit 1
  fi
fi

crates=(
  purrdf-events
  purrdf-iri
  purrdf-xsd
  purrdf-gts
  purrdf-core
  purrdf-sparql-algebra
  purrdf-sparql-results
  purrdf-sparql-eval
  purrdf-rdf
  purrdf-slice
  purrdf-shapes
  purrdf-shex
  purrdf-validate
  purrdf
  purrdf-wasm
)

crate_version_exists() {
  local crate="$1"
  local status
  status="$(curl -sS -H "User-Agent: purrdf-release/${VERSION} (paudley@blackcatinformatics.ca)" \
    -o /tmp/purrdf-crate-version.json -w "%{http_code}" \
    "https://crates.io/api/v1/crates/${crate}/${VERSION}")"
  case "$status" in
    200) return 0 ;;
    404) return 1 ;;
    *)
      cat /tmp/purrdf-crate-version.json
      echo "Unexpected crates.io status ${status} for ${crate} ${VERSION}" >&2
      exit 1
      ;;
  esac
}

crate_record_exists() {
  local crate="$1"
  local status
  status="$(curl -sS -H "User-Agent: purrdf-release/${VERSION} (paudley@blackcatinformatics.ca)" \
    -o /tmp/purrdf-crate-record.json -w "%{http_code}" \
    "https://crates.io/api/v1/crates/${crate}")"
  case "$status" in
    200) return 0 ;;
    404) return 1 ;;
    *)
      cat /tmp/purrdf-crate-record.json
      echo "Unexpected crates.io status ${status} for ${crate}" >&2
      exit 1
      ;;
  esac
}

wait_for_crate_version() {
  local crate="$1"
  for _ in $(seq 1 30); do
    if crate_version_exists "$crate"; then
      return 0
    fi
    sleep 10
  done
  echo "Timed out waiting for crates.io to expose ${crate} ${VERSION}" >&2
  exit 1
}

cargo fmt --all --check
cargo check --workspace --lib --tests --locked
if command -v rustup >/dev/null; then
  if ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
    rustup target add wasm32-unknown-unknown
  fi
fi
cargo_args=()
for crate in "${crates[@]}"; do
  cargo_args+=("-p" "$crate")
done
cargo check --locked --target wasm32-unknown-unknown --lib "${cargo_args[@]}"
cargo test -p purrdf-gts --test transport --locked
cargo test -p purrdf-slice --locked
rm -rf target/package
cargo package --workspace \
  --exclude purrdf-python \
  --exclude purrdf-capi \
  --exclude purrdf-sparql-conformance \
  --locked \
  --no-verify

for idx in "${!crates[@]}"; do
  crate="${crates[$idx]}"
  if crate_version_exists "$crate"; then
    echo "${crate} ${VERSION} already exists on crates.io; skipping"
    continue
  fi
  record_exists_before=false
  if crate_record_exists "$crate"; then
    record_exists_before=true
  fi
  cargo publish -p "$crate" --locked --no-verify
  wait_for_crate_version "$crate"
  if [[ "$record_exists_before" == "false" ]] \
    && ((idx + 1 < ${#crates[@]})) \
    && [[ "${PUBLISH_COOLDOWN_SECONDS}" != "0" ]]; then
    sleep "${PUBLISH_COOLDOWN_SECONDS}"
  fi
done
