#!/usr/bin/env bash
# Prepared command driver, not an executed qualification receipt.
# Run only after root's lane admission. Stage process artifact, never shipping code.
set -euo pipefail
phase=${1:?phase: rust, docs, python, or cost}
root=${PURRDF_454_ROOT:?existing worktree root required}
evidence=${PURRDF_454_EVIDENCE:?fresh task-owned absolute evidence root required}
expected_tree=${PURRDF_454_EXPECT_TREE:?root-reviewed current index tree required}
sdk=/home/paudley/stage/packages/rustup/active-toolchain/bin
cd "$root"
test "$(git write-tree)" = "$expected_tree"
git diff --quiet
test -z "$(git ls-files -u)"
test -z "${CARGO_ENCODED_RUSTFLAGS-}"
mkdir -p "$evidence"
logs=$evidence/${PURRDF_454_LOG_SUBDIR:-$phase}
mkdir "$logs" # refuse overwriting a previous phase's actual evidence
export PATH="$sdk:$PATH" CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 BINARYEN_CORES=8
export CARGO_TARGET_DIR="$evidence/target" CARGO_BUILD_BUILD_DIR="$evidence/build"
export TMPDIR="$evidence/tmp"
mkdir -p "$TMPDIR" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR"
git rev-parse HEAD > "$logs/input-refs.txt"
if test -f "$(git rev-parse --git-path MERGE_HEAD)"; then
    git rev-parse MERGE_HEAD >> "$logs/input-refs.txt"
fi
git write-tree > "$logs/input-tree.txt"
git ls-files -z | xargs -0 sha256sum > "$logs/source-start.sha256"
"$sdk/rustc" -vV > "$logs/compiler.txt"
"$sdk/cargo" -V > "$logs/cargo.txt"
printf '%s\n' "$PATH" > "$logs/path.txt"
for variable in RUSTFLAGS CARGO_PROFILE_DEV_OPT_LEVEL CARGO_PROFILE_DEV_DEBUG_ASSERTIONS CARGO_PROFILE_DEV_OVERFLOW_CHECKS CARGO_PROFILE_TEST_OPT_LEVEL CARGO_PROFILE_TEST_DEBUG_ASSERTIONS CARGO_PROFILE_TEST_OVERFLOW_CHECKS CARGO_PROFILE_RELEASE_LTO CARGO_PROFILE_RELEASE_CODEGEN_UNITS; do
    printf '%s=%s\n' "$variable" "${!variable-<absent>}" >> "$logs/profile-environment.txt"
