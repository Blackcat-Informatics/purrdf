#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
export CI=1
stage="$PWD/.stage/geo-deterministic-hierarchical-spatial"

record() {
    local name="$1" result=0
    shift
    local receipt="$stage/recovery-helper-$name.log"
    test ! -e "$receipt"
    "$@" > "$receipt" 2>&1 || result=$?
    printf 'terminal_status=%s\n' "$result" >> "$receipt"
    return "$result"
}

python_installed() {
    local candidate
    candidate=$(sed -n 's/^Python release candidate: //p' "$stage/recovery-helper-python-artifacts.log") || return
    test -n "$candidate" || return
    test -x "$candidate/install/bin/python" || return
    uv pip install --python "$candidate/install/bin/python" --no-deps \
        pytest==9.0.3 iniconfig==2.3.0 packaging==26.2 pluggy==1.6.0 pygments==2.20.0 || return
    "$candidate/install/bin/python" -I -c \
        'import pathlib,sys,purrdf,purrdf.purrdf_native as native; root=pathlib.Path(sys.prefix).resolve(); paths=[pathlib.Path(m.__file__).resolve() for m in (purrdf,native)]; assert all(p.is_relative_to(root) for p in paths); print(*paths,sep="\n")' || return
    "$candidate/install/bin/python" -I -m pytest -q \
        "$PWD/bindings/python/tests/test_geo.py" \
        "$PWD/bindings/python/tests/test_projection.py::test_lift_returns_a_frozen_dataset_and_write_only_profiles_fail_typed" \
        "$PWD/bindings/python/tests/test_entail_regimes.py::test_materialize_nt_matches_the_dataset_path_byte_for_byte" \
        "$PWD/bindings/python/tests/test_retrieval_vector_producers.py::test_the_two_vector_producers_differ_in_what_they_promise_and_not_in_what_they_rank"
}

wasm_installed() (
    cd crates/rdf-wasm/js || exit
    sha256sum pkg/purrdf_wasm_bg.wasm pkg/purrdf_wasm.js index.mjs || exit
    node --test tests/geo.test.mjs || exit
    node --test --test-name-pattern='a trap poisons every entry point of the instance, and jobs that fault without trapping poison nothing' tests/async-concurrency.test.mjs || exit
    node --test --test-name-pattern='a panic in a synchronous call poisons every entry point, and typed errors poison nothing' tests/panic-poison.test.mjs || exit
    node --test --test-name-pattern='a trap that is not a panic, in a synchronous call, poisons every entry point' tests/trap-poison.test.mjs || exit
    npm run typecheck || exit
    npm run smoke:pack
)

overall=0
record installed-c env -u CARGO_BUILD_BUILD_DIR make capi-bundle || overall=1
if record python-artifacts make python-release-check; then
    record installed-python python_installed || overall=1
else
    overall=1
fi
if record wasm-build make wasm-pkg; then
    record installed-wasm wasm_installed || overall=1
else
    overall=1
fi
exit "$overall"
