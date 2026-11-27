#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

set -euo pipefail

config=aarch64-neoverse-v1
triple=aarch64-unknown-linux-gnu
compiler_sha=a9c1c9ab765535974b0d7e1f4863ee2016f94b13a0a15c9fcd616bb4616f3e22
manifest_sha=23dfb91389e8028e8a7594aab1448b9624569dff43c8c782efbb1b5a5eaab2eb
stock_source=67c6957b8a76f67a694689d9658f64f5f909711e2446f146b5c127c6a7c0d5bb
stock_cells=a806c60f2fb45c784d1a1ea355ead657527cd228adbef4eaa4bd804eb7478cc0
baseline_run=46047659305
baseline_head=fab51ad9f89c160c05089f79bdaa61a3c39a91d1
baseline_branch=paudley/1469-qualify-compiled-assembly-cache-reuse
baseline_context=42846becc244d99157eb60e04c05e3938cec8c344039c89b8ff0941adb84c565
assembly_context=8b2fb22c061b1b984a0934cc1398495344e5eeba878ad212ca9cbf8de50fe5dc
seed_sha=b260969e5774bee5894da292a6e1a8465e5ba5a216c7f7266d0d99b0f89353f3
measured_workspace=/home/runner/work/purrdf/purrdf
output_paths=(target/release "target/$triple/release" target/simd-asm)

fail() {
    printf 'Unqualified: %s\n' "$*" >&2
    exit 1
}

sha() {
    sha256sum "$@" | cut -d ' ' -f 1
}

source_digest() {
    PYTHONPATH=scripts python3 -B -c 'from pathlib import Path; from simd_asm_runtime import source_identity; print(source_identity(Path.cwd()))'
}

validate_report() {
    local file=$1 source=$2
    jq -e --arg config "$config" --arg source "$source" --arg manifest "$manifest_sha" '
        .status == "passed" and .identity.schema == 1 and
        .identity.source == $source and .identity.manifest == $manifest and
        (.cells | keys == [$config]) and (.cells[$config] | length == 116) and
        (.timings | keys == [$config]) and
        (.timings[$config].rustc_invocations | type == "number")
    ' "$file" >/dev/null || fail "incomplete or foreign assembly report: $file"
    [[ $(jq -cj '.identity.compiler' "$file" | sha) == "$compiler_sha" ]] ||
        fail "report used a different compiler: $file"
    [[ $(jq -cS --arg config "$config" '.cells[$config]' "$file" | sha) == "$stock_cells" ]] ||
        fail "assembly cells differ from the checked stock result: $file"
}

validate_baseline() {
    local baseline=$1 current=$2
    [[ $(sha "$baseline/context.txt") == "$baseline_context" &&
        $(cat "$baseline/context.sha256") == "$baseline_context" ]] ||
        fail "retained baseline context digest differs"
    cmp "$baseline/context.txt" "$baseline/stock-context.txt" ||
        fail "original stock arm used a different context"
    cmp "$baseline/stock-context.txt" "$current" ||
        fail "runner, checkout path, compiler or toolchain context differs from stock"
    [[ $(cat "$baseline/stock/gate.exit") == 0 &&
        $(cat "$baseline/stock/graph-count.txt") == 4 &&
        $(cat "$baseline/stock/gate.seconds") == 2455.91 &&
        $(cat "$baseline/stock/rustc-counts.txt") == $'host_rustc=21\narm_rustc=137' &&
        $(cat "$baseline/stock/cache-context.txt") == stock_context=present_in_archive &&
        $(cat "$baseline/seed.sha256") == "$seed_sha" ]] ||
        fail "the retained stock arm did not record the approved successful measurement and seed"
    diff -q "$baseline/stock/graph-contexts.txt" <(
        printf '%s\n' "$measured_workspace/target/simd-asm/contexts/$assembly_context"{,,,}
    ) >/dev/null || fail "the retained stock graph did not use the approved assembly context"
    validate_report "$baseline/stock/report.json" "$stock_source"
    jq -e --arg config "$config" '.timings[$config].rustc_invocations == 137' \
        "$baseline/stock/report.json" >/dev/null || fail "the retained stock report disagrees with the compiler count"
}

