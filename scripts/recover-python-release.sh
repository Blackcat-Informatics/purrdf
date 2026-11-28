#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
export LC_ALL=C
umask 077
unset UNZIP UNZIPOPT ZIPINFO ZIPINFOOPT

fail() {
    printf 'Recovery refused: %s\n' "$*" >&2
    exit 1
}

repo=Blackcat-Informatics/purrdf
repo_id=1286258226
run=45490315518
revision=f0e7e08e144def22fb492cc21ada46d1e2bdb649
ref=refs/heads/main
caller=.github/workflows/release-pypi.yaml
signer=.github/workflows/qualify-python-release.yaml
tag=py-v5.0.1
commit=6e63988e44a1f7a45c9803199e2a37415f85d41f
receipt_digest=20437622d3c781df72dcf42b5cb1cf3ce1c799c3682614c6cc4be324ebfe6bf5
native_id=14970683497
shadow_id=14970234094
evidence_id=14969593240

regular() {
    [[ -f $1 && ! -L $1 ]] || fail "not a regular file: $1"
}

check_attempt() {
    regular "$metadata/attempt.json"
    jq -e --argjson run "$run" --argjson repo "$repo_id" --arg sha "$revision" \
        --arg caller "$caller" --arg signer "$repo/$signer@$revision" --arg ref "$ref" '
        .id == $run and .run_attempt == 1 and
        .repository.id == $repo and .head_repository.id == $repo and
        .path == $caller and .head_branch == "main" and .head_sha == $sha and
        .event == "workflow_dispatch" and .status == "completed" and
        .conclusion == "success" and .referenced_workflows == [{path:$signer, sha:$sha, ref:$ref}]
    ' "$metadata/attempt.json" >/dev/null || fail 'foreign or unsuccessful qualification attempt'
}

check_artifact() {
    local id=$1 name=$2 digest=$3 size=$4
    regular "$metadata/$id.json"
    jq -e --argjson id "$id" --arg name "$name" --arg digest "sha256:$digest" \
        --argjson size "$size" --arg sha "$revision" \
        --argjson repo "$repo_id" --argjson run "$run" \
        --arg url "https://api.github.com/repos/$repo/actions/artifacts/$id/zip" '
        .id == $id and .name == $name and .digest == $digest and .size_in_bytes == $size and
        .expired == false and .archive_download_url == $url and
        .workflow_run == {id:$run, repository_id:$repo, head_repository_id:$repo,
                          head_branch:"main", head_sha:$sha}
    ' "$metadata/$id.json" >/dev/null || fail "foreign, missing or expired artifact metadata: $id"
}

fetch_artifact() {
    local id=$1 name=$2 digest=$3 size=$4 actual
    local endpoint="repos/$repo/actions/artifacts/$id"
    gh api "$endpoint" > "$metadata/$id.json"
    check_artifact "$id" "$name" "$digest" "$size"
    gh api "$endpoint/zip" > "$archives/$id.zip"
    read -r actual _ < <(sha256sum -- "$archives/$id.zip")
    [[ $actual == "$digest" ]] || fail "original ZIP digest differs: $id"
}

authenticate_receipt() {
    gh attestation verify "$1" --repo "$repo" \
        --signer-workflow "$repo/$signer" --source-ref "$ref" \
        --source-digest "$revision" --signer-digest "$revision" \
        --deny-self-hosted-runners --format json > "$verified"
}

