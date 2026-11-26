<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Single-source reifier scans

A stack scan over one certified image or one resident head uses the
source's unique reifier bindings directly. The stack translates RDF
identities as before. Scanning more than one source still suppresses
overlapping bindings and applies layer visibility in chronological order.

The `source_change_counts` benchmark scans 512 and 4,096 distinct
bindings formed from the same 64 reifier resources and 64 triple terms.
Each measurement opens a fresh stack query, reads the complete reifier
stream, and checks readiness. Construction and sealing are outside the
window. The same row witnesses appear with and without the table.

These are process allocation peaks on a Ryzen AI Max+ 395 under Linux,
using the optimized bench profile and Rust nightly `5df8c02af`.

| Source | Bindings | Baseline peak bytes | Current peak bytes |
| --- | ---: | ---: | ---: |
| Resident head | 512 | 127,446 | 79,826 |
| Resident head | 4,096 | 668,630 | 266,194 |
| Certified image | 512 | 235,920 | 198,831 |
| Certified image | 4,096 | 975,944 | 595,391 |

Both source types request 67,596 fewer bytes at 512 bindings and 540,732
fewer bytes at 4,096 bindings. The retained-byte delta is unchanged at
each size. These figures do not establish latency or a bound on total
query memory. Identity translation and authenticated image reads still
have their original storage costs.

```bash
PURRDF_SOURCE_CHANGE_ROWS=512 cargo bench --locked -p purrdf-core --bench source_change_counts -- reifier_scan_
PURRDF_SOURCE_CHANGE_ROWS=4096 cargo bench --locked -p purrdf-core --bench source_change_counts -- reifier_scan_
```
