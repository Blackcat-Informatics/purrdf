# PR502 bounded feedback remediation — 2026-10-08

Status: source and affected qualification PASS; see G1-qualification.md for
actual terminal receipts and preserved initial example compile failure. Root owns the independent
delta review, lane admission, normal commit/push and feedback publication.
Prior full/render PASS at b1833aeed remains attributable historical evidence;
the new parser/drop paths need their affected qualification before current PASS.

Read complete raw PR review5452865445, inline4215915030 and discussion surface,
captured as `T3-pr502-reviews.json`, `T3-pr502-inline-comments.json` and
`T3-pr502-discussion.json`. Stage2 lightweight remediation is appropriate: two
localized validation/error-handling fixes in existing homes, no architecture,
dependency, target-selection or translation-catalogue change. No child agent,
build, Git/index or forge mutation was performed. Shipping source delta is three
paths: glossary/mod.rs, examples/glossary_hook_probe.rs and authoritative glossary
documentation. Direct SDK rustfmt and git diff --check actually exit0.

## G1a: keep-English regex anchor — real functional gap

The existing parser accepted slash-delimited regex anchors on K rows, compiling
the ordinary anchor as regex but its keep token as the literal slash-delimited
text. Actual prose carrying the intended term would not trigger token survival.
The synthetic self-test inserted the raw anchor itself, hiding the unusable
configuration. No existing table row needs this combination, but malformed
external/future glossary input could otherwise pass silently.

The parser now refuses K+regex at the anchor loop before token-pattern creation,
with actual row number/term and offending anchor diagnostic. Arbitrary regexes
cannot specify which concrete literal must survive, so no guessed token or
second K-regex algorithm is added. Non-K regex anchors remain accepted and
active; literal K anchors retain existing whole-token/case behavior. The glossary
documents this concrete literal-only K invariant in one sentence.

Written parser/production-check fixture rejects both a lone regex K anchor and
a mixed literal+regex K list with row-specific diagnostics; proves literal K
drop refuses and kept literal passes through actual offences; proves non-K
`/literal(?!ly)/` still anchors literal and excludes literally. This exercises
real parser/caller logic rather than merely inspecting a generated pattern.

## G1b: restoration Drop double panic — real failure-path gap

An expect-based write in Restore::drop could panic during another panic and
abort without an actionable restoration diagnosis. Replacing it with an always-
nonfatal print would weaken required normal-path qualification failure, so that
suggestion is deliberately narrowed to the actual unwind case.

Restore still writes exact original bytes. A failed write when not already
panicking produces a hard panic with path/error. Only while already unwinding
does it print the actionable failure and preserve the original panic. Existing
normal main-path exact real-index and catalogue assertions remain unchanged;
there is no successful-path swallowed error or silent fallback.

Three contained example unit controls are written: exact restoration after
poison; ordinary restoration failure caught as refusal; and forced write-to-
directory failure during catch_unwind retaining the original panic payload.
They touch only testkit-owned scratch, not the repository/index/catalogue, and
need no resource-heavy subprocess. Example remains test=false for ordinary
target selection. Use explicit `cargo test --example glossary_hook_probe` to
select its unit harness, not the resource-sensitive production main. Cargo's
[test documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
describes explicit example target selection.

## Proportional documentation assessment

CodeRabbit's20.97% versus80% docstring threshold is a generic external review
warning, not a repository/accepted-plan numeric criterion. Repository rules
require meaningful documentation and strict rustdoc/clippy, not commentary on
every private straightforward function. Existing module docs, version/grammar
contract, projection comments, narrow typography policy and actual source
boundaries explain the behavior. The new K rule and Drop rationale are documented
where users/maintainers need them. No boilerplate padding or broad unrelated
private-function documentation rewrite is warranted. Root can publish this
reasoned disposition separately from the two fixed findings.

## Required affected checks (historical preparation; now completed)

With CARGO_BUILD_JOBS8 and explicit --jobs8: native glossary tests (now including
the parser fixture); explicit three-control example tests; strict all-target
helper-census clippy; actual production native gate/self-test and existing caller
tests/parity/helper hygiene as affected. No automatic repeat of the entire full
suite or six-arm rendered book merely because Stage2 advanced: the catalogue,
render machinery, wire callers and existing table inputs are unchanged. Retain
prior full/render evidence with explicit source-delta applicability. Any actual
fixture/warning/hygiene failure is owned and must be fixed before committing.
Root independently reviews this delta and chooses meaningful actual executions;
no success is claimed from these written controls.
