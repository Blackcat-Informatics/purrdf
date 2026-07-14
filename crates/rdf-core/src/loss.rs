// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The machine-readable RDF↔GTS loss ledger (C0).
//!
//! "RDF 1.2 fidelity" cannot be claimed for a conversion until either the GTS
//! representation is extended to carry every RDF 1.2 feature, **or** every
//! intentional loss is enumerated, tested, and exposed as a stable contract. This
//! module is option (b): a small, deterministic ledger of the known, accepted
//! conversion losses between the RDF 1.2 dataset IR and the GTS transport. The
//! gate is simple — `RdfBundle` fidelity is asserted **only** where the relevant
//! ledger [`LossLedger::is_empty`].
//!
//! The ledger is kernel-clean (PyO3-free) and renders to byte-stable JSON sorted
//! by code; the RDF↔GTS-only matrix ([`rdf_gts_loss_matrix_json`]) is committed at
//! `generated/rdf-loss-matrix.json` and the full enumerable registry
//! ([`loss_matrix_json`]) at `generated/transcode-loss-matrix.json`, each with a
//! drift gate in this module's tests that re-derives and compares it.
//!
//! [`LossEntry`]/[`LossLedger`] serve two disciplines (see [`LossLedger::contract`]
//! and [`LossLedger::record`]): a compile-time **contract** (the static ledgers
//! and the transcode matrix in this module) and a runtime **record** (an actual
//! conversion accumulating located losses as it runs). [`registered_pairs`] and
//! [`profile_for`] enumerate the closed set of `(from, to)` pairs and loss codes
//! known across BOTH disciplines, including the non-syntax `shacl`→`json-schema`
//! shapes projection; [`loss_matrix_json`] renders that same enumerable registry.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::RdfLocation;

/// In-band machine code: a `CONSTRUCT` whose `WHERE` bound an RDF-1.2 reifier (via
/// an `rdf:reifies` triple pattern) but whose template drops that reifier — the
/// reification layer is lost at the projection. Declared in-band on the output
/// graph (a caller-configured `ProjectionLoss` node), NOT in this compile-time ledger.
pub const LOSS_REIFIER_LAYER_DROPPED: &str = "reifier-layer-dropped";

/// In-band machine code: a dropped reifier (see [`LOSS_REIFIER_LAYER_DROPPED`])
/// that ALSO carried annotation triples in the `WHERE` — those annotations are lost
/// too. Emitted in addition to the reifier-layer code, never alone.
pub const LOSS_ANNOTATION_LAYER_DROPPED: &str = "annotation-layer-dropped";

/// In-band machine code: a dropped, annotated reifier (see
/// [`LOSS_ANNOTATION_LAYER_DROPPED`]) where one of the dropped annotation
/// predicates was the caller-configured standpoint `accordingTo` predicate — the
/// standpoint scope is lost. Emitted in addition to the annotation-layer code, never alone.
pub const LOSS_STANDPOINT_SCOPE_DROPPED: &str = "standpoint-scope-dropped";

/// One enumerated conversion loss between two representations.
///
/// Two construction disciplines share this type:
///
/// - **contract** (compile-time): built via [`LossLedger::contract`] from
///   `&'static str` literals wrapped in [`Cow::Borrowed`] — zero-alloc. Every
///   `code` in a contract ledger is unique (a duplicate panics at
///   construction); this is the reviewed, stable promise rendered into the
///   committed `generated/rdf-loss-matrix.json` / `generated/transcode-loss-matrix.json`
///   artifacts.
/// - **record** (runtime): built via [`LossLedger::record`] while an actual
///   conversion runs, using owned (`Cow::Owned`) strings. Duplicates are
///   expected — the same code can fire many times over one input — and each
///   entry MAY carry a [`location`](Self::location) pinpointing where the loss
///   occurred.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LossEntry {
    /// Stable machine code, kebab-case (e.g. `direction-dropped`).
    pub code: Cow<'static, str>,
    /// Source representation (e.g. `"rdf-1.2-dataset"`).
    pub from: Cow<'static, str>,
    /// Target representation (e.g. `"gts"`).
    pub to: Cow<'static, str>,
    /// `true` = a known, accepted conversion loss (the only kind this ledger
    /// records). A `false` value would mark an *unintentional* loss, which the
    /// fidelity gate treats as a bug rather than a documented contract.
    pub intentional: bool,
    /// Human-readable explanation of what is dropped and why.
    pub note: Cow<'static, str>,
    /// Where the loss occurred, when known. `None` for every contract entry;
    /// `record`-discipline entries MAY set this to pinpoint the shape/term/
    /// subject the loss concerns.
    pub location: Option<Box<RdfLocation>>,
}

/// An ordered, deterministic set of [`LossEntry`] for one conversion direction
/// (or the combined matrix).
///
/// Entries are kept sorted by `code` so every render is byte-identical regardless
/// of construction order. Codes are unique within a ledger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LossLedger {
    entries: Vec<LossEntry>,
}

impl LossLedger {
    /// Build a **contract** ledger from static entries, sorting by `code` for
    /// determinism.
    ///
    /// Panics on a duplicate `code`: a contract ledger is a compiled-in
    /// promise and a collision is a programming error (hard-fail, per the
    /// no-optionality doctrine), not a runtime condition to tolerate.
    pub fn contract(mut entries: Vec<LossEntry>) -> Self {
        entries.sort_by(|a, b| a.code.cmp(&b.code));
        for pair in entries.windows(2) {
            assert_ne!(
                pair[0].code, pair[1].code,
                "duplicate loss code `{}` in ledger",
                pair[0].code
            );
        }
        Self { entries }
    }

    /// An empty **runtime** ledger, ready to accumulate losses via
    /// [`Self::record`] as a conversion proceeds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a runtime loss entry.
    ///
    /// Unlike [`Self::contract`], duplicates are expected (the same code can
    /// fire many times over one input) and are never rejected; ordering for
    /// [`Self::render_json`] is computed at render time, not at insertion.
    pub fn record(&mut self, entry: LossEntry) {
        self.entries.push(entry);
    }

    /// The ledger entries.
    ///
    /// For a [`Self::contract`] ledger this is sorted by `code` (the
    /// construction-time order). For a [`Self::record`]-built ledger this is
    /// insertion order; use [`Self::render_json`] for the canonical
    /// `(from, to, code, location)` sort.
    pub fn entries(&self) -> &[LossEntry] {
        &self.entries
    }

