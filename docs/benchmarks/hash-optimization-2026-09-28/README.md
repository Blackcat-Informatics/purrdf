<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Hash optimization measurements

Shared-host, CPU-pinned paired measurements. Ratios are candidate time divided
by reference time; smaller is faster. Twelve alternating-order samples per case.
The table comparison uses ahash 0.8.12, native CPU instructions, optimization
level 3, fat LTO and one codegen unit. The direct diagnostic build reuses the
historical harness's reference libraries; its exact compiler commands and source
hashes are in `metadata.json`. It is performance evidence, not a repository gate.

| Case | Bytes | Median ratio | Observed range |
|---|---:|---:|---:|
| table/u32 | 4 | 1.013 | 1.008–1.019 |
| table/u64 | 8 | 0.858 | 0.849–0.864 |
| table/triple | 12 | 0.888 | 0.885–0.891 |
| table/blank | 4 | 1.076 | 1.066–1.094 |
| table/blank | 16 | 1.293 | 1.200–1.308 |
| table/literal | 2 | 0.865 | 0.687–0.888 |
| table/literal | 16 | 0.918 | 0.889–0.938 |
| table/literal | 256 | 0.869 | 0.859–0.909 |
| table/language | 8 | 0.831 | 0.819–0.839 |
| table/language | 32 | 0.365 | 0.363–0.371 |
| table/str | 26 | 0.790 | 0.788–0.796 |
| table/str | 256 | 1.008 | 1.004–1.009 |
| table/iri | 26 | 0.779 | 0.774–0.781 |
| table/iri | 256 | 0.998 | 0.995–1.003 |
| table/bytes | 0 | 1.153 | 1.131–1.166 |
| table/bytes | 8 | 0.960 | 0.945–0.979 |
| table/bytes | 16 | 1.029 | 1.009–1.058 |
| table/bytes | 32 | 0.692 | 0.641–0.865 |
| table/bytes | 64 | 2.461 | 2.367–2.482 |
| table/bytes | 128 | 1.236 | 1.231–1.242 |
| table/bytes | 1024 | 0.778 | 0.773–0.780 |
| table/bytes | 16384 | 0.709 | 0.684–0.716 |
| map/insert-str | 0 | 1.005 | 1.003–1.010 |
| map/insert+lookup-str | 0 | 1.001 | 0.998–1.003 |

The large AES compression body is kept out of line, while the short byte update
and `Hasher::write` wrapper inline. An inner inline attribute alone still left
the ordinary string caller outlined, with accumulator spills. The mixing rounds,
string separator and frozen hash values are unchanged by this codegen repair.

The byte-slice cases at 64 and 128 bytes remain slower than the reference, as
do the 4- and 16-byte blank-label protocols. The table includes these losses:
the string and map results do not establish a universal win. The largest loss
is the 64-byte slice (2.461 times the reference time).

## Integrated workloads

`workloads/` records the final implementation, including the public string
wrapper inlining repair: nine output-identity comparisons, 324 paired timing
samples, allocation measurements and 81 hardware-counter receipts. Every
output identity matches across the three builds. Build receipts retain compiler
configuration, binary digests, source identity and hashes of changed Rust files;
the source snapshots remained immutable through compilation and measurement.
`driver-source.txt` and `workload-source.txt` retain the exact measured harness
sources matching the metadata digests. The maintained driver now inherits
Cargo build parallelism unless `--jobs` is supplied; the recorded build commands
retain the settings actually used for these measurements.

The reference is the historical implementation before dependency removal.
The middle build records the initial native replacements. Each timing pair
alternates execution order; ratios below compare the final candidate with the
historical reference. All measurements use CPU 12 on a shared host.

| Workload | Median time ratio | Observed range | Instruction ratio |
|---|---:|---:|---:|
| intern-iri | 0.929 | 0.885–0.948 | 0.976 |
| intern-mixed | 0.993 | 0.971–1.031 | 1.016 |
| parse-nquads | 0.980 | 0.861–1.141 | 1.008 |
| parse-turtle | 0.983 | 0.971–1.010 | 1.008 |
| query-join | 0.966 | 0.938–1.007 | 0.983 |
| gts-author-4k | 0.624 | 0.622–0.631 | 0.817 |
| gts-read-4k | 0.661 | 0.658–0.663 | 0.956 |
| gts-author-1m | 1.018 | 0.990–1.022 | 0.957 |
| gts-read-1m | 0.997 | 0.973–1.016 | 0.962 |

The 4 KiB GTS author/read workloads improve by about 38%/34%; IRI interning
improves by about 7%. Mixed interning, parsing, query evaluation and large GTS
operations have smaller differences and wider shared-host timing ranges.
The 1 MiB authoring median is 1.8% slower despite executing 4.3% fewer
instructions. These measurements do not establish a universal speedup or
behavior on other physical processors.

Allocation counts are unchanged for interning, parsing, querying and GTS
reading. GTS authoring falls from 17,369 to 16,089 allocations for the 4 KiB
case, and from 726 to 686 for the 1 MiB case. Query peak tracked memory falls
from 2,053,264 to 1,711,120 bytes, with the same allocation count. Counting uses
a separate executable; the timing executable uses the system allocator.
