#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

# Assemble a Git source archive without a gzip filename or timestamp. The audit
# binds the release receipt to the actual archive and its tagged legal texts.
set -euo pipefail
unset GZIP TAR_OPTIONS
export GIT_NO_REPLACE_OBJECTS=1

fail() {
    printf 'Release source archive: %s\n' "$1" >&2
    exit 1
}

[[ $# == 4 ]] || fail 'usage: package-release-source.sh BUILD_OR_VERIFY COMMIT VERSION DIST'
mode=$1
commit=$2
version=$3
dist=$4
[[ "$mode" == build || "$mode" == verify ]] || fail 'use build or verify'
[[ "$commit" =~ ^[a-f0-9]{40}$ && "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail 'expected a full commit ID and a release version'
[[ $(git rev-parse --verify "$commit^{commit}") == "$commit" ]] || fail 'commit is not present'
prefix=$(git rev-parse --show-prefix)
[[ -z "$prefix" ]] || fail 'run from the repository root'

stem="purrdf-source-$version"
archive="$dist/$stem.tar.gz"
receipt="$dist/$stem.license-receipt.json"

if [[ "$mode" == build ]]; then
    git ls-tree -r "$commit" | awk '$1 == "160000" { print "Source archive cannot include a gitlink: " $0 > "/dev/stderr"; refused = 1 } END { exit refused }'
    export GIT_ATTR_NOSYSTEM=1
    git ls-tree -r -t --name-only -z "$commit" |
        git -c core.attributesFile=/dev/null check-attr --source="$commit" -z --stdin export-ignore export-subst |
        while IFS= read -r -d '' path && IFS= read -r -d '' attribute && IFS= read -r -d '' value; do
            [[ "$value" == unspecified || "$value" == unset ]] ||
                fail "$attribute would transform the tagged tree at $path"
        done
    mkdir -p -- "$dist"
    [[ ! -e "$archive" && ! -L "$archive" && ! -e "$receipt" && ! -L "$receipt" ]] ||
        fail 'refusing to replace existing source files'
    git -c core.attributesFile=/dev/null -c tar.umask=0002 archive --format=tar --prefix="$stem/" "$commit" |
        gzip --no-name > "$archive"
fi

[[ -s "$archive" && -f "$archive" && ! -L "$archive" ]] || fail 'missing regular source archive'
archived_commit=$(gzip --decompress --stdout -- "$archive" | {
    git get-tar-commit-id || fail 'cannot read the archived Git commit'
    cat >/dev/null
})
[[ "$archived_commit" == "$commit" ]] || fail 'archive commit differs from tagged source'

notices=(LICENSE-MIT LICENSE-APACHE LICENSE-MULAN LICENSES/CC-BY-4.0.txt LICENSING.md license-inventory.toml)
for notice in "${notices[@]}"; do
    tagged=$(git cat-file blob "$commit:$notice" | sha256sum)
    packaged=$(tar -xOzf "$archive" -- "$stem/$notice" | sha256sum)
    [[ "$tagged" == "$packaged" ]] || fail "tagged legal evidence differs in the archive: $notice"
done

digest=$(sha256sum < "$archive")
inventory=$(git cat-file blob "$commit:license-inventory.toml" | sha256sum)
body=$(jq -nS --arg artifact "$stem.tar.gz" --arg sha256 "${digest%% *}" \
    --arg source_commit "$commit" --arg license_inventory_sha256 "${inventory%% *}" \
    --argjson notice_files "${#notices[@]}" \
    '{schema: 1, artifacts: [{artifact: $artifact, profile: "source", sha256: $sha256,
      source_commit: $source_commit, license_inventory_sha256: $license_inventory_sha256,
      notice_files: $notice_files, runtime_notice_files: 0, status: "passed"}]}')
if [[ "$mode" == build ]]; then
    printf '%s\n' "$body" > "$receipt"
else
    [[ -f "$receipt" && ! -L "$receipt" ]] || fail 'missing regular source license receipt'
    printf '%s\n' "$body" | cmp - "$receipt" || fail 'source license receipt does not match the archive'
fi
