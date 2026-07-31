<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Cutover: gmeow-ontology onto PurRDF

PurRDF was extracted from `gmeow-ontology` (see [`PROVENANCE.md`](../PROVENANCE.md)),
and the consumer relationship runs back the other way: gmeow deletes its own copy of
a surface when the PurRDF replacement is integrated. This document is the record of
that cutover — what has completed, what remains, and the port order for the
reasoning substrate — so that neither repository has to infer the other's state.

Every observation about gmeow's tree below is a **dated measurement against gmeow
snapshot `8906e41b15d5adaeccede35dab7e36c7eab86147`** — the same snapshot PurRDF's
datalog and unify/SLG ports were taken from, which is what makes the port
window drift-free. gmeow's tree moves; re-verify against its HEAD before acting on
a row.

## What has already cut over (the carrier layer)

At the measured snapshot, gmeow consumes the `purrdf` umbrella facade (git-pinned)
and has **deleted its own RDF kernel** — thirteen crates (`rdf`, `rdf-core`,
`rdf-events`, `rdf-capi`, `rdf-wasm`, `iri`, `xsd`, `shacl`, `slice`,
`sparql-algebra`, `sparql-eval`, `sparql-results`, `sparql-conformance`) are gone
from its workspace. Integrated on the PurRDF side of the line:

| gmeow surface | PurRDF replacement | state |
|---|---|---|
| Parquet export stage | `purrdf-columnar` | consumer **deleted outright** (the stage was retired as unused after the cutover) |
| Pydantic package stage | `purrdf::shapes::pydantic::emit_pydantic` | integrated; gmeow keeps only caller-owned identity/prose orchestration |
| LinkML / TypeScript / GraphQL schema stages | `purrdf::shapes::{linkml,typescript,graphql}` over one `CompiledSchema` | integrated; the hand-rolled OWL-reading emitters are deleted |
| LPG / tabular / graph exports | `purrdf::project_lpg*`, `project_csvw_exact`, `project_skos`, `project_obo_graphs` (+ `lift_lpg`) | integrated; gmeow keeps caller-owned config/vocabulary only |
| SHACL validation | `purrdf::shapes::engine` | integrated; no native SHACL engine remains |
| ShEx | purrdf's ShEx 2.1 **parser** as a well-formedness gate | asymmetric: gmeow still authors ShExC itself; a purrdf ShEx *emission* API remains undefined |
| Research-object stage | `purrdf::project_{croissant,datacite,dcat,frictionless,research_object}` | **NOT integrated** — the one remaining carrier-layer gap. Blocker: the gmeow stage's contract is byte-parity with rdflib 7.6 Turtle, Python `json.dumps`, and `ElementTree` XML output; PurRDF's codecs deliberately discard those targets, so the cutover re-blesses gmeow's committed goldens rather than being byte-neutral |

## Prerequisites for the reasoning-substrate cutover

1. **A crates.io record for `purrdf-datalog`.** The crate is new; a Trusted
   Publisher entry can only be configured after a first token-authenticated
   bootstrap publish (`scripts/bootstrap-crates-io.sh`). Until that exists, a
   `rust-v*` tag fails mid-publish. This is a maintainer-token action no
   automation may take.
2. **gmeow re-pins.** The measured pin predates `crates/datalog` existing; every
   row below requires a purrdf version that carries `purrdf-datalog`,
   `purrdf-entail`'s reasoning surface, and `purrdf-xsd::{range,rational}`.
3. **Budget ceilings are sized before the first large closure.**
   `purrdf-datalog` enforces fixed evaluation ceilings (`MAX_JOIN_STEPS`,
   `MAX_STORED_FACTS`, `MAX_TERM_ARENA_BYTES`) — deliberately constants, never
   caller knobs, because a caller-supplied budget is semantic optionality (two
   callers, same input, different answers). `MAX_STORED_FACTS` is calibrated for
   a browser's linear memory and **caps ontology-scale closure**; a corpus the
   size gmeow reasons over exceeds it. The remedy is the documented
   constant-raise procedure, not a parameter: raise the constants with a measured
   justification (the calibration discipline in the Makefile's wasm-budget
   comment is the template — a measured figure, a stated capability, and a cheap
   blow-up test program per ceiling so the suite stays fast). If one value cannot
   serve both native ontology-scale and wasm's linear memory, target-gated
   constants are platform capability, not optionality — the precedent is
   `purrdf-sparql-eval`'s target-gated dependencies.

## The port order (reasoning substrate)

Each row: what gmeow carries, what PurRDF provides, and the seam that makes the
port honest. Costs compound downward — do them in order.

### 1. Module extraction: delete dead code

`crates/logic/src/slme/` (locality-based module extraction) has **zero callers**
in gmeow at the measured snapshot; `purrdf-entail`'s `reasoner::module` is the
ported replacement (BOT/TOP/STAR). Pure deletion plus a dependency on the purrdf
service for any future caller.

### 2. OWL 2 RL rule tables

