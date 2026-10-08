#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Stage-only owning documentation qualification, preserving the source/index.
set -euo pipefail
DOC402_LOGS="$1"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
cd /home/paudley/Active/purrdf/.worktrees/402-shacl-profiles-reports
doc402_run() {
  local name="$1" result
  shift
  test ! -e "${DOC402_LOGS}/${name}.log"
  printf '%q ' "$@" >"${DOC402_LOGS}/${name}.command"
  printf '\n' >>"${DOC402_LOGS}/${name}.command"
  if "$@" >"${DOC402_LOGS}/${name}.log" 2>&1; then result=0; else result=$?; fi
  printf '%s\n' "$result" >"${DOC402_LOGS}/${name}.exit"
  printf '%s actual exit %s\n' "$name" "$result"
  [[ "$result" == 0 ]] || { tail -60 "${DOC402_LOGS}/${name}.log"; return "$result"; }
}
test ! -e "${DOC402_LOGS}/zh-Hans.preimage.po"
cp docs/book/po/zh-Hans.po "${DOC402_LOGS}/zh-Hans.preimage.po"
sha256sum docs/book/po/zh-Hans.po >"${DOC402_LOGS}/po-preimage.sha256"
git ls-files --stage -z >"${DOC402_LOGS}/index-before.entries"
git status --short >"${DOC402_LOGS}/source-status-before.txt"
git rev-parse HEAD MERGE_HEAD >"${DOC402_LOGS}/source-refs.txt"
git ls-files -z | xargs -0 sha256sum >"${DOC402_LOGS}/tracked-files-before.sha256"
doc402_run compiler rustc -vV
doc402_run book-po-update make book-po-update
cp docs/book/po/zh-Hans.po "${DOC402_LOGS}/zh-Hans.updated.po"
git diff -- docs/book/po/zh-Hans.po >"${DOC402_LOGS}/po-unstaged.diff"
doc402_run check-i18n make check-i18n
doc402_run issue-refs python3 scripts/check-issue-refs.py
doc402_run brand-casing python3 scripts/check-brand-casing.py
doc402_run spec-attribution python3 scripts/check-spec-attribution.py --self-test
doc402_run doc-claims python3 scripts/check-doc-claims.py
test ! -e "${DOC402_LOGS}/english-html"
doc402_run english-html mdbook build -d "${DOC402_LOGS}/english-html" docs/book
test ! -e "${DOC402_LOGS}/zh-Hans-html"
doc402_run zh-Hans-html make book-zh "BOOK_ZH_DIR=${DOC402_LOGS}/zh-Hans-html"
git ls-files --stage -z >"${DOC402_LOGS}/index-after.entries"
cmp "${DOC402_LOGS}/index-before.entries" "${DOC402_LOGS}/index-after.entries"
git ls-files -z | xargs -0 sha256sum >"${DOC402_LOGS}/tracked-files-after.sha256"
git status --short >"${DOC402_LOGS}/source-status-after.txt"
printf 'OWNING DOCUMENTATION BATCH COMPLETE; index preserved; timing NOT RUN\n'