done
run() {
    local name=$1; shift
    printf '%q ' "$@" > "$logs/$name.command"
    printf '\n' >> "$logs/$name.command"
    local result=0
    "$@" > "$logs/$name.log" 2>&1 || result=$?
    printf '%s\n' "$result" > "$logs/$name.exit"
    test "$result" -eq 0
}
json_run() {
    local name=$1; shift
    printf '%q ' "$@" > "$logs/$name.command"
    printf '\n' >> "$logs/$name.command"
    local result=0
    "$@" > "$logs/$name.json" 2> "$logs/$name.log" || result=$?
    printf '%s\n' "$result" > "$logs/$name.exit"
    test "$result" -eq 0
}
counted_rust() {
    local name=$1 expected_blocks=$2
    rg '^test result:' "$logs/$name.log" > "$logs/$name.results"
    # Every selected owning target must have actually run nonzero tests, with
    # no failure/ignore standing in for the selected acceptance.
    awk -v expected="$expected_blocks" '
        /^test result: ok\./ { blocks++; if ($4 == 0 || $6 != 0 || $8 != 0) bad=1 }
        END { exit (bad || blocks != expected) }
    ' "$logs/$name.results"
}
validate_emitted_pairs() {
    local expected_pairs=$1; shift
    local llvm_count=0 assembly_count=0 path counterpart candidate counterpart_found
    for path in "$@"; do
        if ! test -s "$path"; then
            printf 'missing or empty emitted artifact: %s\n' "$path" >&2
            return 1
        fi
        case "$path" in
        *.ll) llvm_count=$((llvm_count + 1)); counterpart=${path%.ll}.s ;;
        *.s) assembly_count=$((assembly_count + 1)); counterpart=${path%.s}.ll ;;
        *) printf 'unsupported emitted artifact: %s\n' "$path" >&2; return 1 ;;
        esac
        counterpart_found=0
        for candidate in "$@"; do
            if test "$candidate" = "$counterpart"; then counterpart_found=1; break; fi
        done
        if test "$counterpart_found" -ne 1; then
            printf 'emitted inventory lacks matched counterpart: %s\n' "$counterpart" >&2
            return 1
        fi
    done
    test "$llvm_count" -gt 0
    test "$llvm_count" -eq "$assembly_count"
    if test "$expected_pairs" -gt 0; then test "$llvm_count" -eq "$expected_pairs"; fi
}
lock_registry_rows() {
    # Cargo writes these fields as canonical quoted scalars. Refuse another
    # source/checksum spelling instead of guessing a TOML interpretation.
    awk '
      function finish() {
        if (source != "") {
          if (source != "registry+https://github.com/rust-lang/crates.io-index" || name == "" || version == "" || length(checksum) != 64 || checksum ~ /[^0-9a-f]/) exit 1
          print name "\t" version "\t" source "\t" checksum
        }
      }
      /^\[\[package\]\]$/ { finish(); active=1; name=""; version=""; source=""; checksum=""; next }
      active && /^(name|version|source|checksum) = / {
        if ($0 !~ /^[a-z]+ = "[^"\\]+"$/) exit 1
        value=$0; sub(/^[a-z]+ = "/,"",value); sub(/"$/,"",value)
        if ($1 == "name") name=value
        else if ($1 == "version") version=value
        else if ($1 == "source") source=value
        else checksum=value
      }
      END { finish() }
    ' "$1" | LC_ALL=C sort -u
}
pyo3_build_binding() {
    local name=$1 frames=$2
    # Compare actual build-script configuration, not just the interpreter path.
    # out_dir is an artifact location; every ABI/link/cfg/env field is retained.
    jq -s '[.[] | select(.reason=="build-script-executed" and (.package_id | test("#pyo3(-ffi)?@"))) | {package_id,linked_libs,linked_paths,cfgs,env}] | sort_by(.package_id)' "$frames" > "$logs/$name-pyo3-build-config.json"
    test "$(jq length "$logs/$name-pyo3-build-config.json")" -eq 2
    local out_dir fingerprint
    while IFS= read -r out_dir; do
        fingerprint=${out_dir%/out}/fingerprint/run-build-script-build-script-build.json
        test -s "$fingerprint"
        sha256sum "$fingerprint" >> "$logs/$name-pyo3-fingerprints.sha256"
        jq -c '{rustflags,env:[.local[] | .RerunIfEnvChanged // empty]}' "$fingerprint" >> "$logs/$name-pyo3-build-inputs.jsonl"
    done < <(jq -r 'select(.reason=="build-script-executed" and (.package_id | test("#pyo3(-ffi)?@"))) | .out_dir' "$frames")
    test "$(wc -l < "$logs/$name-pyo3-build-inputs.jsonl")" -eq 2
}
verify_compiled_cohort() {
    local name=$1 metadata=$2 artifacts=$3 selected_lock=$4 production_lock=$5
    lock_registry_rows "$production_lock" > "$logs/$name-production-registry.tsv"
    lock_registry_rows "$selected_lock" > "$logs/$name-selected-registry.tsv"
    LC_ALL=C comm -23 "$logs/$name-selected-registry.tsv" "$logs/$name-production-registry.tsv" > "$logs/$name-unapproved-registry.tsv"
    if test -s "$logs/$name-unapproved-registry.tsv"; then
        printf 'dependency cohort differs from production lock: %s\n' "$name" >&2
        return 1
    fi
    jq -s '[.[] | select(.reason=="compiler-artifact") | .package_id] | unique' "$artifacts" > "$logs/$name-compiled-package-ids.json"
    jq -e --slurpfile ids "$logs/$name-compiled-package-ids.json" '($ids[0] - [.packages[].id]) == []' "$metadata" > "$logs/$name-compiled-metadata-admission.json"
    jq -r --slurpfile ids "$logs/$name-compiled-package-ids.json" '.packages[] | select(.id as $id | $ids[0] | index($id)) | select(.source != null) | [.name,.version,.source] | @tsv' "$metadata" | LC_ALL=C sort -u > "$logs/$name-compiled-external.tsv"
    awk -F '\t' 'NR==FNR {sha[$1 FS $2 FS $3]=$4; next} {key=$1 FS $2 FS $3; if (!(key in sha)) exit 1; print $0 "\t" sha[key]}' "$logs/$name-production-registry.tsv" "$logs/$name-compiled-external.tsv" > "$logs/$name-compiled-external-checksums.tsv"
    test -s "$logs/$name-compiled-external-checksums.tsv"
}
case "$phase" in
feedback-final-gates|feedback-final-gates-remaining)
    export PYO3_PYTHON="$root/bindings/python/.venv/bin/python"
    if test "$phase" = feedback-final-gates; then
        run feedback-graph-callers "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test rdflib_contextual
        counted_rust feedback-graph-callers 1
    fi
    run feedback-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval -p purrdf-python --all-targets -- -D warnings
    run feedback-fmt "$sdk/cargo" fmt --all --check
    run feedback-hygiene make helpers-hygiene layer-hygiene terminal-hygiene build-profile-hygiene rdf-core-hygiene
    ;;
feedback-diagnostic)
    run feedback-lateral-control "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib property_fn_plan::iterative_walk_tests::contextual_inputs_drive_certain_bindings_and_downstream_call_admission -- --exact
    counted_rust feedback-lateral-control 1
    run feedback-compiler-diagnostic "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib property_fn_eval::tests::contextual_mapped_inputs_execute_a_bound_only_registered_relation -- --exact
    ;;
feedback-rust|feedback-rust-remaining)
    export PYO3_PYTHON="$root/bindings/python/.venv/bin/python"
    if test "$phase" = feedback-rust; then
        run feedback-check "$sdk/cargo" check --locked --jobs 8 --verbose -p purrdf-sparql-eval -p purrdf-python
    fi
    run feedback-contextual-unit "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib contextual
    counted_rust feedback-contextual-unit 1
    run feedback-read-walk "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib property_fn_eval::walk_tests
    counted_rust feedback-read-walk 1
    run feedback-plan-walk "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib property_fn_plan::iterative_walk_tests
    counted_rust feedback-plan-walk 1
    run feedback-service-walk "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib service_endpoints::walk_tests
    counted_rust feedback-service-walk 1
    run feedback-callers "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test rdflib_contextual --test prepared_parameters --test property_function_e2e --test property_function_value_sources --test service_resolver
    counted_rust feedback-callers 5
    run feedback-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval -p purrdf-python --all-targets -- -D warnings
    run feedback-fmt "$sdk/cargo" fmt --all --check
    run feedback-hygiene make helpers-hygiene layer-hygiene terminal-hygiene build-profile-hygiene rdf-core-hygiene
    ;;
hosted-fixture-rust)
    run fixture-reasoning "$sdk/cargo" test --locked --jobs 8 -p purrdf --lib contextual_application_visitors_preserve_children_and_ordinary_admission_refuses_it
    counted_rust fixture-reasoning 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/fixture-reasoning.results"
    run fixture-description "$sdk/cargo" test --locked --jobs 8 -p purrdf-rdf --lib contextual_application_visits_both_operands_and_nested_expression_causes
    counted_rust fixture-description 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/fixture-description.results"
    run fixture-ownership "$sdk/cargo" test --locked --jobs 8 -p purrdf-slice --lib contextual_application_collects_both_child_iris_and_nested_exists
    counted_rust fixture-ownership 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/fixture-ownership.results"
    run fixture-workspace-check "$sdk/cargo" check --workspace --lib --tests --locked --jobs 8
    run fixture-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf -p purrdf-rdf -p purrdf-slice --all-targets -- -D warnings
    run fixture-fmt "$sdk/cargo" fmt --all --check
    ;;
