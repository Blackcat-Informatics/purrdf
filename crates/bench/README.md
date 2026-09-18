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

```sh
cargo run -p purrdf-bench --release -- --quads 1000000 --iris 100000 --out corpus.nq
cargo run -p purrdf-bench -- --quads 1000000 --iris 100000 --manifest
# shard 3 of 16:
cargo run -p purrdf-bench --release -- --quads 1000000 --iris 100000 --shard 3 --shards 16
```

The comparison-workload drivers (externally specified suites) will join this
crate once their upstream artifacts clear the acquisition/licensing review.
