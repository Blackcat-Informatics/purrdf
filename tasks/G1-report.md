# Workspace ownership-test layout correction

The completed actual workspace job failed at one owned strict Clippy finding:
`items_after_test_module` for `owl_dl/bounds.rs::owner_tests`. Preserve the original
failure in `hosted-workspace-job-1.json/.log`, run 38055010701, job 114221558900,
head `d5ee0c862536b97dcba3bf0e8b55c28d641f077d`. All preceding workspace and
preserve-order checks passed. No complete fresh hosted acceptance is claimed.

The complete one-file repair moves the unchanged ownership-test module after
all production items. Both original tests, their source inputs and every
allocator/borrow/retained/drop assertion remain intact. Production items are
unchanged; no lint expectation, suppression or gate modification is introduced.
`hosted-workspace-layout.patch` is the ordinary delta. The original nine-criterion
full4/O3/native/portable/host judgment remains applicable to unchanged behavior.

The capped managed focused family is terminal PASS, original session 84279 exit
0, `hosted-workspace-remediation-1.sh/.log`: formatting check, strict all-target
entailment lint (6.82s), and both actual original owner tests (2 PASS, 0 FAIL).
The lint command retains `--locked -- -D warnings`. The tests still measure
original admitted products, covered allocator peak, retained bytes, allocation
count, release after drop and allocation-free borrowing of original guards.

The managed local compiler is 1.100 nightly4b6d04e70 (2026-09-13); the actual
failed CI used 1.101 nightly32dba69d6 (2026-10-09). Automatic tool review rejected
the direct isolated Rustup install: “Rust toolchain installation/selection is
owned by Stage.” The command did not run; no installer or selection guard was
bypassed. Parent directed existing managed focused qualification plus exact
binding hosted CI after normal push. No toolchain/default was mutated and no
unrelated job was cancelled or restarted.

The lightweight remediation plan is posted to both issue and PR through normal
Stagectl comments. The parent independent delta disposition is PASS at
`../reviews/hosted-module-placement-delta.md`, with no source/consumer/ownership
finding. Normal signed fix commit/hooks and push each terminated0, publishing
`b7195b537f535b688c77c8dbd3ee9b97246e198c` to PR527. Exact fresh hosted results,
current-main candidate interaction and issue closure remain pending. Parent owns
final CI, review debt, actual merged candidate and protected ghprsq integration.
Results published through the normal Stagectl path to the
[issue](https://github.com/Blackcat-Informatics/purrdf/issues/499#issuecomment-6098028266)
and [PR](https://github.com/Blackcat-Informatics/purrdf/pull/527#issuecomment-6098036834),
both publication commands terminal 0. These records apply to the signed b719
fix head; the subsequent required actual-main conflict resolution has its own
`main-508-integration-assessment.md` and qualification records.
