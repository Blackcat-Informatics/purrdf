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
## Native immutable prefix plans

The existing Core `ir_layout` benchmark's `word_prefix_plan` group measures the
settled snapshot owner on 128 and 4,096 distinct digests from
`BLAKE3(u64_le(index))`. The original scoped alias owner supplies the real
presentation. Fixtures, alias assignment and source sorting for the explicitly
sorted constructor are outside timed construction. Construction includes the
fallible index allocation, actual input validation, deduplication, neighbour
minimums and destruction. Resolution reuses one immutable plan and previously
shortened typed references in original input order, with the original presentation.

The group uses 20 samples, 100 ms warm-up and 300 ms measurement under the same
managed benchmark profile and host described above. The initial run's load
averages were 3.92, 7.22 and 10.45. Build/sorted/pressure observations come from
the initial run; resolution was repeated after moving report-label formatting
outside its allocation window. No shipping source changed for that correction.

```text
cargo bench --locked -p purrdf-core --bench ir_layout -- word_prefix_plan
cargo bench --locked -p purrdf-core --bench ir_layout -- word_prefix_plan/resolve/
```

| Native operation | 128 identities | 4,096 identities |
|---|---:|---:|
| Build from original input order | 3.033 µs | 317.8 µs |
| Build from validated sorted input | 1.025 µs | 30.73 µs |
| Resolve every typed reference | 5.379 µs | 687.6 µs |
| Resolve cost per reference in that pass | about 42.0 ns | about 167.9 ns |
| Touch 16 MiB, then resolve every reference | — | 921.4 µs |

The pressure case writes one byte per 64-byte cache line before the entire
resolution pass. Its measured region includes that work; it establishes neither
an isolated hardware-cold query latency nor a DRAM-cold guarantee. Warm runs
reuse the same heap index and references. These remain shared-host observations,
without affinity, frequency pinning or an exclusive-machine guarantee.

For these concrete datasets, both uniform plans require two payload words plus
one check. The 128-entry individual histogram is 126 one-word payloads and two
two-word payloads. The 4,096-entry histogram is 1,412 one-word payloads and 2,684
two-word payloads. These are exact observations of those samples, never a
dataset-size rule or probability-based choice.

The existing counting allocator measures construction and a complete repeated
resolution pass separately, with labels and fixtures outside each window:

| Allocation observation | 128 identities | 4,096 identities |
|---|---:|---:|
| Construction allocations | 6 | 11 |
| Total construction bytes allocated across growth | 10,080 | 327,520 |
| Peak live construction bytes | 7,680 | 245,760 |
| Repeated resolution allocations/bytes | 0 / 0 | 0 / 0 |

These counts cover the native prefix owner's calls, excluding the pre-existing
alias owner, input corpus, output formatting and whole-process CLI allocation.
The static dictionary/table footprint remains the shared codec's 51,303 logical
bytes above; the scoped owner adds its dynamic digest/length index and no second
word dictionary or tokenizer state.

## Actual scoped CLI operations

The existing CLI `pack_input` benchmark's `word_ids/cli_prefix` selections use
256 digests from the same deterministic draw above, an actual `PURRAL01` alias
export and its retained real revision. Setup writes the native fixture manifest,
canonical hex input and checked prefixes outside timing. Single operations use
the first member. Every timed process restores the original manifest, checks
the expected scope/revision and builds one prefix plan; buffered batches reuse
that plan across all records. Shortening explicitly selects per-entry lengths.

```text
cargo bench --locked -p purrdf-cli --bench pack_input -- word_ids/cli_prefix
```

| Actual scoped CLI operation | Median | Measured unit |
|---|---:|---|
| Build/persist plan | 1.988 ms | one process, 256 input digests |
| Shorten | 1.840 ms | one process, one member |
| Expand | 1.819 ms | one process, one checked prefix |
| Shorten batch | 2.137 ms | one process, 256 members |
| Expand batch | 1.972 ms | one process, 256 checked prefixes |

These five cases passed using the settled CLI artifact already exercised by its
integration case under the same benchmark profile. The group retains 20 samples,
100 ms warm-up and 300 ms measurement. The host's load averages before this
build/run were 5.81, 7.44 and 10.22. Process startup, strict parsing, native
restore/planning, buffered file I/O, exact output-path preflight and process exit
are included. File pages and metadata are warm from repeated runs. The plan case
also includes the original admitted input collection, one alias extension,
manifest serialization and its exact summary on stderr. These measurements do
not isolate filesystem-cold startup or claim whole-process allocation counts.
