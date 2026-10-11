#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
stage="$root/.worktrees/515-sparql-eval-datalog-per-row-evidence/.stage/sparql-eval-datalog-per-row-evidence"
private_unit=run-p1468757-i18293369.scope
if [[ ${1:-} != --inside ]]; then
    [[ $(systemctl --user show "$private_unit" --property=FreezerState --value) == running ]]
    trap 'systemctl --user thaw "$private_unit"' EXIT
    systemctl --user freeze "$private_unit"
    [[ $(systemctl --user show "$private_unit" --property=FreezerState --value) == frozen ]]
    systemd-run --user --scope --quiet --expand-environment=no -p MemoryMax=16G -p MemorySwapMax=0 -- bash "$0" --inside
    exit
fi
cd "$root"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly
export RUSTC_WRAPPER=''
unset CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
bash "$stage/distinct-fixed-work-baseline-core-build.sh" > "$stage/distinct-fixed-work-baseline-core-build.log" 2>&1
fingerprint="$root/.worktrees/purrdf-evidence-default-baseline/build/release/build/purrdf-core/c15b9442455e77e1/fingerprint/lib-purrdf_core.json"
rlib="$root/.worktrees/purrdf-evidence-default-baseline/build/release/build/purrdf-core/c15b9442455e77e1/out/libpurrdf_core-c15b9442455e77e1.rlib"
cp "$fingerprint" "$stage/distinct-fixed-work-baseline-core-after.json"
if [[ ! -f "$rlib" ]] || ! cmp -s "$stage/distinct-fixed-work-baseline-core-before.json" "$fingerprint"; then
    diff -u "$stage/distinct-fixed-work-baseline-core-before.json" "$fingerprint" > "$stage/distinct-fixed-work-baseline-core-fingerprint.diff" || true
    echo 'Original baseline core fingerprint/artifact identity not reproduced; stop before diagnostic compilation.' >&2
    exit 43
fi
echo 'Original baseline core fingerprint and rlib MATCH.'
for version in baseline current; do
    bash "$stage/distinct-fixed-work-build.sh" "$version" > "$stage/distinct-fixed-work-$version-build.log" 2>&1
    echo "$version fixed-work driver built."
done
cmp "$stage/distinct-fixed-work-baseline.rs" "$stage/distinct-fixed-work-current.rs"
for pair in 1 2 3 4 5; do
    if (( pair % 2 )); then versions=(baseline current); else versions=(current baseline); fi
    for version in "${versions[@]}"; do
        directory="$stage/distinct-fixed-work-pair-$pair-$version"
        [[ ! -e "$directory" ]]
        chrt --batch 0 taskset -c 11 bash -c 'renice -n -4 -p "$$" >/dev/null; exec "$1" --collect "$2"' fixed-work "$stage/distinct-fixed-work-$version" "$directory" > "$stage/distinct-fixed-work-pair-$pair-$version.log" 2>&1
        cat "$stage/distinct-fixed-work-pair-$pair-$version.log"
    done
done
echo 'Five exact paired original-query instruction captures completed; throughput acceptance remains separate.'
