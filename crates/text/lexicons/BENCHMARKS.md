<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Dictionary representation measurements

Command: `cargo bench -p purrdf-text --bench dictionaries --locked -- --quick`.
All 40 cases completed on the native x86-64 development host with 10 samples per case.
Other compilation was active during this quick comparison; timing uncertainty is
substantial, so these measurements support no general throughput claim.
Retained allocation bytes and canonical artifact bytes are deterministic facts.

The flat radix representation remains the default. The minimal acyclic byte
automaton increases retained bytes for four of five complete dictionaries. Its
CJK load interval also exceeds the radix interval. Local lookup or segmentation
improvements therefore do not satisfy the selection rule: a demonstrated latency
gain without construction, load, memory or artifact-size regression outside
uncertainty. Both forms remain directly measurable and conformance-tested.

| Dictionary | Form | Retained bytes | Artifact bytes |
|---|---|---:|---:|
| cjdict | radix | 14747741 | 4833368 |
| cjdict | minimal | 14170669 | 4833368 |
| thaidict | radix | 1556235 | 620428 |
| thaidict | minimal | 2391195 | 620428 |
| laodict | radix | 1935333 | 824965 |
| laodict | minimal | 3271153 | 824965 |
| khmerdict | radix | 5339508 | 2430900 |
| khmerdict | minimal | 9024784 | 2430900 |
| burmesedict | radix | 2754020 | 1362430 |
| burmesedict | minimal | 5545336 | 1362430 |

Intervals below are the harness bootstrap intervals around the sample median.
Lookup probes every 97th canonical entry; segmentation concatenates the first
96 probes. Construction and loading use the entire dictionary. The minimal-form
construction includes canonical radix preparation followed by right-language
minimization; artifacts store the shared canonical vocabulary, so their size is
independent of lookup representation.

| Dictionary/form | Operation | Lower | Median | Upper | MAD |
|---|---|---:|---:|---:|---:|
| cjdict/radix | construct | 362.4 ms | 418.5 ms | 523.1 ms | 63.81 ms |
| cjdict/radix | load | 193.1 ms | 243.0 ms | 254.8 ms | 29.64 ms |
| cjdict/radix | lookup | 660.7 µs | 756.8 µs | 772.0 µs | 38.24 µs |
| cjdict/radix | segment | 66.66 µs | 84.62 µs | 85.89 µs | 3.640 µs |
| cjdict/minimal | construct | 309.9 ms | 353.3 ms | 416.1 ms | 39.48 ms |
| cjdict/minimal | load | 306.6 ms | 333.4 ms | 350.2 ms | 13.86 ms |
| cjdict/minimal | lookup | 500.0 µs | 543.8 µs | 601.7 µs | 53.15 µs |
| cjdict/minimal | segment | 53.74 µs | 65.98 µs | 69.85 µs | 4.345 µs |
| thaidict/radix | construct | 26.12 ms | 29.08 ms | 33.89 ms | 3.245 ms |
| thaidict/radix | load | 36.33 ms | 36.72 ms | 37.24 ms | 260.8 µs |
| thaidict/radix | lookup | 41.52 µs | 45.40 µs | 50.51 µs | 4.078 µs |
| thaidict/radix | segment | 58.85 µs | 83.60 µs | 93.73 µs | 12.06 µs |
| thaidict/minimal | construct | 45.06 ms | 47.88 ms | 49.12 ms | 1.901 ms |
| thaidict/minimal | load | 35.19 ms | 35.61 ms | 40.64 ms | 481.8 µs |
| thaidict/minimal | lookup | 43.28 µs | 47.03 µs | 50.14 µs | 3.381 µs |
| thaidict/minimal | segment | 48.01 µs | 73.63 µs | 80.38 µs | 7.420 µs |
| laodict/radix | construct | 35.20 ms | 42.96 ms | 47.22 ms | 4.692 ms |
| laodict/radix | load | 33.20 ms | 35.40 ms | 39.92 ms | 2.490 ms |
| laodict/radix | lookup | 60.61 µs | 61.60 µs | 62.45 µs | 846.4 ns |
| laodict/radix | segment | 64.77 µs | 65.73 µs | 68.30 µs | 839.4 ns |
| laodict/minimal | construct | 40.47 ms | 43.74 ms | 222.7 ms | 4.272 ms |
| laodict/minimal | load | 170.5 ms | 253.5 ms | 313.8 ms | 62.96 ms |
| laodict/minimal | lookup | 51.99 µs | 54.50 µs | 55.06 µs | 460.7 ns |
| laodict/minimal | segment | 56.95 µs | 75.11 µs | 76.70 µs | 2.868 µs |
| khmerdict/radix | construct | 69.85 ms | 71.01 ms | 76.58 ms | 1.251 ms |
| khmerdict/radix | load | 71.37 ms | 73.23 ms | 75.73 ms | 1.996 ms |
| khmerdict/radix | lookup | 160.0 µs | 165.9 µs | 169.3 µs | 3.602 µs |
| khmerdict/radix | segment | 55.75 µs | 58.40 µs | 59.27 µs | 1.271 µs |
| khmerdict/minimal | construct | 108.5 ms | 113.4 ms | 128.5 ms | 5.404 ms |
| khmerdict/minimal | load | 112.1 ms | 113.6 ms | 118.1 ms | 1.640 ms |
| khmerdict/minimal | lookup | 173.4 µs | 177.6 µs | 184.1 µs | 4.480 µs |
| khmerdict/minimal | segment | 49.45 µs | 49.89 µs | 49.99 µs | 149.2 ns |
| burmesedict/radix | construct | 39.25 ms | 41.03 ms | 42.32 ms | 1.614 ms |
| burmesedict/radix | load | 40.73 ms | 41.37 ms | 43.34 ms | 787.5 µs |
| burmesedict/radix | lookup | 54.99 µs | 57.91 µs | 62.06 µs | 3.055 µs |
| burmesedict/radix | segment | 55.23 µs | 57.63 µs | 62.13 µs | 2.816 µs |
| burmesedict/minimal | construct | 67.17 ms | 68.59 ms | 71.86 ms | 1.488 ms |
| burmesedict/minimal | load | 64.45 ms | 65.70 ms | 69.28 ms | 1.541 ms |
| burmesedict/minimal | lookup | 70.49 µs | 73.05 µs | 75.57 µs | 2.538 µs |
| burmesedict/minimal | segment | 56.50 µs | 57.14 µs | 57.48 µs | 407.5 ns |
