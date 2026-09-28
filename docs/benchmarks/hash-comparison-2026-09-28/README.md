<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Hash comparison: 2026-09-28

Paired public-API comparisons on an AMD Ryzen AI Max+ 395. Sequential cases
were pinned to logical CPU 8; the parallel comparison used eight Rayon workers
within CPUs 8–15 and 24–31. The host has 16 physical cores and 32 logical CPUs.
These are measurements of this host, including when an older ISA is selected.
They do not measure older x86 processors or Arm/wasm hardware.

Compiler: rustc 1.100.0-nightly (4b6d04e70, LLVM 23.1.1). Both builds use opt-level
3, thin LTO, one codegen unit and the normal system allocator. `baseline` uses
`target-cpu=x86-64`; `native` uses `target-cpu=native`. Runtime-dispatched BLAKE3,
SHA-1 and CRC kernels can use detected hardware in either build. The fixed table
hasher's AES path is compile-time selected, so the baseline table comparison uses
the portable function. The host Cargo configuration selects thin LTO over the manifest; reproduction
uses `--lto thin` to make this choice explicit. Receipts record flags, effective profile configuration,
source hashes, compiler, CPU affinity, thread count and system load.

References: ahash 0.8.12, md-5 0.10.6, sha1 0.10.6, sha3 0.10.9,
crc32fast 1.5.0, blake3 1.8.5. Each was built only in the external comparison
workspace, from pins verified against the historical lock. Production dependencies
remain removed. See `scripts/bench-hash-comparison.py` for reproduction and
`crates/hash/PROVENANCE.md` for source consultation.

Each case calibrates its repetitions and collects twelve samples, alternating
reference/candidate and candidate/reference execution order. Raw CSV files keep
all samples. The table below shows the median of the twelve paired candidate /
reference time ratios, with their minimum and maximum in brackets. Below 1 means
PurRDF took less time; above 1 means it took more. This range is the observed
sample spread, not a confidence interval or a guarantee.

## Digests and table keys

Table keys compare the previous protocol with the current typed interner protocol
where applicable. They measure hashing only; map operations are separate cases.
Small integer and short AES-reference keys remain slower, while the hot IRI
terminal protocol is near parity. No distribution/avalanche requirement was
relaxed to improve these timings.

