#!/usr/bin/env bash
set -eu
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 CARGO_TERM_VERBOSE=true
export CARGO_TARGET_DIR=/opt/purrdf-308-current-telemetry.591np1/metadata-current/capi-shipping/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-308-current-telemetry.591np1/metadata-current/capi-shipping/build
export PURRDF_PROFILE_CARGO=/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo
logs=/home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c/.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-metadata-logs
evidence=/opt/purrdf-308-current-telemetry.591np1/metadata-current/invalidations
harness=$(cat "$logs/shipping-harness.path")
library=$CARGO_TARGET_DIR/debug/libpurrdf.so
cd /opt/purrdf-308-current-telemetry.591np1/metadata-current/shipping-source
git diff --exit-code > "$evidence/source-baseline.diff"
cp crates/rdf-capi/src/lib.rs "$evidence/lib.rs.original"
test -f "$library"
b3sum "$library" > "$evidence/source-change-existing-library.blake3"
printf '\n// The C boundary retains caller-owned result lifetimes.\n' >> crates/rdf-capi/src/lib.rs
git diff -- crates/rdf-capi/src/lib.rs > "$evidence/source-mutated.diff"
b3sum crates/rdf-capi/src/lib.rs > "$evidence/source-mutated.blake3"
for label in 03-source-changed 04-source-unchanged 05-source-restored; do
    if [ "$label" = 05-source-restored ]; then
        cp "$evidence/lib.rs.original" crates/rdf-capi/src/lib.rs
        cmp "$evidence/lib.rs.original" crates/rdf-capi/src/lib.rs
        git diff --exit-code > "$evidence/source-restored.diff"
    fi
    test -f "$library"
    b3sum "$library" > "$evidence/$label-library-before.blake3"
    stat -c '%s %Y %n' "$library" > "$evidence/$label-library-before.stat"
    export PURRDF_C_PHASE_RECEIPT="$evidence/$label-phases.json"
    printf '%q ' "$harness" --exact c_abi_smoke --nocapture >> "$evidence/commands.txt"
    printf '\n' >> "$evidence/commands.txt"
    set +e
    "$harness" --exact c_abi_smoke --nocapture > "$evidence/$label.log" 2>&1
    result=$?
    set -e
    printf '%s\t%s\n' "$label" "$result" >> "$evidence/terminals.txt"
    printf '%s exit %s\n' "$label" "$result"
    if [ "$result" -ne 0 ]; then exit "$result"; fi
done