check_members() {
    local archive=$1 summary list lengths entry raw_size raw_tail
    shift
    summary=$(zipinfo -l -T "$archive") || fail "invalid ZIP: $archive"
    list=$(zipinfo -1 "$archive") || fail "cannot enumerate ZIP: $archive"
    # The long listing must contain just a header, its declared number of
    # regular member rows and a footer. Use the short listing for exact names.
    awk -v count="$#" '
        NR == 1 { if ($0 !~ /^Archive: /) exit 1; next }
        NR == 2 { if ($NF != count || $0 !~ /number of entries:/) exit 1; next }
        NR <= count + 2 { if ($1 !~ /^-[rwxstST-]{9}$/ || NF != 9) exit 1; next }
        NR == count + 3 { if ($1 != count || $2 != "files,") exit 1; next }
        { exit 1 }
        END { if (NR != count + 3) exit 1 }
    ' <<< "$summary" || fail "ZIP must contain exactly $# regular members: $archive"
    diff -q <(printf '%s\n' "$@" | sort) <(sort <<< "$list") >/dev/null ||
        fail "ZIP member names are not the exact approved set: $archive"
    # Compare each central-directory filename length as well as the display
    # name so a truncated filename cannot masquerade as an approved member.
    lengths=$(zipinfo -v "$archive" | awk '$1 == "length" && $3 == "filename:" { print $4 }') ||
        fail "cannot read ZIP filename lengths: $archive"
    while IFS= read -r entry && IFS=' ' read -r raw_size raw_tail <&3; do
        [[ $raw_size == "${#entry}" && -z $raw_tail ]] ||
            fail "ZIP filename is not its approved ASCII spelling: $archive"
    done <<< "$list" 3<<< "$lengths"
    [[ $(wc -l <<< "$lengths") -eq $# ]] || fail "wrong ZIP filename census: $archive"
}

extract_members() {
    local archive=$1 destination=$2 member
    shift 2
    for member; do
        unzip -p "$archive" "$member" > "$destination/$member" ||
            fail "cannot read approved member $member"
    done
}

validate_receipt() {
    local directory=$1 digest
    read -r digest _ < <(sha256sum -- "$directory/release-source.json")
    [[ $digest == "$receipt_digest" ]] || fail 'release receipt bytes differ'
    jq -e --arg repo "https://github.com/$repo" --arg sha "$revision" --arg ref "$ref" \
        --arg caller "$caller" --arg signer "$signer" --arg invocation "$run/attempts/1" \
        --arg digest "$digest" '
        any(.[]; .verificationResult as $r | $r.signature.certificate as $c
            | $c.runInvocationURI == ($repo + "/actions/runs/" + $invocation)
              and $c.buildTrigger == "workflow_dispatch" and $c.runnerEnvironment == "github-hosted"
              and $c.sourceRepositoryURI == $repo and $c.sourceRepositoryRef == $ref
              and $c.sourceRepositoryDigest == $sha and $c.buildConfigDigest == $sha
              and $c.buildSignerDigest == $sha
              and $c.buildConfigURI == ($repo + "/" + $caller + "@" + $ref)
              and $c.buildSignerURI == ($repo + "/" + $signer + "@" + $ref)
              and $r.statement.predicateType == "https://slsa.dev/provenance/v1"
              and $r.statement.subject == [{name:"release-source.json", digest:{sha256:$digest}}])
    ' "$verified" >/dev/null || fail 'verified attestation does not bind the approved attempt and receipt'
    jq -e --arg tag "$tag" --arg sha "$commit" --rawfile checksums "$directory/release.sha256" '
        .tag == $tag and .commit == $sha and .checksums == $checksums
    ' "$directory/release-source.json" >/dev/null || fail 'release tag, commit or checksum manifest differs'
}

verify_bundle() {
    local output=$1 scratch platform platforms original row
    local native=(purrdf-5.0.1.tar.gz)
    local shadow=(purrdf_rdflib-5.0.1-py3-none-any.whl purrdf_rdflib-5.0.1.tar.gz)
    local evidence=(release-source.json release.sha256)
    local baseline="$PWD/.github/python-backfill-5.0.1.json"
    jq -e --arg tag "$tag" --arg commit "$commit" '
        .tag == $tag and .commit == $commit and
        (.purrdf | keys == ["purrdf-5.0.1-cp313-abi3-manylinux_2_34_x86_64.whl","purrdf-5.0.1.tar.gz"]) and
        (."purrdf-rdflib" | keys == ["purrdf_rdflib-5.0.1-py3-none-any.whl","purrdf_rdflib-5.0.1.tar.gz"]) and
        all((.purrdf[], ."purrdf-rdflib"[]); type == "string" and test("^[0-9a-f]{64}$"))
    ' "$baseline" >/dev/null || fail 'release baseline is not the approved tag and commit'
    platforms=$(jq -er '
        [.include[].platform] as $p
        | if ($p | length) != 8 or ($p | unique | length) != 8
            or any($p[]; type != "string" or (test("^[A-Za-z0-9_]+$") | not))
          then error("invalid or duplicate wheel platform") else $p[] end
    ' .github/python-wheel-targets.json) || fail 'approved wheel matrix is invalid'
    while IFS= read -r platform; do
        native+=("purrdf-5.0.1-cp313-abi3-$platform.whl")
    done <<< "$platforms"
    check_members "$archives/$native_id.zip" "${native[@]}"
    check_members "$archives/$shadow_id.zip" "${shadow[@]}"
    check_members "$archives/$evidence_id.zip" "${evidence[@]}"
    scratch=$(mktemp -d -- "$(dirname -- "$output")/.pypi-recovery.XXXXXX")
    mkdir -- "$scratch/evidence" "$scratch/release-dist" "$scratch/release-shadow"
    extract_members "$archives/$evidence_id.zip" "$scratch/evidence" "${evidence[@]}"
    authenticate_receipt "$scratch/evidence/release-source.json"
    validate_receipt "$scratch/evidence"
    extract_members "$archives/$native_id.zip" "$scratch/release-dist" "${native[@]}"
    extract_members "$archives/$shadow_id.zip" "$scratch/release-shadow" "${shadow[@]}"
    (
        cd "$scratch"
        sha256sum -- release-dist/* release-shadow/* > actual.sha256
        cmp -s -- actual.sha256 evidence/release.sha256 || fail 'distribution digest census differs'
        original=$(jq -er '
            (.purrdf | to_entries[] | "\(.value)  release-dist/\(.key)"),
            (."purrdf-rdflib" | to_entries[] | "\(.value)  release-shadow/\(.key)")
        ' "$baseline")
        while IFS= read -r row; do
            grep -Fxq -- "$row" actual.sha256 || fail 'approved distribution differs from original baseline'
        done <<< "$original"
    )
    mv -T -- "$scratch" "$output"
    printf 'Verified approved attempt %s/1, three artifact IDs and all 11 distribution paths and digests.\n' "$run"
}

main() {
    [[ $# == 1 && $1 == /* ]] || fail 'expected an absolute new output directory'
    [[ ! -e $1 && ! -L $1 ]] || fail 'verified output must not already exist'
    [[ ${GITHUB_REPOSITORY:-} == "$repo" && ${GITHUB_EVENT_NAME:-} == workflow_dispatch &&
        ${GITHUB_REF:-} == "$ref" && ${GITHUB_REF_PROTECTED:-} == true ]] ||
        fail 'recovery requires a dispatch from protected main'
    local work tags remote_commit tag_ref="refs/tags/$tag"
    tags=$(git ls-remote --exit-code origin "$tag_ref" "$tag_ref^{}")
    remote_commit=$(awk -v ref="$tag_ref" '
        $2 == ref { tag=$1 }
        $2 == ref "^{}" { peeled=$1 }
        END { print peeled ? peeled : tag }
    ' <<< "$tags")
    [[ $remote_commit == "$commit" ]] || fail 'remote release tag no longer points to the approved source'
    work=$(mktemp -d -- "$(dirname -- "$1")/.pypi-recovery-input.XXXXXX")
    local metadata="$work/metadata" archives="$work/archives" verified="$work/verified-attestation.json"
    mkdir -- "$metadata" "$archives"
    gh api "repos/$repo/actions/runs/$run/attempts/1" > "$metadata/attempt.json"
    check_attempt
    fetch_artifact "$native_id" dist-pypi-native-release af67e00aec5387ffb861171698d343794c3d746d25d70ca76d0fa8c1c19600ff 283662749
    fetch_artifact "$shadow_id" dist-pypi-shadow-release 5a3fac379cfa2b15a96e5b5e8f7bec05171842adeee5e10963984d43eb1d24ab 35399
    fetch_artifact "$evidence_id" evidence-pypi-release 70d7bc96f4ba16ed5b062c3e39569ab93217331985cba71db58eae2436d7ebd2 1489
    verify_bundle "$1"
}

main "$@"
