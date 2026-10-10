# Issue #499: Entailment: prepare schema-invariant cardinality contradictions once per class

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

# Entailment: prepare schema-invariant cardinality contradictions once per class

## Problem and result

PurRDF already memoizes parsed concepts and expressions. Its OWL DL cardinality execution in `crates/entail/src/owl_dl/hyper.rs` is branch/node dependent; it does not provide a class-bound front end that prepares schema-invariant restriction contradictions once and instantiates their support for actual individuals. Repeated instances should share this preparation and avoid repeated no-clash proof trees.

Adapt the owned GMEOW extraction, restriction closure and bound-evidence algorithms into the existing entailment compiler/checker. A class with incompatible restrictions may be empty without making its ontology inconsistent. Report class unsatisfiability separately; ontology inconsistency additionally requires the exact supported ABox assertion or entailed existence. Do not infer existence from a class name.

## Source evidence

- [Class-bound contradiction preparation](https://github.com/Blackcat-Informatics/gmeow-ontology/commit/4e5196fd03789322592e78e65163e82012884624), `reason/refute/counting.rs`: extract/close restrictions and cache schema bounds, then attach the actual individual's type support only for a contradiction.
- [Restriction-entry memoization](https://github.com/Blackcat-Informatics/gmeow-ontology/commit/5d43eb77fb87ab53f39fea4fe2f5971cac0aa6d3): shared class-entry preparation.

Historical donor timings and receipt sizes are not PurRDF performance claims. Measure the transferred implementation against an equivalent uncached PurRDF path.

## Acceptance

- [ ] N individuals sharing one schema class perform one schema-bound preparation. Clash-free individuals do not allocate repeated bound-proof trees; expose deterministic preparation/work/allocation controls to verify this.
- [ ] Actual contradictions carry exact qualified restrictions and all required source premises, plus current type/existence support. Every emitted proof passes an independent checker and agrees with the uncached verdict/proof path.
- [ ] Cover qualified object/data restrictions, equivalent/subsumed classes, unrelated qualifiers and exact numeric bounds. Do not transfer a bound across an incompatible qualifier or unsupported datatype approximation.
- [ ] An unsatisfiable uninstantiated class remains ontology-consistent; asserting an instance or proving supported existence produces the corresponding contradiction. Test absent instances and alternate existence/type derivations independently.
- [ ] Equality merges, nominals, successor counts and branch state retain individual/branch checks. Never cache those facts by class alone; cover changes that make apparent class-level answers unsound.
- [ ] Bind preparation to selected ontology/program/revision and every restriction/qualifier source. Source changes, retractions and purges invalidate dependent preparation; current ABox/existence evidence is attached at use time.
- [ ] Budget-truncated analysis retains typed incomplete/obstruction evidence and is never cached as an exhaustive no-clash answer. Account retained evidence, interned preparation and temporary products; release all pins on drop.
- [ ] Compare exact O3 time, allocations and peak retained bytes as both restriction count and shared-instance count grow. Native and actual packaged WASM verdict/proof parity, interruption and refusal controls pass.
- [ ] Keep one entailment engine and checker; no GMEOW vocabulary, global cache, semantic Cargo features or new shipping dependency. Qualify affected conformance and required full gates.

## Dependencies and boundaries

Coordinate with #476's OWL consistency correctness and exact cardinality/numeric prerequisites; preserve its four named divergence controls. This optimization does not replace those corrections or role-chain completeness in #452. It is independent of full incremental maintenance in #493. If later bound to incremental sessions, use their existing revision/support/accounting owner instead of adding a competing lifetime architecture.


## Comments (0)