| Case | Bytes | Baseline ratio [range] | Native ratio [range] |
|---|---:|---:|---:|
| `digest/md5` | 0 | 0.937 [0.937, 0.938] | 0.865 [0.863, 0.869] |
| `digest/sha1` | 0 | 1.053 [1.048, 1.061] | 0.976 [0.973, 0.994] |
| `digest/sha3-224` | 0 | 0.961 [0.948, 0.974] | 0.633 [0.628, 0.636] |
| `digest/sha3-256` | 0 | 1.024 [1.013, 1.036] | 0.657 [0.656, 0.657] |
| `digest/sha3-384` | 0 | 1.011 [1.010, 1.013] | 0.653 [0.651, 0.656] |
| `digest/sha3-512` | 0 | 1.012 [1.008, 1.016] | 0.653 [0.652, 0.654] |
| `digest/crc32` | 0 | 1.856 [1.718, 1.992] | 1.248 [1.247, 1.250] |
| `digest/md5` | 16 | 0.919 [0.919, 0.920] | 0.860 [0.858, 0.863] |
| `digest/sha1` | 16 | 1.164 [1.162, 1.166] | 1.022 [1.004, 1.027] |
| `digest/sha3-224` | 16 | 0.976 [0.961, 0.991] | 0.640 [0.638, 0.641] |
| `digest/sha3-256` | 16 | 1.031 [1.016, 1.045] | 0.660 [0.658, 0.663] |
| `digest/sha3-384` | 16 | 1.014 [1.013, 1.015] | 0.655 [0.654, 0.657] |
| `digest/sha3-512` | 16 | 1.011 [1.009, 1.013] | 0.657 [0.655, 0.658] |
| `digest/crc32` | 16 | 0.377 [0.374, 0.379] | 0.603 [0.597, 0.605] |
| `digest/md5` | 64 | 0.957 [0.957, 0.958] | 0.890 [0.884, 0.892] |
| `digest/sha1` | 64 | 0.989 [0.988, 0.993] | 1.039 [1.037, 1.041] |
| `digest/sha3-224` | 64 | 0.979 [0.972, 0.988] | 0.643 [0.640, 0.646] |
| `digest/sha3-256` | 64 | 1.025 [1.018, 1.032] | 0.661 [0.660, 0.665] |
| `digest/sha3-384` | 64 | 1.019 [1.018, 1.020] | 0.657 [0.655, 0.658] |
| `digest/sha3-512` | 64 | 1.016 [1.014, 1.016] | 0.656 [0.655, 0.657] |
| `digest/crc32` | 64 | 0.762 [0.760, 0.763] | 0.507 [0.504, 0.513] |
| `digest/md5` | 1,024 | 0.978 [0.977, 0.978] | 0.978 [0.978, 0.979] |
| `digest/sha1` | 1,024 | 0.971 [0.970, 0.972] | 0.982 [0.981, 0.982] |
| `digest/sha3-224` | 1,024 | 0.970 [0.969, 0.973] | 0.617 [0.616, 0.618] |
| `digest/sha3-256` | 1,024 | 0.981 [0.979, 0.983] | 0.619 [0.618, 0.620] |
| `digest/sha3-384` | 1,024 | 0.978 [0.975, 0.979] | 0.616 [0.614, 0.617] |
| `digest/sha3-512` | 1,024 | 0.979 [0.976, 0.982] | 0.616 [0.615, 0.618] |
| `digest/crc32` | 1,024 | 1.064 [1.064, 1.065] | 1.008 [1.008, 1.009] |
| `digest/md5` | 65,536 | 0.980 [0.979, 0.980] | 0.993 [0.993, 0.994] |
| `digest/sha1` | 65,536 | 0.999 [0.999, 1.000] | 1.000 [0.999, 1.000] |
| `digest/sha3-224` | 65,536 | 0.975 [0.972, 0.976] | 0.614 [0.612, 0.615] |
| `digest/sha3-256` | 65,536 | 0.973 [0.972, 0.974] | 0.613 [0.612, 0.614] |
| `digest/sha3-384` | 65,536 | 0.975 [0.974, 0.977] | 0.611 [0.610, 0.612] |
| `digest/sha3-512` | 65,536 | 0.975 [0.975, 0.976] | 0.612 [0.611, 0.613] |
| `digest/crc32` | 65,536 | 1.002 [1.001, 1.004] | 1.000 [1.000, 1.001] |
| `digest/md5` | 1,048,576 | 0.980 [0.978, 0.980] | 0.993 [0.993, 0.994] |
| `digest/sha1` | 1,048,576 | 0.998 [0.996, 1.000] | 1.000 [0.999, 1.000] |
| `digest/sha3-224` | 1,048,576 | 0.975 [0.974, 0.976] | 0.614 [0.613, 0.615] |
| `digest/sha3-256` | 1,048,576 | 0.973 [0.971, 0.974] | 0.613 [0.612, 0.614] |
| `digest/sha3-384` | 1,048,576 | 0.976 [0.975, 0.976] | 0.611 [0.610, 0.613] |
| `digest/sha3-512` | 1,048,576 | 0.975 [0.973, 0.976] | 0.612 [0.611, 0.613] |
| `digest/crc32` | 1,048,576 | 1.000 [0.999, 1.001] | 1.001 [1.000, 1.001] |
| `blake3/native` | 0 | 0.839 [0.838, 0.839] | 0.593 [0.592, 0.594] |
| `blake3/native` | 16 | 0.844 [0.842, 0.844] | 0.593 [0.591, 0.594] |
| `blake3/native` | 64 | 0.601 [0.598, 0.602] | 0.601 [0.599, 0.601] |
| `blake3/native` | 1,024 | 0.688 [0.686, 0.689] | 0.647 [0.643, 0.649] |
| `blake3/native` | 4,096 | 1.083 [1.079, 1.087] | 1.108 [1.104, 1.113] |
| `blake3/native` | 65,536 | 1.020 [1.019, 1.020] | 1.016 [1.014, 1.018] |
| `blake3/native` | 131,072 | 1.004 [1.003, 1.005] | 1.010 [1.009, 1.011] |
| `blake3/native` | 1,048,576 | 1.037 [1.035, 1.038] | 1.033 [1.032, 1.033] |
| `blake3/native` | 16,777,216 | 1.008 [1.007, 1.009] | 1.017 [1.009, 1.018] |
| `table/u32` | 4 | 1.223 [1.212, 1.278] | 1.551 [1.422, 1.555] |
| `table/u64` | 8 | 1.153 [1.149, 1.155] | 1.767 [1.631, 1.773] |
| `table/triple` | 12 | 1.066 [1.060, 1.071] | 1.675 [1.646, 1.678] |
| `table/blank` | 16 | 0.981 [0.980, 0.983] | 1.815 [1.777, 1.819] |
| `table/literal` | 16 | 0.796 [0.794, 0.796] | 1.451 [1.436, 1.458] |
| `table/literal` | 256 | 0.786 [0.785, 0.788] | 0.863 [0.861, 0.869] |
| `table/language` | 32 | 0.863 [0.861, 0.867] | 1.515 [1.511, 1.518] |
| `table/str` | 26 | 1.257 [1.254, 1.259] | 1.967 [1.955, 1.973] |
| `table/str` | 256 | 0.940 [0.938, 0.941] | 1.074 [1.071, 1.077] |
| `table/iri` | 26 | 1.013 [1.012, 1.014] | 1.005 [1.001, 1.009] |
| `table/iri` | 256 | 0.854 [0.853, 0.855] | 0.960 [0.948, 0.964] |
| `table/bytes` | 0 | 1.290 [1.288, 1.293] | 1.585 [1.569, 1.639] |
| `table/bytes` | 8 | 1.276 [1.265, 1.290] | 1.705 [1.699, 1.712] |
| `table/bytes` | 16 | 1.235 [1.225, 1.244] | 1.798 [1.784, 1.802] |
| `table/bytes` | 32 | 1.319 [1.308, 1.321] | 2.141 [2.076, 2.150] |
| `table/bytes` | 64 | 1.292 [1.288, 1.294] | 2.146 [2.142, 2.164] |
| `table/bytes` | 128 | 1.054 [1.034, 1.067] | 1.147 [1.144, 1.178] |
| `table/bytes` | 1,024 | 0.859 [0.858, 0.859] | 0.737 [0.731, 0.746] |
| `table/bytes` | 16,384 | 0.781 [0.780, 0.782] | 0.708 [0.705, 0.709] |
| `map/insert-str` | 0 | 1.114 [1.112, 1.115] | 1.153 [1.127, 1.158] |
| `map/insert+lookup-str` | 0 | 1.055 [1.054, 1.056] | 1.279 [1.277, 1.282] |