    /// `true` when no losses are recorded. Fidelity is asserted only here.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render the ledger as deterministic, versioned JSON: a top-level object
    /// `{ "schema_version": 1, "losses": [ ... ] }`, entries sorted by
    /// `(from, to, code, location)`, 2-space indented, with a trailing
    /// newline. A `location` field is emitted per entry when present.
    ///
    /// This is the stable public runtime-ledger schema downstream consumers
    /// (e.g. a `--loss-ledger` CLI flag) pin against; `schema_version` bumps
    /// only on a breaking shape change. The static contract renders
    /// ([`rdf_gts_loss_matrix_json`], [`loss_matrix_json`]) keep the bare
    /// JSON-array shape instead (no `schema_version`), so the committed
    /// `generated/*.json` matrix artifacts stay unchanged.
    pub fn render_json(&self) -> String {
        let mut sorted = self.entries.clone();
        sorted.sort_by(|a, b| {
            a.from
                .cmp(&b.from)
                .then_with(|| a.to.cmp(&b.to))
                .then_with(|| a.code.cmp(&b.code))
                .then_with(|| a.location.cmp(&b.location))
        });
        render_versioned(&sorted)
    }
}

/// The intentional losses incurred projecting the RDF 1.2 dataset IR → GTS.
pub fn rdf_to_gts_loss_ledger() -> LossLedger {
    LossLedger::contract(vec![LossEntry {
        code: Cow::Borrowed("blob-bytes-absent"),
        from: Cow::Borrowed("rdf-1.2-dataset"),
        to: Cow::Borrowed("gts"),
        intentional: true,
        note: Cow::Borrowed(
            "Blob payloads are preserved as content-addressed references (the blob_id digest \
             plus the origin file id), never materialized into the RDF IR, which must stay \
             value-light for arbitrarily large payloads (e.g. multi-terabyte data dumps). A \
             destination GTS carries the reference; the payload bytes are streamed \
             origin->destination on demand (deferred materialization).",
        ),
        location: None,
    }])
}

/// The intentional losses incurred reading GTS → the RDF 1.2 dataset IR.
pub fn gts_to_rdf_loss_ledger() -> LossLedger {
    LossLedger::contract(vec![LossEntry {
        code: Cow::Borrowed("bnode-scope-flatten"),
        from: Cow::Borrowed("gts"),
        to: Cow::Borrowed("rdf-1.2-dataset"),
        intentional: true,
        note: Cow::Borrowed(
            "`purrdf_gts::reader::read()` folds all segments into one term table, collapsing \
             per-segment blank-node scope; the distinct scopes are recovered only via the \
             streaming-event importer.",
        ),
        location: None,
    }])
}

/// The combined RDF↔GTS matrix as a single deterministic, sorted-by-code JSON
/// array — the body of the generated `generated/rdf-loss-matrix.json` artifact.
///
/// This renders only the two direction ledgers ([`rdf_to_gts_loss_ledger`] /
/// [`gts_to_rdf_loss_ledger`]); for the full enumerable registry of every
/// registered `(from, to)` pair (RDF↔GTS directions plus every syntax/
/// projection transcode pair and the shapes projection), see
/// [`loss_matrix_json`].
pub fn rdf_gts_loss_matrix_json() -> String {
    let mut entries: Vec<LossEntry> = Vec::new();
    entries.extend_from_slice(rdf_to_gts_loss_ledger().entries());
    entries.extend_from_slice(gts_to_rdf_loss_ledger().entries());
    let ledger = LossLedger::contract(entries);
    render(ledger.entries(), false)
}

/// Syntax codecs: serialization formats that carry RDF triples/quads faithfully
/// (no semantic projection). Order is stable and matches the canonical list.
const SYNTAX_CODECS: &[&str] = &[
    "turtle",
    "ntriples",
    "nquads",
    "trig",
    "jsonld",
    "jsonld-star",
    "yaml-ld-star",
    "rdfxml",
    "gts",
    "owl-rdf12",
];

/// Projection codecs: lossy targets that select a semantic subset of the
/// source graph (decidable fragments, rule languages, foundational profiles).
pub const PROJECTION_CODECS: &[&str] = &[
    "owl-dl",
    "owl-el",
    "datalog",
    "n3",
    "nemo",
    "gufo",
    "canonical-rdf12",
];

/// Map a codec name to its `&'static str` literal.
///
/// Panics on any name not found in `SYNTAX_CODECS` or [`PROJECTION_CODECS`].
/// Hard-fail per the no-optionality doctrine.
pub fn canonical_codec_name(name: &str) -> &'static str {
    for s in SYNTAX_CODECS.iter().chain(PROJECTION_CODECS.iter()) {
        if *s == name {
            return s;
        }
    }
    panic!("unknown codec name: `{name}`");
}

/// `true` when the codec can carry named-graph (quad) information.
///
/// Panics on unknown codec name.
pub fn supports_quads(name: &str) -> bool {
    match canonical_codec_name(name) {
        "nquads" | "trig" | "jsonld" | "jsonld-star" | "yaml-ld-star" | "gts" => true,
        "turtle" | "ntriples" | "rdfxml" | "owl-rdf12" => false,
        // Projection codecs do not carry named graphs.
        "owl-dl" | "owl-el" | "datalog" | "n3" | "nemo" | "gufo" | "canonical-rdf12" => false,
        _ => unreachable!(),
    }
}

/// `true` when the codec can represent RDF-1.2 quoted triples (star syntax).
///
/// Panics on unknown codec name.
pub fn supports_stars(name: &str) -> bool {
    match canonical_codec_name(name) {
        "turtle" | "ntriples" | "nquads" | "trig" | "jsonld-star" | "yaml-ld-star" | "gts"
        | "owl-rdf12" => true,
        "jsonld" | "rdfxml" => false,
        // Projection codecs do not carry star syntax.
        "owl-dl" | "owl-el" | "datalog" | "n3" | "nemo" | "gufo" | "canonical-rdf12" => false,
        _ => unreachable!(),
    }
}

