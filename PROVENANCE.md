# Provenance

This repository is a copy-and-rename extraction assembled on the
`purrdf-extraction` branch.

Source snapshots:

- `../gmeow-ontology` at `2e613ac36c1ba896d7593585424e62d64d2a560a`
- `../gmeow-gts` at `a7949a474a5670a25fdde8f0b76146b1fa0f458c`

Copied from `gmeow-ontology`:

- RDF kernel crates: `rdf`, `rdf-core`, `rdf-events`, `rdf-capi`, `rdf-wasm`
- IRI and XSD support crates: `iri`, `xsd`
- SPARQL crates: `sparql-algebra`, `sparql-eval`, `sparql-results`,
  `sparql-conformance`
- SHACL/shape validation: `shacl` copied as `shapes`
- Carrier IR and dataset/slice wrappers: `slice`
- Python package sources under `python/src/purrdf`
- The normalized five-table Parquet projection in
  `crates/pipeline/src/stages/parquet.rs` was used as the migration reference for
  `purrdf-columnar`; PurRDF replaces its Arrow/Snappy writer-only path with the
  first-party bidirectional RDF 1.2-complete codec documented in
  `docs/COLUMNAR.md`.
- The deterministic `$defs` transliteration in
  `crates/pipeline/src/stages/pydantic.rs` at
  `6cfd86d0ac9450e8cfdc1ae0c54acfea326b186e` was used as the migration reference
  for `purrdf-shapes::pydantic`. PurRDF removes its repository, ontology,
  namespace, slice-routing, and fixed-package coupling; the carrier API consumes
  `CompiledSchema` in memory, takes package prose from the caller, and records
  runtime projection gaps on the shared closed loss ledger. The verified
  `import_pydantic_package` reverse path retains the exact source schema and
  rejects artifact/model-map drift. The legacy implementation is disposable
  migration material, intended for deletion when gmeow integrates this
  replacement; the consumer cutover is not yet complete and no downstream type
  contract is preserved.
- The legacy LinkML YAML model in
  `crates/pipeline/src/stages/schemas.rs` at
  `c91195e0c300cad9c9a32c8580c2910a6fd48fc1` was used solely as migration
  evidence for behavior that PurRDF must subsume and replace. Its private
  OWL/FoldView structures, fixed identity, shallow range mapping, and coupled
  TypeScript/GraphQL model are not reused architecture. The replacement
  `purrdf-shapes::linkml` API consumes `CompiledSchema`, requires all identity
  and vocabulary from the caller, preserves a canonical LinkML 1.11 document,
  reads native LinkML back into SHACL, verifies emitted packages, and records
  projection/import gaps through direction-specific closed loss ledgers. The
  legacy model is intended for deletion once this replacement is integrated,
  not preservation as a downstream contract.
- The legacy `render_typescript` path in
  `crates/pipeline/src/stages/schemas.rs` at
  `c91195e0c300cad9c9a32c8580c2910a6fd48fc1` was used only as evidence of the
  consumer artifact to replace. Its LinkML-coupled private model, normalized
  property identifiers, all-optional fields, local-name runtime enums, scalar
  fallbacks, and fixed downstream identity are deliberately discarded. The
  replacement `purrdf-shapes::typescript` projection consumes
  `CompiledSchema`, preserves exact JSON property names and requiredness,
  requires caller-owned package identity and prose, exposes a reversible type
  map, verifies intact packages before reverse SHACL import, and locates every
  non-projectable assertion on a closed loss ledger. The old renderer and its
  shared private schema model are intended for deletion once this replacement
  is integrated; no downstream type contract is being preserved.