## Streaming BLAKE3

Fresh default streaming states hash a 1 MiB message. Small writes benefit from
batching; large updates process caller memory directly and retain the final
subtree's compression inputs. These cases include state initialization and
finalization. The 16 KiB state is intended for bulk streams; `RecordHasher` is
the smaller state used for short, framed identities.

| Update size | Baseline ratio [range] | Native ratio [range] |
|---|---:|---:|
| 1 | 0.475 [0.459, 0.488] | 0.496 [0.495, 0.502] |
| 8 | 0.280 [0.277, 0.283] | 0.289 [0.287, 0.290] |
| 64 | 0.129 [0.129, 0.130] | 0.124 [0.124, 0.124] |
| 1024 | 0.098 [0.097, 0.100] | 0.095 [0.094, 0.095] |
| 16384 | 1.059 [1.059, 1.060] | 1.026 [1.023, 1.028] |
| 65536 | 1.057 [1.053, 1.058] | 1.000 [1.000, 1.001] |
| irregular | 0.228 [0.221, 0.236] | 0.216 [0.215, 0.217] |

## Parallel tree scheduling

`join/blake3-join.csv` compares each tested grain with blake3's `update_rayon`
and, in the `-vs-serial` cases, with PurRDF's own sequential one-shot call. Grain
is the recursive split threshold, not a worker count. Thread-pool startup is
warmed before sampling. The hash crate owns no threads; its `Join` caller chooses
the scheduler and threshold.

## Rejected experiments

- SHA-3: expanding all 24 rounds and expanding four rounds per loop performed
  worse than exposing constant lane indices and expanding two rounds per loop.
- BLAKE3: specialized SIMD single-root variants lost to scalar short-message
  compression; an eight-lane AVX-512VL batch did not improve the 8 KiB case enough
  to retain another kernel. The four-lane variant improved messages through 4 KiB.
- Fixed table hashing: a cheaper unary mixer failed the avalanche requirement;
  it was rejected. Packing caller metadata reduced protocol overhead without
  changing the generic hash function or its frozen self-vectors.

The maintained codegen and measurement rules are in
[the Rust optimization guide](../../design/purrdf-simd.md).
