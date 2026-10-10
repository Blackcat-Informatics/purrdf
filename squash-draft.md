Remove fixed text-ranking input ceilings with exact native arithmetic

Closes #482

The public text index and six-cell search relation accept large query-term,
field-token, corpus-population and BM25F field inputs without the former chosen
ceilings. Counts use their physical native widths; promoted intermediates use
the existing exact Integer/BigInt home. Original score rounding and the sixteen
independent BM25F frozen vectors remain unchanged. The v2 scoring profile and
index identity explicitly describe the wider domain; each result carries an
input-derived score certificate rather than the old global bound.

The complete contract is issue 482. Changes cover text fixed/ranking/index/score
and relation code, BM25F and scoring fixtures, benchmark compatibility, published
ranking documentation and the existing portable-test inventory. No runtime
dependency, semantic Cargo feature, fabricated namespace or fallback is added.

Qualification: one original required full local make check invocation passed,
including native workspace tests, doctests, the independent preserve-order
consumer, kernel ring-fence and all 32 publishable release-WASM crates. Actual
5,000-term producer and 33,554,432-token field acceptance passed. All sixteen
unchanged independent reference vectors passed; strict affected Clippy passed;
all eight actual portable cases passed. Two native metadata expectations were
corrected and the complete scoring target subsequently passed. The historical
failure is preserved, and no failed invocation is represented as a pass.

The existing complete independent applied review and per-task reviews pass.
The clean candidate against main's schema/calendar delivery preserves all
qualified ranking and arithmetic-owner bytes. The unrelated base delta does
not invalidate their qualification; the branch is preserved without restarting
its already-passed local gate. All hosted checks are settled: 48 SUCCESS and
five conditional SKIPPED, with no pending or failing check. Distinct final
review-debt is PASS: all three comments are dispositioned, CodeRabbit reports
no actionable finding, and there are no submitted reviews or unresolved threads.
The final clean candidate remains unchanged. No conflict requires resolution.

Standing goals: one original arithmetic home, exact deterministic output,
portable shipping crates, no silent errors, unchanged result arity and no
deferred acceptance criteria. There are no declined required behaviors.

Authoritative plan: .stage/text-rank-inputs-past-query-terms-max/plan.md.
Selected evidence directory: .stage/text-rank-inputs-past-query-terms-max.
The original task autostash is preserved losslessly in
original-autostash-recovery.patch before any owned worktree cleanup.

Defect-Class: performance
