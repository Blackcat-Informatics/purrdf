<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# purrdf-bench

Benchmark tooling (never published): `bench-corpus`, the deterministic,
shardable scale-corpus generator. Every IRI is minted purely from its index
under a fixed seed, across five deliberately adversarial classes
(front-codable, long zero-padded numeric, raw-Han Chinese, host-scattered
irregular, very-long), so no single dictionary trick can flatter a capacity
claim. Shard `k` of `n` emits exactly its slice; concatenating all shards is
byte-identical to one whole run, and a golden digest test pins the profile
(`purrdf-scale-mixed-v1`).

Two independent mixes shape the output, and `--manifest` reports both:

* `entity_class_mix_per_mille` — the shape of an entity IRI, over the **entity
  index space**: plain 400, numeric-long 200, chinese 200, irregular 150,
  very-long 50. The key names its basis because the class of an entity is a
  function of the entity *index*, and slots draw indexes under a skew, so the
  class distribution measured over rows is a different number.
* `row_mix_per_mille` — the shape of a row, over the **emitted rows**:
  entity-edge 500, plain-literal 150, zh-literal 100, typed-literal 100,
  long-text-literal 50, reified 60, blank-node 40.

The manifest records; it digests nothing. A capture is the manifest **plus** a
digest of the bytes that were actually consumed. It also carries
`emitted_lines`, this shard's exact line count (`end - start` of the shard
range, so it is free at any scale), and `iris` — which is the **entity index
space, a target**, not an achieved distinct-entity count. The distinct entities
a run actually names is at most `min(iris, emitted_lines)` and in practice far
lower; nothing here reports it, because establishing it means enumerating the
corpus, which is exactly what streaming at full scale exists to avoid. A
capacity claim must name which of the two numbers it uses.

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

## The lane that drives it

`scripts/scale-corpus.sh` (`make scale-corpus`) runs the shards and is
documented in [`docs/BENCHMARKS.md`](../../docs/BENCHMARKS.md), which owns the
parameters, the manifest-plus-digest capture rule and the storage arithmetic.

Index-pure minting means shard `k` needs no coordination with shard `j` — no
shared dictionary, no ordering barrier, no merge — so shards are independent
processes at any scale. That is a property of the algorithm, and it is not the
same as a claim that any particular run has been performed: density is a
function of the entity index space (measured **177.4 bytes per row** at a
10^6-entity space, rising to **182.8 bytes per row** at a 10^10-entity space),
so 10^10 rows over a 10^10-entity index space is ~1.83 TB, not the flat 1.77 TB
a smaller-scale density would suggest — see the density table in
`docs/BENCHMARKS.md` for the full measurement. A run that size is streamed
into whatever consumes it, not stored, which is why the lane's default mode
keeps nothing and writing shard files is opt-in.

## The other deterministic generator

There are two deterministic corpus generators in this repository and they own
different regimes. The keystone fixture generator (`purrdf_rdf::gts_fixtures`,
driven by [`purrdf-envelope-probe`](../envelope-probe/)) is small, fixed and
statement-layer-focused: it makes the micro-hardware envelope's ceilings
meaningful on a constrained deployment. This profile is the opposite regime —
shardable, anti-compressible, and parameterized for capacity at scale. Neither
substitutes for the other.
