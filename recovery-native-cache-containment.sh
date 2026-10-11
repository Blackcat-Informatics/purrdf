#!/usr/bin/env bash
set -euo pipefail
worktree=/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence
cache=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification
stage="$worktree/.stage/sparql-eval-datalog-per-row-evidence"
[[ -L "$worktree/target" && $(readlink "$worktree/target") == "$cache/target" ]]
[[ -d "$cache/target" && ! -L "$cache/target" && -d "$cache/build" && ! -L "$cache/build" ]]
[[ ! -e "$cache/target/native-build" ]]
[[ $(stat -c %d "$worktree") == $(stat -c %d "$cache/target") ]]
[[ $(stat -c %d "$worktree") == $(stat -c %d "$cache/build") ]]
for link in /proc/[0-9]*/exe; do
    executable=$(readlink "$link" 2>/dev/null || true)
    case "$executable" in
        "$cache/build/"*|"$cache/target/"*) echo "REFUSED: live cache user $link $executable"; exit 40;;
    esac
done
binary="$cache/build/debug/build/purrdf-bench/bd45cd0069bd4eb6/out/lubm_cli-bd45cd0069bd4eb6"
[[ -x "$binary" ]]
before_identity=$(stat -Lc '%d:%i:%s' "$binary")
before_hash=$(sha256sum "$binary")
printf 'BEFORE identity=%s\n%s\n' "$before_identity" "$before_hash"
unlink "$worktree/target"
mv -T "$cache/target" "$worktree/target"
ln -s "$worktree/target" "$cache/target"
mv -T "$cache/build" "$worktree/target/native-build"
ln -s "$worktree/target/native-build" "$cache/build"
for directory in "$worktree/target" "$worktree/target/native-build"; do
    [[ ! -e "$directory/CACHEDIR.TAG" ]]
    (set -o noclobber; cat > "$directory/CACHEDIR.TAG" <<'TAG'
Signature: 8a477f597d28d172789f06886806bc55
# Cache metadata restored for this established Cargo build directory.
# Cargo did not create the tag because this root was precreated.
# For information about cache directory tags see https://bford.info/cachedir/
TAG
    )
done
[[ $(stat -Lc '%d:%i:%s' "$binary") == "$before_identity" ]]
[[ $(sha256sum "$binary") == "$before_hash" ]]
printf 'AFTER identity=%s\n%s\ncanonical=%s\n' "$(stat -Lc '%d:%i:%s' "$binary")" "$(sha256sum "$binary")" "$(readlink -f "$binary")"
for directory in "$cache/target" "$cache/build"; do
    actual=$(readlink -f "$directory")
    relative=${actual#"$worktree/"}
    provenance=$(git -C "$worktree" -c core.excludesFile=/dev/null check-ignore --verbose --no-index "$relative/")
    printf 'IGNORE %s\n' "$provenance"
    [[ "$provenance" == .gitignore:* ]]
done
git -C "$worktree" ls-files --error-unmatch -- .gitignore
printf 'PASS: cache containment, original inode/hash and tracked ignore provenance. No source/test changes.\n'
