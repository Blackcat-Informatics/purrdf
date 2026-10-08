# Issue #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

`purrdf rules --srl` uses memory and time that grow **quadratically** with the data when a rule body has two `rdf:type` patterns whose subjects share no variable, even when the other patterns in the body connect them. Remove one of the two `rdf:type` patterns and the same rule runs in linear time and memory.

It looks as if the two type patterns are joined first, producing every Vault × every Spool, and only then filtered by `k:target`, which actually links them.

Found by Katamari while measuring placement admissibility as SPARQL 1.2 RL rules with `--explain` proofs. An unbounded run at 100,000 vaults used all memory on a 128 GB machine.

## Environment

- `purrdf --version`: `purrdf 3.0.1` (release build of the CLI, built 2026-10-05)
- Local checkout at `dfc0c21ad`
- Linux x86_64

## Reproduction

`gen.py` writes N vaults. Each vault has exactly three targets, one of them a spool, so the work per vault is constant:

```python
import sys
n = int(sys.argv[1])
print("@prefix k: <https://example.invalid/k#> .")
for i in range(n):
    print(f"k:v{i} a k:Vault ; k:target k:v{i}a, k:v{i}b, k:v{i}c .")
    print(f"k:v{i}a a k:Store .")
    print(f"k:v{i}b a k:Store .")
    print(f"k:v{i}c a k:Spool .")
```

`single.srl` has two `rdf:type` patterns joined through `k:target`:

```sparql
PREFIX k: <https://example.invalid/k#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
RULE { ?v k:hasSpool ?s } WHERE {
  ?v rdf:type k:Vault ; k:target ?s . ?s rdf:type k:Spool .
}
```

`notype.srl` is the same rule without `?v rdf:type k:Vault`:

```sparql
PREFIX k: <https://example.invalid/k#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
RULE { ?v k:hasSpool ?s } WHERE {
  ?v k:target ?s . ?s rdf:type k:Spool .
}
```

`pair.srl` has three type patterns, two of them on paired variables. This was the original case:

```sparql
PREFIX k: <https://example.invalid/k#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
RULE { ?v k:hasTwoSpools true } WHERE {
  ?v rdf:type k:Vault ; k:target ?s1, ?s2 .
  ?s1 rdf:type k:Spool . ?s2 rdf:type k:Spool .
  FILTER(?s1 != ?s2)
}
```

Run with:

```sh
python3 gen.py 4000 > d4000.ttl
purrdf rules --srl single.srl --max-join-steps 18446744073709551615 d4000.ttl out.nt
```

`--max-join-steps` is raised so that the default ceiling does not stop the run before the growth shows. The memory figures below come from running under a 4 GB address-space limit (`RLIMIT_AS`) and reading peak RSS with `getrusage(RUSAGE_CHILDREN)`.

## Measurements

| vaults | rule | time | peak RSS | result |
|---:|---|---:|---:|---|
| 1,000 | `single` (2 type patterns) | 0.40 s | 243 MB | ok |
| 2,000 | `single` | 1.56 s | 912 MB | ok |
| 4,000 | `single` | 5.88 s | 3,579 MB | ok |
| 1,000 | `pair` (3 type patterns) | 1.18 s | 304 MB | ok |
| 2,000 | `pair` | 4.30 s | 1,157 MB | ok |
| 4,000 | `pair` | — | > 4 GB | aborted by the 4 GB limit |
| 1,000 | `notype` (1 type pattern) | 0.07 s | 23 MB | ok |
| 4,000 | `notype` | 0.12 s | 61 MB | ok |
| 20,000 | `notype` | 0.77 s | 286 MB | ok, 20,000 inferred |

Each time the input doubles, peak memory for `single` and `pair` grows about 4×, which is quadratic. For `notype` it grows about 2×, which is linear.

## Expected

Join order should follow the shared variables, or selectivity, rather than putting two disconnected `rdf:type` patterns next to each other. With `?v` bound first and each vault reaching three targets, all three rules should be linear in the number of vaults.

## Related

Possibly worth a separate issue: the default `--max-join-steps` ceiling (1,048,576) can be raised but not removed, and when it is reached the run fails with no partial result. In Katamari's highest-assurance profile, chosen limits are not permitted, only physical ones.


## Comments (1)

### paudley — 2026-10-07T22:56:54Z

