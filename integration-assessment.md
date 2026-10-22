# Main integration assessment

PR #681 initially published closure commit 578bb65da against main 6848fc886.
The clean-tree assessment returned a genuine conflict in
crates/entail/tests/current_clash_support.rs, so main was merged into the issue
branch rather than landing a conflicted or unchecked combination.

The conflict added functions and harness registrations at the same insertion
point. Reading both sides showed the branch signed_frozen scenario body and
main frozen_warrants_reject_empty_and_weakened_premises body are byte-for-byte
identical. Both first executed in the 17-case native integration run. The
resolution retains the existing main function once, all its assertions, and
the distinct signed_refutation scenario from the branch. A guarded source
comparison required equality before deleting the duplicate. No unique behavior
or assertion was discarded; final public refutation count is 16.

Other base changes are the already-landed columnar integrity/reconstruction
native causes, additive structured ContentDigest/ContentStoreError visitors,
their controls/documentation, and associated helper constructor bindings.
The freeze production merge only adds the main documentation statement:
the actual asserted-input binding already exists in the branch implementation.
Content-store visitor additions change no existing algorithm, layout or API
signature. Datalog/lexical maintenance implementation remains unchanged by
this synchronization. Existing model, work-cut, scale and bootstrap receipts
retain their inspected scopes; do not describe the earlier complete native gate
as a new execution on this combination.

Executed combined-tree checks:
- raw/main-integration-native.log, session 68373 exit 0: 6 columnar structured-error
  cases and 17 refutation cases before the identical test consolidation. Every
  retained final test body executed unchanged.
- raw/main-integration-public-wasm.log, session 14010 exit 0: 6 columnar cases and
  all 16 final refutation cases after consolidation, actual supported runner.
- Six-package strict compilation session 29422 exited 0.
- Helper/capacity bindings session 43820 exited 0. Normal merge commit session 3807
  and push session 34044 both exited 0, publishing 9a695a072.
- Independent full and incremental review now covers the current head; paginated
  feedback is exhausted and the sole substantive thread is resolved.

Current candidate inputs are base 6848fc88642a40183b885c387ffcd253b8ff5d2b and
head 55e3b052762f18d5850734ee7ff2970dd86bb5c9. The head already includes the
base. git merge-tree returned exit 0 and candidate tree
0e5c52146dd4f8a8c2125ce12f30134a29de662e. There is no intervening base delta
to qualify. Correction-path native, supported WASM, strict, allocation and helper
receipts are listed in validation.md. All 51 current hosted checks have succeeded;
four additional checks are configured skips. RISC-V multiworker attempt 2 passed
unchanged tests on the same source at 2026-10-22T20:29:33Z. The first attempt hit
the existing 30 minute limit after lengthy setup. No waiver was used.

Final stagectl metadata confirms these same base/head inputs, an open non-draft
PR and no merge yet. Paginated feedback refreshed after CI completion has no new
review, inline comment or issue comment. No implementation acceptance gap remains;
the candidate is ready for ghprsq. Actual landing, archival and cleanup are still
to be verified after publication.

The first stagectl pr-create returned a JSON parsing error after successfully
creating PR #681. Direct forge read confirmed its head/base and URL. No creation
retry occurred. Inspected stagectl create_pr calls its JSON-only gh wrapper on
gh pr create, whose output is a plain URL. This is a tooling-output gap, not a
failed publication or authorization blocker; no external tooling was modified.
