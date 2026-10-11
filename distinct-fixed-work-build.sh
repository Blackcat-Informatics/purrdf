#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
stage="$root/.worktrees/515-sparql-eval-datalog-per-row-evidence/.stage/sparql-eval-datalog-per-row-evidence"
version=${1:?baseline or current}
case "$version" in baseline|current) ;; *) exit 2 ;; esac
cache="$root/.worktrees/purrdf-evidence-default-$version"
export TMPDIR="$cache/tmp"
export RUSTUP_TOOLCHAIN=nightly
export RUSTC_WRAPPER=''
args=(--edition=2024 --crate-name query_eval -C opt-level=3 -C lto=thin -C codegen-units=1 -C debuginfo=0 -C strip=none -D warnings)
mapfile -t artifacts < <(rg --files --hidden --no-ignore "$cache/build/release" | rg '\.rlib$')
for artifact in "${artifacts[@]}"; do
    args+=(-L "dependency=${artifact%/*}")
done
for name in purrdf_core purrdf_sparql_eval purrdf_testkit purrdf_xsd; do
    matches=()
    for artifact in "${artifacts[@]}"; do
        case "${artifact##*/}" in "lib${name}-"*.rlib) matches+=("$artifact") ;; esac
    done
    if [[ ${#matches[@]} != 1 ]]; then
        echo "Required original $version $name rlib missing or ambiguous; reconstruct the original dependency with matching profile, never substitute another source." >&2
        exit 42
    fi
    args+=(--extern "$name=${matches[0]}")
    metadata="${matches[0]%.rlib}.rmeta"
    if [[ -f "$metadata" ]]; then args+=(--extern "$name=$metadata"); fi
done
/home/paudley/stage/packages/rustup/active-toolchain/bin/rustc "${args[@]}" "$stage/distinct-fixed-work-$version.rs" -o "$stage/distinct-fixed-work-$version"
