<!-- SPDX-License-Identifier: CC-BY-4.0 -->
# Consolidated native repair rationale

The original native matrix is terminal failure, not a qualifying pass. The
repair preserves its original row order, score, allocation and governor pins.

The physical window comparator previously used the selected permutation alone.
The actual store's `candidate_access` switches broad prefix runs to sequential
SPOG access. `compare_pattern_probe_rows` now calls that original selection home
for each fixed prefix and delegates to the original permutation comparator.
Cross-prefix comparison keeps a total order; per-prefix traversal agrees with
the real native probe. The existing seeded all-mask cursor/probe differential
also checks that reverse input sorted by the certificate reproduces actual reads.

Phrase matching previously declared an unlimited row bound despite its frozen
finite source. A single nonempty phrase emits at most one row at each original
token start; empty analyzed phrases emit none. The relation now records the
original index's token-position count once, including auxiliary fields, with no
caller policy increase or new authored-size ceiling.

Prepared parameter coverage now uses `prepare_execution` and its declared
parameter binding slots. `declared_prebound` is an additional group-only
declaration, not the retained execution's parameter door. Both shape-enabled
and literal engines bind and execute the same original three phrase values.

The cache-refusal fixture's sticky-source assertion contradicts the original
segmented storage contract: refused workspace growth keeps the immutable
snapshot readable, while the query carries the original typed operational
refusal. The required assertion is the exact Residency error at the original
ceiling, no successful execution, no replacement of the previous cache owner,
and destruction/refund of the successfully retried native owner. Making a
workspace denial corrupt the source would change an unrelated storage contract.

These repairs require native/portable execution and complete qualification.
Strict compilation alone is not runtime evidence or issue completion.