# The hosted gate draws its source identity from the measured checkout, not this harness.
case "${1:-}" in
    validate-report) validate_report "$2" "${3:-$stock_source}"; exit ;;
    validate-baseline) validate_baseline "$2" "$3"; exit ;;
esac

cd "${GITHUB_WORKSPACE:?}"
evidence=${TRIAL:?}/evidence
report=target/asm-reports/$config.json

context() {
    printf '%s\n' "$PWD" "${ImageOS:?}" "${ImageVersion:?}" "$RUNNER_OS" "$RUNNER_ARCH"
    nproc
    rustc -vV
    cargo -Vv
    getconf GNU_LIBC_VERSION
    cc --version
    ld --version
    ar --version
    clang --version
    llvm-ar --version
    make --version
    aarch64-linux-gnu-gcc --version
    aarch64-linux-gnu-ld --version
    aarch64-linux-gnu-ar --version
    aarch64-linux-gnu-gcc -print-sysroot
    sha256sum "$(aarch64-linux-gnu-gcc -print-file-name=libc.so)"
    dpkg-query -W clang llvm gcc-aarch64-linux-gnu binutils-aarch64-linux-gnu libc6-dev-arm64-cross
    sha256sum Makefile Cargo.toml Cargo.lock rust-toolchain.toml \
        .github/actions/simd-assembly/action.yml scripts/check-simd-asm.py \
        scripts/simd_asm_runtime.py scripts/simd-asm-manifest.toml docs/design/purrdf-simd.md
    cargo -Z unstable-options config get --format json 2>/dev/null | sha
    local name
    while IFS= read -r name; do
        case "$name" in
            CARGO_TOKEN|CARGO_REGISTRY_TOKEN|CARGO_REGISTRIES_*_TOKEN) ;;
            CARGO_*|RUST*|MAKEFLAGS|MAKEOVERRIDES|MFLAGS|CC|CC_*|AR|AR_*|CFLAGS|CFLAGS_*|CXX|CXX_*|CXXFLAGS|CXXFLAGS_*)
                printf '%s\0%s\0' "$name" "${!name}" ;;
        esac
    done < <(compgen -e | LC_ALL=C sort -u) | sha
}

check_context() {
    local arm=$1
    context > "$evidence/$arm-context.txt"
    cmp "$evidence/context.txt" "$evidence/$arm-context.txt" ||
        fail "the build context changed for $arm"
}

check_clean_source() {
    [[ -z $(git status --porcelain=v1) && $(source_digest) == "$stock_source" ]] ||
        fail "the pinned source is not clean or differs from the stock report"
}

fetch_original() {
    local id=$1 kind=$2 digest=$3 bytes=$4
    local endpoint="repos/$GITHUB_REPOSITORY/actions/artifacts/$id"
    gh api "$endpoint" > "$evidence/original-$kind.json"
    jq -e --arg name "neoverse-trial-$kind-$baseline_run-1" \
        --arg digest "sha256:$digest" --arg branch "$baseline_branch" --arg head "$baseline_head" \
        --argjson id "$id" --argjson run "$baseline_run" --argjson bytes "$bytes" '
        .id == $id and .name == $name and .digest == $digest and
        .size_in_bytes == $bytes and .expired == false and
        (.workflow_run | .id == $run and .head_sha == $head and .head_branch == $branch and
            .repository_id == 1286258226 and .head_repository_id == 1286258226)
    ' "$evidence/original-$kind.json" >/dev/null || fail "original $kind artifact identity differs or expired"
    /usr/bin/time -f '%e' -o "$evidence/$kind-download.seconds" \
        gh api "$endpoint/zip" > "$TRIAL/$kind.zip"
    [[ $(sha "$TRIAL/$kind.zip") == "$digest" && $(stat -c%s "$TRIAL/$kind.zip") == "$bytes" ]] ||
        fail "original $kind ZIP bytes differ from the approved artifact"
    sha "$TRIAL/$kind.zip" > "$evidence/$kind-zip.sha256"
}

