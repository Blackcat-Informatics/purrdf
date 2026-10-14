# Task 5 GTS identity — focused evidence, broader gate pending

Source: #365 worktree HEAD ce2be1058 (the three issue commits rebased directly
onto origin/main dd79ef77e) plus the current uncommitted writer.rs diff. The
rebase removed rejected #535 tooling commit 96f8c5f63; the committed diff
contains none of its six tooling files. The dirty writer diff was preserved
byte-for-byte through the rebase by a Stage patch comparison.

The acyclic production path computes depths once, retains each flat canonical
CBOR identity once, interns exact bottom-up structural classes for flat bytes,
missing IDs and ordered triple children, and memoizes compared nested-pair
orders in a bounded map. Canonical byte order is still decided by the original
CBOR structure comparison. Cyclic malformed input retains expanded keys to
preserve path-relative cycle markers. Both production walkers now use O(1)
active marks rather than path contains scans. Recursive test references still
use the old scan as an independent oracle.

The focused library command passed 7/7 on this source. It compares 500
generated acyclic DAGs and 500 malformed tables against the old expanded-key
order, verifies zero allocations for 1,023 ordinary flat comparisons, and
writes/reads all scaling snapshots. After final lint corrections,
the full purrdf-gts crate test passed terminal 0: 126 library tests, all
integration targets including the byte-exact frozen canonical vectors and
12 self-reaching triple-term tests, and all doctests. Affected all-target
Clippy with -D warnings and cargo fmt check passed terminal 0. Independent
Task 5 source re-review is PASS (T5-review.md). The earlier broad test was
interrupted during compilation by turn cancellation and did not pass; the
first attempt before cache pruning failed with /opt full.

| Chain levels | Visits | Peak identity bytes | Prior expanded key bytes |
|---:|---:|---:|---:|
| 128 | 1,037 | 13,606 | 498,766 |
| 256 | 2,061 | 26,918 | 1,980,494 |
| 512 | 4,109 | 53,542 | 7,893,070 |
| 1,024 | 8,206 | 106,790 | 31,514,702 |

| Disjoint chain levels | Equal leaves: visits / bytes | Different leaves: visits / bytes |
|---:|---:|---:|
| 128 | 2,206 / 24,704 | 2,334 / 32,609 |
| 256 | 4,412 / 49,280 | 4,668 / 64,833 |
| 512 | 8,795 / 98,432 | 9,307 / 129,281 |
| 1,024 | 17,550 / 196,736 | 18,574 / 258,177 |

| Shared-DAG width | Visits | Flat key bytes | Peak identity bytes |
|---:|---:|---:|---:|
| 64 | 2,057 | 2,254 | 27,550 |
| 128 | 4,301 | 4,430 | 53,342 |
| 256 | 9,146 | 8,782 | 149,726 |
| 512 | 19,832 | 17,486 | 297,694 |

Visits and retained-state bytes stay below 3x at each doubling, including
equal and leaf-different duplicate-chain adversaries. Peak state counts
retained identity structures and capacity estimates, not whole-process RSS
or transient CBOR encoder memory. The class and pair maps are bounded by
terms, edges and flat payload; malformed cyclic fallback has no linear-state
performance claim.

Remaining before Task 5 checkpoint: normal commit hook, push, and partial
#365 issue update. Whole #365 remains open, including Task 4 and Task 6.
