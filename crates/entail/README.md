<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-entail` — Native Entailment Engines

[![crates.io](https://img.shields.io/crates/v/purrdf-entail.svg)](https://crates.io/crates/purrdf-entail)
[![docs.rs](https://docs.rs/purrdf-entail/badge.svg)](https://docs.rs/purrdf-entail)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
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
| `materialize_dl_reported(ds, bgp)` | `OWL-Direct` | Open-world OWL DL over the hypertableau with regular SROIQ role languages, directed by the query's class expressions — what `materialize(ds, Materialization::OwlDirect(bgp))` delegates to. Answers a BGP whose variables are all distinguished; a query blank node is a non-distinguished variable and raises the `NonDistinguishedVariable` boundary rather than being answered incompletely in silence. |
| `Reasoner::new(ds)` | `OWL-Direct` | The Description-Logic services — consistency, class satisfiability, classification, realization, instance retrieval and axiom entailment. Each answer arrives as a `Certified<T>` carrying a `DlCertificate`: the DL lane's own completeness notion, which reports both the constructs the reverse mapping could not read and a search that ran out of deterministic steps. Records no proof term: `Certified::proof()` is `None`. |
| `Reasoner::with_proofs(ds)` | `OWL-Direct` | The same services, each answer additionally carrying a `ServiceProof` a consumer replays against their own copy of the data. Opt-in because it costs an RDFC-1.0 canonicalization of the dataset, a clausification contract per call, and one instrumented tableau trace per run; the verdict and every `DlCertificate` counter are identical either way. |
| `extract_module(ds, signature, method)` | — | Syntactic locality module extraction (`BOT` / `TOP` / `STAR`). Sound, not minimal: a construct whose locality is not decided exactly is kept conservatively and the keep is reported. |
| `extract_module_with_proofs(ds, signature, method)` | — | The same extraction, carrying a proof term that binds the signature, the notion and the extracted module's own canonical identity. Its ZERO run count is a real measurement — this service opens no tableau — which is why an unrecorded answer says `None` rather than shipping an empty proof. |
| `profile(ds)` | — | OWL 2 profile certification: which of EL, QL, RL, DL and Full the ontology is *provably* in, with a violation list. A certification proves membership; a violation proves only that the cheap structural condition failed. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `parse_rif_xml(...)` / `resolve_rif_imports(...)` | `RIF` | Normative RIF-XML parsing with caller-owned, I/O-free import resolution. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |
| `rules(regime)` / `implemented(regime)` | — | The rule table a regime is *defined by*, and the subset this crate fires. Their difference is the measurable gap. |
| `calculus_program(regime)` | — | The regime's calculus as DL-clause data — the very program `materialize` evaluates, so its `purrdf-datalog` contract hash is recomputable by a consumer. |

## Prepared class cardinality bounds

`Reasoner` prepares schema-invariant cardinality contradictions with its selected
ontology. Instances sharing a class reuse that class entry. A healthy entry holds
no copied bound-proof tree. A contradictory entry retains its exact qualified
restrictions, source class implications and compatible role path; concrete bounds
use the exact native datatype range algebra. Unsupported datatype approximations
cannot certify a bound transfer.

A contradictory class can be empty. Its prepared clause fires only for a current
inhabitant: an asserted type, a supported derived type, an existential witness or
the explicit witness of a class-satisfiability question. The existing branch
engine still decides individual equality, nominals and actual successor counts.
Preparation never caches those facts or creates existence from a class name.

`schema_preparation()` reports deterministic work, successful allocation counts,
retained capacity bytes (including the original compiled guard bodies and trigger
directory) and admitted temporary peak bytes. The matcher and independent checker
borrow those guards without allocating another schema clause. Temporary peak is
an admission measurement; allocator peak is measured separately in the native
ownership and scaling fixtures. `with_preparation_budget` and
`with_proofs_and_preparation_budget` accept caller-selected work and storage
ceilings, with `None` meaning unlimited. Refusal preserves a typed
`schema_obstruction()` and leaves services undecided. A sufficient
`retry_schema_preparation` finishes incomplete entries; a stopped operation never
marks them clear. The table belongs to one source owner, so rebuilt revised,
retracted or purged inputs cannot reuse it.

Recorded prepared clashes carry a source derivation and the actual finite
current-support prefix. The checker validates each supplied step against its own
ontology and refutation assumptions, then checks the bound and inhabitant. It
does not rerun the producer's schema closure or trust its cached verdict. These
traces use the canonical v4 DL proof layout; other DL proof bytes remain v3.
Preparation-refusal receipts use service layout v3; other service bytes remain
v2. Native support recording preserves its first typed obstruction and discloses
truncation while leaving search verdicts and counters unchanged. Proofs retain
owned payloads, and a clone remains valid after its producer and checker die.

The native report-only `schema_preparation_o3_scaling` fixture measures cold
preparation, cold reasoner construction, shared-instance execution and the
equivalent uncached execution through the same algorithm. Its measurements use
the workspace release/O3 profile and counting allocator. Set `PURRDF_BENCH_HOME`
to a caller-selected writable artifact directory when running this ignored unit
fixture; it requires all 36 standard estimate records to be saved successfully.
Canonical shared vectors
exercise the production string boundary and its packaged WebAssembly checker.

**There is no unsupported-regime error.** `materialize` takes a `Materialization`,
not a `Regime`, and a `Materialization` carries what its regime is defined by — a
basic graph pattern for `OWL-Direct`, a `RuleSet` for `RIF`. All seven inhabitants
are served, so a caller cannot hand the function a value it accepts and get a
refusal instead of an answer. `Regime` remains the *reporting and identity* type:
what `ReasoningReport::regime()` names, what `rules()`/`implemented()` are indexed
by, and what `Regime::from_iri` parses a `sparql:entailmentRegime` IRI into.
`Materialization::regime()` is the map from the input to the identity.

Regular role chains use the original SROIQ automaton obligations in the
hypertableau, including its blocking labels, rather than treating finite ABox
closure as a decision procedure. Admission first checks the authored OWL 2
Structural Specification §11.2 order (literal recursive endpoints are checked
before equivalent-property normalization), then checks the mixed simple/complex
dependency condition after genuine simple equivalences are quotiented. Passing
the printed order alone does not establish a regular language: Stefanoni's
[Example 12.2 and Theorem 12.4](https://www.cs.ox.ac.uk/files/7941/paper.pdf)
give the counterexample and regular-calculus condition. A cycle through a complex
dependency refuses with `RoleHierarchyError`; ordinary simple equivalence cycles
remain legal. Inverse heads reverse the whole inclusion, and top/bottom properties
retain their fixed semantics. Native storage refusal and cancellation remain
operational causes, never a decided consistency verdict.

Semantic hierarchy refusals retain the original source terms in their typed
diagnostic presentation, so a caller can identify an offending property after
the private interner has been destroyed. `RoleHierarchyError::classification()`
borrows the original syntax/order/dependency refusal; `presentation()` exposes
the retained arguments and `Display` includes their original term spellings.
Allocation failures while constructing that witness remain typed storage errors.

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
| `OWL-Direct` | — (hypertableau with regular role languages, not a fixed table) | 0 | 0 |
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
  *rule-table coverage*; the two are measured separately, and on this vendored W3C corpus of
  OWL 2 RL entailment tests this chase scores **27 of 27 positive and 23 of 23
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
  `Boundary`s the run met and why, what it consumed against the evaluation
  limits it ran under, and the contract hash of the calculus it ran under those
  limits, so a cached closure minted under a different rule set, or under other
  limits, can be refused rather than trusted. `materialize_with` states the
  stored-fact and join-step limits (`EvalOptions`); `materialize` runs the target's
  defaults — 4,194,304 facts and 1,048,576 join steps natively, 131,072 and
  1,048,576 on `wasm32`. A report that claims
  `Exact` while naming a boundary is a test failure.
* **wasm-clean and dependency-lean.** Dependencies are `purrdf-core`,
  `purrdf-datalog`, `purrdf-xsd`, `purrdf-lex`, `purrdf-iri`, `purrdf-hash`, and `hashbrown`'s raw
  hash table, keyed by `purrdf-core`'s fixed-key hasher — all
  `wasm32-unknown-unknown`-clean, so
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
C (the GTS container itself reaches Python and C, not the wasm package). Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::entail`; depend on `purrdf-entail` directly only when you want the
entailment engines alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