active_assembly_context() {
    # Ask the pinned checker which receipt context it would open, and stop
    # before the lease or any compilation. This uses the gate's own identity.
    PYTHONPATH=scripts python3 -B -c '
import contextlib
import importlib.util
import sys
import tomllib
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location("trial_gate", "scripts/check-simd-asm.py")
gate = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = gate
spec.loader.exec_module(gate)
manifest = gate.load_manifest(tomllib.loads(gate.MANIFEST.read_text(encoding="utf-8")))
config = gate.CONFIG_BY_NAME[sys.argv[1]]
runner = gate.runtime.Runner(gate, SimpleNamespace(fresh=False))

class BeforeBuild(Exception):
    pass

def identify(identity):
    raise BeforeBuild(gate.runtime.digest(gate.runtime.encoded(identity)))

runner.context = identify
try:
    with contextlib.redirect_stdout(sys.stderr):
        runner.configuration(config, manifest, gate.manifest_keep(manifest)(config))
except BeforeBuild as result:
    print(result)
else:
    raise SystemExit("checker did not select an assembly context")
' "$config"
}

record_graphs() {
    local arm=$1 path graph line graphs=0
    local dir=$evidence/$arm
    local pattern='^== aarch64-neoverse-v1 graph ([0-3]): (.*)$'
    while IFS= read -r line; do
        if [[ $line =~ $pattern ]]; then
            graph=${BASH_REMATCH[1]}
            path=${BASH_REMATCH[2]}
            [[ ! -e "$dir/graph-$graph.stderr" ]] || fail "graph $graph appeared twice"
            dirname "$path" >> "$dir/graph-contexts.txt"
            cp "$path" "$dir/graph-$graph.stderr"
            cp "${path%.stderr.log}.stdout.jsonl" "$dir/graph-$graph.jsonl"
            graphs=$((graphs + 1))
        fi
    done < "$dir/gate.log"
    printf '%s\n' "$graphs" > "$dir/graph-count.txt"
    if ((graphs)); then
        awk -v target="$triple" '
            /^ *Running `/ && / --crate-name / {
                if (index($0, " --target " target " ")) arm++
                else if ($0 !~ / --target /) host++
            }
            END {printf "host_rustc=%d\narm_rustc=%d\n", host, arm}
        ' "$dir"/graph-*.stderr > "$dir/rustc-counts.txt"
    fi
}

measure() {
    local arm=$1 source=$2 rc=0
    local dir=$evidence/$arm
    mkdir "$dir"
    [[ ! -e $report ]] || fail "old report still present before $arm"
    [[ ! -v ARTIFACT_ID && ! -v ARTIFACT_DIGEST && ! -v GH_TOKEN ]] ||
        fail "artifact transport environment leaked into the checker step"
    active_assembly_context > "$dir/preflight-context.txt"
    [[ $(cat "$dir/preflight-context.txt") == "$assembly_context" ]] ||
        fail "the checker would not use the exact original receipt context"
    compgen -G "target/simd-asm/contexts/$assembly_context/evidence/*.json" >/dev/null ||
        fail "the exact original assembly receipts are missing"
    {
        pwd
        git rev-parse HEAD 'HEAD^{tree}'
        git status --porcelain=v1
        git diff --no-ext-diff
    } > "$dir/source.txt"
    /usr/bin/time -f '%e' -o "$dir/gate.seconds" \
        make simd-asm SIMD_ASM_ARGS="--config $config --jobs 1 --report $report" \
        > "$dir/gate.log" 2>&1 || rc=$?
    printf '%d\n' "$rc" > "$dir/gate.exit"
    if [[ -f $report ]]; then
        mv "$report" "$dir/report.json"
    fi
    record_graphs "$arm"
    if ((rc != 0)); then
        tail -n 35 "$dir/gate.log" >&2
        fail "$arm gate exited $rc; no replacement cache or alternate command was used"
    fi
    [[ $(cat "$dir/graph-count.txt") == 4 ]] || fail "$arm missed a required Cargo graph"
    cmp "$dir/graph-contexts.txt" "$evidence/baseline/stock/graph-contexts.txt" ||
        fail "$arm used a different assembly context from stock"
    validate_report "$dir/report.json" "$source"
    local reported
    reported=$(jq -er --arg config "$config" '.timings[$config].rustc_invocations' "$dir/report.json")
    grep -Fxq "arm_rustc=$reported" "$dir/rustc-counts.txt" || fail "$arm Cargo counts disagree"
    [[ $(source_digest) == "$source" ]] || fail "the gate changed source during $arm"
    df -B1 "$PWD" "$TRIAL" > "$dir/disk.txt"
}

# The same graph must reuse the measured unit unchanged, then rebuild it after
# the one-byte edit. This discriminates a source-triggered rebuild from an
# output that Cargo was going to rebuild anyway.
require_arm_wasm_freshness() {
    local arm=$1 fresh=$2
    local graph=$evidence/$arm/graph-1
    jq -se --argjson fresh "$fresh" '[
        .[] | select(.reason == "compiler-artifact" and
            .target.name == "purrdf_wasm" and
            any(.filenames[]; contains("/aarch64-unknown-linux-gnu/")))
        | .fresh
    ] | length > 0 and all(.[]; . == $fresh)' "$graph.jsonl" ||
        fail "the measured Arm artifact in $arm did not have freshness $fresh"
    awk -v target="$triple" -v fresh="$fresh" '
        /^ *Running `/ && / --crate-name purrdf_wasm / && index($0, " --target " target " ") { found = 1 }
        END { exit !((found == 1) == (fresh == "false")) }
    ' "$graph.stderr" || fail "the Arm invocation in $arm disagrees with Cargo freshness"
}

