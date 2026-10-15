# Current hosted stack and assembly repairs

The current CI compiler is the standard nightly 7df81381d (2026-10-14,
LLVM 23.1.3). Run 38681827689 fails the original evaluation_stack sweep for
2500 sibling OPTIONALs: none answers within its unchanged 6368 KiB maximum.
Five native assembly shards also fail to find a separately emitted
xml_escape_text<TextSink>; the wasm writer measurement already uses write_term.
These are new acceptance failures, separate from the previous cleared review.

The source correction adds OPTIONAL's left-child suspension to the existing
OrdinaryRunner. Its unit pending tag adds no second execution algorithm or
completion body: resume calls the original finish_left_join, now crate-visible.
All original node boundaries, error handling and completion logic are reused.
The original stack sweep remains unchanged and passes locally, as do all eleven
stack controls, fifteen focused OPTIONAL controls, three ownership controls,
all fifteen active frozen governor-corpus controls, six host-ID/carrier cases,
the complete default-query corpus replay and the actual WASM bag/OPTIONAL case.
Affected all-target warning-denied Clippy passes.

The first extra 256 KiB whole-request expectation was incorrect: independent
whole-plan admission still charges 512 bytes per algebra level. Its query-algebra
refusal is retained as a failed diagnostic, not proof of the recursive evaluator
cost. The corrected additional control admits the plan at 2 MiB, below the
original sweep's maximum, and verifies the flat spine answers. No admission law,
original test ceiling, default refusal or wasm host-stack limit was weakened.

The assembly manifest now measures the original out-of-line write_term<TextSink>
on all seven configurations. Its existing source annotation explicitly states
that it owns the inlined escape calls. No production inlining attribute is added;
the scanner/escape-driver measurements, site, seven configurations, floors,
FMA ceilings and relaxed-instruction prohibitions remain. The final emitted
counts and complete hosted parity are not yet available.

The native source index retains the exact constructor identity and source set.
Only eval.rs/binop.rs capacity-source fingerprints change; neither is a recorded
ledger authority. The unchanged actual helper gate passes 82 enforced jobs,
118 distinct rows and 21 variants; no ledger refresh is needed.

The invalidated complete producer 38682735961 was cancelled after the concrete
CI failures were diagnosed. It is terminal cancelled and its partial diagnostics
are retained under tasks/invalidated-projection-38682735961. No complete result
is inferred. Publish the bounded repairs normally, obtain a source-delta review,
run the complete producer on that coherent source and then finish integration.

Receipts: tasks/review-fixes-hosted-*-direct.log,
tasks/review-stack-{green-corrected,ownership,strict,optional-controls,
host-identity,query-corpus,optional-wasm,governor-corpus,helper-gate}.log and
tasks/review-stack-proof-comparison.log. review-stack-flat-red.log and the first
review-stack-green.log retain the invalid 256 KiB expectation. The original
hosted stack failure is the actual whole-contract red evidence.
