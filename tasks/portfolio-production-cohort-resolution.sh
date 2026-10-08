#!/usr/bin/env bash
# Stage-only offline resolver witness; no compiler or artifact generation.
set -euo pipefail
root=/home/paudley/Active/purrdf/.worktrees/454-rdflib-shim-algebra-level-reassignment
baseline=/opt/purrdf-454-native-cost-main-2eb04d61
sdk=/home/paudley/stage/packages/rustup/active-toolchain/bin
logs=${1:?fresh owned resolver root}
test "$(git -C "$root" write-tree)" = d632307fb725304ed864967d5234cdab35bc5c99
git -C "$root" diff --quiet
mkdir "$logs"
export PATH="$sdk:$PATH" CARGO_BUILD_JOBS=8
export CARGO_TARGET_DIR="$logs/target" CARGO_BUILD_BUILD_DIR="$logs/build" TMPDIR="$logs/tmp"
mkdir "$TMPDIR"
sha256sum "$root/Cargo.lock" "$baseline/Cargo.lock" > "$logs/production-start.sha256"
"$sdk/cargo" -V > "$logs/cargo.txt"
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

for role in baseline candidate; do
 if test "$role" = baseline; then source_root=$baseline; else source_root=$root; fi
 probe=$logs/probe-$role
 mkdir -p "$probe/src"
 cp "$root/.stage/rdflib-shim-algebra-level-reassignment/raw/native-cost-t3-candidate-probe/src/main.rs" "$probe/src/main.rs"
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
 cp "$probe/Cargo.lock" "$logs/$role-seed.lock"
 for mode in resolve locked; do
  args=(metadata --offline --format-version 1 --manifest-path "$probe/Cargo.toml")
  if test "$mode" = locked; then args+=(--locked); fi
  printf '%q ' "$sdk/cargo" "${args[@]}" > "$logs/$role-$mode.command"
  result=0
  "$sdk/cargo" "${args[@]}" > "$logs/$role-$mode.json" 2> "$logs/$role-$mode.log" || result=$?
  printf '%s\n' "$result" > "$logs/$role-$mode.exit"
  test "$result" -eq 0
 done
 lock_registry_rows "$source_root/Cargo.lock" > "$logs/$role-production.tsv"
 lock_registry_rows "$probe/Cargo.lock" > "$logs/$role-selected.tsv"
 LC_ALL=C comm -23 "$logs/$role-selected.tsv" "$logs/$role-production.tsv" > "$logs/$role-unapproved.tsv"
 test ! -s "$logs/$role-unapproved.tsv"
 jq -r '.packages[] | select(.source != null) | [.name,.version,.source] | @tsv' "$logs/$role-locked.json" | LC_ALL=C sort -u > "$logs/$role-resolved-external.tsv"
 awk -F '\t' 'NR==FNR {sha[$1 FS $2 FS $3]=$4;next} {key=$1 FS $2 FS $3;if(!(key in sha)) exit 1;print $0 "\t" sha[key]}' "$logs/$role-production.tsv" "$logs/$role-resolved-external.tsv" > "$logs/$role-resolved-external-checksums.tsv"
 test -s "$logs/$role-resolved-external-checksums.tsv"
 diff -u "$logs/$role-seed.lock" "$probe/Cargo.lock" > "$logs/$role-lock-resolution.diff" || test "$?" -eq 1
 sha256sum "$probe/Cargo.lock" > "$logs/$role-selected-lock.sha256"
done
cmp "$logs/probe-baseline/Cargo.lock" "$logs/probe-candidate/Cargo.lock" > "$logs/matched-locks.log"
cmp "$logs/baseline-resolved-external-checksums.tsv" "$logs/candidate-resolved-external-checksums.tsv" > "$logs/matched-external-closure.log"
sha256sum --check --quiet "$logs/production-start.sha256"
test "$(git -C "$root" write-tree)" = d632307fb725304ed864967d5234cdab35bc5c99
git -C "$root" diff --quiet
printf '0\n' > "$logs/resolver.exit"
