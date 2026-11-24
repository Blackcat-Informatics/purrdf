#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail

destination=$1
manifest=.github/python-backfill-5.0.1.json
version=$(jq -er '.tag | sub("^py-v"; "")' "$manifest")

fetch() {
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error --max-time 120 --retry 3 "$@"
}

for project in purrdf purrdf-rdflib; do
  mkdir -p -- "$destination/$project"
  inventory=$(fetch "https://pypi.org/pypi/$project/$version/json")
  files=$(jq -er --arg project "$project" '.[$project] | to_entries[] | [.key, .value] | @tsv' "$manifest")
  while IFS=$'\t' read -r name digest; do
    [[ "$name" =~ ^[0-9A-Za-z_.+-]+$ && "$digest" =~ ^[0-9a-f]{64}$ ]]
    url=$(jq -er --arg name "$name" --arg digest "$digest" '
      [.urls[] | select(.filename == $name and .digests.sha256 == $digest)]
      | if length == 1 then .[0].url else error("missing or ambiguous baseline artifact") end
      | select(startswith("https://files.pythonhosted.org/"))
    ' <<< "$inventory")
    path="$destination/$project/$name"
    fetch --output "$path" "$url"
    sha256sum --strict --check <<< "$digest  $path"
  done <<< "$files"
done
