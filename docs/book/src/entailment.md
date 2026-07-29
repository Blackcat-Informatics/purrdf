<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Entailment

[`purrdf-entail`](https://docs.rs/purrdf-entail) (re-exported as
`purrdf::entail`) is native, `wasm32`-clean entailment for the PurRDF
`RdfDataset` IR. A family of engines sits behind one facade, each the right
tool for its SPARQL entailment regime — closing a dataset to its inferred
fixpoint entirely in interned `TermId` space, with **no** external reasoner,
no async runtime, and no string round-trip.

## Surface map

| Entry point | Regime(s) | Engine |
| --- | --- | --- |
| `materialize(ds, regime)` | `Simple`, `RDF`, `RDFS`, `OWL-RL`, `D` | Forward materialization ("chase") of the regime's declared clause program via a native semi-naive fixpoint. Returns `(closure, ReasoningReport)`; the report is not optional. |
| `materialize_dl(...)` | `OWL-Direct` | Open-world OWL DL over an ALCOIQ tableau — it needs the query's class expressions, so it is not reachable through the plain `materialize` facade. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `parse_rif_xml(...)` / `resolve_rif_imports(...)` | `RIF` | RIF-XML parsing with caller-owned, I/O-free import resolution. |
| `rules(regime)` / `implemented(regime)` | — | The rule table a regime is *defined by*, and the subset this workspace fires. Their difference is the measurable gap. |
| `calculus_program(regime)` | — | The regime's calculus as DL-clause data — the very program `materialize` evaluates, so a consumer can recompute its contract hash. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |

```rust,ignore
use purrdf::entail::{materialize, Regime};

// Close a frozen dataset to its RDFS fixpoint; the result is a new dataset
// AND a report of what the run did.
let (closed, report) = materialize(&ds, Regime::Rdfs).expect("materializes");
assert!(!report.overclaims());
```

## The same engine in four hosts

Entailment is not re-implemented per host. Python, WebAssembly, and the C ABI all
route through one shared string boundary (`purrdf_validate::regime`) that wraps
the Rust engine, and all four surfaces are checked against a single committed
golden-vector artifact — so a divergence shows up as one vector failing rather
than as three surfaces that quietly stopped agreeing. The regime spellings
(`simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d`) are the same
everywhere.

| Host | Materialize | Defined rule table | Implemented rules |
| --- | --- | --- | --- |
| Rust | `materialize(&ds, Regime::Rdfs)` | `rules(Regime::Rdfs)` | `implemented(Regime::Rdfs)` |
| CLI | `purrdf reason --regime rdfs`, `purrdf convert --entailment rdfs`, `purrdf query --entailment rdfs` | — | — |
| Python | `purrdf.entail.materialize(dataset, "rdfs")`, `purrdf.entail.materialize_nt(text, "rdfs")` | `purrdf.entail.rules("rdfs")` | `purrdf.entail.implemented_rules("rdfs")` |
| JavaScript / WebAssembly | `entailMaterialize(doc, "rdfs")` | `entailRules("rdfs")` | `entailImplementedRules("rdfs")` |
| C | `purrdf_entail_materialize_to_nquads(...)` | `purrdf_entail_rules(...)` | `purrdf_entail_implemented_rules(...)` |

Two host-specific bounds, stated rather than glossed:

- The **CLI** materializes `simple`, `rdf`, `rdfs`, and `owl-rl`. It refuses
  `owl-direct`, `rif`, and `d` with exit code 3.
- The **WebAssembly** module also exports `entailCheckGoldenVectors()`, which
  replays the committed tri-host vector artifact inside the module a consumer
  actually loaded — so agreement with the reference implementation can be checked
  without trusting this repository's CI.

## Rule coverage

`rules(regime)` is the rule table the specification defines the regime by;
`implemented(regime)` is the subset the evaluator fires. Both are `&'static`
slices in specification table order, so the gap is an executable artifact instead
of a sentence:

| Regime | Rule table | Defined | Implemented |
| --- | --- | ---: | ---: |
| `Simple` | — (identity closure) | 0 | 0 |
| `RDF` | RDF 1.2 Semantics §8.1.1 | 3 | 1 |
| `RDFS` | RDF 1.2 Semantics §8.1.1 + §9.2.1 | 18 | 14 |
| `OWL-RL` | OWL 2 Profiles §4.3 Tables 4–9 | 78 | 78 |
| `D` | OWL 2 Profiles §4.3 Table 8 | 5 | 5 |
| `OWL-Direct` | — (ALCOIQ tableau, not a fixed table) | 0 | 0 |
| `RIF` | — (caller-supplied rule set) | 0 | 0 |

The per-rule breakdown — every rule id, its specification citation, and whether
it is fired — is [generated from that API](entailment-rules.md) and
drift-guarded, so it cannot fall behind the code.

Where the numbers stop:

- **The four RDF/RDFS residuals are one gap, not four.** `rdfD1`, `rdfD1a`,
  `rdfs14`, and `rdfs14a` each conclude about a *fresh* blank node. That is an
  existentially quantified head, which the Datalog evaluator refuses by
  construction rather than approximating with a minted surrogate.
- **A complete rule table is not a complete closure.** `OWL-RL` fires all 78
  rules, and a run that met a boundary still reports
  `Completeness::ExactWithinBoundaries` rather than `Exact`. The two claims are
  reported separately on purpose.
- **Seventeen OWL 2 RL rules conclude `false`.** "Implemented" for those means
  *decided*: a body match becomes `EntailError::Inconsistent` carrying a witness
  that names the rule and the asserted triples that satisfied it. That is the
  only thing a rule with no conclusion can do.

