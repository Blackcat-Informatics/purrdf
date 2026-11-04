<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Checked word identity measurements

Measurements on November 4, 2026 used an AMD Ryzen AI Max+ 395 host with 32
logical CPUs and `rustc 1.101.0-nightly (21b707e3f 2026-09-30)`. The repository
benchmark profile inherits release optimization level three, fat LTO and one
code generation unit, retaining symbols. The managed build used
`-C target-cpu=native` and warnings as errors.

These are bounded shared-host observations, with no affinity/frequency pinning
or exclusive-machine guarantee. Immediately before the settled native run,
there was no active `rustc` process; load averages were 6.55, 11.96 and 10.97.
The earlier lookup comparison started at lower host load. Results support the
retained implementation choice on these inputs; they establish neither
mathematical optimality nor superiority over hex.

## Native conversion

The existing `purrdf-hash-conformance` `digests` benchmark has one `word_ids`
group, using 20 samples, 100 ms warm-up and 300 ms measurement. The initial
group compared both lookup candidates and the scalar conversion paths. After
selecting the shipping perfect hash, only the four affected decode cases were
repeated:

```text
cargo bench --locked -p purrdf-hash-conformance --bench digests -- word_ids
cargo bench --locked -p purrdf-hash-conformance --bench digests -- word_ids/decode
```

Single conversion uses `Digest32::new([0x5a; 32])`. Batch conversion uses 256
different digests from `BLAKE3(u64_le(index))`, indices zero through 255. The
invalid input changes the last word's final byte to `z`; fixture admission
verifies refusal. Setup, owned input strings and callback work outside the
shown conversion are excluded from timed conversion. Timed batch callbacks
only consume the borrowed result through `black_box`.

| Native operation | Median | Measured unit |
|---|---:|---|
| Encode into caller buffer | 118.5 ns | one full ID |
| Encode owned text | 128.5 ns | one full ID including allocation |
| Encode batch | 24.47 µs | 256 IDs, about 95.6 ns each |
| Settled full decode | 400.5 ns | one full ID |
| Settled invalid decode | 336.6 ns | one late unknown-word refusal |
| Settled varied batch decode | 127.4 µs | 256 IDs, about 497.7 ns each |
| Cache pressure plus settled decode | 188.6 µs | touch 16 MiB, then decode one ID |

The cache-pressure case writes one byte per 64-byte line across a reusable
16 MiB allocation before decoding. Its measured region includes that pressure
work. It is neither an isolated hardware-cold decoder latency nor a DRAM-cold
guarantee. The pressure observation varied substantially between executions;
no lookup speed claim is inferred from it.

## Actual CLI conversion

The existing CLI `pack_input` benchmark's `word_ids` group runs the built
binary with buffered input/output files, using the same 20-sample timing
configuration. Process startup, argument parsing, file-identity inspection,
buffered I/O and exit are included. Input/output metadata and file pages are
warm from repeated runs. The invalid single input uses an unsupported header
and requires exit status one. Batch inputs use the same 256 varied digests as
the native benchmark. The five measured operations are:

```text
cargo bench --locked -p purrdf-cli --bench pack_input -- word_ids
```

| Actual CLI operation | Median | Measured unit |
|---|---:|---|
| Encode | 1.650 ms | one process, one ID |
| Decode | 1.604 ms | one process, one ID |
| Invalid decode | 1.580 ms | one process, one refusal |
| Encode batch | 2.118 ms | one process, 256 IDs |
| Decode batch | 1.941 ms | one process, 256 IDs |

The host load averages before the settled CLI build/run were 5.02, 7.57 and
10.54. The native consumer fixture links finished during compilation, before
the timed CLI process runs. These measurements include process and filesystem
costs; the native conversion table isolates the corresponding library calls.

## Lookup choice and footprint

Both host candidates use the same contiguous word bytes and ordered 4,096-word
input. Invalid inputs append `z` to each word. Both validate complete candidate
bytes. The comparison uses the existing benchmark harness in the same run:

| Candidate | Valid 4,096-word pass | Invalid 4,096-word pass | Reverse metadata |
|---|---:|---:|---:|
| Displaced perfect hash | 28.04 µs | 25.22 µs | 12,288 bytes |
| Two-letter buckets and binary search | 63.33 µs | 70.16 µs | 1,354 bytes |

The measured time/space tradeoff selected the perfect hash. Its shipping data
contains 30,821 word bytes, 8,194 offset bytes, 4,096 displacement bytes and
8,192 slot bytes: 51,303 logical static bytes, before linker alignment.
There is one runtime lookup path. Both candidates remain reproducible in the
host generator/benchmark tooling.

The shared counting allocator's existing Hash allocation target measures the
actual native entry points. Encoding/decoding into caller storage and exact
length calculation made zero allocations. Owned full text made exactly one
allocation. These are library-call counts, excluding fixture setup, and are
not whole-process CLI allocation claims.

## Matched native binary impact

Two small retained Rust consumers both read a canonical hex argument, render
text, verify the round trip and write stdout. The baseline renders/reads hex;
the other renders/reads the checked full word form. Both link the same native
Hash artifact and use these identical managed compiler flags:

```text
--edition 2024 -C opt-level=3 -C lto=fat -C codegen-units=1
-C strip=symbols -C target-cpu=native -D warnings
```

This nightly's separate metadata format requires both the `.rlib` and full
`.rmeta` paths for the same dependency, obtained from Cargo's artifact output.
No compiler or cache data was copied or changed.

| Matched round-trip consumer | Stripped executable bytes |
|---|---:|
| Hex | 303,272 |
| Full words | 367,464 |

The 64,192-byte delta covers the linked word tables, codec, validation and
diagnostic glue in this matched application. Its scope is these native
consumers, with identical standard I/O and optimization settings.

## Serialized reference token costs

The alphabet provenance gives the exact pinned vocabularies, independent
source vectors and ASCII pretokenizer equivalence boundary. Offline analysis
serializes 4,096 actual native references at every payload count one through
22, with headers, separators, fresh checks and full padding included.

| Form | Mean bytes | `cl100k_base` mean tokens | `o200k_base` mean tokens |
|---|---:|---:|---:|
| One payload word plus prefix check | 21.996338 | 8.312256 | 8.199219 |
| Full 22 payload words plus check | 200.310303 | 43.087646 | 41.814209 |

Full-form observed token ranges were 32–56 and 32–52 respectively. These are
sample extrema, not universal bounds. All 22 measured rows and individual
word costs are retained in `crates/hash-conformance/data/word-id/`.
