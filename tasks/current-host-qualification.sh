#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Stage-only host boundary qualification; existing tests, no semantic mirrors.
set -euo pipefail
host402_repo=/home/paudley/Active/purrdf/.worktrees/402-shacl-profiles-reports
host402_stage="${host402_repo}/.stage/shacl-profiles-reports"
host402_logs=$(cat "${host402_stage}/tasks/current-host-log-root.txt")
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 TMPDIR="${host402_logs}/tmp"
mkdir "${TMPDIR}" "${host402_logs}/wheel" "${host402_logs}/npm-pack"
cd "${host402_repo}"
host402_run() {
  local name="$1" result
  shift
  [[ ! -e "${host402_logs}/${name}.log" ]]
  printf '%q ' "$@" >"${host402_logs}/${name}.command"
  printf '\n' >>"${host402_logs}/${name}.command"
  if "$@" >"${host402_logs}/${name}.log" 2>&1; then result=0; else result=$?; fi
  printf '%s\n' "${result}" >"${host402_logs}/${name}.exit"
  printf '%s actual exit %s\n' "${name}" "${result}"
  [[ "${result}" == 0 ]] || { tail -n 50 "${host402_logs}/${name}.log"; return "${result}"; }
}
host402_run compiler rustc -vV
host402_run python-wheel uv build --no-create-gitignore --wheel --project bindings/python --out-dir "${host402_logs}/wheel" --config-setting 'maturin.build-args=--locked --jobs 8'
host402_wheels=("${host402_logs}"/wheel/*.whl)
[[ ${#host402_wheels[@]} == 1 && -f "${host402_wheels[0]}" ]]
host402_run python-install uv pip install --python bindings/python/.venv/bin/python --no-deps --reinstall "${host402_wheels[0]}"
host402_run python-module bindings/python/.venv/bin/python -I -c 'import purrdf,purrdf.purrdf_native as n,sys,hashlib,pathlib; p=pathlib.Path(n.__file__); print("interpreter",sys.executable); print("package",purrdf.__file__); print("native",p); print("sha256",hashlib.sha256(p.read_bytes()).hexdigest())'
host402_run python-boundaries bindings/python/.venv/bin/python -I -m pytest -v \
  bindings/python/tests/test_sarif.py::test_to_sarif_reports_the_violation \
  bindings/python/tests/test_shacl_result_annotations.py::test_py_validate_result_annotations \
  bindings/python/tests/test_shacl_product.py::test_restored_product_matches_direct_validation \
  bindings/python/tests/test_shacl_product.py::test_rebuild_also_matches_direct_validation \
  bindings/python/tests/test_shacl_product.py::test_a_corrupted_product_is_refused_with_its_structured_dimension \
  bindings/python/tests/test_shacl_product.py::test_a_shapes_document_that_does_not_parse_names_no_dimension \
  bindings/python/tests/test_shacl_product.py::test_a_restored_preparation_names_the_product_it_came_from \
  bindings/python/tests/test_shacl_product.py::test_a_parsed_preparation_names_no_artifact \
  bindings/python/tests/test_shapes_product_identity.py::test_a_selector_that_is_not_a_digest_names_no_dimension
# The actual production private-output helper needs Cargo to honor its private
# target, which Stage wrapper deliberately strips. Same active SDK bin first only
# for this scoped package build; no hooks or global policy bypass/change.
host402_sdk="${RUSTUP_HOME:-/home/paudley/stage/packages/rustup}/active-toolchain/bin"
test -x "${host402_sdk}/cargo"
host402_run wasm-package env PATH="${host402_sdk}:${PATH}" make wasm-pkg
cd crates/rdf-wasm/js
host402_run npm-pack npm pack --json --ignore-scripts --pack-destination "${host402_logs}/npm-pack"
host402_tarballs=("${host402_logs}"/npm-pack/*.tgz)
[[ ${#host402_tarballs[@]} == 1 && -f "${host402_tarballs[0]}" ]]
mkdir "${host402_logs}/packed-runtime"
host402_run npm-extract tar -xzf "${host402_tarballs[0]}" -C "${host402_logs}/packed-runtime"
host402_package="${host402_logs}/packed-runtime/package"
mkdir "${host402_package}/tests"
cp tests/shacl.test.mjs tests/shacl-product.test.mjs tests/async-shacl.test.mjs "${host402_package}/tests/"
cd "${host402_package}"
host402_run node-report node --test --test-name-pattern='^shaclValidateToSarif emits SARIF|^shaclValidateToSarif rejects malformed|^wasm_shacl_conformance_disallows|^wasm_shacl_shapes_graph_well_formed|^wasm_shacl_message_languages' tests/shacl.test.mjs
host402_run node-product node --test tests/shacl-product.test.mjs
host402_run node-async node --test tests/async-shacl.test.mjs
printf 'HOST BOUNDARY BATCH COMPLETE; no semantic/vendor matrix or timing run\n'