## The chase (Simple / RDF / RDFS / OWL-RL / D)

`materialize` runs a forward-materialization chase: a fixed rule set for the
selected regime, applied by a semi-naive fixpoint until no new quads appear.
Because it runs over the frozen IR, it is deterministic — a given input and
regime always yields the same closure — and because it works in `TermId`
space, no term is ever re-parsed or re-serialized along the way.

Typical use: materialize first, then query with the plain
[SPARQL engine](sparql/querying.md) or validate the closure with
[SHACL](validation/shacl.md) (the SHACL validator itself performs no
inference).

The rule set is not written twice. `calculus_program(regime)` renders it as
DL clauses and `materialize` evaluates exactly those clauses through
[`purrdf-datalog`](datalog.md)'s semi-naive evaluator, so the contract hash a
report carries identifies the clauses that actually ran.

## Every run says what it did

`materialize` returns `(closure, ReasoningReport)`. There is deliberately no
report-free variant, because the alternative — two entry points, one of which
discards the evidence — is how a partial rule set comes to be described as a
complete one. The report carries:

- **`Completeness`** — derived from `rules(regime)` minus `implemented(regime)`,
  so it improves by itself as rules are added, and it names the `missing` rules
  rather than merely counting them;
- **per-rule firing counts** — which rules fired and how many conclusions each
  contributed;
- **`Boundary`s** — the constructs the run met and could not close over, each
  with its reason;
- **the evaluation budget** — what the run consumed of the evaluator's fixed
  ceilings;
- **a contract hash** — `purrdf-datalog`'s digest of the clause program, so a
  cached closure minted under a different calculus can be *refused* rather than
  trusted;
- **an inconsistency witness**, when a rule that concludes `false` matched: the
  rule id, the asserted triples that satisfied its premises in premise order, and
  the graph they were read from.

A report that claims `Exact` while naming a boundary is a test failure, and
`report.overclaims()` is the gate that says so.

The rendering is byte-stable, so the Python, WebAssembly, and C hosts hand back
the same report text as Rust for the same input.

## OWL-Direct: the tableau

`OWL-Direct` semantics is open-world Description Logic, which a forward chase
cannot answer. `materialize_dl` runs an **ALCOIQ tableau** instead — answering
instance and subsumption queries via classification, realization, and
query-directed materialization. Because it needs the query's class
expressions, it has its own entry point rather than hiding behind
`materialize`.

## RIF

`materialize_rif` evaluates **RIF-Core** rules over a parsed `RuleSet`,
covering the SPARQL RIF entailment regime.

## D (datatype) entailment

`D` is materialized, not refused. PurRDF realizes it as Simple entailment plus
the five `dt-*` rules of OWL 2 Profiles §4.3 Table 8 — the fixed rule table a
forward chase can enumerate for it — decided over the XSD *value* space by
`purrdf-xsd` rather than by comparing lexical forms.

What Table 8 does not cover is the infinite value spaces themselves, and that is
reported as a `Construct::DatatypeValueSpace` boundary on the run rather than
claimed. So a `D` closure is complete *within its stated boundary*, and the report
is where the boundary is stated.

The command-line tool is the one host that does not expose `d`: `purrdf reason
--regime d` and `purrdf convert --entailment d` exit 3. The Rust, Python,
WebAssembly, and C surfaces all materialize it.

`EntailError::Unsupported` is therefore reached by exactly two regimes,
`OWL-Direct` and `RIF`, and in both cases because the plain `materialize` facade
has no way to supply the input they need — not because the regime is
unimplemented.

## Invariants

- **No minted vocabulary.** Every constant in the crate's `vocab` module is a
  standard `rdf:`/`rdfs:`/`owl:` IRI drawn from the entailment specs
  themselves — the crate fabricates none, per the
  [toolkit-not-ontology rule](project/design-rules.md).
- **Dependency-lean and wasm-clean.** The dependencies are `purrdf-core`,
  [`purrdf-datalog`](datalog.md), `purrdf-xsd`, `roxmltree`, and two fixed-key
  hashers (`ahash`, `hashbrown`) — every one of them
  `wasm32-unknown-unknown`-clean, so the engines carry into Rust, Python,
  WebAssembly, and C unchanged, with no threads, filesystem, or RNG dependency.
- **Deterministic.** Same input + regime → same closure, always — and the same
  report, byte for byte.

## Conformance

Two corpora measure two different things, and the distinction matters:

- **W3C SPARQL 1.1 entailment-regime group — 70 of 70 cases pass**, with zero
  ledgered residuals: the RDF/RDFS/OWL-RL chase, the OWL-Direct (DL) tableau, the
  RIF-Core rule engine, and RDF-axiomatic predicate typing, all run through the
  SPARQL conformance harness.
- **W3C OWL 2 test suite — 233 of 261 cases agree, 28 ledgered**, zero
  unledgered. This corpus is *consistency*-shaped: all 261 vendored cases are
  `otest:ConsistencyTest` (226) or `otest:InconsistencyTest` (35), because the
  upstream material contains no entailment tests. It therefore grades the
  DL/tableau lane's satisfiability verdicts and says nothing about the OWL 2 RL
  rule table, which is a forward chase covered by authored per-rule fixtures.
  Every one of the 28 divergences is named in a typed ledger; an unledgered
  divergence, and a ledgered case that has started agreeing, are both hard
  failures.

The live scoreboard is
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).
