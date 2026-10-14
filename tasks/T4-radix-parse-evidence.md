# Balanced decimal parse: partial evidence

Current source uses the original `10^19` Horner path through 32 complete
groups. A single validating reader skips lexical leading zeros without
skipping invalid suffixes. Longer streams enter a separate binary carry tree
frame: each occupied rank holds
`2^rank` consecutive groups, and lazily cached powers of `10^19` combine
adjacent blocks through the existing admitted multiplication and addition.
The final partial group retains its exact `10^tail` factor. This reads the
original iterator once and preserves invalid-byte and leading-zero behavior.

The final-source native boundary control passed 1/1 for 608, 626, 627, 628,
1,216, 4,000 and 40,000 digits, plus invalid-after-tree and long leading-zero
inputs. The production `try_parse_with_memory` control passed 1/1 with a real
40,000-digit integer: measured working peak stayed below admission, returned
ownership matched retained charging, and long padded integer, all-zero integer
and fractional inputs used zero allocations under a zero-byte grant. A padded
invalid suffix still refused without allocation. The actual Node/wasm32
determinism target passed 5/5, including this reader against the separate
division-based renderer; XSD all-targets strict Clippy passed. A direct wasm32
`--lib` attempt failed before case selection in an unrelated native-thread
unit test, so that route is not passing evidence.

The unchanged release benchmark's pre-balanced-parser binary, SHA256
`b4554b09e2cd449527aaa713954878da22934494f67f713c4fd5777a839678fb`,
remained available. Its nonexclusive three-repeat instruction controls were
595,356,179 for the empty case and 595,665,870 / 595,710,318 / 596,994,029 /
714,124,546 for `parse_canonical` at 40 / 400 / 4,000 / 40,000 digits. This
benchmark includes fixture construction and rendering. A final-source rebuild
of that unchanged bench passed; its SHA256 is
`9bdfb5700e13cb8bad382030c1a49f4cfcc9713b4d17680ba663a512afc3058c`.
The matching nonexclusive instruction counts are 363,315,370 empty and
363,336,206 / 363,358,897 / 365,094,818 / 456,002,410 at those four sizes.
Subtracting each build's empty setup gives 309,691 -> 20,836 at 40,
354,139 -> 43,527 at 400, 1,637,850 -> 1,779,448 at 4,000 (an 8.6%
regression), and 118,768,367 -> 92,687,040 at 40,000 (a 22% reduction).
These are complete parse-and-render benchmark cases, not parse-only timings;
the 4,000-digit regression and threshold/architecture qualification remain.

The benchmark's `--test --exact` route invokes a fixed one iteration
(`time_iterations(routine, 1)` in the existing testkit), so the differing
instruction counts do not come from a time-calibrated sample count. Its
`parse_canonical/4000` body parses and renders a 4,000-digit decimal
coefficient. That is 210 complete 19-digit groups and a 10-digit tail. The
tree starts with 32 groups, inserts 178 further complete groups, performs
175 carry merges and three final occupied-slot joins, and computes up to
six cached power squares. Thus 178 general BigInt products and
adds replace 210 single-limb Horner multiply-adds at this size. The retained
renderer still performs its separate repeated small divisions. General
product dispatch stays below the 512-limb Toom threshold here. This source
operation count explains a plausible crossover cost, but is not an isolated
parse profile or proof of the 8.6% net regression's exact attribution. No
dispatch threshold has been changed on this basis.

A subsequent **unqualified source candidate** retains the same validating
reader but accumulates each completed 32-group block with the original
single-limb Horner operation, then inserts that block at rank 5. The final
incomplete block is appended with the same scalar operation and no zero
padding. For 210 complete groups the source now makes five general block
products/joins rather than 178; it still does 210 scalar steps, on bounded
32-group blocks or the final accumulator. The 40,000-digit case has 65 full
blocks and 25 further complete groups, giving 64 block products/joins. This
candidate then passed native boundary 1/1, production governed parser 1/1,
actual Node/wasm32 harness 5/5, strict XSD all-target Clippy, and an unchanged
release bench build. Its bench executable SHA-256 is
`f2af2602ca5fd54d6576a7dd705a74c184edae15685b0272287f0ce00b236dbd`.
The same nonexclusive fixed one-iteration instruction route recorded empty
356,221,403, 4,000-digit 357,947,788, and 40,000-digit 448,355,660,
giving complete-case increments of 1,726,385 and 92,134,257. The prior
per-group tree increments were 1,779,448 and 92,687,040; the pre-tree
increments were 1,637,850 and 118,768,367. Thus the block candidate improves
the prior tree on these two complete cases, but remains above the pre-tree
4,000-digit count. Small 40/400-digit case subtraction is setup-sensitive
(the 40-digit case even counted fewer instructions than the no-match run)
and establishes no small-value win. Independent final-source review and
whole-issue performance qualification remain pending. The earlier measured
numbers above bind the per-group tree source, not this block candidate.

`numeric_parse_layout` retains the original two-destination bound for at most
32 groups and uses 32 times the checked result-limb bound after 32 groups.
For G complete groups and K=floor(log2 G), cached powers retain at most
2G+K limbs, slots and carried block at most G+K, recursive multiplication at
most 16(G+K), and simultaneous combine values at most 3(G+K). The sum is
22G+21K; for G>=33, K<=G/5 and G<64R/63, where R is the checked result
limb bound, so this is below 32R. This source-level inequality and physical
owners require independent review.

This is parse-only progress. Rendering still repeats `10^19` divisions, and
the normalized general division remains quadratic. The full #443 contract,
large transform, thresholds, portability and issue qualification are open.
