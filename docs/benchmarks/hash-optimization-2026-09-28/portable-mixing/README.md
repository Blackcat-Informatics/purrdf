<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Portable pair mixing

A two-word folded-multiply update must seed its second lane. With a nonzero
multiplier, both zero and all-ones words are fixed points of XOR-folding the
128-bit product. An unseeded lane lets consecutive zero/all-ones fields cancel.
The selected update XORs the second word with an independent dense key before
multiplication. It retains two independent products and constant-foldable metadata.
These public keys do not provide resistance to deliberately constructed collisions.

## Measurements

Six synthetic update protocols, twelve samples each, three million iterations
per sample, pinned to CPU 12 on a shared host. The three implementations run in
rotating, reversed order. Builds target baseline x86-64, so this measures the
portable implementation even on an AES-capable host. Ratios are elapsed time
divided by the original unseeded implementation. They are diagnostic update
costs, not end-to-end RDF workload measurements or claims about other processors.

| Protocol | Successive folds | Selected seeded lane | Selected observed range |
|---|---:|---:|---:|
| u64 | 1.000 | 1.001 | 0.979–1.034 |
| triple-local | 1.354 | 1.001 | 0.986–1.017 |
| triple-global | 1.609 | 1.001 | 0.985–1.009 |
| bytes16 | 0.981 | 1.037 | 1.024–1.039 |
| str26 | 1.023 | 1.026 | 1.022–1.122 |
| bytes64 | 0.996 | 1.019 | 1.005–1.027 |

The selected form preserves the measured packed triple costs. The byte16 case
costs about 3.7% more. The 26-byte string and 64-byte paths do not use the changed
pair update, and their roughly 2–3% differences illustrate sensitivity to code
layout and shared-host timing. The unchanged u64 case is within about 0.1%.

`metadata.json` records compiler flags, CPU information, load, and source/binary
SHA-256 digests. `samples.csv` retains every sample. Variant names are `before`
(original), `after` (rejected successive folds), and `offset` (selected seeded
lane). `driver-source.txt` is the exact measured driver. Copy it to `main.rs`
beside a snapshot of `crates/hash/src/fixed.rs` and the `fixed/` directory, then
compile with the flags in the metadata. `variant-patches.txt` records the two
alternatives relative to the selected source; apply each variant's patch with `git apply --unidiff-zero` in its own copy.
Run `taskset -c 12 ./run <protocol> 3000000`. The driver calls `black_box` on
iteration input, slice input, and each output; case dispatch is outside timing.

## Correctness evidence

The new portable structured-field regression fails on the original law. The
selected law passes all 23 native fixed-hasher tests, including avalanche,
distribution and collision requirements. The same structured-field regression
and 3,676 portable frozen vectors pass in the i686 build. Portable tests run
explicitly on an AES host, so host dispatch cannot conceal this defect again.
All six portable WebAssembly tests pass, and the actual RDF packed-field
regression passes in a baseline-x86-64 build with AES disabled.
The changed portable vectors and AES short-terminal fallback vectors pin the
new law; AES accumulation and cryptographic digests are unchanged.
