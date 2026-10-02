#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
python3 scripts/check-versions.py
version="$(python3 -c "import tomllib; print(tomllib.load(open('bindings/python/pyproject.toml','rb'))['project']['version'])")"
python3 scripts/package-licenses.py --profile python --check
python3 scripts/package-licenses.py --profile python-rdflib --check
python3 bindings/python/purrdf_build_backend.py --self-test
output="$repo/target/python-release-candidate"
main_output="$output/purrdf"
shadow_output="$output/purrdf-rdflib"
mkdir -p "$main_output" "$shadow_output"
find "$main_output" "$shadow_output" -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) -delete
RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}" uv build --no-create-gitignore --sdist --project bindings/python --out-dir "$main_output"
RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}" uv build --no-create-gitignore --wheel "$main_output"/purrdf-[0-9]*.tar.gz --out-dir "$main_output" \
  --config-setting 'maturin.build-args=--compatibility manylinux_2_34'
uv build --no-create-gitignore --project bindings/python-rdflib-shadow --out-dir "$shadow_output"
python3 bindings/python/purrdf_build_backend.py "$main_output"/purrdf-[0-9]*.whl "$main_output"/purrdf-[0-9]*.tar.gz
python3 scripts/package-licenses.py --profile python \
  --audit "$main_output"/purrdf-[0-9]*.whl "$main_output"/purrdf-[0-9]*.tar.gz \
  --receipt "$output/license-main.json"
python3 scripts/package-licenses.py --profile python-rdflib \
  --audit "$shadow_output"/purrdf_rdflib-*.whl "$shadow_output"/purrdf_rdflib-*.tar.gz \
  --receipt "$output/license-shadow.json"
publisher_env="$(mktemp -d "$output/publisher-tooling.XXXXXX")"
uv venv --python 3.13 "$publisher_env"
uv pip sync --python "$publisher_env/bin/python" --require-hashes --only-binary :all: \
  scripts/python-publisher-requirements.txt
"$publisher_env/bin/python" -I scripts/publish-python.py --self-test
"$publisher_env/bin/python" -I scripts/publish-python.py check \
  --project purrdf --version "$version" --dist-dir "$main_output"
"$publisher_env/bin/python" -I scripts/publish-python.py check \
  --project purrdf-rdflib --version "$version" --dist-dir "$shadow_output"
uv venv --clear "$output/install"
uv pip install --python "$output/install/bin/python" --no-deps "$main_output"/*.whl "$shadow_output"/*.whl
"$output/install/bin/python" scripts/check-python-release-install.py "$version"