/// `true` when the codec is a projection (semantic subset) rather than a
/// syntax serialization.
///
/// Panics on unknown codec name.
pub fn is_projection(name: &str) -> bool {
    let n = canonical_codec_name(name);
    PROJECTION_CODECS.contains(&n)
}

/// Compute the static loss contract for a `from → to` transcoding pair.
///
/// Rules:
/// - `from` MUST be a syntax codec. Panics if `from` is a projection.
/// - If `from == to`, returns an empty ledger.
/// - Named-graph and star losses are accumulated as needed; projection targets
///   add a single per-target entry.
///
/// Panics on unknown codec names or a projection source.
pub fn pair_loss_ledger(from: &str, to: &str) -> LossLedger {
    let from = canonical_codec_name(from);
    let to = canonical_codec_name(to);

    assert!(
        !is_projection(from),
        "pair_loss_ledger: `from` must be a syntax codec, not projection `{from}`"
    );

    if from == to {
        return LossLedger::default();
    }

    // Build a contract entry for this `(from, to)` pair from `&'static str`
    // literals — zero-alloc, per the contract discipline (see [`LossEntry`]).
    let entry = |code: &'static str, note: &'static str| LossEntry {
        code: Cow::Borrowed(code),
        from: Cow::Borrowed(from),
        to: Cow::Borrowed(to),
        intentional: true,
        note: Cow::Borrowed(note),
        location: None,
    };

    let mut entries: Vec<LossEntry> = Vec::new();

    if is_projection(to) {
        if supports_quads(from) {
            entries.push(entry(
                "named-graph-dropped",
                "The target syntax has no named-graph construct; quads are folded into the \
                 default graph and graph names are dropped.",
            ));
        }
        let proj_entry = match to {
            "owl-dl" => entry(
                "owl-dl-projection",
                "Projection to OWL 2 DL: rules and constructs outside the decidable DL \
                 fragment are dropped; the result is a sound view.",
            ),
            "owl-el" => entry(
                "owl-el-projection",
                "Projection to the OWL 2 EL profile: constructs outside EL are dropped; \
                 the result is a sound, PTIME-decidable view.",
            ),
            "datalog" => entry(
                "datalog-projection",
                "Projection to Datalog: non-rule axioms and existentials outside the \
                 Datalog fragment are dropped.",
            ),
            "n3" => entry(
                "n3-projection",
                "Projection to Notation3 rules: validation-only and non-rule constructs \
                 are dropped.",
            ),
            "nemo" => entry(
                "nemo-projection",
                "Projection to Nemo existential rules: constructs outside the supported \
                 rule fragment are dropped.",
            ),
            "gufo" => entry(
                "gufo-projection",
                "Projection to gUFO foundational classes: structure without a gUFO \
                 correspondence is dropped.",
            ),
            "canonical-rdf12" => entry(
                "canonical-rdf12-projection",
                "Projection to the canonical RDF-1.2 logic form: non-logic RDF structure \
                 is dropped.",
            ),
            _ => unreachable!("unhandled projection codec `{to}`"),
        };
        entries.push(proj_entry);
    } else {
        // to is a syntax codec
        if supports_quads(from) && !supports_quads(to) {
            entries.push(entry(
                "named-graph-dropped",
                "The target syntax has no named-graph construct; quads are folded into the \
                 default graph and graph names are dropped.",
            ));
        }
        if supports_stars(from) && !supports_stars(to) {
            let star_entry = match to {
                "rdfxml" => entry(
                    "rdf12-star-unrepresentable",
                    "RDF/XML has no triple-term (RDF-1.2 quoted triple) syntax; reifying \
                     triples and their annotations are dropped.",
                ),
                "jsonld" => entry(
                    "rdf12-star-jsonld-rejected",
                    "The JSON-LD 1.1 serializer rejects RDF-1.2 quoted triples; reifying \
                     triples and their annotations are dropped (use jsonld-star or \
                     yaml-ld-star to retain them).",
                ),
                _ => unreachable!(
                    "supports_stars(from)=true but supports_stars({to})=false for unhandled target"
                ),
            };
            entries.push(star_entry);
        }
    }

    LossLedger::contract(entries)
}

/// Every [`LossEntry`] the transcode matrix and the shapes-projection profile
/// contribute: all non-identity `(from ∈ SYNTAX_CODECS) × (to ∈ SYNTAX_CODECS ∪
/// PROJECTION_CODECS)` pairs (via [`pair_loss_ledger`]) PLUS the non-syntax
/// `("shacl", "json-schema")` shapes projection ([`shacl_json_schema_entries`]).
///
/// This is one of the sources [`registry_entries`] folds in — together with the
/// RDF↔GTS direction ledgers — to build the enumerable registry that
/// [`loss_matrix_json`] renders and [`registry`] indexes. Order is unspecified;
/// callers sort as needed.
fn transcode_and_shapes_entries() -> Vec<LossEntry> {
    let all_targets: Vec<&str> = SYNTAX_CODECS
        .iter()
        .chain(PROJECTION_CODECS.iter())
        .copied()
        .collect();

    let mut entries: Vec<LossEntry> = Vec::new();
    for &from in SYNTAX_CODECS {
        for &to in &all_targets {
            if from == to {
                continue;
            }
            let ledger = pair_loss_ledger(from, to);
            entries.extend_from_slice(ledger.entries());
        }
    }
    entries.extend(shacl_json_schema_entries());
    entries
}

// ── Enumerable loss registry ─────────────────────────────────────────────────

