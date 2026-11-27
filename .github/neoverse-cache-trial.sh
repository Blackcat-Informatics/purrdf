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

# The hosted gate draws its source identity from the measured checkout, not this harness.
if [[ ${1:-} == validate-report ]]; then
    validate_report "$2" "${3:-$stock_source}"
    exit
fi

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
    if [[ $arm == stock && -s "$dir/graph-contexts.txt" ]]; then
        local active
        IFS= read -r active < "$dir/graph-contexts.txt"
        if grep -Fxq "${active##*/}" "$evidence/stock-cached-contexts.txt"; then
            echo 'stock_context=present_in_archive' > "$dir/cache-context.txt"
        else
            echo 'stock_context=not_in_archive' > "$dir/cache-context.txt"
            fail "the restored archive does not contain the active stock assembly context"
        fi
    fi
}

measure() {
    local arm=$1 source=$2 rc=0
    local dir=$evidence/$arm
    mkdir "$dir"
    [[ ! -e $report ]] || fail "old report still present before $arm"
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
    stock)
        date +%s%N > "$evidence/stock-restore-end.ns"
        printf 'requested=%s\nmatched=%s\nhit=%s\nobserved_key_bytes=%s\n' \
            "$STOCK_KEY" "$CACHE_MATCH" "$CACHE_HIT" 15296439 \
            > "$evidence/stock-cache.txt"
        [[ $CACHE_HIT == true && $CACHE_MATCH == "$STOCK_KEY" && -d target/simd-asm ]] ||
            fail "the exact stock evidence was not restored"
        [[ ! -e ${output_paths[0]} && ! -e ${output_paths[1]} ]] ||
            fail "the stock arm already had compiled Cargo output"
        du -sb target/simd-asm > "$evidence/stock-expanded-bytes.txt"
        for directory in target/simd-asm/contexts/*/; do
            if [[ -d $directory ]]; then
                basename "$directory"
            fi
        done > "$evidence/stock-cached-contexts.txt"
        check_clean_source
        check_context stock
        measure stock "$stock_source"
        ;;
    pack)
        du -sb "${output_paths[@]}" > "$evidence/seed-expanded-bytes.txt"
        /usr/bin/time -f '%e' -o "$evidence/pack.seconds" \
            tar --format=posix --zstd -cf "$TRIAL/seed.tar.zst" --exclude='cargo.graph-*' \
                --exclude='cargo-timing*' "${output_paths[@]}"
        sha "$TRIAL/seed.tar.zst" > "$evidence/seed.sha256"
        stat -c%s "$TRIAL/seed.tar.zst" > "$evidence/seed-packed-bytes.txt"
        ;;
    download)
        [[ $ARTIFACT_ID =~ ^[0-9]+$ && $ARTIFACT_DIGEST =~ ^[0-9a-f]{64}$ ]] ||
            fail "the upload did not return a valid artifact receipt"
        endpoint="repos/$GITHUB_REPOSITORY/actions/artifacts/$ARTIFACT_ID"
        gh api "$endpoint" > "$evidence/artifact.json"
        jq -e --arg name "neoverse-trial-seed-$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" \
            --arg digest "sha256:$ARTIFACT_DIGEST" \
            --argjson id "$ARTIFACT_ID" --argjson run "$GITHUB_RUN_ID" \
            '.id == $id and .workflow_run.id == $run and .name == $name and .digest == $digest and .expired == false' \
            "$evidence/artifact.json" >/dev/null || fail "seed artifact metadata did not match this run"
        printf '%s\n' "$ARTIFACT_DIGEST" > "$evidence/artifact-upload-digest.txt"
        /usr/bin/time -f '%e' -o "$evidence/download.seconds" \
            gh api "$endpoint/zip" > "$TRIAL/received.zip"
        stat -c%s "$TRIAL/received.zip" > "$evidence/download-bytes.txt"
        /usr/bin/time -f '%e' -o "$evidence/unzip.seconds" \
            unzip -p "$TRIAL/received.zip" seed.tar.zst > "$TRIAL/received.tar.zst"
        [[ $(sha "$TRIAL/received.tar.zst") == "$(cat "$evidence/seed.sha256")" ]] ||
            fail "downloaded seed bytes differ from the original tar"
        mkdir -p "$TRIAL/stock-output/$triple"
        mv "${output_paths[0]}" "${output_paths[2]}" "$TRIAL/stock-output/"
        mv "${output_paths[1]}" "$TRIAL/stock-output/$triple/"
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
