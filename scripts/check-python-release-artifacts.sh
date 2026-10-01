#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
python3 scripts/check-versions.py
python3 scripts/package-licenses.py --profile python --check
python3 scripts/package-licenses.py --profile python-rdflib --check
python3 bindings/python/purrdf_build_backend.py --self-test
output="$repo/target/python-release-candidate"
mkdir -p "$output"
find "$output" -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) -delete
RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}" uv build --sdist --project bindings/python --out-dir "$output"
RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}" uv build --wheel "$output"/purrdf-[0-9]*.tar.gz --out-dir "$output" \
  --config-setting 'maturin.build-args=--compatibility manylinux_2_34'
uv build --project bindings/python-rdflib-shadow --out-dir "$output"
python3 bindings/python/purrdf_build_backend.py "$output"/purrdf-[0-9]*.whl "$output"/purrdf-[0-9]*.tar.gz
python3 scripts/package-licenses.py --profile python \
  --audit "$output"/purrdf-[0-9]*.whl "$output"/purrdf-[0-9]*.tar.gz \
  --receipt "$output/license-main.json"
python3 scripts/package-licenses.py --profile python-rdflib \
  --audit "$output"/purrdf_rdflib-*.whl "$output"/purrdf_rdflib-*.tar.gz \
  --receipt "$output/license-shadow.json"
uv run --no-project --with 'twine>=6,<7' twine check --strict "$output"/*.whl "$output"/*.tar.gz
uv venv --clear "$output/install"
uv pip install --python "$output/install/bin/python" --no-deps "$output"/*.whl
version="$(python3 -c "import tomllib; print(tomllib.load(open('bindings/python/pyproject.toml','rb'))['project']['version'])")"
"$output/install/bin/python" scripts/check-python-release-install.py "$version"