/// The SHACL → JSON Schema/OpenAPI shapes projection's closed loss profile:
/// each `(code, note)` pair `shapes::json_schema::Ctx::record`/the
/// value-vocabulary `loss_entry` builders use (`crates/shapes/src/json_schema.rs`),
/// all recorded via the shared runtime [`LossLedger`]. `sh:sparql` /
/// `sh:expression` (SHACL-AF constraints), property-level `sh:not`, and a
/// `rdfs:range` clash with a value-vocabulary projection are recorded via
/// `Ctx::record`; `sh:SPARQLTarget`-targeted shapes (`Target::Sparql` in
/// `crates/shapes/src/shapes.rs`) have no `$def` equivalent — the emitter has
/// no class extension to key a `$def` by — and are excluded from the compiled
/// schema, but (unlike a bare exclusion) each one records a `sh:SPARQLTarget`
/// loss on the shape's own subject rather than vanishing silently;
/// `value-vocabulary` covers an enum-with-no-members projection and
/// `value-vocabulary member` a dropped blank-node enum member. Mirrored here
/// (rather than depended-on from this crate) because `purrdf-core` never
/// depends on the `shapes` crate. The note text is behavioral (what is
/// dropped and why), never an issue/PR reference or a minted IRI.
const SHACL_JSON_SCHEMA_PROFILE: &[(&str, &str)] = &[
    (
        "rdfs:range",
        "A predicate's rdfs:range names a value-vocabulary class that conflicts with (or is \
         shadowed by) an explicit sh:class on the same property; the range-derived enum $ref is \
         suppressed in favor of sh:class, and the conflict is recorded rather than silently \
         dropped.",
    ),
    (
        "sh:SPARQLTarget",
        "A shape targeted only via SHACL-AF sh:target/sh:SPARQLTarget (an arbitrary SPARQL \
         SELECT, not a class extension) has no closed-world JSON Schema equivalent; the shape is \
         excluded from the compiled $defs.",
    ),
    (
        "sh:expression",
        "A SHACL-AF sh:expression node-expression constraint has no JSON Schema equivalent and is \
         dropped.",
    ),
    (
        "sh:not",
        "A sh:not negation whose inner shape is not losslessly expressible as a JSON Schema \
         negation (or that appears at property/value position, where negating a base value \
         schema would be vacuous) is dropped rather than emitted unsoundly.",
    ),
    (
        "sh:sparql",
        "A SHACL-SPARQL constraint (sh:sparql) has no closed-world JSON Schema equivalent and is \
         dropped.",
    ),
    (
        "value-vocabulary",
        "A value-vocabulary class projected to an enum $def has no seeded named-individual \
         members to enumerate; the $def is emitted with an empty enum and the omission is \
         recorded.",
    ),
    (
        "value-vocabulary member",
        "A value-vocabulary class's member is identified by a blank node rather than a named \
         individual, so it has no stable CURIE to enumerate; the member is dropped from the \
         projected enum.",
    ),
];

/// Build the runtime-shaped [`LossEntry`] rows for the
/// `("shacl", "json-schema")` shapes profile from [`SHACL_JSON_SCHEMA_PROFILE`]
/// — one contract entry per declared `(code, note)` pair, `location: None`
/// (the registry enumerates the closed set of possible codes, not a specific
/// occurrence).
fn shacl_json_schema_entries() -> Vec<LossEntry> {
    SHACL_JSON_SCHEMA_PROFILE
        .iter()
        .map(|&(code, note)| LossEntry {
            code: Cow::Borrowed(code),
            from: Cow::Borrowed("shacl"),
            to: Cow::Borrowed("json-schema"),
            intentional: true,
            note: Cow::Borrowed(note),
            location: None,
        })
        .collect()
}

/// Extract the `&'static str` payload of a **contract**-discipline
/// [`LossEntry`] field (one built via [`Cow::Borrowed`]).
///
/// The registry only ever enumerates contract entries (the direction ledgers
/// and [`pair_loss_ledger`]); a `Cow::Owned` reaching here would mean a
/// runtime `record`-discipline entry leaked into the static registry, which is
/// a programming error — hard-fail, per the no-optionality doctrine.
///
/// Takes `&Cow<'static, str>` rather than `&str` on purpose (this is NOT the
/// usual over-indirection `clippy::ptr_arg` warns about): the whole point is
/// to inspect which *variant* the caller built, which a plain `&str` cannot
/// distinguish.
#[allow(clippy::ptr_arg)]
fn static_str(cow: &Cow<'static, str>) -> &'static str {
    match cow {
        Cow::Borrowed(s) => s,
        Cow::Owned(_) => unreachable!("registry codes are always static contract literals"),
    }
}

/// The single source of truth every enumerable-registry consumer reads: the
/// RDF↔GTS direction ledgers ([`rdf_to_gts_loss_ledger`] /
/// [`gts_to_rdf_loss_ledger`]) plus [`transcode_and_shapes_entries`] (the full
/// syntax/projection transcode matrix over `SYNTAX_CODECS` × `(SYNTAX_CODECS ∪
/// PROJECTION_CODECS)` AND the non-syntax shapes pair `("shacl", "json-schema")`).
///
/// [`loss_matrix_json`] renders these rows (with their `note` text intact) and
/// [`registry`] folds them into a `(from, to) -> codes` lookup table — both
/// derive from this ONE function, so the rendered matrix and the
/// `registered_pairs()`/`profile_for()` contract can never drift apart. Order
/// is unspecified; callers sort as needed.
fn registry_entries() -> Vec<LossEntry> {
    let mut entries: Vec<LossEntry> = Vec::new();
    entries.extend_from_slice(rdf_to_gts_loss_ledger().entries());
    entries.extend_from_slice(gts_to_rdf_loss_ledger().entries());
    entries.extend(transcode_and_shapes_entries());
    entries
}

/// The enumerable loss registry as deterministic JSON: every `(from, to)` pair
/// [`registered_pairs`] reports — the RDF↔GTS directions, every non-identity
/// syntax/projection transcode pair, and the `("shacl", "json-schema")` shapes
/// projection — rendered from [`registry_entries`] sorted by `(from, to, code)`.
/// Unlike a single [`LossLedger::contract`], codes are NOT assumed unique here
/// — the same code recurs for different `(from, to)` pairs.
///
/// The acceptance criterion this enumerator satisfies: the set of
/// `(from, to)` pairs this renders is exactly `registered_pairs()` (see the
/// `loss_matrix_json_pairs_match_registered_pairs` test). The rendered output
/// is committed at `generated/transcode-loss-matrix.json`. For the narrower
/// RDF↔GTS-only view committed at `generated/rdf-loss-matrix.json`, see
/// [`rdf_gts_loss_matrix_json`].
pub fn loss_matrix_json() -> String {
    let mut entries = registry_entries();

    // Sort by (from, to, code) for full determinism.
    entries.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.code.cmp(&b.code))
    });

    render(&entries, false)
}

