# Published remediation review debt

VERDICT: REQUIRED CI PENDING for delivery. Review debt DISCHARGED; substantive published remediation PASS.

Reviewed PR528 remediation a6fe69de7d10c8c26b1b5425f7bfb26c6141f4bd
against original findings and current source. Base is the recorded a4bb123 main.
No source mutation, test invocation, posting, commit, or merge performed here.

## Coverage

Stagectl merge brief refreshed pr-checks.txt and pr-comments.md. Because its
wrapper does not cover all reviews/inline threads, retrieved complete paginated
REST reviews, inline comments and issue comments into published-reviews.json,
published-inline-comments.json and published-issue-comments.json. Their counts
are10,13 and4. GraphQL published-threads.json reports five threads, with
hasNextPage false both at thread and every comment connection. All four original
threads are resolved; the fifth CodeQL thread was separately adjudicated and is now resolved. Current-head
review submissions after the original review contain no new substantive bodies;
CodeRabbit explicitly acknowledges each corrected original thread. The coverage
is a snapshot and must be refreshed for newly arriving feedback before merge.

## Per-finding adjudication

| Finding | Current judgment and disposition |
| --- | --- |
|4238597380 / graph read cancellation | ADDRESSED. Graph::refused latches the original stop into each driver's stopped bit. Existing after-round check_work now observes it before fixpoint/clash interpretation, and both final solve adapters check again before constructing a decision. The private both-calculus sweep and public mid-round sweep are meaningful controls, alongside healthy neighbors. Source matches root's independent review; native2 and final native/portable16 controls qualify changed behavior. Original thread resolved and reviewer accepts. |
|4238597387 / repeated proof lookup | ADDRESSED. CompletionView::of resolves top_role once; both independent finite closure and neighbors use that scalar. No repeated id_of_iri remains in neighbors and no new heap cache is introduced. Proof/control evidence applies; resolved and reviewer accepts. |
|4238597393 / top outside order panic | ADDRESSED. Each top and inverse-top membership is independently guarded before includes. Parser retains actual top ABox assertions and their individuals, preventing the additional false inventory boundary exposed by the reproducer. Public fixture puts top only in ABox outside the unrelated regular RBox and asserts universal and chain consequences. Native and optimized portable16 controls pass. Resolved and reviewer accepts. |
|4238597413 / Tableau fabricated inconsistency | ADDRESSED. The original consistency caller passes through decision_result, which checks storage_refusal before stopped, exhausted or consistent. Its control contrasts a healthy actual decision with the original typed admission refusal; it does not claim injected allocator OOM. Resolved and reviewer accepts. |
|Review5480160086 / local term IDs | ADDRESSED. All semantic hierarchy errors exiting parser compilation and original profile construction gain retained DiagnosticPresentation source spellings while the interner exists. Public classification preserves the original typed kind; presentation exposes admitted source terms without private interner access. Physical/stop failures remain typed failures. SourceTermDisplay uses the original native term debug writer and separately preserves scratch refusal. Public synthetic short-chain fixture confirms original offendingRole spelling after construction refusal. Final native/portable16 controls qualify this adapter. |
|Bot doc coverage70.35/80 | LEGITIMATELY DECLINED. Public diagnostic contracts, original classification, retained presentation and failure behavior are documented. No repository numerical documentation percentage gate exists. Blanket private/test comments would not address a missing behavior. Reason posted in PR comment6100693743; no unresolved inline documentation thread. |
|Original failed SIMD measurements | Historical failures retained. Original seven-column replay repaired the maintained projection using original reports. This is not new-head acceptance. New-head run38074965475 v3 job114279920483 is now SUCCESS; other actual current-head assembly/jobs remain separately pending below. |
|4238621231 / CodeQL alert241 | FALSE POSITIVE, LEGITIMATELY DECLINED. Actual location crates/entail/tests/reasoner.rs1691 prints certificate.violations() only as failed-assertion diagnostics in a fixed synthetic example.org OWL fixture. This certificate is an OWL profile classification, containing no credential, private key, secret, production input or user data. Alert API explicitly classifies the location as test and attributes original head2ce33c668. The useful diagnostic and CodeQL remain enabled. Root posted the reasoned reply4238714616 and resolved PRRT_kwDOTKq-Ms6rGXtx; published-threads-final.json independently confirms all five resolved. |

## Checks and actual live handles

published-ci-jobs.json captures complete jobs for current run38074965475.
Its checked runtime/native/assembly result is not inferred from original reports.
At capture, no run job failed;12 succeeded and four profile/projection jobs were
explicitly skipped. Remaining jobs are in_progress, not passed. Stagectl check
snapshot additionally records CodeRabbit SUCCESS, CodeQL NEUTRAL and independent
CodeQL language checks, including Rust still running. Mandatory pending checks
block merge despite the substantive source verdict.

Actual ongoing job handles captured:
workspace114279920020; capi114279920128; cross-arch114279920180;
integration-1114279920192; avx512-sde114279920225; aarch64114279920263;
wasm-exec114279920267; integration-3114279920282;
conformance-python114279920284; pytest114279920289;
wasm-package114279920326; integration-2114279920332;
miri114279920335; integration-5114279920346;
conformance-core114279920361; lib114279920365;
riscv64-determinism-serial114279920388; integration-4114279920391;
doc-tests114279920423; riscv64-determinism-one-worker114279920429;
riscv64-determinism-more-workers114279920447; riscv64-suites114279920452;
assembly-neoverse114279920489; assembly-aarch64114279920516;
assembly-v4114279921502.

Evidence reused after reading relevant logs and source:
remediation-native-2.log entire affected entail/validate all-target surface;
remediation-native-final-diagnostic.log and
remediation-portable-final-diagnostic.log each16 passed, zero failed/ignored/filtered;
strict5, metadata1 and helpers1 recorded terminal PASS in validation.md.
No extra broad suite is required merely for this review. Current CI still must
finish and its exact assembly results must cover maintained columns. Final integration,
protected merge and cleanup remain root-owned. No issue-completion claim.

## Current-head measured assembly evidence

Downloaded six actual checked artifacts under hosted-asm-published/, covering
x86_64, x86_64-v3, x86_64-v4, wasm32, wasm32-simd128 and aarch64. Every report
has status passed and identical original report identity: compiler rustc1.101.0-nightly
32dba69d69c5b10ea89a4042de8d0619f7756203, LLVM23.1.3;
source1996ed5fd9aeb25cebf2f1da8bdc0d0aec370113edbec043682bed4c0ccc6114;
manifestd695708fa7553f107c245589cf8c5aa71cb5845f834ae50262d32ef1da057349.
These current-run reports qualify their complete measured columns through the
unchanged original checker, rather than borrowing original-head measurements.
The affected entail.classify cells are v3 decide v9/f0/r0, saturate v4/f0/r0;
v4 decide v0/f0/r0, saturate v3/f0/r0, agreeing with the repaired maintained table.
Current jobs-final snapshot confirms those six configuration jobs SUCCESS.
Seventh neoverse-v1 job114279920489 is still in_progress, so seven-column and
aggregate acceptance remain PENDING. The aggregate simd-asm job starts after the
entire configuration matrix, requires every job success, downloads all checked
reports, and invokes the original --merge-reports checker to enforce complete
matrix, compiler/source/manifest coherence and maintained document parity.
simd-asm-projection is SKIPPED by its explicit workflow_dispatch-only condition;
it is a separate generator, not the mandatory ordinary aggregate parity gate.
