#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
shopt -s nullglob

target=$1
platform=$2
version=$3
minimum=$4
shift 3
output=${PURRDF_RELEASE_ARTIFACTS:-.}
source_dir=${PURRDF_RELEASE_SOURCE:-.}
output=$(cd -- "$output" && pwd)
if [[ -n "${EXPECTED_RUST:-}" ]]; then
  read -r _ installed_rust _ <<< "$(rustc --version)"
  if [[ "$installed_rust" != "$EXPECTED_RUST" ]]; then
    printf 'Rust %s differs from the planned release compiler %s\n' "$installed_rust" "$EXPECTED_RUST" >&2
    exit 1
  fi
fi

shadow=("$output"/release-input/shadow/*.whl)
[[ ${#shadow[@]} == 1 ]]
[[ ${shadow[0]##*/} == "purrdf_rdflib-$version-py3-none-any.whl" ]]

build_args="--locked --target $target"
case "$target" in
  *-linux-gnu) build_args+=" --compatibility manylinux_2_34" ;;
  *-linux-musl)
    build_args+=" --compatibility musllinux_1_2"
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C target-feature=-crt-static"
    ;;
esac

build_source=(--project "$source_dir/bindings/python")
if [[ -n "${PURRDF_WHEEL_SDIST:-}" ]]; then
  build_source=("$PURRDF_WHEEL_SDIST")
fi
uv build --python "$minimum" --no-create-gitignore --wheel "${build_source[@]}" \
  --out-dir "$output/release-wheel" --config-setting "maturin.build-args=$build_args"
wheel=("$output"/release-wheel/*.whl)
if [[ ${#wheel[@]} != 1 || ${wheel[0]##*/} != "purrdf-$version-cp313-abi3-$platform.whl" ]]; then
  printf 'Expected a single CPython 3.13 abi3 wheel for %s\n' "$platform" >&2
  printf '%s\n' "${wheel[@]}" >&2
  exit 1
fi

"$minimum" -I scripts/check-python-wheel-sbom.py "${wheel[0]}" "$version" \
  "$output/release-wheel-sbom.json"

for interpreter in "$@"; do
  candidate=$(mktemp -d)
  uv venv --python "$interpreter" "$candidate"
  installed="$candidate/bin/python"
  if [[ -f "$candidate/Scripts/python.exe" ]]; then
    installed="$candidate/Scripts/python.exe"
  fi
  uv pip install --python "$installed" --no-index --no-deps --only-binary :all: \
    "${wheel[0]}" "${shadow[0]}"
  "$installed" -I scripts/check-python-release-install.py "$version" "$platform"
done

(
  cd -- "$source_dir"
  "$minimum" scripts/fetch-locked-deps.py
  "$minimum" scripts/package-licenses.py --profile python --audit "${wheel[0]}" \
    --receipt "$output/release-wheel-license.json"
)
