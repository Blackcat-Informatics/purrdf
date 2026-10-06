# Stage 2 remediation plan

Issue 457, PR 466. Reviewed head b2edf7450cf654d20ae97bd856d67102d0916b7b,
tree13496228db0e5ec79074acad9020cac5aab7556e; base b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac.
The Stage 1 plan and its hash remain authoritative and unchanged. No scope cut.

## G1: preserve composite blank registration in checked reinterning

HF1 / completion F1, MEDIUM. The new checked literal branch bypasses the native
embedded-blank registration that the original validated interner performed.
Route checked and ordinary literal insertion through one registration home,
retaining checked allocation/address refusal and exact lexical/scoped identity.
Add cold-dictionary ordinary/checked/validated parity witnesses for nested CDT
and recursive triples, idempotence, scoped/default blanks and opaque quoted text.

Acceptance: meaningful cold regression fails before repair and passes afterward;
global unit suite, affected core paging suite, guarded evaluator stack target and
affected clippy/hygiene pass. Independently review the exact source. Commit this
coherent repair with normal signing/hooks, push and read back, then publish actual
evidence and commit to both issue and PR.

## G2: bound graph-scoped physical candidates before row classification

PF1, HIGH. The new stream range probes walk every physical page slot per
ordinary row while looking for graph-scoped reifiers, even when that graph has
no reifier candidates. Reuse the native stream graph postings and intersect the
chronological range before inspecting summaries. Preserve indexed subject scans,
row filters, chronology, exact request order, shared admission and failure gates.
Avoid scanning the entire posting list once per layer.

New captured feedback PF2/CR1 (MEDIUM), discussion_r4200056289, is part of this
same metadata hot-path repair: use the existing physical failure latch in
logical_rows/reifiers per-row filters. Retain complete descriptor checks for
materialization, point/metadata reads and final operation_status before publishing.
Measure raw ID-row drain provider checkpoint calls at fixed pages/sources with
growing rows, and prove delayed drift still refuses guarded query/fold. Term
resolution can still legitimately check descriptors; make no broader call-count
claim. See gap-feedback-addendum.md for the independent adjudication.

Acceptance: bounded candidate-work evidence for growing plain-RDF stacks and
mixed streams/graphs; representative default/named/Any and range correctness;
core stack/fallible/admission/paging and evaluator stack suites pass with unchanged
receipt and carrier laws. Measure the actual metadata bottleneck without brittle
timing assertions or diagnostic debris. Independently review, signed/hooked commit,
push/readback and publication to both surfaces as a separate coherent fix.

## G3: qualify the assembly projection against its actual compiler and source

Hosted x86-v3/v4 jobs fail document parity for core.paged-map-quad, with no
required SIMD floor failed. Its old description names map_quad_to_global, which
has already been replaced by the shared QuadIds::map_ids home. The selector
measures the enclosing eager seal; individual body/call-site attribution is
required before any projection rewrite. See reviews/S2-simd-diagnosis.md and
its exact CI merge/compiler/manifest/job/artifact receipts.

After G1/G2 source stabilizes, inspect emitted eager-seal/mapping bodies on the
actual hosted compiler, retaining source/context/compiler-command/assembly hashes.
Correct the shared-home description and any evidenced selector discrepancy.
Regenerate the complete seven-configuration document through the existing tool,
then check all seven reports and their aggregate on final source/manifest/compiler.
Do not zero cells from old logs or qualify them with a different local compiler.
Obtain independent review, normal signed/hooked commit, push/readback and both
publication updates. Final hosted matrix and aggregate must pass for that head.

## Final gates

Carry any new feedback into this plan. Refresh all comments, reviews, inline
threads and hosted checks. Current hosted CI is running. Bind the final independent
completion audit to the actual final head/base/candidate, qualifying unchanged
evidence by source inputs; execution and CI claims remain separate. Stage 3 review
debt, final notes, ghprsq archive/integration verification and scoped cleanup still
must run before the issue is complete.
