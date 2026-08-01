<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-entail` — Native Entailment Engines

[![crates.io](https://img.shields.io/crates/v/purrdf-entail.svg)](https://crates.io/crates/purrdf-entail)
[![docs.rs](https://docs.rs/purrdf-entail/badge.svg)](https://docs.rs/purrdf-entail)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-entail` is native, `wasm32`-clean entailment for the PurRDF
[`RdfDataset`](https://docs.rs/purrdf-core) IR. A family of engines sits behind
one façade, each the right tool for its SPARQL entailment regime — closing a
dataset to its inferred fixpoint entirely in interned `TermId` space, with **no**
external reasoner, no `tokio`, and no string round-trip.

## Surface Map

| Entry point | Regime(s) | Engine |
| --- | --- | --- |
| `materialize(ds, plan)` | **all seven** | Forward materialization ("chase") of `calculus_program(regime)` through `purrdf-datalog`'s native semi-naive fixpoint — the declared rule set *is* the executable, so the contract hash a report carries names the clauses that ran. Returns `(closure, ReasoningReport)` — the report is not optional. `plan` is a `Materialization`, which carries each regime's own input, so the function is TOTAL: `OwlDirect(&[QTriple])` and `Rif(&RuleSet)` delegate to the two entry points below rather than being refused. |
| `materialize_dl_reported(ds, bgp)` | `OWL-Direct` | Open-world OWL DL over a SHOIQ(D) hypertableau, directed by the query's class expressions — what `materialize(ds, Materialization::OwlDirect(bgp))` delegates to. Answers a BGP whose variables are all distinguished; a query blank node is a non-distinguished variable and raises the `NonDistinguishedVariable` boundary rather than being answered incompletely in silence. |
| `Reasoner::new(ds)` | `OWL-Direct` | The Description-Logic services — consistency, class satisfiability, classification, realization, instance retrieval and axiom entailment. Each answer arrives as a `Certified<T>` carrying a `DlCertificate`: the DL lane's own completeness notion, which reports both the constructs the reverse mapping could not read and a search that ran out of deterministic steps. |
| `extract_module(ds, signature, method)` | — | Syntactic locality module extraction (`BOT` / `TOP` / `STAR`). Sound, not minimal: a construct whose locality is not decided exactly is kept conservatively and the keep is reported. |
| `profile(ds)` | — | OWL 2 profile certification: which of EL, QL, RL, DL and Full the ontology is *provably* in, with a violation list. A certification proves membership; a violation proves only that the cheap structural condition failed. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `parse_rif_xml(...)` / `resolve_rif_imports(...)` | `RIF` | Normative RIF-XML parsing with caller-owned, I/O-free import resolution. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |
| `rules(regime)` / `implemented(regime)` | — | The rule table a regime is *defined by*, and the subset this crate fires. Their difference is the measurable gap. |
| `calculus_program(regime)` | — | The regime's calculus as DL-clause data — the very program `materialize` evaluates, so its `purrdf-datalog` contract hash is recomputable by a consumer. |

**There is no unsupported-regime error.** `materialize` takes a `Materialization`,
not a `Regime`, and a `Materialization` carries what its regime is defined by — a
basic graph pattern for `OWL-Direct`, a `RuleSet` for `RIF`. All seven inhabitants
are served, so a caller cannot hand the function a value it accepts and get a
refusal instead of an answer. `Regime` remains the *reporting and identity* type:
what `ReasoningReport::regime()` names, what `rules()`/`implemented()` are indexed
by, and what `Regime::from_iri` parses a `sparql:entailmentRegime` IRI into.
`Materialization::regime()` is the map from the input to the identity.

## Rule coverage

The rule tables are data, not prose. `rules(regime)` is what the specification
defines the regime by; `implemented(regime)` is what the chase fires. The
difference is the gap, and it is also what a `ReasoningReport` reports as
`missing`:

| Regime | Rule table | Defined | Implemented |
| --- | --- | ---: | ---: |
| `Simple` | — (identity closure) | 0 | 0 |
| `RDF` | RDF 1.2 Semantics §8.1.1 | 3 | 3 |
| `RDFS` | RDF 1.2 Semantics §8.1.1 + §9.2.1 | 18 | 18 |
| `OWL-RL` | OWL 2 Profiles §4.3 Tables 4–9 | 78 | 78 |
| `D` | OWL 2 Profiles §4.3 Table 8 | 5 | 5 |
| `OWL-Direct` | — (SHOIQ(D) hypertableau, not a fixed table) | 0 | 0 |
| `RIF` | — (caller-supplied rule set) | 0 | 0 |

The per-rule table is generated from this crate's own API and drift-guarded:
[`docs/book/src/entailment-rules.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/book/src/entailment-rules.md).

Neither column counts an **extension** — a rule this crate fires that no
specification table states. `OWL-RL` has one, `ext-eq-diff-sym` (symmetry of
`owl:differentFrom`, sound and shaped exactly like `prp-symp`); every other regime
has none. It is in neither `rules()` nor `implemented()` for any regime, because
those name specification rules and adding a sound rule the table omits does not
change what the table says. Ask `extensions(regime)` for the list, or read the
`extension` lines of any report: a caller who must act only on normative
conclusions can see exactly what to discount.

Three bounds are stated rather than papered over:

* **The four existential rules fire, and their conclusions are withheld.**
  `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` each conclude about a *fresh* blank
  node. All four run, through `purrdf-datalog`'s restricted chase, which mints
  each surrogate as a frontier-addressed Skolem witness — and every conclusion
  mentioning one is withheld at the materialization boundary, because a SPARQL
  entailment regime draws its answers from the scoping graph and a minted blank
  node is not in it. The report says so with a `Construct::Surrogate` boundary
  rather than with a missing rule.
* **A complete rule table is not a complete closure.** `OWL-RL` fires all 78
  rules, and a report still says `ExactWithinBoundaries` rather than `Exact`
  whenever the run met a `Boundary` (an infinite datatype value space, for
  instance). `D` is realized as Simple entailment plus the five `dt-*` rules,
  which is the part of D-entailment a forward chase can produce; the value
  spaces themselves are reported as `Construct::DatatypeValueSpace`.
* **A complete rule table is not entailment conformance either.** 78 / 78 is
  *rule-table coverage*; the two are measured separately, and on W3C's own OWL 2
  RL entailment tests this chase scores **26 of 27 positive and 23 of 23
  negative**, the latter meaning no unsoundness was found. Both numbers are true
  and stating only the first is the overclaim the reasoning report exists to
  prevent. See
  [`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).

Seventeen of the 78 OWL 2 RL rules conclude `false` rather than a triple.
"Implemented" for those means *decided*: a body match becomes
`EntailError::Inconsistent` carrying an `InconsistencyWitness` — the only thing a
rule with no conclusion can do.

## Invariants

* **No minted vocabulary.** Every constant in `vocab` is a standard
  `rdf:`/`rdfs:`/`owl:` IRI drawn from the entailment spec itself — this crate
  fabricates none.
* **Every run states what it did.** `materialize` returns a `ReasoningReport`
  with every closure. It carries the regime's `Completeness` — *derived* from
  `rules(regime)` minus `implemented(regime)`, so it improves by itself as rules
  are added — which rules fired and how many conclusions each contributed, the
  `Boundary`s the run met and why, what it consumed of the evaluation ceilings,
  and the contract hash of the calculus it ran, so a cached closure minted under
  a different rule set can be refused rather than trusted. A report that claims
  `Exact` while naming a boundary is a test failure.
* **wasm-clean and dependency-lean.** Dependencies are `purrdf-core`,
  `purrdf-datalog`, `purrdf-xsd`, `roxmltree`, `blake3`, and two fixed-key
  hashers (`ahash` and `hashbrown`) — all `wasm32-unknown-unknown`-clean, so
  this crate carries into Rust, Python, WebAssembly, and C without a
  threads/filesystem/RNG dependency.
* **Determinism.** The chase is a fixpoint over the frozen IR; a given input and
  regime always yields the same closure — and the same report, byte for byte.

## The same engine in four hosts

Rust is the reference surface; Python, WebAssembly and the C ABI reach the chase
through one shared string boundary (`purrdf_validate::regime`), not through three
re-implementations. All four are checked against one committed golden-vector
artifact, so a divergence is one vector failing rather than three surfaces that
quietly stopped agreeing.

| Host | Materialize | Defined rule table | Implemented rules |
| --- | --- | --- | --- |
| Rust | `materialize(&ds, Regime::Rdfs)` | `rules(Regime::Rdfs)` | `implemented(Regime::Rdfs)` |
| Python | `purrdf.entail.materialize(dataset, "rdfs", "")` | `purrdf.entail.rules("rdfs")` | `purrdf.entail.implemented_rules("rdfs")` |
| JavaScript / WebAssembly | `entailMaterialize(doc, "rdfs", "")` | `entailRules("rdfs")` | `entailImplementedRules("rdfs")` |
| C | `purrdf_entail_materialize_to_nquads(...)` | `purrdf_entail_rules(...)` | `purrdf_entail_implemented_rules(...)` |

## Local Checks

```bash
cargo test -p purrdf-entail
# Regenerate the drift-guarded rule inventory:
cargo run -p purrdf-entail --example gen_rule_inventory
```

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::entail`; depend on `purrdf-entail` directly only when you want the
entailment engines alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
