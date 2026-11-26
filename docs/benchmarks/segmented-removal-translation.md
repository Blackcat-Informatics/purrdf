<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Segmented removal translations

The `source_change_counts` bench row `removal_visibility` scans an ordinary
stream with three nonempty resident removal carriers and a newer reinsertion.
Fixture construction is outside the measured window. Each size checks the
visible ordinary row count. A separate native regression compares complete
RDF values, including a removal that stays absent and a later reinsertion.

```bash
PURRDF_SOURCE_CHANGE_ROWS=128 cargo bench -p purrdf-core --bench source_change_counts -- removal_visibility --counts
PURRDF_SOURCE_CHANGE_ROWS=512 cargo bench -p purrdf-core --bench source_change_counts -- removal_visibility --counts
```

Measurements use a Ryzen AI Max+ 395 and Rust nightly `5df8c02af` with the
release-derived bench profile. Instruction counts are medians of three fresh
processes. Allocation measures are from separate runs of the same cases.

| Visible rows | Retired instructions before / after | Allocation calls before / after | Requested bytes before / after |
| ---: | ---: | ---: | ---: |
| 128 | 68,081,286 / 39,015,373 | 24,229 / 8,941 | 1,432,892 / 774,082 |
| 512 | 266,278,045 / 145,547,437 | 96,832 / 35,464 | 5,562,889 / 2,904,207 |

Peak working bytes are unchanged at 430,225 and 1,244,769. The bench
window drops its captured publication, so its negative retained deltas
are not query footprints. In the separate 128-input control with shared
subject, predicate and graph, retaining the query through counter close
gives 48,019 live bytes before and after. Peak declines from 59,171 to
58,047. All four heap measures match in the distinct-term control.

These counts establish less work and allocation traffic for the measured
scan. They do not establish an elapsed-time result, retained-memory savings
in other query shapes, or a throughput result for Wasm.