- The legacy `render_graphql` path in
  `crates/pipeline/src/stages/schemas.rs` at
  `c91195e0c300cad9c9a32c8580c2910a6fd48fc1` was likewise used only as
  migration evidence for a consumer artifact that PurRDF must subsume and
  replace. Its output-only, LinkML-coupled private model, fabricated `id`/`iri`
  fields, normalized names without a reverse map, all-nullable fields, broad
  scalar collapse, and fixed GMEOW identity are deliberately discarded. The
  replacement `purrdf-shapes::graphql` projection consumes `CompiledSchema`,
  requires caller-owned identity, prose, and fallback-scalar name, emits paired
  output/input GraphQL September 2025 SDL, retains a canonical reversible name
  map and value codec, verifies intact packages before reverse SHACL import, and
  locates every coercion difference on a closed loss ledger verified against
  GraphQL.js. The old renderer and shared legacy model are intended for deletion
  when gmeow integrates this replacement; that consumer cutover is not yet
  complete and no downstream type contract is being preserved.

The five schema-language reverse paths are therefore PurRDF replacements, not
compatibility layers around gmeow's private models. They share one deterministic
schema-to-SHACL engine and caller-owned vocabulary boundary.

- The legacy graph/tabular writers in `crates/pipeline/src/stages/lpg.rs` and
  `crates/pipeline/src/stages/export.rs` at
  `d7745068f59b6dee187ab6b806bd2c04c9a1280a` were used solely as migration
  evidence for outputs that PurRDF must subsume and replace. Their private
  carrier structs, hardcoded GMEOW graph/vocabulary identity, local-name and
  prefix shortening, fixed filenames, coupled pipeline context, writer-only
  behavior, and ad hoc layouts are deliberately discarded rather than retained
  as reusable models. The replacement `purrdf-rdf::projections` surface uses one
  caller-configured canonical LPG model with four strict adapters, a standards
  CSVW engine plus exact RDF 1.2 profile, and typed OBO Graphs 0.3.2 and SKOS
  views. It requires caller-owned identity, vocabulary, limits, and policy;
  produces deterministic bounded archives; and computes closed located loss
  ledgers on every path. The legacy types and writers are intended for deletion
  when gmeow integrates these replacements. That consumer cutover is not yet
  complete, and no downstream type or byte-layout contract is being preserved.
- The legacy research-object stage in
  `crates/pipeline/src/stages/research_objects.rs` at
  `154921ddce1797b220877598f75d838e2075dc42` was used solely as migration
  evidence for value correspondences that PurRDF must subsume and replace. Its
  worked-example store/model, fixed GMEOW vocabulary and identities,
  placeholder DOI generation, filesystem-bound graph loading, declared-loss
  strings, Python/rdflib/ElementTree byte-parity targets, and writer-only
  Croissant/RO-Crate/DataCite/DCAT/Frictionless outputs are deliberately
  discarded. The replacement `purrdf-rdf::projections::research_object` surface
  uses one typed caller-vocabulary semantic pivot, five strict bidirectional
  versioned codecs, offline JSON-LD interpretation, deterministic bounded USTAR
  carriers, and closed located runtime loss ledgers. The legacy types and stage
  are intended for deletion when gmeow integrates the replacement; that
  consumer cutover is not yet complete, and no legacy type or byte-layout
  contract is preserved.

Copied from `gmeow-gts`:

- Rust GTS transport engine copied as `crates/gts`
- GTS conformance vectors under `vectors`
- GTS specification and implementer docs under `docs`
- SVG brand assets under `docs`

Extraction policy:

- Source repositories are read-only during this phase.
- `purrdf-core` is the transport-independent primitive layer.
- `purrdf-slice` carries the ontology-structure layer: slice catalogs,
  dataset-level wrappers, ownership/dependency analysis, and generated
  projection inputs.
- `purrdf-gts` is the GTS container engine: CBOR sequence, transforms, fold,
  verification, signing, encryption, files, and transport policy.
- RDF text/native-store/profile codecs formerly exposed by `gmeow-gts` are not
  exported by `purrdf-gts`; purrdf owns those adapters on top of purrdf
  primitives.
- SHACL and ShEx are part of purrdf's shape scope. SHACL is present in
  `crates/shapes`; current source checkouts expose ShEx as projection/export
  logic and dependency metadata rather than a standalone crate, so a purrdf ShEx
  API still needs to be defined.

Cutover staging:

- `../gmeow-ontology/.worktrees/purrdf-cutover` exists on branch
  `paudley/purrdf-cutover`.