case "${1:-}" in
    prepare)
        [[ $RUNNER_OS == Linux && $RUNNER_ARCH == X64 && $(git rev-parse HEAD) == "$MEASURED_SHA" &&
            $(git rev-parse 'HEAD^{tree}') == b1aa19681465361cd0638ed5e52d51de96f089ec ]] ||
            fail "wrong measured commit, tree or runner architecture"
        [[ -z ${CARGO_TARGET_DIR:-} && $CARGO_BUILD_BUILD_DIR == "$PWD/target" &&
            $CARGO_UNSTABLE_CHECKSUM_FRESHNESS == true && $CARGO_BUILD_FINGERPRINT == content ]] ||
            fail "trial needs the default Cargo target directory and exact checksum settings"
        [[ -z ${MAKEFILES:-} && -z ${RUSTFLAGS:-} && -z ${CARGO_ENCODED_RUSTFLAGS:-} ]] ||
            fail "inherited settings can override the assembly gate"
        [[ $(rustc -vV | sha) == "$compiler_sha" ]] || fail "resolved nightly differs from stock"
        check_clean_source
        context > "$evidence/context.txt"
        sha "$evidence/context.txt" > "$evidence/context.sha256"
        df -B1 "$PWD" "$TRIAL" > "$evidence/disk-before.txt"
        ;;
    download)
        [[ $GITHUB_REPOSITORY == Blackcat-Informatics/purrdf ]] || fail "the trial must run in its original repository"
        gh api "repos/$GITHUB_REPOSITORY/actions/runs/$baseline_run/attempts/1" > "$evidence/original-attempt.json"
        jq -e --argjson run "$baseline_run" --arg branch "$baseline_branch" --arg head "$baseline_head" '
            .id == $run and .run_attempt == 1 and .head_sha == $head and .head_branch == $branch and
            .path == ".github/workflows/neoverse-cache-trial.yaml" and .event == "push" and
            .status == "completed" and .conclusion == "failure" and
            .repository.id == 1286258226 and .head_repository.id == 1286258226
        ' "$evidence/original-attempt.json" >/dev/null || fail "not the original paired trial attempt"
        fetch_original 15225466319 evidence 0b1f7e9d24a09f929e191153c4cb72f137a90b48e9f65b3b71f09bf380643dfc 73822
        unzip -q "$TRIAL/evidence.zip" -d "$evidence/baseline"
        validate_baseline "$evidence/baseline" "$evidence/context.txt"
        fetch_original 15225376593 seed 23ec62ebe8e435f1c681bb71782e8251f02e88d8801bd7a68679a09f05c663ba 366284447
        ;;
    restore)
        check_clean_source
        check_context restore
        for directory in "${output_paths[@]}"; do
            [[ ! -e $directory && ! -L $directory ]] || fail "resume has existing output at $directory"
        done
        /usr/bin/time -f '%e' -o "$evidence/unzip.seconds" \
            unzip -p "$TRIAL/seed.zip" seed.tar.zst > "$TRIAL/received.tar.zst"
        [[ $(sha "$TRIAL/received.tar.zst") == "$seed_sha" ]] ||
            fail "downloaded seed bytes differ from the original tar"
        /usr/bin/time -f '%e' -o "$evidence/extract.seconds" \
            tar --zstd -xf "$TRIAL/received.tar.zst"
        df -B1 "$PWD" "$TRIAL" > "$evidence/disk-after-restore.txt"
        ;;
    compiled)
        check_clean_source
        check_context compiled
        measure compiled "$stock_source"
        require_arm_wasm_freshness compiled true
        ;;
    edited)
        check_clean_source
        check_context edited
        source_file=crates/rdf-wasm/src/convert.rs
        backup="$TRIAL/convert.rs"
        stat -c '%i %s %y' "$source_file" > "$evidence/edit-before.stat"
        cp --preserve=mode,timestamps "$source_file" "$backup"
        trap 'cat "$backup" > "$source_file"; touch -r "$backup" "$source_file"' EXIT
        match=$(LC_ALL=C grep -Fbo '// The language tag is lowercased' "$source_file")
        [[ $match =~ ^([0-9]+):// ]] && [[ $match != *$'\n'* ]] ||
            fail "the edit anchor must occur exactly once"
        offset=$(("${BASH_REMATCH[1]}" + 3))
        printf t | dd of="$source_file" bs=1 seek="$offset" count=1 conv=notrunc status=none
        touch -r "$backup" "$source_file"
        stat -c '%i %s %y' "$source_file" > "$evidence/edit-after.stat"
        cmp "$evidence/edit-before.stat" "$evidence/edit-after.stat" ||
            fail "the source edit changed inode, length or modification time"
        diff_exit=0
        cmp -l "$backup" "$source_file" > "$evidence/edited-byte.txt" || diff_exit=$?
        ((diff_exit <= 1)) || fail "could not compare edited and original bytes"
        awk -v offset="$((offset + 1))" '
            $1 == offset && $2 == 124 && $3 == 164 { found = 1 }
            END { exit !(NR == 1 && found) }
        ' "$evidence/edited-byte.txt" ||
            fail "the source edit did not change exactly one byte"
        edited_source=$(source_digest)
        [[ $edited_source != "$stock_source" ]] || fail "source identity missed the edit"
        measure edited "$edited_source"
        require_arm_wasm_freshness edited false
        cat "$backup" > "$source_file"
        touch -r "$backup" "$source_file"
        check_clean_source
        trap - EXIT
        ;;
    *) fail "unknown trial phase: ${1:-missing}" ;;
esac