gmeow's `reason/{rl,rl_rules}.rs` implement 32 rules; `purrdf-entail` implements
the full 78 of OWL 2 Profiles §4.3 Tables 4–9, with `rules()` /
`implemented_rules()` / `extensions()` exposed as data and a reasoning report on
every closure. Seams:

* **World ↔ graph.** Both engines evaluate over an arity-4
  `triple(?s, ?p, ?o, ?g)` relation with the predicate as data, but the fourth
  position differs in meaning: gmeow's is a *world* identifier scoped by its own
  semantics; PurRDF's is the **RDF graph name**, with the documented dataset
  semantics that each named graph closes against the union of itself and the
  default graph. A port maps worlds onto named graphs explicitly — stating which
  world becomes the default graph — or flattens to the default graph before
  closure. Leaving the mapping implicit silently changes which premises can meet
  which.
* **Rule identity.** gmeow mints rule IRIs in its own namespace and commits them
  in goldens and divergence ledgers; PurRDF names rules by their specification
  ids (`cax-sco`) and never mints IRIs. Every gmeow golden carrying a rule IRI
  re-blesses.
* **Contract hash.** gmeow's `native_contract_hash` and PurRDF's
  `contract_hash()` are computed over different data; cached verdicts do not
  carry across.

### 3. Datatype / facet decision

gmeow's `reason/refute/datatype.rs` decides facet satisfiability on an exact-ℚ
tower; `purrdf-xsd::range` is the replacement (three-valued: `Empty` /
`Inhabited` / `Undecided`, never a guess), with `purrdf-xsd::rational` deciding
the rational↔decimal identity exactly (`"0.5"^^xsd:decimal` ≡
`"1/2"^^owl:rational`; floats stay a disjoint branch). Seams:

* Facet ranges **over** rationals (a `minInclusive "1/3"^^owl:rational` bound)
  remain in `range.rs`'s named `Undecided` residue; extending the decimal
  stratum to rational endpoints is the receiving seam, and
  `purrdf-xsd::rational` is the value type it extends over.
* gmeow's finite-cardinality table (e.g. `xsd:byte` = 256) is dogfooded from its
  ontology slices with a projection proof; PurRDF derives the same facts in
  code. A port either accepts PurRDF's derivation or keeps the slice-grounding
  on the gmeow side as caller configuration.

### 4. Goal-directed / SLG layer

PurRDF carries faithful ports of gmeow's `unify` and `resolve_fol` (SLG–WFS)
modules, taken at the same snapshot, with the gmeow couplings (`gmeow_errors`,
`gmeow_term_arena`, ℚ builtins) stripped at the boundary — see `PROVENANCE.md`
for the module-for-module record. `goal_directed.rs` itself is deliberately NOT
ported: its substance is lowering gmeow's RDF-authored reasoning-program corpus,
a vocabulary PurRDF will never mint; the backward-evaluation capability it
exposed is delivered by `resolve_fol` plus the `solve_datalog_goal` bridge onto
the DL-clause IR. gmeow's corpus-lowering layer therefore ports ONTO these
modules — the lowering stays on the gmeow side, calling a substrate that is now
shared — and the ported tests are the compatibility contract.

### 5. Substrate features with no PurRDF home yet

The remainder of gmeow's `physical/` has no counterpart and stays gmeow-native
until it is ported *into* `purrdf-datalog` (each with its receiving seam):

| gmeow module | capability | receiving seam in `purrdf-datalog` |
|---|---|---|
| `magic.rs`, `magic_generic.rs` | magic-set rewriting | a plan-pipeline pass over the existing clause IR |
| `incremental.rs`, `incremental_grounding.rs` | Z-set / Backward-Forward maintenance sessions | the store's provenance counts, carried for exactly this |
| `builtin_eval.rs` | moded builtins, exact-ℚ arithmetic | the clause IR's builtin slot; `purrdf-xsd::rational` as the value type |
| `nary.rs` (src level) | fixed-arity n-ary EDBs | lowering onto the quad-4 store, as gmeow already lowers n-ary to reified binary |
| `annotation.rs` | annotation algebra | new; no constraint in the store shape blocks it |

The quad-4 store is the one fixed point: gmeow's binary-relation encodings lower
onto it (the predicate is data), and nothing in the current evaluator assumes
the reverse.

### Proof and provenance IRIs: a layering, not a conflict

gmeow's proofs project to content-addressed provenance IRIs that ship in its
diagnostics graphs. PurRDF mints no vocabulary IRIs, ever — its proofs are
checkable terms with a BLAKE3 content digest. These compose: the digest is the
content address, and deriving an IRI from it is caller-supplied vocabulary,
which is PurRDF's standing doctrine applied to proofs. The gmeow side keeps its
IRI scheme; the PurRDF side never learns it.

### A gmeow-side policy, described not prescribed

gmeow's repository lint seals its native reasoner as the *single reasoning
authority* and hard-fails any live differential-oracle gate. A staged cutover
with an A/B lane would require amending that lint on the gmeow side; without
amendment, each row above is a single-authority swap validated by gmeow's own
frozen goldens. That trade-off belongs to gmeow's maintainers; this guide only
records that the lint exists and what it forbids.