- The downstream cutover is still in progress. Legacy consumer models and
  renderers are migration evidence to delete as their PurRDF replacements are
  integrated, not compatibility surfaces to preserve.
- See `docs/CUTOVER.md` for the publish order, local gates, and dependency
  replacement rules.

## Datalog physical primitives

A later snapshot than the original extraction above: `../gmeow-ontology` at
`8906e41b15d5adaeccede35dab7e36c7eab86147`.

Every module of `purrdf-datalog` is accounted for below, ported or authored.
An earlier revision of this section named only the first four and was corrected:
the relicensing basis has to cover what was actually taken, and an incomplete
record is the one kind of provenance error that cannot be caught by a gate —
`scripts/check-licenses.py` verifies that a file *declares* an identifier, never
that the declaration is warranted.

Ported from that snapshot's `crates/logic/src/physical/`, module for module:

| module | upstream |
| --- | --- |
| `id` | `physical/id.rs`, with the branded-id type from `crates/term-arena/src/id.rs` |
| `arena` | `physical/arena.rs` |
| `bitset` | `physical/bitset.rs` |
| `binding_pattern` | `physical/binding_pattern.rs` |
| `store` | `physical/store.rs` |
| `cursor` | `physical/cursor.rs` |
| `plan` | `physical/plan.rs` |
| `seminaive` | `physical/seminaive.rs` |
| `chase` | `physical/chase.rs` |
| `proof` | `physical/proof.rs` |
| `synth_corpus` | `crates/logic/src/synth_corpus.rs` |

`cache` is a split: its plan identity, cache and canonical rule hash come from
the tail of `physical/plan.rs`; its contract hash is authored here, modelled on
the upstream contract-hash idea but computed over data rather than source text,
because this workspace's wasm artifact is size-budgeted and embedding source to
checksum it would spend the budget on a checksum.

`clause` and `lib` are authored here. The DL-clause IR has no upstream
counterpart: the disjunctive-TGD shape carrying atomic, conjunctive, disjunctive
and empty heads in one type was designed for this crate.

The source is licensed `AGPL-3.0-only`; the port is relicensed
`MIT OR Apache-2.0` under common ownership by the copyright holder. Every ported
file carries a fresh SPDX header and no upstream licence text survives.

The port is not a transcription. Three couplings were removed rather than
vendored: the sister project's error and arbitrary-precision crates (unused by
these modules), its shared term arena (the branded `Id<C>` is now defined here),
and `smallvec` (replaced by a fixed-capacity inline tuple, so the crate keeps its
two-dependency budget and its handles become `Copy`). Ordering was tightened
beyond the source — `BindingPattern` and `TermRef` gained a total order so index
selection can key an ordered map rather than a hash map, because in this crate no
map iteration order may reach an output path. Nightly `portable_simd` and
`unsafe` are absent from the ported subset, so the crate holds
`#![forbid(unsafe_code)]` on stable without rewriting.

## Goal-directed backward resolution (SLG/WFS)

The same later snapshot as above — `../gmeow-ontology` at
`8906e41b15d5adaeccede35dab7e36c7eab86147` — for a second port, into the same
`purrdf-datalog` crate:

| module | upstream |
| --- | --- |
| `unify` | `crates/logic/src/physical/unify.rs` |
| `resolve_fol` | `crates/logic/src/physical/resolve_fol.rs` (+ `resolve_fol/tests.rs`) |

`term` is authored here: upstream's `unify.rs`/`resolve_fol.rs` operate over
`gmeow-term-arena`'s hash-consed `TermDag`/`NodeData` (compound applications and
simple binders over locally-nameless de Bruijn variables, plus first-class
unification metavariables), a dependency this crate does not have and does not
want (it is a whole sister crate, not a module). `term` is a from-scratch,
self-contained arena holding exactly the same shape, keyed by this crate's own
branded `Id<C>` (three new brands: `Node`, `Meta`, `Sym`), hash-consed with the
same fixed-key-`ahash` + `hashbrown::HashTable` pattern `crate::proof::ProofArena`
already established in this crate — so the port introduces no new dependency and
no new interning idiom.