/// The single source of truth backing [`registered_pairs`] and [`profile_for`]
/// — built by folding [`registry_entries`] (the same rows [`loss_matrix_json`]
/// renders) down to a `(from, to) -> codes` lookup, dropping the `note` text —
/// mapping every `(from, to)` pair this crate knows a loss profile for to the
/// closed set of codes that pair's conversion may drop.
///
/// [`registry_entries`] is pure, compile-time-derived data (the same rows on
/// every call), so the folded map is built once behind a [`OnceLock`] and
/// reused for the process lifetime — `profile_for` sits on the soundness
/// check's hot path (once per code checked), and `registered_pairs`/
/// [`loss_matrix_json`]'s registry rendering both want the same table, so
/// rebuilding a fresh `BTreeMap` on every call was pure wasted allocation.
fn registry() -> &'static BTreeMap<(&'static str, &'static str), BTreeSet<&'static str>> {
    static REGISTRY: OnceLock<BTreeMap<(&'static str, &'static str), BTreeSet<&'static str>>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut table: BTreeMap<(&'static str, &'static str), BTreeSet<&'static str>> =
            BTreeMap::new();
        for entry in &registry_entries() {
            table
                .entry((static_str(&entry.from), static_str(&entry.to)))
                .or_default()
                .insert(static_str(&entry.code));
        }
        table
    })
}

/// Every `(from, to)` pair with a registered loss profile: the RDF↔GTS
/// directions, every non-identity syntax/projection transcode pair, and the
/// `("shacl", "json-schema")` shapes projection.
pub fn registered_pairs() -> impl Iterator<Item = (&'static str, &'static str)> {
    registry().keys().copied()
}

/// The closed set of loss codes a `from -> to` conversion may drop, per the
/// enumerable registry.
///
/// Empty when the pair is unregistered (an identity pair, or one this crate
/// has no declared profile for) — never a panic, since callers may probe
/// arbitrary pairs to check whether a profile is known.
pub fn profile_for(from: &str, to: &str) -> BTreeSet<&'static str> {
    registry()
        .iter()
        .find(|((f, t), _)| *f == from && *t == to)
        .map(|(_, codes)| codes.clone())
        .unwrap_or_default()
}

// ── Sound + complete verification surface ────────────────────────────────────
//
// A runtime `record`-discipline [`LossLedger`] gives exactly three verifiable
// promises about one conversion, and every one of them is checkable without
// serde or any dependency beyond this module:
//
// - **lossless**: [`LossLedger::is_empty`] — nothing was dropped at all. No
//   dedicated helper is needed; this is the existing method.
// - **complete**: [`check_ledger_complete`]/[`assert_ledger_complete`] — the
//   set of codes `ledger` actually recorded exactly matches the caller's
//   declared `expected_codes`. A code missing from `ledger` means a
//   construct vanished silently instead of being declared; a code `ledger`
//   recorded that is NOT in `expected_codes` means the caller's declaration
//   has drifted behind reality (a new/changed loss nobody updated the test
//   for) — both directions are "incomplete" in the sense this check cares
//   about.
// - **sound**: [`check_ledger_sound`]/[`assert_ledger_sound`] — every code the
//   ledger actually recorded is inside the declared [`profile_for`] contract
//   for that `(from, to)` pair. A recorded code OUTSIDE the profile is an
//   *unintentional* loss reaching the ledger — the live "this is a bug" case;
//   soundness is what gives [`LossEntry::intentional`] a real, checkable
//   meaning, rather than a field nobody verifies.
//
// Each check has a `Result`-returning core (composable, testable without
// unwinding) and a panicking `assert_*` wrapper (for direct use in tests and
// call-site invariants) — the crate's existing style for hard-fail checks
// (e.g. [`LossLedger::contract`]'s duplicate-code assertion).

/// Verify the set of codes `ledger` actually recorded exactly matches
/// `expected_codes` — "complete" meaning every expected dropped construct is
/// recorded (no silent loss) AND nothing outside `expected_codes` slipped in
/// unnoticed (no undeclared drift). Order and duplicate occurrences in
/// `ledger` are irrelevant; only the set of distinct codes is compared.
///
/// # Errors
///
/// Returns `Err` naming every code missing from `ledger` and/or every code
/// `ledger` recorded that is not in `expected_codes`, when the two sets
/// differ.
pub fn check_ledger_complete(ledger: &LossLedger, expected_codes: &[&str]) -> Result<(), String> {
    let present: BTreeSet<&str> = ledger.entries().iter().map(|e| e.code.as_ref()).collect();
    let expected: BTreeSet<&str> = expected_codes.iter().copied().collect();
    let missing: Vec<&str> = expected.difference(&present).copied().collect();
    let unexpected: Vec<&str> = present.difference(&expected).copied().collect();
    if missing.is_empty() && unexpected.is_empty() {
        return Ok(());
    }
    let mut reasons = Vec::new();
    if !missing.is_empty() {
        reasons.push(format!("expected code(s) {missing:?} were never recorded"));
    }
    if !unexpected.is_empty() {
        reasons.push(format!(
            "ledger recorded unexpected code(s) {unexpected:?} not in expected_codes"
        ));
    }
    Err(format!("loss ledger incomplete: {}", reasons.join("; ")))
}

/// Panicking wrapper over [`check_ledger_complete`]: the set of codes
/// `ledger` recorded MUST exactly match `expected_codes`.
///
/// # Panics
///
/// Panics with the same message [`check_ledger_complete`] would return as
/// `Err`, when the two code sets differ in either direction.
pub fn assert_ledger_complete(ledger: &LossLedger, expected_codes: &[&str]) {
    if let Err(message) = check_ledger_complete(ledger, expected_codes) {
        panic!("{message}");
    }
}

/// Verify every code `ledger` actually recorded is a member of
/// `profile_for(from, to)` — "sound" meaning nothing surprising reached the
/// ledger. A code outside the declared profile is flagged by name: it is an
/// undeclared, unintentional loss (a bug), never an accepted contract.
///
/// # Errors
///
/// Returns `Err` naming every offending code (and the declared profile) when
/// `ledger` carries at least one code outside `profile_for(from, to)`.
pub fn check_ledger_sound(ledger: &LossLedger, from: &str, to: &str) -> Result<(), String> {
    let profile = profile_for(from, to);
    let offenders: BTreeSet<&str> = ledger
        .entries()
        .iter()
        .map(|e| e.code.as_ref())
        .filter(|code| !profile.contains(code))
        .collect();
    if offenders.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "loss ledger unsound for (\"{from}\", \"{to}\"): code(s) {offenders:?} were \
             recorded but are not in the declared profile {profile:?} — a recorded code outside \
             the profile is an undeclared, unintentional loss (a bug), not an accepted contract"
        ))
    }
}

