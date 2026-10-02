#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Verify isolated empty 0.0.0 packages before creating registry records.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=release-crates.sh
# shellcheck disable=SC1091 # The test seam may select an external fixture ledger.
source "${PURRDF_RELEASE_CRATES_FILE:-${repo}/scripts/release-crates.sh}"
exec python3 "${repo}/scripts/bootstrap-crates-io.py" --crates "${PURRDF_UNBOOTSTRAPPED_CRATES[@]}" -- "$@"
