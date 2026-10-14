<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Current optimized-package stack-byte controls

Root executed standard `node --input-type=module` against the existing optimized
package via `Dataset`, `QueryEngine`, `ready` and `queryGovernedAsync`. No source,
tooling, stack budget, test floor or package was changed. The package is the retained
final numeric artifact preceding the later SCC/lifetime/lint-only source corrections;
this is diagnosis of that artifact, not validation of a freshly rebuilt838667cab.
Command terminal0; all18 queries reported isComplete=true.

Inputs and query builders copy the actual depth controls in
`crates/rdf-wasm/js/tests/query.test.mjs` and `async-concurrency.test.mjs`:
four subjects each with one p statement and only s1 with q; nested LATERAL or
FILTER NOT EXISTS around `?s <q> ?z`; the nine-edge WALK graph and nestedWalk
with the original x0 comparison. Observations are production evidence's
`async.stackHighWaterBytes` at depths1,2,4,8,16,32. Timing fields were emitted by
the API but are not used for performance claims.

| Shape | 1 | 2 | 4 | 8 | 16 | 32 |
|---|---:|---:|---:|---:|---:|---:|
| LATERAL |30220|34300|42460|58780|91420|156700|
| Repeated NOT EXISTS |32236|44956|59164|87580|144412|258076|
| Walk NOT EXISTS |40668|47772|61980|90396|147228|260892|

LATERAL increments are exactly4080 bytes per added level across this range.
Both NOT EXISTS controls grow exactly7104 bytes per added level from depth2;
the repeated NOT EXISTS depth1→2 transition differs and must not be fitted as
the same baseline. Matching slopes across the two EXISTS inputs point to their
shared recursive execution path, not walk-data growth alone. These measurements
do not identify individual compiler frames or prove a source correction.

Standard `wasm-dis` searches of the shipped module found none of the source
function names, so that observation cannot attribute individual frame sizes.
The checkout-local target release module was stale (October11), so its attempted
name search is excluded from current-artifact evidence. No disassembly files
were retained and no debug compiler flags or custom tools were introduced.

Next correction must reduce actual live stack bytes while preserving answers,
evidence, first errors and original refusal floors. This report does not clear
the three failed full npm assertions, qualify the package, or establish any
exclusive-timing/performance result.

Source-shape correction: the actual nestedAround LATERAL control nests its right
operand, not its left child. Its repeated descent occurs in eval_application's
per-row correlated/deferred-right loop. Extracting all post-left work into a
finisher would leave that finisher live across the same right recursion, so it
does not target the demonstrated shape and must not be adopted as its fix.
The later schema-union/output-row merge machinery also shares this frame despite
being used only after collection completes. A bounded final-merge extraction is
under source assessment; it remains unimplemented and unmeasured. No floor or
budget change is authorized by this diagnosis.
