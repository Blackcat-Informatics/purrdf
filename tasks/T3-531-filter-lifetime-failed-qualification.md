<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 filter-sequence no-inline candidate: rejected

The sole source change replaced `eval_filter_sequence`'s `#[inline(always)]` with
`#[inline(never)]`, intending to remove post-child row-loop temporaries from the
recursive `eval_filter` frame. The standard default-directory release Cargo build,
provided wasm-bindgen, `wasm-opt -Oz`, and provided `wasm-link` all returned zero.
The exact raw compiler artifact SHA-256 was
`4702857216612400008fb0152d751c3348c9964ebd4e5aec3cce4900680beb67`;
the linked package module SHA-256 was
`1a28fd0bdc806e04c7459da3b0c7071a38981385af1a8a29df2db8b18178671a`.
The linker confirmed the suspend/run/poison transformation and SIMD features.

The unchanged `node --test --test-name-pattern='nested LATERAL|nested FILTER NOT EXISTS' tests/query.test.mjs`
on that exact linked package returned **exit 1**: LATERAL answered **234** levels
against the required **281**, and FILTER NOT EXISTS answered **133** against the
required **180**. Both were typed refusals and neither original depth floor passed.

Production `queryGovernedAsync` on the same linked module and unchanged query
shapes returned complete results at depths 2/4/8/16/32:

| Shape | 2 | 4 | 8 | 16 | 32 | Per-level slope |
|---|---:|---:|---:|---:|---:|---:|
| Right-nested LATERAL | 34,300 | 42,460 | 58,780 | 91,420 | 156,700 | 4,080 bytes |
| Original nine-edge WALK NOT EXISTS | 46,844 | 61,116 | 89,660 | 146,748 | 260,924 | 7,136 bytes |

The WAT name search on the release compiler artifact yielded no usable function
names. The candidate had no measured stack benefit; its single attribute/comment
change was reverted. The checkout source is back at its preceding clean head.
The earlier diagnostic that removed the inner inequality `FILTER` changed the
query-plan shape as well as its expression, so its lower slope cannot by itself
attribute 1,552 bytes to an inlined `eval_filter_sequence` body. Another source
change requires direct linked-path evidence; neither package nor this receipt
qualifies #531 or #470.

The actual recursive edge clarifies the failed theory: `eval_filter` first
evaluates its data child, but `eval_filter_sequence` then calls `linked.filter`
inside its sequential row loop. For the outer `FILTER NOT EXISTS`, that
expression evaluates the nested `EXISTS` body. Thus the sequence's linked
program, checkpoint and row builder remain live *during* the nested recursive
evaluation. Outlining that sequence cannot remove its live state from each
recursive level; it merely relocates the state to another active frame. This
source-path finding is consistent with, but not a separate measurement of, the
unchanged optimized stack slope.

The repeated substituted window also checks `ctx.deferred_exists` at
`exists_recorded` entry. Its merged deferred arm calls
`binop::eval_substituted` directly, rather than the ordinary
`exists_prepared` definition path. Consequently the earlier named raw WASM
`exists_prepared` 3,952-byte frame cannot be assigned to every inner level of
this workload from that old artifact alone. This is a source-path distinction,
not a fresh linked-frame measurement.