outer-inline-diagnostic|outer-owning-inline-diagnostic)
    baseline_logs=$evidence/final-lower-native-cost-1
    candidate_logs=$evidence/final-project-outer-bulk-native-cost-1
    cmp "$logs/compiler.txt" "$baseline_logs/compiler.txt"
    cmp "$logs/cargo.txt" "$baseline_logs/cargo.txt"
    cmp "$logs/path.txt" "$baseline_logs/path.txt"
    cmp "$logs/profile-environment.txt" "$baseline_logs/profile-environment.txt"
    test "$(cat "$baseline_logs/driver.exit")" = 0
    test "$(cat "$candidate_logs/driver.exit")" = 0
    sha256sum --check --quiet "$baseline_logs/baseline-source-start.sha256" > "$logs/baseline-source-readback-start.txt"
    export RUSTFLAGS='-Dwarnings -Ctarget-cpu=native'
    diagnostic_final_flags=(-Cremark=inline)
    if test "$phase" = outer-owning-inline-diagnostic; then
        export RUSTFLAGS='-Dwarnings -Ctarget-cpu=native -Cremark=inline'
        diagnostic_final_flags=()
    fi
    export CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_STRIP=symbols
    export CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=false CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false CARGO_PROFILE_RELEASE_LTO=thin
    for role in baseline candidate; do
        if test "$role" = baseline; then original=$baseline_logs; source_root=/opt/purrdf-454-native-cost-main-df2cec88; else original=$candidate_logs; source_root=$root; fi
        probe=$original/artifacts/probe-$role
        sha256sum --check --quiet "$original/$role-native-inputs.sha256" > "$logs/$role-original-inputs-start.txt"
        sha256sum --check --quiet "$original/$role-native-emitted.sha256" > "$logs/$role-original-emits-start.txt"
        export CARGO_TARGET_DIR="$logs/artifacts/$role-native-target" CARGO_BUILD_BUILD_DIR="$logs/artifacts/$role-native-build"
        json_run "$role-inline-emit" "$sdk/cargo" rustc --locked --jobs 8 --release --manifest-path "$probe/Cargo.toml" --message-format=json-render-diagnostics --verbose -- --emit=link,llvm-ir,asm "${diagnostic_final_flags[@]}"
        if test "$phase" = outer-owning-inline-diagnostic; then
            rg 'Running `.*rustc --crate-name purrdf_sparql_eval ' "$logs/$role-inline-emit.log" > "$logs/$role-owning-rustc-argv.txt"
            rg 'Running `.*rustc --crate-name qualification_454_native_cost ' "$logs/$role-inline-emit.log" > "$logs/$role-final-rustc-argv.txt"
            rg -q -- '-Cremark=inline' "$logs/$role-owning-rustc-argv.txt"
            rg -q -- '-Cremark=inline' "$logs/$role-final-rustc-argv.txt"
        fi
        verify_compiled_cohort "$role-diagnostic" "$original/$role-native-metadata.json" "$logs/$role-inline-emit.json" "$probe/Cargo.lock" "$source_root/Cargo.lock"
        mapfile -t selected < <(jq -r 'select(.reason=="compiler-artifact" and .target.name=="qualification-454-native-cost" and .profile.test==false) | .executable // empty' "$logs/$role-inline-emit.json")
        test "${#selected[@]}" -eq 1
        test -x "${selected[0]}"
        sha256sum "${selected[0]}" "$probe/src/main.rs" "$probe/Cargo.toml" "$probe/Cargo.lock" > "$logs/$role-diagnostic-inputs.sha256"
        run "$role-untimed-probe" "${selected[0]}"
        test "$(wc -l < "$logs/$role-untimed-probe.log")" -eq 32
        run "$role-original-equality" cmp "$original/$role-untimed-probe.log" "$logs/$role-untimed-probe.log"
        mapfile -t emitted < <(rg --files --no-ignore "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" | rg '/qualification_454_native_cost[^/]*\.(ll|s)$' | sort)
        validate_emitted_pairs 0 "${emitted[@]}"
        printf '%s\n' "${emitted[@]}" > "$logs/$role-diagnostic-emitted-paths.txt"
        sha256sum "${emitted[@]}" > "$logs/$role-diagnostic-emitted.sha256"
        jq -r 'select(.reason=="compiler-message") | .message.rendered // empty' "$logs/$role-inline-emit.json" > "$logs/$role-inline-rendered.txt"
        sha256sum --check --quiet "$logs/$role-diagnostic-inputs.sha256" > "$logs/$role-diagnostic-inputs-final.txt"
        sha256sum --check --quiet "$logs/$role-diagnostic-emitted.sha256" > "$logs/$role-diagnostic-emitted-final.txt"
        sha256sum --check --quiet "$original/$role-native-inputs.sha256" > "$logs/$role-original-inputs-final.txt"
        sha256sum --check --quiet "$original/$role-native-emitted.sha256" > "$logs/$role-original-emits-final.txt"
    done
    sha256sum --check --quiet "$baseline_logs/baseline-source-start.sha256" > "$logs/baseline-source-readback-final.txt"
    ;;