## Likely root cause: `sips_order` has no connectivity rule

Reading `crates/datalog/src/plan.rs` at `6273b6173`, the acyclic path (`plan.rs:1080-1082`, taken whenever `certified_cyclic_components` finds no cycle, which is the case for all three rules in this issue) orders the positive body with `sips_order` (`plan.rs:358-387`). The ranking key for each remaining atom is:

```rust
(known, constants, repeated, usize::MAX - positive_position)
```

- `known` counts positions that are a constant **or** an already-bound variable (`term_is_known`, `plan.rs:313-318`). `ClauseTerm::DefaultGraph` counts as known.
- `constants` counts positions that are not variables, so the default-graph slot counts here too.
- Nothing in the key asks whether the atom **shares a variable** with what is already bound, and no cardinality is consulted. The doc comment says so on purpose (`plan.rs:345-346`: "With no store in hand, cardinalities cannot be consulted soundly").

Tracing `single.srl` (body: `T(?v, rdf:type, k:Vault, G)`, `T(?v, k:target, ?s, G)`, `T(?s, rdf:type, k:Spool, G)`):

| step | bound | `?v a k:Vault` | `?v k:target ?s` | `?s a k:Spool` | chosen |
|---|---|---|---|---|---|
| 1 | {} | (3, 3) | (2, 2) | (3, 3) | `?v a k:Vault` (tie, authored position) |
| 2 | {?v} | — | (3, 2) | (3, 3) | **`?s a k:Spool`** — wins on constants |
| 3 | {?v, ?s} | — | (4, 2) | — | `?v k:target ?s` (now a bound-bound probe) |

At step 2 the disconnected `?s a k:Spool` ties the connected `?v k:target ?s` on `known` and beats it on `constants`, so the plan enumerates every Vault x every Spool and only then probes `k:target`. That is the quadratic growth in the measurements. `pair.srl` does it twice (after `?s1` is bound, `?s2 a k:Spool` beats `?v k:target ?s2` the same way). `notype.srl` has only one type atom, so the connected atom is chosen and the run is linear, as measured.

A smaller observation from the same trace: because every atom carries the graph slot and `DefaultGraph` counts as both known and constant, every score is inflated by one uniformly. It does not change the order here, but it means the key is measuring the quad carrier as well as the triple.

### Suggested direction (a sketch)

1. **Connectivity before constants.** Once any variable is bound, only atoms that share a bound variable are eligible; a new connected component is started only when none remain. This alone makes all three rules in this issue linear and needs no store.
2. **Never materialize a cross product between components.** When a body genuinely has disconnected components, evaluate each component separately and combine them only at the head (factorized), so the join-step meter and memory see `|A| + |B|`, not `|A| x |B|`, unless the head actually needs the product.
3. **Cardinality as a tie-break, decided at round start.** Relation sizes per predicate (and per `rdf:type` object) are known at the start of each semi-naive round. Using them only to break ties among connected candidates keeps the plan deterministic for a given store and cannot reintroduce a cross product. If the plan must stay store-independent, the plan can carry several connected orders and pick one per round from the counts.

### Acceptance criteria

- `single.srl`, `pair.srl` and `notype.srl` from this issue are linear in time and peak memory from 1,000 to at least 100,000 vaults, under the **default** join-step limit.
- A unit test on `sips_order` (or its replacement): for a body `A(?v)`, `E(?v, ?s)`, `B(?s)` with constants in `A` and `B`, the second atom chosen is `E`.
- A body with two genuinely disconnected components and a head that uses only one of them does not enumerate their product (join steps grow additively).
- Proof output (`--explain`) is unchanged in content: the order of evaluation is not part of a derivation's identity, or if it is, the change is recorded.

### Why Katamari cares

Katamari plans to evaluate its derived views ("current claims" at given coordinates) as SPARQL 1.2 RL rule sets over every claim's reifier, roughly 300 million claims in one mail vault and decades of an AI's memory in another. Almost every such rule pairs a typed entity with a typed claim through a linking predicate, which is exactly the `single.srl` shape. At that scale a cross product is not a slowdown but an out-of-memory; this is the first item on our list of purrdf needs, and #364's join-step and term-arena items are the second.

### Related

#483 (the same safety property for the SPARQL planner), #484 (statistics the tie-break could use), #364 and #488 (removable limits, certified partial results).


