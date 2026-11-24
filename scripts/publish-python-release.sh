#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
shopt -s nullglob

project=$1
version=$2
directory=$3
python=$4
mode=${5:-publish}
[[ "$mode" == publish || "$mode" == preflight ]]
case "$project" in
  purrdf | purrdf-rdflib) ;;
  *) printf 'Unknown PyPI project: %s\n' "$project" >&2; exit 1 ;;
esac
[[ "$version" =~ ^[0-9A-Za-z.+!-]+$ ]]

"$python" -I scripts/publish-python.py check \
  --project "$project" --version "$version" --dist-dir "$directory"

work=$(mktemp -d)
mkdir "$work/missing"
endpoint="https://pypi.org/pypi/$project/$version/json"

inventory() {
  local status
  status=$(curl --proto '=https' --tlsv1.2 --silent --show-error --max-time 30 \
    --retry 3 --header 'Cache-Control: no-cache' --output "$work/pypi.json" \
    --write-out '%{http_code}' --get \
    --data-urlencode "inventory=$(date +%s)-$$-$RANDOM-$RANDOM" "$endpoint")
  case "$status" in
    200)
      jq -ceS '
        .urls | if type != "array" or length == 0 then error("PyPI returned no files") else . end
        | map({key: .filename, value: .digests.sha256})
        | if any(.[]; (.key | type != "string") or
                      (.value | type != "string" or (test("^[0-9a-f]{64}$") | not)))
             or (map(.key) | length != (unique | length))
          then error("PyPI returned invalid or duplicate file records")
          else from_entries end
      ' "$work/pypi.json"
      ;;
    404) printf '{}\n' ;;
    *) printf 'PyPI inventory request for %s failed with HTTP %s\n' "$project" "$status" >&2; return 1 ;;
  esac
}

before=$(inventory)
baseline='{}'
if [[ "${GITHUB_EVENT_NAME:-}" == workflow_dispatch ]]; then
  [[ "$version" == 5.0.1 && "$GITHUB_REF" == refs/heads/main && "$GITHUB_REF_PROTECTED" == true ]]
  baseline=$(jq -ceS --arg project "$project" '.[$project]' .github/python-backfill-5.0.1.json)
  if ! jq -en --argjson before "$before" --argjson baseline "$baseline" \
    'all($baseline | to_entries[]; $before[.key] == .value)' >/dev/null; then
    printf 'The published PyPI baseline for %s %s is missing or has changed\n' "$project" "$version" >&2
    exit 1
  fi
fi
expected=$baseline
for candidate in "$directory"/*; do
  name=${candidate##*/}
  if jq -e --arg name "$name" 'has($name)' <<< "$baseline" >/dev/null; then
    continue
  fi
  read -r digest _ < <(sha256sum -- "$candidate")
  expected=$(jq -cS --arg name "$name" --arg digest "$digest" '. + {($name): $digest}' <<< "$expected")
  published=$(jq -r --arg name "$name" '.[$name] // empty' <<< "$before")
  if [[ -z "$published" ]]; then
    cp -- "$candidate" "$work/missing/"
  elif [[ "$published" != "$digest" ]]; then
    printf 'PyPI already has different bytes for %s\n' "$name" >&2
    exit 1
  fi
done

if ! jq -en --argjson before "$before" --argjson expected "$expected" \
  '(($before | keys) - ($expected | keys)) == []' >/dev/null; then
  printf 'PyPI contains unexpected files for %s %s\n' "$project" "$version" >&2
  exit 1
fi

missing=("$work/missing"/*)
if [[ "$mode" == preflight ]]; then
  printf '%s %s: validated PyPI baseline and %s missing files\n' "$project" "$version" "${#missing[@]}"
  exit 0
fi
if ((${#missing[@]})); then
  "$python" -I scripts/publish-python.py publish \
    --project "$project" --version "$version" --dist-dir "$work/missing"
fi

after=$(inventory)
if [[ "$after" != "$expected" ]]; then
  printf 'PyPI file list or digests differ for %s %s\n' "$project" "$version" >&2
  exit 1
fi
printf '%s %s: verified PyPI filenames and SHA-256 digests\n' "$project" "$version"