smallvec-rust)
    run smallvec-unit "$sdk/cargo" test --locked --jobs 8 -p purrdf-core --lib small::tests
    counted_rust smallvec-unit 1
    rg -q '^test result: ok\. 18 passed; 0 failed; 0 ignored;' "$logs/smallvec-unit.results"
    run smallvec-project "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib modifier::tests::project_keeps_only_listed_vars_in_order
    counted_rust smallvec-project 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/smallvec-project.results"
    run smallvec-contextual "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test rdflib_contextual --test prepared_parameters
    counted_rust smallvec-contextual 2
    run smallvec-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-core -p purrdf-sparql-eval --all-targets -- -D warnings
    run smallvec-fmt "$sdk/cargo" fmt --all --check
    run smallvec-hygiene make helpers-hygiene rdf-core-hygiene
    ;;
project-view-lift-rust)
    run project-unit "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib modifier::tests::project_keeps_only_listed_vars_in_order
    counted_rust project-unit 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/project-unit.results"
    run project-contextual "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test rdflib_contextual --test prepared_parameters
    counted_rust project-contextual 2
    rg -q '^test result: ok\. 12 passed; 0 failed; 0 ignored;' "$logs/project-contextual.results"
    rg -q '^test result: ok\. 22 passed; 0 failed; 0 ignored;' "$logs/project-contextual.results"
    run project-governors "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test governor_correctness --test governed_query
    counted_rust project-governors 2
    rg -q '^test result: ok\. 9 passed; 0 failed; 0 ignored;' "$logs/project-governors.results"
    rg -q '^test result: ok\. 27 passed; 0 failed; 0 ignored;' "$logs/project-governors.results"
    run project-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval --all-targets -- -D warnings
    run project-fmt "$sdk/cargo" fmt --all --check
    ;;
project-rust|project-kernel-rust)
    run project-unit "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib modifier::tests::project_keeps_only_listed_vars_in_order
    counted_rust project-unit 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/project-unit.results"
    run project-contextual "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test rdflib_contextual --test prepared_parameters
    counted_rust project-contextual 2
    run project-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval --all-targets -- -D warnings
    run project-fmt "$sdk/cargo" fmt --all --check
    if test "$phase" = project-kernel-rust; then
        run project-helpers make helpers-hygiene
    fi
    ;;
mint-rust)
    run mint-checkpoints "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib row_checkpoint::tests
    counted_rust mint-checkpoints 1
    rg -q '^test result: ok\. 9 passed; 0 failed; 0 ignored;' "$logs/mint-checkpoints.results"
    run mint-governors "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test governor_correctness --test governed_query
    counted_rust mint-governors 2
    run mint-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval --all-targets -- -D warnings
    run mint-fmt "$sdk/cargo" fmt --all --check
    run mint-helpers make helpers-hygiene
    ;;
lower-rust)
    # Project, three lower checkpoint annotations and Application deferred-map
    # Option dispatch changed since the actual 196-test qualification. Preserve
    # evidence for unchanged admission/parser/Shapes/serializer source.
    run lower-owning-check "$sdk/cargo" check --locked --jobs 8 --verbose -p purrdf-sparql-eval -p purrdf-python
    run lower-evaluator "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test graph_var_narrowing --test graph_constant_membership --test graph_scope_admission --test rdflib_contextual --test prepared_parameters --test numeric_governance --test numeric_parallel_determinism --test governed_query --test governor_correctness --test lateral_e2e --test exists_sep0007 --test correlated_positive_regions
    counted_rust lower-evaluator 12
    run lower-checkpoints "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib row_checkpoint::tests
    counted_rust lower-checkpoints 1
    rg -q '^test result: ok\. 9 passed; 0 failed; 0 ignored;' "$logs/lower-checkpoints.results"
    run lower-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval -p purrdf-python --all-targets -- -D warnings
    run lower-fmt "$sdk/cargo" fmt --all --check
    run lower-hygiene make helpers-hygiene layer-hygiene terminal-hygiene rdf-core-hygiene python-binding-hygiene
    run lower-ratchet "$sdk/cargo" run --locked --jobs 8 -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
    ;;
final-rust)
    run final-owning-check "$sdk/cargo" check --locked --jobs 8 --verbose -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-python -p purrdf-shapes
    run final-evaluator "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test graph_var_narrowing --test graph_constant_membership --test graph_scope_admission --test rdflib_contextual --test prepared_parameters --test native_xpath --test numeric_governance --test numeric_parallel_determinism --test governed_query --test governor_correctness
    counted_rust final-evaluator 10
    run final-invocation "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --lib user_fn::tests::invocation_admission
    counted_rust final-invocation 1
    run final-shapes "$sdk/cargo" test --locked --jobs 8 -p purrdf-shapes --test dated_execution --test prebinding_lanes --test blank_focus_relation_prebinding --test exists_relation_prebinding --test prebinding_above_hidden_columns
    counted_rust final-shapes 5
    run final-contextual-admission "$sdk/cargo" test --locked --jobs 8 -p purrdf-shapes --lib contextual_application
    counted_rust final-contextual-admission 1
    rg -q '^test result: ok\. 2 passed; 0 failed; 0 ignored;' "$logs/final-contextual-admission.results"
    run final-shape-source-admission "$sdk/cargo" test --locked --jobs 8 -p purrdf-shapes --lib shapes::parser::admission::tests
    counted_rust final-shape-source-admission 1
    run final-serializer "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-algebra --test serializer_roundtrip_sweep
    counted_rust final-serializer 1
    rg -q '^test result: ok\. 4 passed; 0 failed; 0 ignored;' "$logs/final-serializer.results"
    run final-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-python -p purrdf-shapes --all-targets -- -D warnings
    run final-fmt "$sdk/cargo" fmt --all --check
    run final-stub python3 scripts/check-python-stub-parity.py
    run final-hygiene make helpers-hygiene layer-hygiene terminal-hygiene rdf-core-hygiene python-binding-hygiene
    run final-ratchet "$sdk/cargo" run --locked --jobs 8 -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
    ;;
pre-sync-remainder)
    run affected-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval --all-targets -- -D warnings
    run fmt "$sdk/cargo" fmt --all --check
    run english-doc-claims python3 scripts/check-doc-claims.py
    run english-brand python3 scripts/check-brand-casing.py
    run english-issue-refs python3 scripts/check-issue-refs.py
    run english-attribution python3 scripts/check-spec-attribution.py --self-test
    ;;
