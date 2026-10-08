#!/usr/bin/env bash
set -euo pipefail
root=${PURRDF_454_ROOT:?existing worktree root required}
evidence=${PURRDF_454_EVIDENCE:?owned evidence root required}
expected_tree=${PURRDF_454_EXPECT_TREE:?root-reviewed source tree required}
logs=$evidence/${PURRDF_454_LOG_SUBDIR:?fresh generation log label required}
cd "$root"
test "$(git write-tree)" = "$expected_tree"
git diff --quiet
test -z "$(git ls-files -u)"
mkdir "$logs"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 BINARYEN_CORES=8
export CARGO_TARGET_DIR="$evidence/target" CARGO_BUILD_BUILD_DIR="$evidence/build" TMPDIR="$evidence/tmp"
cp docs/book/po/zh-Hans.po "$logs/merged-preimage.po"
cp .stage/rdflib-shim-algebra-level-reassignment/raw/portfolio-main402-premerge-main-zh-Hans.po "$logs/main-preimage.po"
cp .stage/rdflib-shim-algebra-level-reassignment/raw/portfolio-main402-premerge-contextual-zh-Hans.po "$logs/contextual-preimage.po"
sha256sum "$logs"/*preimage.po > "$logs/preimages.sha256"
git ls-files -z | xargs -0 sha256sum > "$logs/source-start.sha256"
git rev-parse HEAD MERGE_HEAD > "$logs/refs.txt"
printf '%s\n' 'make book-po-update' > "$logs/generation.command"
result=0
make book-po-update > "$logs/generation.log" 2>&1 || result=$?
printf '%s\n' "$result" > "$logs/generation.exit"
test "$result" -eq 0
cp docs/book/po/zh-Hans.po "$logs/generated.po"
cp docs/book/po/messages.pot "$logs/generated.pot"
sha256sum "$logs/generated.po" "$logs/generated.pot" > "$logs/generated.sha256"
git diff -- docs/book/po/zh-Hans.po > "$logs/po.diff"
git diff --name-only > "$logs/changed-paths.txt"
# A byte-identical owning regeneration is valid; any other shipping change is not.
if test -s "$logs/changed-paths.txt"; then
    test "$(cat "$logs/changed-paths.txt")" = docs/book/po/zh-Hans.po
fi
msgfmt --statistics -o /dev/null "$logs/merged-preimage.po" > "$logs/preimage-statistics.txt" 2>&1
msgfmt --statistics -o /dev/null "$logs/generated.po" > "$logs/generated-statistics.txt" 2>&1
python3 -I .stage/rdflib-shim-algebra-level-reassignment/tasks/portfolio-catalogue-preservation.py "$root" "$logs" > "$logs/translation-preservation.log" 2>&1
sha256sum --check --quiet "$logs/preimages.sha256"
test "$(git write-tree)" = "$expected_tree"
printf '0\n' > "$logs/driver.exit"
