<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# purrdf-bench

Benchmark tooling (never published). Today: `bench-corpus`, the deterministic,
shardable scale-corpus generator. Every IRI is minted purely from its index
under a fixed seed, across five deliberately adversarial classes
(front-codable, long zero-padded numeric, raw-Han Chinese, host-scattered
irregular, very-long), so no single dictionary trick can flatter a capacity
claim. Shard `k` of `n` emits exactly its slice; concatenating all shards is
byte-identical to one whole run, and a golden digest test pins the profile
(`purrdf-scale-mixed-v1`).

Two independent mixes shape the output, and `--manifest` reports both:

* `class_mix_per_mille` — the shape of an entity IRI, over the **entity
  space**: plain 400, numeric-long 200, chinese 200, irregular 150,
  very-long 50.
* `row_mix_per_mille` — the shape of a row, over the **emitted rows**:
  entity-edge 500, plain-literal 150, zh-literal 100, typed-literal 100,
  long-text-literal 50, reified 60, blank-node 40.

Term-kind coverage is therefore a property of the profile rather than an
accident of it: the corpus always contains RDF 1.2 reifier rows binding triple
terms, `xsd:`-typed literals with lexical forms valid for their datatype,
language-tagged literals, and blank nodes in both subject and object position.
One slot emits exactly one line for every row kind, so `lines == --quads`
holds unconditionally and shard stitching stays byte-exact.

```sh
cargo run -p purrdf-bench --release -- --quads 1000000 --iris 100000 --out corpus.nq
cargo run -p purrdf-bench -- --quads 1000000 --iris 100000 --manifest
# shard 3 of 16:
cargo run -p purrdf-bench --release -- --quads 1000000 --iris 100000 --shard 3 --shards 16
```

The comparison-workload drivers (externally specified suites) will join this
crate once their upstream artifacts clear the acquisition/licensing review.