pre-sync)
    run graph-contextual "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test graph_var_narrowing --test graph_constant_membership --test graph_scope_admission --test rdflib_contextual
    counted_rust graph-contextual 4
    run affected-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-eval --all-targets -- -D warnings
    run fmt "$sdk/cargo" fmt --all --check
    run english-doc-claims python3 scripts/check-doc-claims.py
    run english-brand python3 scripts/check-brand-casing.py
    run english-issue-refs python3 scripts/check-issue-refs.py
    run english-attribution python3 scripts/check-spec-attribution.py --self-test
    ;;
rust-row)
    run row-binding-check "$sdk/cargo" check --locked --jobs 8 --verbose -p purrdf-python
    run row-binding-clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-python --all-targets -- -D warnings
    run row-fmt "$sdk/cargo" fmt --all --check
    run row-stub-parity python3 scripts/check-python-stub-parity.py
    run row-binding-hygiene make python-binding-hygiene
    run row-ratchet "$sdk/cargo" run --locked --jobs 8 -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
    run row-shared-hygiene make helpers-hygiene layer-hygiene
    ;;
rust-final-gates)
    run shapes-application-controls "$sdk/cargo" test --locked --jobs 8 -p purrdf-shapes --lib contextual_application
    counted_rust shapes-application-controls 1
    rg -q '^test result: ok\. 2 passed; 0 failed; 0 ignored;' "$logs/shapes-application-controls.results"
    run clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-python -p purrdf-shapes --all-targets -- -D warnings
    run fmt "$sdk/cargo" fmt --all --check
    run combined-hygiene make helpers-hygiene layer-hygiene terminal-hygiene build-profile-hygiene rdf-core-hygiene
    ;;
rust-remainder)
    run serializer-oracle "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-algebra --test serializer_roundtrip_sweep
    counted_rust serializer-oracle 1
    rg -q '^test result: ok\. 4 passed; 0 failed; 0 ignored;' "$logs/serializer-oracle.results"
    run clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-python -p purrdf-shapes --all-targets -- -D warnings
    run fmt "$sdk/cargo" fmt --all --check
    run combined-hygiene make helpers-hygiene layer-hygiene terminal-hygiene build-profile-hygiene rdf-core-hygiene
    ;;
rust)
    run owning-check "$sdk/cargo" check --locked --jobs 8 --verbose -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-python
    run new-propagation "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-eval --test native_xpath contextual_reassignment_reuses_current_dated_laws_and_refuses_resource_exhaustion -- --exact
    run algebra "$sdk/cargo" test --locked --jobs 8 -p purrdf-sparql-algebra --lib --test iterative_traits
    run evaluator "$sdk/cargo" test --locked --jobs 8 --verbose -p purrdf-sparql-eval --lib --test rdflib_contextual --test exists_sep0007 --test prebound_grouping --test prepared_admission --test prepared_reuse --test native_xpath --test numeric_governance --test numeric_parallel_determinism --test governed_query --test governed_update --test governor_correctness
    run shapes "$sdk/cargo" test --locked --jobs 8 -p purrdf-shapes --test prebinding_lanes --test blank_focus_relation_prebinding --test exists_relation_prebinding --test prebinding_above_hidden_columns --test native_xpath
    counted_rust new-propagation 1
    rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$logs/new-propagation.results"
    counted_rust algebra 2
    counted_rust evaluator 12
    counted_rust shapes 5
    run clippy "$sdk/cargo" clippy --locked --jobs 8 -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-python -p purrdf-shapes --all-targets -- -D warnings
    run fmt "$sdk/cargo" fmt --all --check
    run combined-hygiene make helpers-hygiene layer-hygiene terminal-hygiene build-profile-hygiene rdf-core-hygiene
    ;;