/// Panicking wrapper over [`check_ledger_sound`]: every code `ledger`
/// recorded MUST be a member of `profile_for(from, to)`.
///
/// # Panics
///
/// Panics with the same message [`check_ledger_sound`] would return as `Err`,
/// when `ledger` carries at least one code outside the declared profile.
pub fn assert_ledger_sound(ledger: &LossLedger, from: &str, to: &str) {
    if let Err(message) = check_ledger_sound(ledger, from, to) {
        panic!("{message}");
    }
}

/// Render already-sorted entries to deterministic JSON: 2-space indent, fixed
/// field order, trailing newline.
///
/// Hand-rolled to avoid pulling serde into the kernel rlib (the crate does not
/// depend on it). Codes are NOT assumed unique — callers sort first; this
/// function only renders. `emit_location` gates the `location` field: the
/// static-contract renders ([`rdf_gts_loss_matrix_json`], [`loss_matrix_json`])
/// pass `false` so the committed artifacts stay byte-identical (contract
/// entries never carry a location anyway); [`LossLedger::render_json`] passes
/// `true` so a runtime `record`-discipline location is visible.
fn render(entries: &[LossEntry], emit_location: bool) -> String {
    let mut out = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        out.push_str("  {\n");
        push_field(&mut out, "code", entry.code.as_ref(), false);
        push_field(&mut out, "from", entry.from.as_ref(), false);
        push_field(&mut out, "to", entry.to.as_ref(), false);
        push_bool_field(&mut out, "intentional", entry.intentional);
        let location = emit_location.then_some(entry.location.as_deref()).flatten();
        push_field(&mut out, "note", entry.note.as_ref(), location.is_none());
        if let Some(location) = location {
            push_field(&mut out, "location", &location.display(), true);
        }
        out.push_str("  }");
        if i + 1 < entries.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

/// Wrap an already-sorted runtime ledger's [`render`] output (with locations)
/// in the versioned envelope `{ "schema_version": 1, "losses": [ ... ] }` —
/// the schema [`LossLedger::render_json`] exposes. `schema_version` is a plain
/// integer bumped only on a breaking shape change to this envelope.
fn render_versioned(entries: &[LossEntry]) -> String {
    let array = render(entries, true);
    let mut out = String::from("{\n  \"schema_version\": 1,\n  \"losses\": ");
    for (i, line) in array.lines().enumerate() {
        if i > 0 {
            out.push('\n');
            out.push_str("  ");
        }
        out.push_str(line);
    }
    out.push_str("\n}\n");
    out
}

/// Append `  "key": "value",\n` (or no trailing comma when `last`).
fn push_field(out: &mut String, key: &str, value: &str, last: bool) {
    out.push_str("    \"");
    out.push_str(key);
    out.push_str("\": \"");
    escape_json_into(out, value);
    out.push('"');
    if !last {
        out.push(',');
    }
    out.push('\n');
}

/// Append `  "key": true|false,\n` (booleans are never the last field here).
fn push_bool_field(out: &mut String, key: &str, value: bool) {
    out.push_str("    \"");
    out.push_str(key);
    out.push_str("\": ");
    out.push_str(if value { "true" } else { "false" });
    out.push_str(",\n");
}

/// Escape a string per the JSON string grammar (RFC 8259) into `out`.
fn escape_json_into(out: &mut String, value: &str) {
    use std::fmt::Write as _;
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The intentional loss codes this ledger is required to enumerate.
    /// `direction-dropped` was retired once `Term.direction` round-tripped
    /// losslessly, and `multi-reifier-collapsed` once reifier-id-keyed
    /// `Graph.reifiers` did the same; neither is an accepted loss any more.
    const EXPECTED_CODES: [&str; 2] = ["blob-bytes-absent", "bnode-scope-flatten"];

    fn matrix_codes() -> Vec<String> {
        let mut codes: Vec<String> = Vec::new();
        for entry in rdf_to_gts_loss_ledger().entries() {
            codes.push(entry.code.to_string());
        }
        for entry in gts_to_rdf_loss_ledger().entries() {
            codes.push(entry.code.to_string());
        }
        codes.sort();
        codes
    }

    #[test]
    fn render_is_deterministic() {
        assert_eq!(rdf_gts_loss_matrix_json(), rdf_gts_loss_matrix_json());
        let ledger = rdf_to_gts_loss_ledger();
        assert_eq!(ledger.render_json(), ledger.render_json());
    }

    #[test]
    fn all_intentional_codes_present() {
        let codes = matrix_codes();
        for expected in EXPECTED_CODES {
            assert!(
                codes.iter().any(|c| c == expected),
                "missing intentional loss code `{expected}`"
            );
        }
        assert_eq!(codes.len(), EXPECTED_CODES.len(), "unexpected extra codes");
    }

    #[test]
    fn every_recorded_loss_is_intentional() {
        for entry in rdf_to_gts_loss_ledger().entries() {
            assert!(entry.intentional, "{} not marked intentional", entry.code);
        }
        for entry in gts_to_rdf_loss_ledger().entries() {
            assert!(entry.intentional, "{} not marked intentional", entry.code);
        }
    }

    #[test]
    fn directions_are_correct() {
        for entry in rdf_to_gts_loss_ledger().entries() {
            assert_eq!(entry.from, "rdf-1.2-dataset");
            assert_eq!(entry.to, "gts");
        }
        for entry in gts_to_rdf_loss_ledger().entries() {
            assert_eq!(entry.from, "gts");
            assert_eq!(entry.to, "rdf-1.2-dataset");
        }
    }

    #[test]
    fn is_empty_reflects_contents() {
        assert!(!rdf_to_gts_loss_ledger().is_empty());
        assert!(!gts_to_rdf_loss_ledger().is_empty());
        assert!(LossLedger::default().is_empty());
        assert!(LossLedger::contract(vec![]).is_empty());
        assert!(LossLedger::new().is_empty());
    }

    #[test]
    fn json_is_structurally_valid() {
        let json = rdf_gts_loss_matrix_json();
        // Deterministic shape: a JSON array with a trailing newline.
        assert!(json.starts_with("[\n"));
        assert!(json.ends_with("]\n"));
        // One object per intentional code, with each field key present once.
        assert_eq!(json.matches("\"code\":").count(), EXPECTED_CODES.len());
        assert_eq!(
            json.matches("\"intentional\": true").count(),
            EXPECTED_CODES.len()
        );
        for code in EXPECTED_CODES {
            assert!(
                json.contains(&format!("\"code\": \"{code}\"")),
                "missing {code}"
            );
        }
        // Sorted-by-code: codes appear in ascending order in the rendered text.
        let mut last = 0usize;
        for code in EXPECTED_CODES {
            let at = json.find(code).expect("code present");
            assert!(at >= last, "codes not sorted: {code}");
            last = at;
        }
    }

    #[test]
    fn json_escapes_control_characters() {
        let mut s = String::new();
        escape_json_into(&mut s, "a\"b\\c\nd\te\u{01}");
        assert_eq!(s, "a\\\"b\\\\c\\nd\\te\\u0001");
    }

    /// Drift gate: the committed artifact must byte-equal the freshly rendered
    /// matrix. Regenerate `generated/rdf-loss-matrix.json` when the ledger
    /// changes.
    #[test]
    fn generated_artifact_has_not_drifted() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("generated")
            .join("rdf-loss-matrix.json");
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(
            committed,
            rdf_gts_loss_matrix_json(),
            "generated/rdf-loss-matrix.json is stale; regenerate it from rdf_gts_loss_matrix_json()"
        );
    }

    #[test]
    fn loss_matrix_json_deterministic() {
        assert_eq!(loss_matrix_json(), loss_matrix_json());
    }

    #[test]
    fn transcode_matrix_has_not_drifted() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../generated/transcode-loss-matrix.json");
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("generated file missing: {}", path.display()));
        assert_eq!(
            on_disk,
            loss_matrix_json(),
            "transcode-loss-matrix.json has drifted"
        );
    }

    /// The registry matrix is backed by the same enumerable registry as
    /// [`profile_for`] (via [`registry_entries`]), so it must enumerate the
    /// non-syntax `("shacl", "json-schema")` shapes pair too — not just the
    /// syntax/projection codec pairs.
    #[test]
    fn transcode_matrix_includes_shacl_json_schema_pair() {
        let json = loss_matrix_json();
        assert!(
            json.contains("\"from\": \"shacl\""),
            "transcode-loss-matrix.json must include the shacl->json-schema pair"
        );
        assert!(json.contains("\"to\": \"json-schema\""));
        for (code, _) in SHACL_JSON_SCHEMA_PROFILE {
            assert!(
                json.contains(&format!("\"code\": \"{code}\"")),
                "transcode-loss-matrix.json missing shapes-profile code `{code}`"
            );
        }
    }

    #[test]
    fn pair_loss_identity_is_empty() {
        assert!(pair_loss_ledger("turtle", "turtle").is_empty());
        assert!(pair_loss_ledger("gts", "gts").is_empty());
    }

    #[test]
    fn pair_loss_known_lossy_pairs() {
        let trig_to_turtle = pair_loss_ledger("trig", "turtle");
        assert!(
            trig_to_turtle
                .entries()
                .iter()
                .any(|e| e.code == "named-graph-dropped")
        );

        let nquads_to_rdfxml = pair_loss_ledger("nquads", "rdfxml");
        assert!(
            nquads_to_rdfxml
                .entries()
                .iter()
                .any(|e| e.code == "named-graph-dropped")
        );
        assert!(
            nquads_to_rdfxml
                .entries()
                .iter()
                .any(|e| e.code == "rdf12-star-unrepresentable")
        );

        let turtle_to_jsonld = pair_loss_ledger("turtle", "jsonld");
        assert!(
            turtle_to_jsonld
                .entries()
                .iter()
                .any(|e| e.code == "rdf12-star-jsonld-rejected")
        );
    }

    #[test]
    #[should_panic(expected = "projection")]
    fn pair_loss_panics_on_projection_source() {
        let _ = pair_loss_ledger("owl-dl", "turtle");
    }

    #[test]
    fn rdf_gts_loss_matrix_json_is_deterministic_across_calls() {
        assert_eq!(rdf_gts_loss_matrix_json(), rdf_gts_loss_matrix_json());
    }

    /// The issue-named enumerator: the set of `(from, to)` pairs
    /// [`loss_matrix_json`] renders must equal `registered_pairs()` exactly —
    /// this is the executable proof that `loss_matrix_json()` enumerates
    /// precisely the registered pairs, no more and no fewer.
    #[test]
    fn loss_matrix_json_pairs_match_registered_pairs() {
        let json = loss_matrix_json();
        let mut rendered_pairs: BTreeSet<(String, String)> = BTreeSet::new();
        let mut lines = json.lines();
        while let Some(line) = lines.next() {
            if line.trim() != "{" {
                continue;
            }
            let mut from = None;
            let mut to = None;
            for field_line in lines.by_ref() {
                let trimmed = field_line.trim().trim_end_matches(',');
                if let Some(v) = trimmed.strip_prefix("\"from\": \"") {
                    from = Some(v.trim_end_matches('"').to_owned());
                } else if let Some(v) = trimmed.strip_prefix("\"to\": \"") {
                    to = Some(v.trim_end_matches('"').to_owned());
                }
                if trimmed == "}" {
                    break;
                }
            }
            rendered_pairs.insert((
                from.expect("every row has a `from` field"),
                to.expect("every row has a `to` field"),
            ));
        }

        let expected_pairs: BTreeSet<(String, String)> = registered_pairs()
            .map(|(from, to)| (from.to_owned(), to.to_owned()))
            .collect();

        assert_eq!(
            rendered_pairs, expected_pairs,
            "loss_matrix_json() must enumerate exactly registered_pairs()"
        );
    }

    #[test]
    fn runtime_ledger_records_owned_entries_with_location() {
        let mut ledger = LossLedger::new();
        assert!(ledger.is_empty());

        ledger.record(LossEntry {
            code: Cow::Owned("runtime-code-1".to_owned()),
            from: Cow::Owned("some-runtime-from".to_owned()),
            to: Cow::Owned("some-runtime-to".to_owned()),
            intentional: true,
            note: Cow::Owned("a runtime-recorded loss, owned end to end".to_owned()),
            location: Some(Box::new(
                RdfLocation::logical("shapes:compile").with_subject("ex:Cat"),
            )),
        });
        assert!(!ledger.is_empty());

        let rendered = ledger.render_json();
        assert_eq!(
            rendered,
            ledger.render_json(),
            "render_json must be deterministic"
        );
        assert!(
            rendered.contains("\"location\":"),
            "location field missing: {rendered}"
        );
        assert!(
            rendered.contains("subject=ex:Cat"),
            "subject not present in rendered location: {rendered}"
        );
    }

    #[test]
    fn registered_pairs_includes_shacl_json_schema() {
        assert!(
            registered_pairs().any(|(from, to)| from == "shacl" && to == "json-schema"),
            "registered_pairs() must include (\"shacl\", \"json-schema\")"
        );
    }

    /// `registered_pairs()` must yield the SAME ordered sequence across two
    /// independent calls (it is backed by a `BTreeMap`, so this is order, not
    /// merely set-equality) — callers rendering it directly (e.g. a diagnostic
    /// listing) must not observe run-to-run reordering.
    #[test]
    fn registered_pairs_order_is_stable_across_calls() {
        let first: Vec<(&str, &str)> = registered_pairs().collect();
        let second: Vec<(&str, &str)> = registered_pairs().collect();
        assert_eq!(
            first, second,
            "registered_pairs() must return a stable, deterministic order"
        );
        assert!(
            first.contains(&("shacl", "json-schema")),
            "registered_pairs() must include (\"shacl\", \"json-schema\"): {first:?}"
        );
    }

    #[test]
    fn profile_for_shacl_json_schema_includes_sparql() {
        let profile = profile_for("shacl", "json-schema");
        assert!(
            profile.contains("sh:sparql"),
            "profile_for(\"shacl\", \"json-schema\") missing sh:sparql: {profile:?}"
        );
    }

    #[test]
    fn profile_for_trig_turtle_includes_named_graph_dropped() {
        let profile = profile_for("trig", "turtle");
        assert!(
            profile.contains("named-graph-dropped"),
            "profile_for(\"trig\", \"turtle\") missing named-graph-dropped: {profile:?}"
        );
    }

    /// Build a hand-rolled, owned runtime [`LossEntry`] for the mechanical
    /// helper tests below — no `RdfLocation`, since these tests exercise only
    /// `code`/`from`/`to`, never a subject.
    fn owned_entry(code: &str, from: &str, to: &str) -> LossEntry {
        LossEntry {
            code: Cow::Owned(code.to_owned()),
            from: Cow::Owned(from.to_owned()),
            to: Cow::Owned(to.to_owned()),
            intentional: true,
            note: Cow::Owned("mechanical test entry".to_owned()),
            location: None,
        }
    }

    #[test]
    fn check_ledger_complete_passes_when_all_expected_codes_present() {
        let mut ledger = LossLedger::new();
        ledger.record(owned_entry("a", "x", "y"));
        ledger.record(owned_entry("b", "x", "y"));
        assert!(check_ledger_complete(&ledger, &["a", "b"]).is_ok());
        // Order/duplicates in the ledger are irrelevant to completeness.
        ledger.record(owned_entry("a", "x", "y"));
        assert!(check_ledger_complete(&ledger, &["b", "a"]).is_ok());
    }

    #[test]
    fn check_ledger_complete_flags_missing_code() {
        let mut ledger = LossLedger::new();
        ledger.record(owned_entry("a", "x", "y"));
        let err =
            check_ledger_complete(&ledger, &["a", "b"]).expect_err("code `b` was never recorded");
        assert!(err.contains('b'), "error must name the missing code: {err}");
    }

    #[test]
    fn check_ledger_complete_flags_unexpected_extra_code() {
        // The other incompleteness direction: `ledger` recorded a real code
        // that the caller's `expected_codes` never declared — a silently
        // drifted contract, not merely a silently dropped construct.
        let mut ledger = LossLedger::new();
        ledger.record(owned_entry("a", "x", "y"));
        ledger.record(owned_entry("b", "x", "y"));
        let err = check_ledger_complete(&ledger, &["a"])
            .expect_err("code `b` was recorded but not declared in expected_codes");
        assert!(
            err.contains('b'),
            "error must name the unexpected code: {err}"
        );
    }

    #[test]
    #[should_panic(expected = "b")]
    fn assert_ledger_complete_panics_on_missing_code() {
        let ledger = LossLedger::new();
        assert_ledger_complete(&ledger, &["a", "b"]);
    }

    #[test]
    fn check_ledger_sound_passes_for_a_known_contract_ledger() {
        // Every code the real rdf-1.2-dataset -> gts contract ledger records is,
        // by construction, in its own declared profile.
        let ledger = rdf_to_gts_loss_ledger();
        assert!(check_ledger_sound(&ledger, "rdf-1.2-dataset", "gts").is_ok());
    }

    /// Soundness RED case: a ledger carrying a code OUTSIDE the declared
    /// `("shacl", "json-schema")` profile must be flagged by name. No real
    /// `purrdf_shapes::json_schema::compile` path produces an out-of-profile
    /// code — that is the point of soundness — so this is a hand-built ledger.
    #[test]
    fn check_ledger_sound_flags_out_of_profile_code() {
        let mut ledger = LossLedger::new();
        ledger.record(owned_entry("not-a-real-code", "shacl", "json-schema"));
        let err = check_ledger_sound(&ledger, "shacl", "json-schema")
            .expect_err("`not-a-real-code` is outside the declared profile");
        assert!(
            err.contains("not-a-real-code"),
            "error must name the offending code: {err}"
        );
    }

    #[test]
    #[should_panic(expected = "not-a-real-code")]
    fn assert_ledger_sound_panics_on_out_of_profile_code() {
        let mut ledger = LossLedger::new();
        ledger.record(owned_entry("not-a-real-code", "shacl", "json-schema"));
        assert_ledger_sound(&ledger, "shacl", "json-schema");
    }
}
