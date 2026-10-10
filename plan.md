# Stack-safe structural Turtle and TriG output

Complete the original structural renderer's blank-node walk with a heap work
list and retain its permanent structural-key depth guard. The public contract
requires actual 100,000- and 1,000,000-deep blank chains to complete natively and
on wasm32 with identical bytes, shallow packed output unchanged, deterministic
pack handling, and a benchmark. Main remains protected; the old dirty
stack-safe-evaluator worktree and its scratch example remain preserved.

Public acceptance names the packed render/render_canonical_turtle surface and
the separate native serialize_dataset Turtle/TriG surfaces. TriG includes a
named-graph chain and must preserve graph identity. Deep native execution uses
the ordinary default thread stack, with no on_stack size override.

## Context and prior attempts

The current kernel renderer recursively alternates render_props/render_object;
its content-key guard is still40. Preserved turtle_render source contains an
explicit Emit walk, cycle/shared/quoted blank classification and capped indentation,
but is unreviewed and has no qualifying current execution. Reconcile its source
with current native homes; use purrdf_lex::walk::WorkList rather than inventing a
second traversal container. Keep the original literal/IRI/term writer and fixed
hashers. Source-only work is not closure. No semantic features, dependencies or
namespace defaults are added. The baseline's no-deferrals rule applies; no new
production issue references or process prose belongs in shipped documentation.

The Stage brief captures the complete two-comment preservation record and related
serializer/helper issues; those old patches are donors, not acceptance evidence.
Known recurrence is the tool's captured trailer census; do not infer absence from
its finite search. No governing ADR is cited by the issue. Repository goals require
maximal utility/performance/portability and RDF1.2 preservation.

## Task 1: Complete the coherent renderer and acceptance source

Reconcile the preserved native output walk against current main, retaining both
source sides. Remove recursive property/object/collection output paths in favor
of the one WorkList emission body. Evaluate cycle, shared blank and quoted-term
references correctly before deciding which nodes pack. Keep the existing deep
structural-key guard permanently; ensure the guard applies deterministically and
does not silently drop triples. Avoid quadratic indentation/output or repeated
whole-output copies for deep chains. Confirm any remaining key recursion is
physically bounded by the preserved guard, including nested quoted terms.

The finalized kernel enforces events::MAX_TERM_NESTING_DEPTH16; preserve this
existing term law alongside the blank-key depth40 guard. Indentation remains
byte-identical to the original through level40, then saturates at that established
guard. Do not copy the donor's independent32 ceiling. Capture exact original
31/32/33/39/40/41 neighbor output; the41 case records the deliberate linear-output
indentation change. Keep shallow packing and every statement unchanged.

Write a shared Rust native/portable public corpus through the actual structural
render entry and actual Turtle/TriG serializer routes. It must exercise both deep
sizes, shallow packed goldens, cycles/shared references, RDF1.2 triple terms and
statement metadata, deterministic graph/interning permutation, and pack-guard
neighbors. Preserve every input statement. Compare frozen digest/length receipts
for the exact deep outputs on native and wasm; compilation alone is not runtime
parity. Use the existing portable testkit runner, not a new runtime. Add a report-only
benchmark under the existing first-party harness measuring actual render work at
shallow/100k/1M sizes. The old unrelated scratch example is not shipping evidence.

After complete source, run affected strict all-target compilation and native
renderer/serializer tests, actual portable deep controls and the benchmark. Fix
concrete findings coherently in their original homes. Reuse unaffected evidence;
perform the required single full qualification when the complete source settles.
Run metadata only if actual registered artifacts require regeneration. Obtain
independent implementation/completion judgment, then commit/push the intended
files normally with configured hooks/signing. Preserve unrelated work. Update
the issue with exact results; no partial-success claims or skipped gates.

## Task 2: Publish and integrate the complete contract

Publish a PR closing the issue only after all acceptance is proven. Record source,
native/portable/benchmark/full qualification and independent verdicts in validation.md.
Complete Stage2 feedback remediation and current-head hosted CI; Stage3 assesses
the actual clean integration tree and reuses qualifying affected evidence. Prepare
selected Stage archive and notes, merge only with the required ghprsq, verify issue
closure and archive, then clean the owned new worktree/branch. Keep the preserved
donor until its unrelated work is separately handled. Completion means merged main,
closed issue and owned cleanup; planning/source/checks count no closure.

## Completeness map and alternatives

Deep native/wasm output and byte parity map to Task1 shared public controls and
final CI; shallow packing/guard correctness map to Task1 exact goldens, cycle and
guard-neighbor inputs; the bench maps to Task1's actual existing harness; protected
publication/cleanup map to Task2. Replacing only the parser, lifting the permanent
guard, running with an oversized thread stack, truncating output, or asserting
only kernel unit success cannot satisfy the contract. Combining this contained
renderer delivery with the independent regex matcher is declined because their
distinct source and acceptance would hold an otherwise complete renderer behind
a large multi-host regex campaign. Top-tier writers and ready452 integration
continue concurrently; this is preserved middle-tier work, not recency priority.