docs)
    # book-po-update MUST have settled before root binds EXPECT_TREE. It changes
    # tracked source and therefore is deliberately not hidden inside this gate.
    run glossary-self-test "$sdk/cargo" run --locked --jobs 8 -p helper-census -- --glossary-gate --self-test
    run i18n make check-i18n
    run generated bash scripts/check-generated.sh
    run english-brand python3 scripts/check-brand-casing.py
    run english-issue-refs python3 scripts/check-issue-refs.py
    run english-attribution python3 scripts/check-spec-attribution.py --self-test
    run english-doc-claims python3 scripts/check-doc-claims.py
    run english-book make book
    # book-samples may regenerate owned projections. Require byte identity before
    # the zh build or any successful final source-binding claim.
    sha256sum --check --quiet "$logs/source-start.sha256" > "$logs/english-book-source-readback.txt"
    git diff --quiet
    run chinese-book make book-zh
    # Preserve the complete per-language HTML inventories, rather than count
    # the nested zh tree as part of the English book or infer render coverage
    # from source Markdown alone.
    find docs/book/book -path docs/book/book/zh-Hans -prune -o -type f -name '*.html' -print | sort > "$logs/english-html-paths.txt"
    find docs/book/book/zh-Hans -type f -name '*.html' -print | sort > "$logs/chinese-html-paths.txt"
    wc -l "$logs/english-html-paths.txt" "$logs/chinese-html-paths.txt" > "$logs/html-inventory-counts.txt"
    while IFS= read -r source; do
        if test "$source" = docs/book/src/SUMMARY.md; then continue; fi
        relative=${source#docs/book/src/}
        test -s "docs/book/book/${relative%.md}.html"
        test -s "docs/book/book/zh-Hans/${relative%.md}.html"
    done < <(find docs/book/src -type f -name '*.md' | sort)
    find docs/book/book -type f -print0 | sort -z | xargs -0 sha256sum > "$logs/rendered-html-artifacts.sha256"
    sha256sum --check --quiet "$logs/rendered-html-artifacts.sha256" > "$logs/rendered-html-readback.txt"
    ;;
python)
    run locked-dependencies uv sync --project bindings/python --locked --group dev --no-install-project
    mkdir "$logs/wheel"
    run production-wheel uv build --no-create-gitignore --wheel --project bindings/python --out-dir "$logs/wheel" --config-setting 'maturin.build-args=--locked --jobs 8'
    wheels=("$logs"/wheel/*.whl)
    test "${#wheels[@]}" -eq 1
    test -f "${wheels[0]}"
    run exact-wheel-install uv pip install --python bindings/python/.venv/bin/python --no-deps --reinstall "${wheels[0]}"
    identity=$root/.stage/rdflib-shim-algebra-level-reassignment/tasks/portfolio-python-wheel-identity.py
    run installed-wheel-identity bindings/python/.venv/bin/python -I "$identity" "${wheels[0]}" "$root/bindings/python/.venv"
    sha256sum "${wheels[0]}" bindings/python/uv.lock bindings/python/tests/test_contextual_mappings.py bindings/python/tests/test_solution_row_protocol.py "$identity" > "$logs/binding-start.sha256"
    run only-boundary-collection bindings/python/.venv/bin/python -I -m pytest --collect-only -q bindings/python/tests/test_contextual_mappings.py bindings/python/tests/test_solution_row_protocol.py
    rg -q '^28 tests collected' "$logs/only-boundary-collection.log"
    run only-boundary bindings/python/.venv/bin/python -I -m pytest -v bindings/python/tests/test_contextual_mappings.py bindings/python/tests/test_solution_row_protocol.py
    rg -q '(^| )28 passed([,= ]|$)' "$logs/only-boundary.log"
    if rg -q '[0-9]+ (failed|skipped|xfailed|xpassed)' "$logs/only-boundary.log"; then exit 1; fi
    sha256sum --check "$logs/binding-start.sha256" > "$logs/binding-readback.txt"
    run installed-wheel-readback bindings/python/.venv/bin/python -I "$identity" "${wheels[0]}" "$root/bindings/python/.venv"
    cmp "$logs/installed-wheel-identity.log" "$logs/installed-wheel-readback.log"
    ;;
cost|cost-reuse-baseline)
    roles=(baseline candidate)
    baseline_logs=$logs
    if test "$phase" = cost-reuse-baseline; then
        roles=(candidate)
        baseline_logs=${PURRDF_454_BASELINE_EVIDENCE:?immutable settled baseline evidence required}
        test "$baseline_logs" = /opt/purrdf-454-current-qualification-20261008/final-lower-native-cost-1
        jq -e ' .session == 85204 and .observed_terminal_exit == 0' "$baseline_logs/session-terminal.json" > "$logs/baseline-terminal-admission.txt"
        test "$(cat "$baseline_logs/driver.exit")" = 0
        cmp "$logs/compiler.txt" "$baseline_logs/compiler.txt"
        cmp "$logs/cargo.txt" "$baseline_logs/cargo.txt"
        cmp "$logs/path.txt" "$baseline_logs/path.txt"
        cmp "$logs/profile-environment.txt" "$baseline_logs/profile-environment.txt"
        test "${RUSTC_WRAPPER-}" = kache
        test -z "${RUSTC_WORKSPACE_WRAPPER-}"
        rg -q 'Running `kache rustc' "$baseline_logs/baseline-native-emit.log"
        rg -q 'Running `kache rustc' "$baseline_logs/baseline-host-emit.log"
        # Freeze original metadata/argv/cohort/manifest/records receipts, not a
        # rewritten old frame with new paths and never an old candidate artifact.
        find "$baseline_logs" -maxdepth 1 -type f -name 'baseline-*' -print0 | sort -z | xargs -0 sha256sum > "$logs/reused-baseline-evidence.sha256"
        printf '%s\n' "$baseline_logs" > "$logs/reused-baseline-root.txt"
    fi
    cost_root=$logs/artifacts
    mkdir "$cost_root" # every retry owns fresh probe/native/host roots
    baseline=${PURRDF_454_BASELINE_ROOT:?root-materialized exact main source required}
    baseline_oid=${PURRDF_454_BASELINE_OID:?actual base OID required}
    export PYO3_PYTHON=${PURRDF_454_PYO3_PYTHON:?same absolute supported Python executable required for both emissions}
    test -x "$PYO3_PYTHON"
    test -f "$baseline/Cargo.toml"
    test "$baseline_oid" = "$(git rev-parse MERGE_HEAD)"
    printf '%s\n' "$baseline_oid" "$baseline" > "$logs/baseline-source.txt"
    printf '%s\n' "$PYO3_PYTHON" > "$logs/matched-python.txt"
    git ls-tree -r --name-only "$baseline_oid" > "$logs/baseline-tracked-paths.txt"
    while IFS= read -r path; do
        git cat-file blob "$baseline_oid:$path" | cmp - "$baseline/$path"
        sha256sum "$baseline/$path"
    done < "$logs/baseline-tracked-paths.txt" > "$logs/baseline-source-start.sha256"
    # Root must capture the exact base archive/readback before this phase. This
    # driver creates only owned probe manifests, never edits either source root.
    template=$root/.stage/rdflib-shim-algebra-level-reassignment/raw/native-cost-t3-candidate-probe/src/main.rs
    sha256sum "$template" > "$logs/common-probe-source.sha256"
    for role in "${roles[@]}"; do
        if test "$role" = baseline; then source_root=$baseline; else source_root=$root; fi
        probe=$cost_root/probe-$role
        mkdir "$probe"
        mkdir "$probe/src"
        cp "$template" "$probe/src/main.rs"
        cat > "$probe/Cargo.toml" <<EOF
[package]
name = "qualification-454-native-cost"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
purrdf-sparql-algebra = { path = "$source_root/crates/sparql-algebra" }
purrdf-sparql-eval = { path = "$source_root/crates/sparql-eval" }
purrdf-core = { path = "$source_root/crates/rdf-core" }
purrdf-alloc-probe = { path = "$source_root/crates/alloc-probe" }
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
debug-assertions = false
overflow-checks = false
EOF
        cp "$source_root/Cargo.lock" "$probe/Cargo.lock"
        sha256sum "$source_root/Cargo.lock" "$probe/Cargo.lock" > "$logs/$role-production-lock-seed.sha256"
        # Add the owned probe root while retaining the existing locked versions.
        # Unlike generate-lockfile, ordinary offline resolution reuses this seed.
        json_run "$role-native-resolve" "$sdk/cargo" metadata --offline --format-version 1 --manifest-path "$probe/Cargo.toml"
        lock_registry_rows "$source_root/Cargo.lock" > "$logs/$role-production-registry.tsv"
        lock_registry_rows "$probe/Cargo.lock" > "$logs/$role-probe-registry.tsv"
        LC_ALL=C comm -23 "$logs/$role-probe-registry.tsv" "$logs/$role-production-registry.tsv" > "$logs/$role-probe-unapproved.tsv"
        test ! -s "$logs/$role-probe-unapproved.tsv"
        json_run "$role-native-metadata" "$sdk/cargo" metadata --offline --locked --format-version 1 --manifest-path "$probe/Cargo.toml"
    done
    base_probe=$cost_root/probe-baseline
    if test "$phase" = cost-reuse-baseline; then
        base_probe=$baseline_logs/artifacts/probe-baseline
        cmp "$logs/matched-python.txt" "$baseline_logs/matched-python.txt"
        cmp "$logs/baseline-source.txt" "$baseline_logs/baseline-source.txt"
        for receipt in baseline-native-inputs baseline-native-emitted baseline-host-emitted baseline-host-library-artifacts; do
            sha256sum --check --quiet "$baseline_logs/$receipt.sha256" > "$logs/$receipt-reuse-admission.txt"
        done
        cmp "$logs/baseline-source-start.sha256" "$baseline_logs/baseline-source-start.sha256"
        verify_compiled_cohort reused-baseline-native "$baseline_logs/baseline-native-metadata.json" "$baseline_logs/baseline-native-emit.json" "$base_probe/Cargo.lock" "$baseline/Cargo.lock"
        verify_compiled_cohort reused-baseline-host "$baseline_logs/baseline-host-metadata.json" "$baseline_logs/baseline-host-emit.json" "$baseline/Cargo.lock" "$baseline/Cargo.lock"
        mapfile -t base_native_paths < "$baseline_logs/baseline-native-emitted-paths.txt"
        mapfile -t base_host_paths < "$baseline_logs/baseline-host-emitted-paths.txt"
        validate_emitted_pairs 54 "${base_native_paths[@]}"
        validate_emitted_pairs 1 "${base_host_paths[@]}"
        test "$(jq -s '[.[] | select(.reason=="compiler-artifact" and .target.name=="qualification-454-native-cost" and .profile.test==false and .executable!=null)] | length' "$baseline_logs/baseline-native-emit.json")" -eq 1
        test "$(jq -s '[.[] | select(.reason=="compiler-artifact" and .target.name=="purrdf_native" and .profile.test==false)] | length' "$baseline_logs/baseline-host-emit.json")" -eq 1
        pyo3_build_binding reused-baseline "$baseline_logs/baseline-host-emit.json"
        # Source/runtime SDK has not been replaced; current candidate's actual
        # emitted PyO3 input/config binding must nevertheless equal this below.
        for variable in PYO3_CONFIG_FILE PYO3_CROSS PYO3_CROSS_LIB_DIR PYO3_CROSS_PYTHON_VERSION PYO3_CROSS_PYTHON_IMPLEMENTATION PYO3_NO_PYTHON PYO3_ENVIRONMENT_SIGNATURE PYO3_PRINT_CONFIG; do
            test -z "${!variable-}"
        done
    fi
    cmp "$base_probe/src/main.rs" "$cost_root/probe-candidate/src/main.rs"
    cmp "$base_probe/Cargo.lock" "$cost_root/probe-candidate/Cargo.lock"
    # Matched explicit effective settings reproduce the retained actual native
    # ThinLTO stage, not the stale fat-LTO/strip-none template defaults.
    export RUSTFLAGS='-Dwarnings -Ctarget-cpu=native'
    export CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_STRIP=symbols
    export CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=false CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false
    printf '%s\n' 'Matched native proof stage: O3, ThinLTO, codegen1, strip symbols, assertions/overflow false, target-cpu native, warnings denied.' 'Matched host proof stage: same settings except LTO false; pre-link body/ABI proof, not final fat-LTO identity.' > "$logs/effective-proof-profiles.txt"
    if test "$phase" = cost-reuse-baseline; then
        cmp "$logs/effective-proof-profiles.txt" "$baseline_logs/effective-proof-profiles.txt"
    fi
    for role in "${roles[@]}"; do
        if test "$role" = baseline; then source_root=$baseline; else source_root=$root; fi
        probe=$cost_root/probe-$role
        export CARGO_TARGET_DIR="$cost_root/$role-native-target" CARGO_BUILD_BUILD_DIR="$cost_root/$role-native-build"
        export CARGO_PROFILE_RELEASE_LTO=thin
        json_run "$role-native-emit" "$sdk/cargo" rustc --locked --jobs 8 --release --manifest-path "$probe/Cargo.toml" --message-format=json-render-diagnostics --verbose -- --emit=link,llvm-ir,asm
        verify_compiled_cohort "$role-native" "$logs/$role-native-metadata.json" "$logs/$role-native-emit.json" "$probe/Cargo.lock" "$source_root/Cargo.lock"
        mapfile -t selected < <(jq -r 'select(.reason=="compiler-artifact" and .target.name=="qualification-454-native-cost" and .profile.test==false) | .executable // empty' "$logs/$role-native-emit.json")
        test "${#selected[@]}" -eq 1
        test -x "${selected[0]}"
        sha256sum "${selected[0]}" "$probe/src/main.rs" "$probe/Cargo.toml" "$probe/Cargo.lock" > "$logs/$role-native-inputs.sha256"
        run "$role-untimed-probe" "${selected[0]}"
        test "$(wc -l < "$logs/$role-untimed-probe.log")" -eq 32
        mapfile -t emitted < <(rg --files --no-ignore "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" | rg '/qualification_454_native_cost[^/]*\.(ll|s)$' | sort)
        validate_emitted_pairs 0 "${emitted[@]}"
        for module in purrdf_sparql_algebra purrdf_sparql_eval purrdf_xsd; do
            printf '%s\n' "${emitted[@]}" | rg -q "/qualification_454_native_cost[^/]*$module[^/]*\.ll$"
        done
        printf '%s\n' "${emitted[@]}" | rg -q '/qualification_454_native_cost\.qualification_454_native_cost[^/]*\.ll$'
        sha256sum "${emitted[@]}" > "$logs/$role-native-emitted.sha256"
        printf '%s\n' "${emitted[@]}" > "$logs/$role-native-emitted-paths.txt"
    done
    run untimed-equality cmp "$baseline_logs/baseline-untimed-probe.log" "$logs/candidate-untimed-probe.log"
    # No-LTO host emission is a DIFFERENT stage. Full bodies/callers/capture/drop
    # require independent inspection; equality of probe totals is not that proof.
    export CARGO_PROFILE_RELEASE_LTO=false
    for role in "${roles[@]}"; do
        if test "$role" = baseline; then source_root=$baseline; else source_root=$root; fi
        export CARGO_TARGET_DIR="$cost_root/$role-host-target" CARGO_BUILD_BUILD_DIR="$cost_root/$role-host-build"
        cd "$source_root"
        json_run "$role-host-metadata" "$sdk/cargo" metadata --offline --locked --format-version 1
        json_run "$role-host-emit" "$sdk/cargo" rustc --locked --jobs 8 --release -p purrdf-python --lib --message-format=json-render-diagnostics --verbose -- --emit=link,llvm-ir,asm
        verify_compiled_cohort "$role-host" "$logs/$role-host-metadata.json" "$logs/$role-host-emit.json" "$source_root/Cargo.lock" "$source_root/Cargo.lock"
        jq -c 'select(.reason=="compiler-artifact" and .target.name=="purrdf_native" and .profile.test==false)' "$logs/$role-host-emit.json" > "$logs/$role-host-selected-frame.json"
        test "$(wc -l < "$logs/$role-host-selected-frame.json")" -eq 1
        mapfile -t host_libraries < <(jq -r '.filenames[] | select(endswith(".so") or endswith(".rlib"))' "$logs/$role-host-selected-frame.json")
        test "${#host_libraries[@]}" -eq 2
        for path in "${host_libraries[@]}"; do test -s "$path"; done
        printf '%s\n' "${host_libraries[@]}" | rg -q '\.so$'
        printf '%s\n' "${host_libraries[@]}" | rg -q '\.rlib$'
        sha256sum "${host_libraries[@]}" > "$logs/$role-host-library-artifacts.sha256"
        mapfile -t emitted < <(rg --files "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" | rg '/purrdf_native[^/]*\.(ll|s)$')
        validate_emitted_pairs 1 "${emitted[@]}"
        sha256sum "${emitted[@]}" > "$logs/$role-host-emitted.sha256"
        printf '%s\n' "${emitted[@]}" > "$logs/$role-host-emitted-paths.txt"
    done
    cd "$root"
    for role in "${roles[@]}"; do
        sha256sum --check --quiet "$logs/$role-native-inputs.sha256" > "$logs/$role-native-inputs-readback.txt"
        sha256sum --check --quiet "$logs/$role-native-emitted.sha256" > "$logs/$role-native-emitted-readback.txt"
        sha256sum --check --quiet "$logs/$role-host-emitted.sha256" > "$logs/$role-host-emitted-readback.txt"
        sha256sum --check --quiet "$logs/$role-host-library-artifacts.sha256" > "$logs/$role-host-library-readback.txt"
    done
    if test "$phase" = cost-reuse-baseline; then
        pyo3_build_binding candidate "$logs/candidate-host-emit.json"
        cmp "$logs/reused-baseline-pyo3-build-config.json" "$logs/candidate-pyo3-build-config.json"
        cmp "$logs/reused-baseline-pyo3-build-inputs.jsonl" "$logs/candidate-pyo3-build-inputs.jsonl"
        sha256sum --check --quiet "$logs/reused-baseline-pyo3-fingerprints.sha256" > "$logs/reused-baseline-pyo3-final-readback.txt"
        sha256sum --check --quiet "$logs/candidate-pyo3-fingerprints.sha256" > "$logs/candidate-pyo3-final-readback.txt"
        for receipt in baseline-native-inputs baseline-native-emitted baseline-host-emitted baseline-host-library-artifacts; do
            sha256sum --check --quiet "$baseline_logs/$receipt.sha256" > "$logs/$receipt-reuse-final-readback.txt"
        done
        sha256sum --check --quiet "$logs/reused-baseline-evidence.sha256" > "$logs/reused-baseline-evidence-final-readback.txt"
    fi
    sha256sum --check --quiet "$logs/baseline-source-start.sha256" > "$logs/baseline-source-readback.txt"
    printf '%s\n' 'Actual artifact generation only; independent emitted-code/native-cost adjudication remains REQUIRED.' > "$logs/cost-verdict-pending.txt"
    ;;
*) printf 'unknown phase: %s\n' "$phase" >&2; exit 2 ;;
esac
cd "$root"
sha256sum --check --quiet "$logs/source-start.sha256" > "$logs/source-readback.txt"
test "$(git write-tree)" = "$expected_tree"
git diff --quiet
printf '0\n' > "$logs/driver.exit"
