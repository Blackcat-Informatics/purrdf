# Current402 owning PO/render/doc-count preparation

Status: READY FOR ROOT ADMISSION; execution NOT RUN. Native and narrow host
qualification are separately settled PASS; local lane FREE. No translation or
shipping/index mutation occurred during the host run. Report timing remains held
while308 remote profiling is active. This is the owning documentation remainder,
not a discretionary workspace/native/host rerun.

Use the same402 WT and preserved4fa/resolved staged538 source. Root owns the index
and commit. Create a new owned `/opt/purrdf-shacl-402-docs-20261008.XXXXXXXX`
directory on admission. Record exact argv, complete per-command logs and actual
exits under it; stop on the first real failure and retain all initial evidence.
Run one scoped Bash controller with CARGO_BUILD_JOBS=8,RUST_TEST_THREADS=8,
MemoryMax64GiB/MemorySwapMax0. Normal Cargo children retain effective Stage
48GiB/8GiB swap policy; no compiler no-swap claim or global change.

Read-only prerequisites inspected2026-10-08: mdbook onPATH reports0.5.3, matching
docs.yaml; `cargo install --list` reports mdbook-i18n-helpers0.4.0 with gettext,
xgettext andnormalize, matching Makefile; msgmerge1.0 is available. Existing
production render gate rechecks both pins; no install/toolchain workaround.

Execute in this order after admission:

```sh
# Preserve exact pre-update owned bytes/digest of docs/book/po/zh-Hans.po.
make book-po-update
# Existing target runs extraction followed by msgmerge --quiet --update
# --backup=none. messages.pot is ignored; the only shipping write is zh-Hans.po.
make check-i18n
# Actual native helper-census --glossary-gate, then existing render --self-test:
# require all seven poisoned specimens to fail and actual normal render to pass
# all six gates. Capture actual page/fence/catalogue counts, not historical ones.
python3 scripts/check-issue-refs.py
python3 scripts/check-brand-casing.py
python3 scripts/check-spec-attribution.py --self-test
python3 scripts/check-doc-claims.py
mdbook build -d "$DOC402_LOGS/english-html" docs/book
make book-zh BOOK_ZH_DIR="$DOC402_LOGS/zh-Hans-html"
```

DOC402_LOGS is the owned task variable, never HOME. The English build is the
actual docs workflow's mdbook build, with output directed to retained owned disk;
the zh build uses the owning Make target/siteURL/search policy. book-samples is
unaffected generated SVG source, so this does not regenerate unchanged assets.
The render gate writes its own task-owned target/gate-scratch and removes it
normally; retain full reports and final HTML trees. No fabricated fixture or
new host semantic algorithm is introduced; existing legacy document gate code
is invoked as-is.

Source-impact assessment: msgmerge may adjust PO references, catalogue metadata,
fuzzy/obsolete entries and existing translations' association to changed source;
it does not invent translations. Preserve pre-update catalogue, report exact
diff/translated-fuzzy-untranslated counts, and inspect changed associations for
real glossary/render failures before any additional editing. Do not discard
existing translated strings or touch frozen external license whitespace. Root
must review and normally commit any resulting PO change. Other source/index
bytes and current source reference identities must remain unchanged.

Count applicability: the settled native actual communityCLI has64 cases,
REC57/WD58 applicable runs,115 passed executions and535 passed observations,
0 failed/unsupported. docs/CONFORMANCE.md already states these exact totals in
the matrix, scoreboard and corpus description. Confirm these prose rows against
retained native records.json/scoreboard, not a rerun of a host/vendor semantic
suite. The owning doc-claims gate additionally derives existing SHACL and other
documented numeric claims from authoritative generated/corpus source. Report
any actual stale claim as an owning finding before source remediation; do not
change numerical claims merely because narrow Python/Node selected totals differ
from the whole historical host census. Already qualified Rust/native corpus and
generated/interface gates retain their separate source scope.

After actual terminal, root must assess the PO-only source delta and current
main475 bench integration, retain the current-source evidence inventory, and
admit held matched report timing after308 campaign terminal. Fresh published-head
hosted gates and independent whole402 completion remain required. No forge,
commit, source/default-policy switch or other-repository submission is authorized
by this preparation.
