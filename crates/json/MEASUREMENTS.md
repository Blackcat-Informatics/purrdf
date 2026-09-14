<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Ordered JSON codec measurements

Measured on an x86-64 Linux host on 2026-09-14 with
`rustc 1.100.0-nightly (4b6d04e70 2026-09-13)`, using the workspace's optimized
`dev` profile with debug assertions and overflow checks enabled. Other build
work shared the host, so elapsed times are observations, not speed guarantees.
The shipped benchmarks contain no timing assertions.

## Correctness surface

The frozen corpus contains 19 documents. Every document crosses production
Turtle with absolute IRIs, Turtle with an explicit document base, N-Triples and
JSON-LD: 76 serialization/reparse/byte-reconstruction crossings. Its complete
framed RDF output SHA-256 is
`580633f8ca53cbdca3045ec708a9b9acf9631ff007457fb8878ab93454eee0fc`.
The native suite passes 23 tests plus the public example doctest. The same
integration tests and output golden are wired into `make wasm-test`.

## Forward parser scaling

Run `cargo bench -p purrdf-json --bench ordered_json_corpus --profile dev`
without corpus paths. Each input uses the committed deterministic generator;
the measurement runs 100 full analyses, including occurrence paths and identity.

| Rows | Source bytes | Total microseconds, 100 analyses |
| ---: | ---: | ---: |
| 100 | 13,899 | 12,840 |
| 1,000 | 140,799 | 161,613 |
| 10,000 | 1,427,799 | 1,422,635 |

The scan advances through source bytes once, with a forward complement pass.
Stored pointer construction costs the aggregate pointer length, bounded by the
profile. There is no repeated character count from the beginning of the source.

## Allocation probe

Run `cargo bench -p purrdf-json --bench ordered_json_alloc --profile dev`.
The allocator probe executes separately from the latency benchmark. Below is
the 1,000-row, 140,799-byte input; requested bytes include temporary allocations,
while retained and peak bytes are measured relative to each operation's entry.

| Operation | Allocations | Requested bytes | Retained bytes | Peak working bytes |
| --- | ---: | ---: | ---: | ---: |
| Analyze | 12,040 | 2,818,592 | 1,507,529 | 1,507,815 |
| Project | 244,164 | 47,602,291 | 7,533,214 | 14,843,936 |
| Decode | 96,099 | 30,851,389 | 140,823 | 16,100,758 |

The occurrence model occupies 72 bytes on this host. The input produces 12,002
value occurrences, 8,001 structure runs and 160,030 RDF statements. Absolute
N-Triples occupies 31,671,755 bytes. This deliberately dense small-value fixture
is not representative of an AST's average scalar or path length.

## Real-file corpus and RDF expansion

The native corpus harness read 367 local JSON files totaling 20,270,623 bytes,
including Rust ASTs and configuration data. All 367 were accepted, none refused,
and no symlinks were skipped. Every file passed production Turtle serialization,
reparse and byte-identical decode. The input corpus is not shipped; its content
fingerprint is `2b59b528ce957dfa43213137e11e7746212f446f60fca9c489a17ee009f7fa5e`.
This is SHA-256 over sorted `(SHA-256(source bytes), u64 little-endian length)`
records, computed from accepted bytes during the run, independent of filenames.

Run `cargo bench -p purrdf-json --bench ordered_json_corpus --profile dev --
/path/to/corpus`. The harness fails on I/O errors, an empty accepted set or any
round-trip mismatch, and reports individual refusals plus separate counts.

| Representation of the same graph | Turtle bytes | Source expansion |
| --- | ---: | ---: |
| Absolute IRIs | 1,753,595,985 | 86.51× |
| Explicit document base and relative IRIs | 562,875,820 | 27.77× |

The graph contains 611,269 value occurrences, 533,007 structure runs and
8,779,560 statements. The explicit base reduces Turtle bytes by 67.90% while
retaining every statement and full 256-bit document identity. The production
serializer factors the common document base into `@base`; this measurement
does not depend on custom Turtle generation or truncating a digest. N-Triples
has no base directive and therefore retains absolute identifiers.

Aggregate accepted-input analysis took 130,284 microseconds, IR projection
8,610,704 microseconds, and verified decode 5,886,341 microseconds. These stage
timings exclude RDF serialization and RDF parsing; they are not total pipeline
latency. Scalar `text` facts contain raw JSON lexical bytes, including escapes
and number spellings. Container sizes, paths, parentage and occurrence order
are queryable independently of the verbatim structure cover.