The relicensing statement is the same as above: the source is licensed
`AGPL-3.0-only`; the port is relicensed `MIT OR Apache-2.0` under common
ownership by the copyright holder, with a fresh SPDX header on every file and no
upstream licence text retained.

Couplings removed, beyond the term arena already named: `gmeow_errors` (unify.rs
has none; resolve_fol.rs's one error site — an internal-invariant panic path —
is not reachable from a normal call and is not carried as a typed error here);
`gmeow_math::Rational` (neither file uses it at all — checked by grep, not
assumed); `smallvec` (not used directly by either file upstream either — it is
`gmeow-term-arena`'s internal `MetaSet` representation, which does not exist
here because `free_meta` is a plain sorted-deduplicated `Vec<MetaId>` per node);
`blake3` content-addressed rule-firing IRIs (upstream's `resolve_fol.rs` names a
ground rule application by a BLAKE3 digest over the rule IRI's lexical text,
because its proofs are read back into RDF by `goal_directed.rs`'s projection —
this port's proofs are a plain Rust tree, never serialized to RDF, so a rule is
named by its authored `usize` index instead, exactly as this crate's own
`crate::proof::ProofArena` already names a rule by index rather than by a minted
identity). `HashMap`/`HashSet` become `BTreeMap`/`BTreeSet` throughout both
files — a deliberate strengthening, not an oversight: this crate's determinism
doctrine forbids a map whose iteration order could reach an output path, and
upstream's unsorted hash containers would have been exactly that path for the
tabling engine's demand/answer sets.

`goal_directed.rs` was read in full and NOT ported. Its own module doc states
its entire reason for existing: it is "the single thin, honest `pub` façade"
that lowers gmeow's RDF-AUTHORED `logic:ReasoningProgram` corpus (parsed by the
sister crate `gmeow-logic-compile`'s Turtle frontend) into `resolve_fol`'s
input, proof-checks every answer, and projects checked answers into RDF for a
downstream pipeline stage. Every one of those three responsibilities is coupled
to authoring machinery this repository does not have and is not building —
purrdf mints no vocabulary, so it has no `logic:ReasoningProgram` authoring
vocabulary to parse in the first place, and building one would be a separate,
unrequested capability. What `goal_directed.rs` does NOT provide is any
resolution logic of its own: by its own doc, "it is NOT a fork of the engine…
never re-implementing resolution," and a sibling module's doc (`runtime.rs`)
independently classifies it as "a downstream consumer of the decision path… not
part of what dispatch_query decides." The goal-directed BACKWARD-RESOLUTION
capability the task asks for is therefore delivered in full by the `resolve_fol`
port alone; what `goal_directed.rs` would have added on top — a program
lowering, proof-checking, and a projection — is instead provided by
`resolve_fol::solve_datalog_goal`, a lowering FROM this crate's own `DlClause`/
`ClauseAtom` Datalog IR (not from an RDF-authored program the way upstream's
lowering was), so the capability is real, tested machinery over the crate's own
data rather than a standalone engine nobody calls, exactly as `goal_directed.rs`
was for gmeow's data.

## The combined approach (`purrdf-entail::combined`)

`crates/entail/src/combined.rs` is ORIGINAL work, not a port: `gmeow-ontology`
at the snapshot above has no combined-approach, rolling-up, or filtration
implementation anywhere in `crates/logic` or `crates/conformance` — grepped and
confirmed absent before writing it. It implements the Lutz/Toman/Wolter and
Stefanoni/Motik/Horrocks combined approach for the Horn fragment
`purrdf-datalog`'s existing restricted chase (`crate::chase`) already
certifies: a TBox's `A ⊑ B` / `A ⊑ ∃r.B` axioms over named vocabulary lower into
`DlClause`s, the chase materializes the existential witnesses those axioms
license, and a caller filters out any answer that would bind a distinguished
query variable to one — see that module's own doc for the full account of the
gap it closes in `owl_dl::query::materialize_dl_reported`'s query-independent
augmentation.
