#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

VERSION="${1:-}"
if [[ -z "${VERSION}" ]]; then
  VERSION="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
fi
# Pause after publishing a crate that CREATED a new crates.io record, before
# the next publish. Default 0, for two reasons. First, crates.io's new-crate
# rate limit is enforced AT the publish: a limited `cargo publish` exits
# non-zero on the registry's 429, `set -e` stops this script before the next
# crate, nothing is half-uploaded, and a re-run resumes because every version
# already published is skipped above — so a hit limit is a visible, resumable
# refusal, not a corrupted release. Second, the limit is a burst allowance
# refilled at one new crate per ten minutes, and the old default of 620 s was
# that refill interval plus slack: it modelled the WORST case (burst already
# spent) unconditionally, adding ~31 minutes of dead time to a three-record
# run whose every other step is observable. Set PUBLISH_COOLDOWN_SECONDS=620
# only if a run actually meets a 429 with several records still to create.
PUBLISH_COOLDOWN_SECONDS="${PUBLISH_COOLDOWN_SECONDS:-0}"
# Set PUBLISH_NO_VERIFY=true to skip `cargo publish`'s verification build. See
# the comment above the publish loop for why the default is to verify.
PUBLISH_NO_VERIFY="${PUBLISH_NO_VERIFY:-false}"

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

# One definition of the release set, shared with the release workflow and with
# scripts/check-crates-io-records.sh.
# shellcheck source=scripts/release-crates.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/release-crates.sh"
crates=("${PURRDF_RELEASE_CRATES[@]}")

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

# State the irreversible plan BEFORE the long gates run, not one crate at a
# time while publishing. This script is the token lane, so unlike the release
# workflow it is ALLOWED to create crate records — but creating one is a
# permanent, outward-facing act, so which records it will create is stated up
# front where the operator can still stop. Any crates.io status other than
# 200/404 aborts inside the helpers above rather than being read as "missing".
echo "crates.io plan for ${VERSION}:"
new_records=0
for crate in "${crates[@]}"; do
  if crate_version_exists "$crate"; then
    printf '  skip           %s (version already published)\n' "$crate"
  elif crate_record_exists "$crate"; then
    printf '  publish        %s (crate record exists)\n' "$crate"
  else
    printf '  CREATE RECORD  %s (new crate — needs this token; a Trusted Publisher cannot create it)\n' "$crate"
    new_records=$((new_records + 1))
  fi
done
if [[ "$new_records" -gt 0 ]]; then
  cat <<EOF

${new_records} crate record(s) above do not exist yet. This run will create them.
After it finishes, add a crates.io Trusted Publisher entry for each new crate
(GitHub Actions / Blackcat-Informatics / purrdf / release-cargo.yaml / no
environment) — until then the rust-v* release workflow refuses to publish, by
design (scripts/check-crates-io-records.sh).
EOF
fi
echo

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
# `--no-verify` is LOAD-BEARING here and only here. This packages every crate
# BEFORE any of them is published, so a verification build of crate N would
# have to resolve crate N-1 at ${VERSION} from crates.io, where it does not
# exist yet. Verification is impossible at this point by construction; the
# `.crate` files are produced so that a packaging failure (a missing file, a
# path dependency with no version) is found before the first irreversible
# upload rather than midway through the set.
cargo package --workspace \
  --exclude purrdf-python \
  --exclude purrdf-capi \
  --exclude purrdf-sparql-conformance \
  --locked \
  --no-verify

# The publish loop VERIFIES by default. `cargo publish` unpacks the `.crate` it
# is about to upload and builds it against the registry, so a wrong-version or
# broken artifact is refused BEFORE it is uploaded — and an upload is the one
# step here that cannot be undone. Two things make that build resolvable, and
# both are gated rather than assumed:
#
#   1. `wait_for_crate_version` below blocks until crate N-1 is actually served
#      by crates.io before crate N is published, so N's normal dependencies
#      resolve; and
#   2. scripts/check-publish-order.py (`make check`) proves the release order is
#      a topological order of DEV-dependencies too, because verification
#      resolves the full graph. `purrdf-geo` dev-depends on `purrdf-rdf` and
#      `purrdf-shapes`; while it was ordered before them, verification could
#      not work for this set at all, which is why this loop used to pass
#      `--no-verify` and why the flag is not simply "safe to drop".
#
# PUBLISH_NO_VERIFY=true restores the old behaviour for one run. Use it only if
# a verification build fails for a reason that is demonstrably not a broken
# artifact (a registry outage mid-run, say) — and note that it is then YOUR
# build of the artifact that is the last check before permanence.
publish_args=(--locked)
if [[ "${PUBLISH_NO_VERIFY}" == "true" ]]; then
  publish_args+=(--no-verify)
  echo "PUBLISH_NO_VERIFY=true: cargo publish will NOT build the packaged crates before uploading them" >&2
fi

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
  cargo publish -p "$crate" "${publish_args[@]}"
  wait_for_crate_version "$crate"
  if [[ "$record_exists_before" == "false" ]] \
    && ((idx + 1 < ${#crates[@]})) \
    && [[ "${PUBLISH_COOLDOWN_SECONDS}" != "0" ]]; then
    sleep "${PUBLISH_COOLDOWN_SECONDS}"
  fi
done
