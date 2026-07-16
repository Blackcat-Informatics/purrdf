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
//! shapes projection, the `json-schema`→`pydantic-v2` code-generation profile,
//! the `json-schema`→`linkml-1.11` schema projection,
//! the `json-schema`→`typescript-7.0` declaration projection,
//! the `json-schema`→`graphql-september-2025` type-system projection,
//! the bidirectional RDF 1.2 dataset↔OKF profile, the bidirectional RDF↔LPG
//! semantic lowering, and the OBO Graphs/SKOS views;
//! [`loss_matrix_json`] renders that same enumerable registry.

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
        note: Cow::Borrowed(
            "`purrdf_gts::reader::read()` folds all segments into one term table, collapsing \
             per-segment blank-node scope; the distinct scopes are recovered only via the \
             streaming-event importer.",
        ),
        location: None,
    }])
}

/// The closed loss contract for projecting an arbitrary RDF 1.2 dataset to an
/// OKF Markdown bundle.
///
/// OKF is a deliberately narrow authoring profile. Profile quads in the default
/// graph and its derived Markdown-link statement layer are representable; named
/// graph placement, other RDF/OWL statements, and unrelated RDF 1.2
/// reifier/annotation rows are not. The native OKF writer records only the codes
/// that occur for the dataset being written, while this contract enumerates every
/// code it is permitted to record.
pub fn rdf_to_okf_loss_ledger() -> LossLedger {
    LossLedger::contract(vec![
        LossEntry {
            code: Cow::Borrowed("named-graph-dropped"),
            from: Cow::Borrowed("rdf-1.2-dataset"),
            to: Cow::Borrowed("okf"),
            note: Cow::Borrowed(
                "OKF documents have no named-graph placement; a quad asserted outside the \
                 default graph cannot be represented in Markdown frontmatter or body text.",
            ),
            location: None,
        },
        LossEntry {
            code: Cow::Borrowed("okf-annotation-dropped"),
            from: Cow::Borrowed("rdf-1.2-dataset"),
            to: Cow::Borrowed("okf"),
            note: Cow::Borrowed(
                "An RDF 1.2 annotation outside the exact caller-configured OKF Markdown-link \
                 profile has no OKF representation and is omitted from the bundle.",
            ),
            location: None,
        },
        LossEntry {
            code: Cow::Borrowed("okf-non-profile-quad-dropped"),
            from: Cow::Borrowed("rdf-1.2-dataset"),
            to: Cow::Borrowed("okf"),
            note: Cow::Borrowed(
                "An RDF statement outside the caller-configured OKF profile, including an \
                 OWL axiom, has no Markdown/frontmatter field and is omitted from the bundle.",
            ),
            location: None,
        },
        LossEntry {
            code: Cow::Borrowed("okf-reifier-dropped"),
            from: Cow::Borrowed("rdf-1.2-dataset"),
            to: Cow::Borrowed("okf"),
            note: Cow::Borrowed(
                "An RDF 1.2 reifier outside the exact caller-configured OKF Markdown-link \
                 profile has no OKF representation and is omitted from the bundle.",
            ),
            location: None,
        },
    ])
}

/// The closed loss contract for lifting an OKF Markdown bundle into an RDF 1.2
/// event stream.
///
/// Frontmatter-less `index.md` files are navigation-only pages: their body is not
/// attached to an OKF concept and therefore does not enter the RDF profile. The
/// reader records each skipped page with this code and a file location.
pub fn okf_to_rdf_loss_ledger() -> LossLedger {
    LossLedger::contract(vec![LossEntry {
        code: Cow::Borrowed("okf-navigation-page-dropped"),
        from: Cow::Borrowed("okf"),
        to: Cow::Borrowed("rdf-1.2-dataset"),
        note: Cow::Borrowed(
            "A frontmatter-less index.md navigation page has no OKF concept subject and is \
             omitted from the RDF profile; its path is recorded on the runtime loss entry.",
        ),
        location: None,
    }])
}

/// RDF→LPG loss code: an RDF predicate/object statement becomes a native property
/// graph edge or property whose target data model has no RDF model-theoretic meaning.
pub const LOSS_LPG_EDGE_SEMANTICS_LOWERED: &str = "lpg-edge-semantics-lowered";
/// RDF→LPG loss code: `rdf:type` becomes an LPG label.
pub const LOSS_LPG_TYPE_SEMANTICS_LOWERED: &str = "lpg-type-semantics-lowered";
/// RDF→LPG loss code: an RDF literal becomes an LPG property value.
pub const LOSS_LPG_LITERAL_SEMANTICS_LOWERED: &str = "lpg-literal-semantics-lowered";
/// RDF→LPG loss code: blank-node identity/scope moves to RDF sideband.
pub const LOSS_LPG_BLANK_SCOPE_SIDEBAND: &str = "lpg-blank-scope-sideband";
/// RDF→LPG loss code: named-graph placement moves to RDF sideband.
pub const LOSS_LPG_NAMED_GRAPH_SIDEBAND: &str = "lpg-named-graph-sideband";
/// RDF→LPG loss code: an RDF 1.2 triple term moves to structural sideband.
pub const LOSS_LPG_TRIPLE_TERM_SIDEBAND: &str = "lpg-triple-term-sideband";
/// RDF→LPG loss code: an RDF 1.2 reifier binding moves to structural sideband.
pub const LOSS_LPG_REIFIER_SIDEBAND: &str = "lpg-reifier-sideband";
/// RDF→LPG loss code: an RDF 1.2 annotation moves to structural sideband.
pub const LOSS_LPG_ANNOTATION_SIDEBAND: &str = "lpg-annotation-sideband";

/// LPG→RDF loss code: a native node identifier is interpreted under caller policy.
pub const LOSS_LPG_NODE_ID_INTERPRETED: &str = "lpg-node-id-interpreted";
/// LPG→RDF loss code: a native LPG label is interpreted as an RDF class.
pub const LOSS_LPG_LABEL_INTERPRETED: &str = "lpg-label-interpreted";
/// LPG→RDF loss code: a native edge type is interpreted as an RDF predicate.
pub const LOSS_LPG_EDGE_TYPE_INTERPRETED: &str = "lpg-edge-type-interpreted";
/// LPG→RDF loss code: a native property key is interpreted as an RDF predicate.
pub const LOSS_LPG_PROPERTY_KEY_INTERPRETED: &str = "lpg-property-key-interpreted";
/// LPG→RDF loss code: a native scalar/list value is interpreted as an RDF term.
pub const LOSS_LPG_VALUE_INTERPRETED: &str = "lpg-value-interpreted";
/// LPG→RDF loss code: native edge identity has no RDF statement identity by itself.
pub const LOSS_LPG_EDGE_ID_DROPPED: &str = "lpg-edge-id-dropped";

/// RDF→OBO Graphs loss code: named-graph placement is absent from OBO Graphs.
pub const LOSS_OBO_NAMED_GRAPH_DROPPED: &str = "obo-named-graph-dropped";
/// RDF→OBO Graphs loss code: a statement outside the configured OBO/OWL profile is omitted.
pub const LOSS_OBO_NON_PROFILE_STATEMENT_DROPPED: &str = "obo-non-profile-statement-dropped";
/// RDF→OBO Graphs loss code: blank-node identity cannot be retained as a stable OBO id.
pub const LOSS_OBO_BLANK_IDENTITY_DROPPED: &str = "obo-blank-identity-dropped";
/// RDF→OBO Graphs loss code: a literal facet exceeds the OBO Graphs scalar surface.
pub const LOSS_OBO_LITERAL_FIDELITY_WIDENED: &str = "obo-literal-fidelity-widened";
/// RDF→OBO Graphs loss code: an RDF 1.2 triple term has no OBO Graphs term slot.
pub const LOSS_OBO_TRIPLE_TERM_DROPPED: &str = "obo-triple-term-dropped";
/// RDF→OBO Graphs loss code: a reifier binding is not retained.
pub const LOSS_OBO_REIFIER_DROPPED: &str = "obo-reifier-dropped";
/// RDF→OBO Graphs loss code: a statement annotation is not retained.
pub const LOSS_OBO_ANNOTATION_DROPPED: &str = "obo-annotation-dropped";

/// RDF→SKOS loss code: named-graph placement is absent from the selected SKOS view.
pub const LOSS_SKOS_NAMED_GRAPH_DROPPED: &str = "skos-named-graph-dropped";
/// RDF→SKOS loss code: a statement outside the configured concept profile is omitted.
pub const LOSS_SKOS_NON_PROFILE_STATEMENT_DROPPED: &str = "skos-non-profile-statement-dropped";
/// RDF→SKOS loss code: a blank-node resource lacks stable concept identity.
pub const LOSS_SKOS_BLANK_IDENTITY_DROPPED: &str = "skos-blank-identity-dropped";
/// RDF→SKOS loss code: an RDF 1.2 triple term is outside the concept view.
pub const LOSS_SKOS_TRIPLE_TERM_DROPPED: &str = "skos-triple-term-dropped";
/// RDF→SKOS loss code: a reifier binding is outside the concept view.
pub const LOSS_SKOS_REIFIER_DROPPED: &str = "skos-reifier-dropped";
/// RDF→SKOS loss code: a statement annotation is outside the concept view.
pub const LOSS_SKOS_ANNOTATION_DROPPED: &str = "skos-annotation-dropped";

const RDF_LPG_PROFILE: &[(&str, &str)] = &[
    (
        LOSS_LPG_ANNOTATION_SIDEBAND,
        "An RDF 1.2 statement annotation has no native property-graph semantics. Its exact term, \
         predicate, object, and graph remain in the canonical RDF sideband for reversal.",
    ),
    (
        LOSS_LPG_BLANK_SCOPE_SIDEBAND,
        "A blank node's RDF label scope has no native property-graph equivalent. Exact label and \
         scope remain in the canonical RDF identity sideband.",
    ),
    (
        LOSS_LPG_EDGE_SEMANTICS_LOWERED,
        "An RDF predicate/object statement is lowered to an LPG edge or property. Full RDF term \
         identity remains in sideband, but an LPG consumer does not inherit RDF semantics.",
    ),
    (
        LOSS_LPG_LITERAL_SEMANTICS_LOWERED,
        "An RDF literal is lowered to an LPG property value. Lexical form, datatype, language, and \
         RDF 1.2 base direction remain in sideband for exact reversal.",
    ),
    (
        LOSS_LPG_NAMED_GRAPH_SIDEBAND,
        "Property graphs have no RDF named-graph slot. Exact graph placement remains in sideband \
         but is invisible to native LPG graph semantics.",
    ),
    (
        LOSS_LPG_REIFIER_SIDEBAND,
        "An RDF 1.2 reifier binding has no native property-graph construct. It remains in canonical \
         structural sideband for exact reversal.",
    ),
    (
        LOSS_LPG_TRIPLE_TERM_SIDEBAND,
        "An RDF 1.2 quoted triple term has no native LPG scalar or node kind. Its recursive RDF \
         value remains in canonical structural sideband.",
    ),
    (
        LOSS_LPG_TYPE_SEMANTICS_LOWERED,
        "An rdf:type statement is lowered to an LPG label under caller-supplied vocabulary. Exact \
         RDF identity remains in sideband, while native label semantics are implementation-defined.",
    ),
];

const LPG_RDF_PROFILE: &[(&str, &str)] = &[
    (
        LOSS_LPG_EDGE_ID_DROPPED,
        "A native LPG edge id has no RDF statement identity unless caller configuration maps it to \
         a reifier; an unmapped edge id is omitted while the edge statement is retained.",
    ),
    (
        LOSS_LPG_EDGE_TYPE_INTERPRETED,
        "A native LPG edge type is interpreted as an RDF predicate under mandatory caller mapping.",
    ),
    (
        LOSS_LPG_LABEL_INTERPRETED,
        "A native LPG label is interpreted as an RDF class under mandatory caller mapping.",
    ),
    (
        LOSS_LPG_NODE_ID_INTERPRETED,
        "A native LPG node id without canonical RDF identity sideband is interpreted as an RDF IRI \
         or blank node under mandatory caller policy.",
    ),
    (
        LOSS_LPG_PROPERTY_KEY_INTERPRETED,
        "A native LPG property key is interpreted as an RDF predicate under mandatory caller mapping.",
    ),
    (
        LOSS_LPG_VALUE_INTERPRETED,
        "A native LPG scalar or list without canonical RDF term sideband is interpreted as one or \
         more RDF terms under mandatory caller policy.",
    ),
];

const RDF_OBO_GRAPHS_PROFILE: &[(&str, &str)] = &[
    (
        LOSS_OBO_ANNOTATION_DROPPED,
        "An RDF 1.2 statement annotation outside an exact configured OBO metadata slot has no OBO \
         Graphs representation and is omitted.",
    ),
    (
        LOSS_OBO_BLANK_IDENTITY_DROPPED,
        "A blank-node resource has no stable OBO node id; profile statements rooted at that blank \
         node are omitted rather than assigned a fabricated IRI.",
    ),
    (
        LOSS_OBO_LITERAL_FIDELITY_WIDENED,
        "An RDF literal facet not carried by the selected OBO Graphs scalar field is widened to the \
         representable lexical value, with the source location recorded.",
    ),
    (
        LOSS_OBO_NAMED_GRAPH_DROPPED,
        "OBO Graphs 0.3.2 has no RDF named-graph placement; the selected graph content may remain \
         while its graph-name slot is omitted.",
    ),
    (
        LOSS_OBO_NON_PROFILE_STATEMENT_DROPPED,
        "An RDF statement outside the mandatory caller-configured OBO/OWL role profile has no OBO \
         Graphs object and is omitted.",
    ),
    (
        LOSS_OBO_REIFIER_DROPPED,
        "An RDF 1.2 reifier binding outside an exact OBO axiom-metadata mapping has no OBO Graphs \
         representation and is omitted.",
    ),
    (
        LOSS_OBO_TRIPLE_TERM_DROPPED,
        "An RDF 1.2 quoted triple term has no OBO Graphs 0.3.2 term kind and is omitted from the view.",
    ),
];

const RDF_SKOS_PROFILE: &[(&str, &str)] = &[
    (
        LOSS_SKOS_ANNOTATION_DROPPED,
        "An RDF 1.2 statement annotation has no configured SKOS concept-view role and is omitted.",
    ),
    (
        LOSS_SKOS_BLANK_IDENTITY_DROPPED,
        "A blank-node resource cannot serve as a stable concept identity under the configured SKOS \
         view and is omitted rather than assigned a fabricated IRI.",
    ),
    (
        LOSS_SKOS_NAMED_GRAPH_DROPPED,
        "The selected SKOS concept-scheme view does not carry source named-graph placement; the \
         mapped statement may remain while its graph-name slot is omitted.",
    ),
    (
        LOSS_SKOS_NON_PROFILE_STATEMENT_DROPPED,
        "An RDF statement outside the caller-configured SKOS concept, label, hierarchy, note, \
         mapping, membership, and top-concept roles is omitted from the view.",
    ),
    (
        LOSS_SKOS_REIFIER_DROPPED,
        "An RDF 1.2 reifier binding has no configured SKOS concept-view role and is omitted.",
    ),
    (
        LOSS_SKOS_TRIPLE_TERM_DROPPED,
        "An RDF 1.2 quoted triple term is outside the SKOS concept-view term surface and is omitted.",
    ),
];

fn contract_profile(
    from: &'static str,
    to: &'static str,
    profile: &'static [(&'static str, &'static str)],
) -> LossLedger {
    LossLedger::contract(
        profile
            .iter()
            .map(|&(code, note)| LossEntry {
                code: Cow::Borrowed(code),
                from: Cow::Borrowed(from),
                to: Cow::Borrowed(to),
                note: Cow::Borrowed(note),
                location: None,
            })
            .collect(),
    )
}

/// Closed RDF 1.2 dataset→canonical LPG semantic-lowering contract.
pub fn rdf_to_lpg_loss_ledger() -> LossLedger {
    contract_profile("rdf-1.2-dataset", "lpg", RDF_LPG_PROFILE)
}

/// Closed canonical/native LPG→RDF 1.2 interpretation contract.
pub fn lpg_to_rdf_loss_ledger() -> LossLedger {
    contract_profile("lpg", "rdf-1.2-dataset", LPG_RDF_PROFILE)
}

/// Closed RDF 1.2 dataset→OBO Graphs 0.3.2 view contract.
pub fn rdf_to_obo_graphs_loss_ledger() -> LossLedger {
    contract_profile(
        "rdf-1.2-dataset",
        "obo-graphs-0.3.2",
        RDF_OBO_GRAPHS_PROFILE,
    )
}

/// Closed RDF 1.2 dataset→SKOS concept-scheme view contract.
pub fn rdf_to_skos_loss_ledger() -> LossLedger {
    contract_profile("rdf-1.2-dataset", "skos", RDF_SKOS_PROFILE)
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
/// `sh:targetNode` / `sh:targetSubjectsOf` / `sh:targetObjectsOf`-targeted
/// shapes (`Target::Node` / `Target::SubjectsOf` / `Target::ObjectsOf`) are
/// the same story — none of the three is a class extension, so none can key a
/// `$def`, and each records its own loss instead of vanishing (a shape's
/// `Target::ImplicitClass` — the shape node is itself `rdfs:Class` — IS a
/// class extension and genuinely gets a `$def`, so it records no loss);
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
        "sh:targetNode",
        "A shape targeted only via sh:targetNode selects specific focus nodes, not a class \
         extension; it has no closed-world JSON Schema $def and its constraints are not \
         enforced by the emitted schema.",
    ),
    (
        "sh:targetObjectsOf",
        "A shape targeted only via sh:targetObjectsOf selects focus nodes by a predicate's \
         object position, not a class extension; no closed-world JSON Schema $def can be keyed \
         and its constraints are not enforced.",
    ),
    (
        "sh:targetSubjectsOf",
        "A shape targeted only via sh:targetSubjectsOf selects focus nodes by a predicate's \
         subject position, not a class extension; no closed-world JSON Schema $def can be keyed \
         and its constraints are not enforced.",
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

/// The JSON Schema → Pydantic v2 emitter's closed loss profile.  The emitter
/// preserves the source validation schema on Pydantic's JSON-schema surface,
/// but a Python annotation cannot enforce every JSON Schema vocabulary rule at
/// model-validation time.  Each runtime widening is therefore explicit and
/// located rather than hidden behind `Any` or a permissive scalar.
const JSON_SCHEMA_PYDANTIC_PROFILE: &[(&str, &str)] = &[
    (
        "array-contains-validation-dropped",
        "JSON Schema contains/minContains/maxContains constraints have no direct Pydantic v2 \
         field-annotation equivalent; the generated model validates the array container and \
         item type but does not enforce the contains predicate at runtime.",
    ),
    (
        "conditional-validation-dropped",
        "JSON Schema if/then/else dependent validation has no direct Pydantic v2 type-annotation \
         equivalent; the conditional is preserved in model_json_schema() but is not enforced by \
         model validation.",
    ),
    (
        "format-validation-widened",
        "A JSON Schema string format has no exact dependency-free Pydantic v2 standard-library \
         type; the generated runtime annotation accepts a strict string while \
         model_json_schema() retains the format.",
    ),
    (
        "inline-object-validation-widened",
        "A Pydantic model or TypedDict with named properties cannot enforce a JSON \
         Schema-valued additionalProperties assertion on every arbitrary extra value; named \
         fields and extra-key policy remain enforced while model_json_schema() retains the \
         complete object schema.",
    ),
    (
        "intersection-validation-widened",
        "JSON Schema allOf, or anyOf combined conjunctively with structural siblings, has no \
         exact general Pydantic v2 type annotation; runtime validation uses the representable \
         branch information while model_json_schema() retains the complete intersection.",
    ),
    (
        "keyword-validation-dropped",
        "A JSON Schema assertion keyword outside the emitter's closed annotation grammar cannot \
         be enforced by the generated Pydantic v2 runtime type; it remains present on \
         model_json_schema() and is recorded at its schema location.",
    ),
    (
        "negation-validation-dropped",
        "JSON Schema not has no direct Pydantic v2 type-annotation equivalent; the generated \
         runtime annotation validates the positive carrier type while model_json_schema() \
         retains the negation.",
    ),
    (
        "one-of-validation-widened",
        "Pydantic v2 unions implement any-branch semantics and cannot enforce JSON Schema \
         oneOf's exactly-one matching rule; runtime validation uses a union while model_json_schema() \
         retains oneOf.",
    ),
];

/// The JSON Schema → LinkML 1.11 emitter's closed loss profile. LinkML carries
/// the representable schema structure directly, while each JSON Schema
/// assertion without an exact LinkML 1.11 metamodel expression is widened or
/// omitted explicitly at its JSON Pointer location.
const JSON_SCHEMA_LINKML_PROFILE: &[(&str, &str)] = &[
    (
        "additional-properties-validation-widened",
        "LinkML 1.11 has no per-class equivalent for JSON Schema's \
         additionalProperties policy or schema. Named attributes retain their constraints, but \
         acceptance of unlisted object keys is widened and recorded on the object schema.",
    ),
    (
        "array-contains-validation-dropped",
        "JSON Schema contains/minContains/maxContains assertions have no LinkML 1.11 slot \
         expression. List item and cardinality constraints remain, while the contains predicate \
         and its match-count bounds are omitted.",
    ),
    (
        "conditional-validation-dropped",
        "JSON Schema if/then/else conditional validation has no LinkML 1.11 expression. \
         Independently representable carrier constraints remain, while the conditional \
         relationship is omitted.",
    ),
    (
        "dependency-validation-dropped",
        "JSON Schema dependentRequired/dependentSchemas validation has no LinkML 1.11 \
         expression. Attribute constraints remain, while cross-property dependencies are \
         omitted.",
    ),
    (
        "exclusive-bound-validation-widened",
        "LinkML 1.11 exposes inclusive minimum_value/maximum_value bounds but no exclusive \
         numeric bounds. An exclusive JSON Schema bound is projected as the corresponding \
         inclusive bound, widening acceptance at the boundary value.",
    ),
    (
        "format-validation-widened",
        "A JSON Schema format does not have an exact, uniformly enforced LinkML 1.11 \
         equivalent. The scalar carrier remains constrained and the format name is retained \
         when representable, while validation semantics are widened.",
    ),
    (
        "keyword-validation-dropped",
        "A JSON Schema assertion outside the emitter's closed LinkML 1.11 capability table has \
         no sound projection. The assertion is omitted and recorded at its schema location \
         rather than disappearing silently.",
    ),
    (
        "multiple-of-validation-dropped",
        "JSON Schema multipleOf has no LinkML 1.11 numeric expression. The numeric carrier and \
         other representable bounds remain, while divisibility validation is omitted.",
    ),
    (
        "non-scalar-enum-validation-widened",
        "LinkML 1.11 permissible values are scalar identifiers and cannot encode arbitrary JSON \
         object or array enum members. The representable carrier remains, while non-scalar \
         membership validation is widened.",
    ),
    (
        "property-count-validation-dropped",
        "JSON Schema minProperties/maxProperties assertions have no LinkML 1.11 class \
         expression. Named attributes and their requiredness remain, while total property-count \
         validation is omitted.",
    ),
    (
        "string-length-validation-dropped",
        "JSON Schema minLength/maxLength assertions have no LinkML 1.11 slot expression. The \
         string carrier and representable pattern remain, while code-point length validation is \
         omitted.",
    ),
    (
        "tuple-array-validation-widened",
        "LinkML 1.11 lists are homogeneous and cannot preserve JSON Schema prefixItems or \
         position-specific item schemas. The list is projected to a sound common item carrier, \
         widening position-specific validation.",
    ),
    (
        "unevaluated-validation-dropped",
        "JSON Schema unevaluatedProperties/unevaluatedItems assertions depend on applicator \
         evaluation state that LinkML 1.11 does not expose. Representable local constraints \
         remain, while the unevaluated assertion is omitted.",
    ),
];

/// The JSON Schema → TypeScript 7.0 declaration emitter's closed loss profile.
/// The emitter targets the fixed `strict` + `exactOptionalPropertyTypes`
/// assignability relation over JSON values. TypeScript declarations preserve
/// the representable carrier graph, while every JSON Schema assertion without
/// an exact structural type expression is widened or omitted explicitly at its
/// JSON Pointer location.
const JSON_SCHEMA_TYPESCRIPT_PROFILE: &[(&str, &str)] = &[
    (
        "additional-properties-validation-widened",
        "TypeScript uses structural object compatibility and index signatures apply to every \
         string key, so it cannot constrain only JSON object properties not named by the schema. \
         Named properties retain their exact declarations while the extra-property policy or \
         value schema is widened.",
    ),
    (
        "array-cardinality-validation-widened",
        "A JSON Schema minItems/maxItems assertion exceeds the declaration emitter's fixed \
         tuple-expansion budget. Item and tuple-prefix carriers remain, while exact length \
         validation is widened to keep declaration size bounded deterministically.",
    ),
    (
        "array-contains-validation-dropped",
        "JSON Schema contains/minContains/maxContains assertions quantify matching array \
         elements and have no TypeScript assignability equivalent. Item and cardinality \
         declarations remain while the contains predicate and match count are omitted.",
    ),
    (
        "conditional-validation-dropped",
        "JSON Schema if/then/else validates one branch according to runtime instance content; a \
         general conditional relationship has no finite TypeScript declaration equivalent. \
         Independently representable carrier constraints remain.",
    ),
    (
        "dependency-validation-dropped",
        "JSON Schema dependentRequired/dependentSchemas assertions impose cross-property runtime \
         relationships that a general structural TypeScript declaration cannot enforce. \
         Individual property declarations remain.",
    ),
    (
        "integer-validation-widened",
        "TypeScript has one number type and cannot distinguish all JSON integers from fractional \
         numbers. The generated declaration retains the number carrier while integer-only \
         validation is widened.",
    ),
    (
        "keyword-validation-dropped",
        "A JSON Schema assertion outside the emitter's closed TypeScript 7.0 capability table has \
         no sound declaration projection. The assertion is omitted and recorded at its schema \
         location rather than disappearing silently.",
    ),
    (
        "negation-validation-dropped",
        "General JSON Schema not is a set complement over instance validation. TypeScript's \
         Exclude utility distributes over union members and cannot express that general \
         complement, so the positive carrier remains while negation is omitted.",
    ),
    (
        "numeric-validation-dropped",
        "JSON Schema minimum/maximum/exclusive bounds and multipleOf are runtime numeric \
         predicates with no exact TypeScript number-type expression; integer const/enum values \
         outside the IEEE-754 safe range also have no exact TypeScript number literal. The \
         numeric carrier or closest literal remains while the predicate or excess precision is \
         omitted.",
    ),
    (
        "object-literal-validation-widened",
        "A JSON Schema object-valued const or enum member requires exact key membership. \
         TypeScript can preserve every named literal field but structural compatibility still \
         accepts values with additional fields outside fresh-object checks.",
    ),
    (
        "one-of-validation-widened",
        "A TypeScript union implements any-branch assignability and cannot generally enforce JSON \
         Schema oneOf's exactly-one matching rule. Branch declarations remain as a union while \
         overlap exclusivity is widened.",
    ),
    (
        "pattern-properties-validation-dropped",
        "Arbitrary JSON Schema patternProperties regular expressions have no equivalent key-space \
         expression in TypeScript. Named and additional-property declarations remain while \
         regex-selected property validation is omitted.",
    ),
    (
        "property-count-validation-dropped",
        "JSON Schema minProperties/maxProperties count runtime object keys; TypeScript structural \
         declarations express named requiredness but cannot bound the total property count.",
    ),
    (
        "property-name-validation-dropped",
        "JSON Schema propertyNames validates every runtime object key with a schema, which a \
         general TypeScript string-key declaration cannot express. Property value declarations \
         remain while key validation is omitted.",
    ),
    (
        "string-validation-dropped",
        "JSON Schema minLength/maxLength, pattern, format, and content assertions are runtime \
         predicates without a general TypeScript string-type equivalent. The string carrier \
         remains while the predicate is omitted.",
    ),
    (
        "tuple-array-validation-widened",
        "A JSON Schema prefixItems tuple exceeds the declaration emitter's fixed tuple-expansion \
         budget. The common item carrier remains while position-specific validation beyond the \
         budget is widened deterministically.",
    ),
    (
        "unevaluated-validation-dropped",
        "JSON Schema unevaluatedProperties/unevaluatedItems depends on applicator evaluation state \
         that TypeScript's structural type system does not expose. Representable local \
         declarations remain while the unevaluated assertion is omitted.",
    ),
    (
        "unique-items-validation-dropped",
        "JSON Schema uniqueItems compares runtime array values for equality; TypeScript array and \
         tuple declarations cannot require pairwise-distinct elements. Item and length \
         declarations remain while uniqueness is omitted.",
    ),
];

/// The JSON Schema → GraphQL September 2025 type-system emitter's closed loss
/// profile. GraphQL separates input and output types and defines variable
/// coercion independently of JSON Schema validation. The emitter preserves the
/// representable carrier graph and records every remaining difference at its
/// JSON Pointer location; custom scalars are caller-owned behavior and are
/// never treated as an implicit exact validator.
const JSON_SCHEMA_GRAPHQL_PROFILE: &[(&str, &str)] = &[
    (
        "additional-properties-validation-narrowed",
        "GraphQL input objects reject every field not declared by the input type before resolver \
         execution. A JSON Schema object that permits additional properties therefore accepts \
         source keys that the generated GraphQL input type rejects; named fields remain \
         available and the narrowing is recorded on the object schema.",
    ),
    (
        "array-cardinality-validation-dropped",
        "GraphQL list types have no minimum- or maximum-length expression. The generated list \
         retains its item carrier while JSON Schema minItems/maxItems validation is omitted.",
    ),
    (
        "array-contains-validation-dropped",
        "JSON Schema contains/minContains/maxContains quantifies matching array elements. \
         GraphQL list input coercion has no corresponding predicate, so the item carrier remains \
         while contains validation is omitted.",
    ),
    (
        "conditional-validation-dropped",
        "JSON Schema if/then/else chooses validation constraints from runtime instance content. \
         GraphQL input object definitions cannot express that conditional relationship; \
         independently representable fields remain while the conditional is omitted.",
    ),
    (
        "custom-scalar-validation-delegated",
        "GraphQL SDL declares a custom scalar name but does not define its parseValue, \
         parseLiteral, or serialization behavior. Validation carried by the caller-named \
         fallback scalar is therefore delegated to application code and is not an exact \
         self-contained SDL projection.",
    ),
    (
        "dependency-validation-dropped",
        "JSON Schema dependentRequired/dependentSchemas imposes cross-field validation. GraphQL \
         input object definitions express each field independently and cannot enforce those \
         dependencies, so the relationship is omitted.",
    ),
    (
        "integer-domain-validation-delegated",
        "GraphQL Int accepts only signed 32-bit integers. A JSON Schema integer domain that is \
         not exactly that domain is carried through the caller-named fallback scalar, whose \
         integer acceptance behavior is application-defined.",
    ),
    (
        "intersection-validation-delegated",
        "General JSON Schema allOf is an intersection of validation sets. GraphQL has no input \
         intersection type, so a non-mergeable intersection is carried through the \
         caller-named fallback scalar and its validation is delegated.",
    ),
    (
        "keyword-validation-delegated",
        "A JSON Schema assertion outside the emitter's closed GraphQL capability table has no \
         sound SDL expression. Its value is carried through the caller-named fallback scalar \
         and the assertion is recorded rather than silently treated as enforced.",
    ),
    (
        "negation-validation-delegated",
        "JSON Schema not is a complement of an instance-validation set. GraphQL input types have \
         no complement operator, so the value is carried through the caller-named fallback \
         scalar and negation validation is delegated.",
    ),
    (
        "nullable-presence-validation-widened",
        "GraphQL uses one nullable field form for both omission and an explicit null. JSON Schema \
         can distinguish required nullable properties from optional non-null properties; either \
         source distinction must be widened when represented by a nullable GraphQL input field.",
    ),
    (
        "numeric-validation-dropped",
        "GraphQL Int and Float types do not express JSON Schema bounds, exclusive bounds, or \
         multipleOf. The built-in numeric carrier remains while those runtime predicates are \
         omitted.",
    ),
    (
        "one-of-validation-delegated",
        "JSON Schema oneOf requires exactly one branch to validate. GraphQL has no input union \
         with exactly-one validation semantics, so a non-finite oneOf is carried through the \
         caller-named fallback scalar and exclusivity is delegated.",
    ),
    (
        "pattern-properties-validation-changed",
        "JSON Schema patternProperties can admit dynamic regex-selected keys and can add \
         conjunctive validation to declared keys. GraphQL input objects instead expose a fixed \
         field set with one type per field, so dynamic keys are narrowed while overlapping \
         declared-field validation may be widened.",
    ),
    (
        "property-count-validation-dropped",
        "JSON Schema minProperties/maxProperties counts runtime object keys. GraphQL input object \
         definitions express field requiredness but cannot bound the total supplied-field count.",
    ),
    (
        "property-name-validation-changed",
        "JSON Schema propertyNames validates every runtime key, including declared properties. \
         GraphQL input objects expose only a fixed declared field set: unknown names are narrowed, \
         while a source property-name rule that rejects a generated declared field is not \
         enforced and may widen acceptance.",
    ),
    (
        "recursive-input-nullability-relaxed",
        "GraphQL forbids an unbroken cycle of singular non-null input-object fields because no \
         finite value can satisfy it. The emitter makes one deterministic cycle edge nullable \
         and records that required/non-null source constraint at the field location.",
    ),
    (
        "singleton-list-coercion-widened",
        "GraphQL variable coercion accepts a non-list value for list input and wraps it as a \
         single-element list. JSON Schema array validation rejects that same non-array value, so \
         every generated list input has this explicit coercion widening.",
    ),
    (
        "string-validation-dropped",
        "GraphQL String has no length, regular-expression, format, or content assertion. The \
         string carrier remains while JSON Schema string predicates are omitted.",
    ),
    (
        "tuple-array-validation-delegated",
        "JSON Schema prefixItems and closed tuple tails assign schemas by array position. GraphQL \
         lists are homogeneous, so a non-homogeneous tuple is carried through the caller-named \
         fallback scalar and position-specific validation is delegated.",
    ),
    (
        "unevaluated-validation-dropped",
        "JSON Schema unevaluatedProperties/unevaluatedItems depends on applicator evaluation \
         state that GraphQL input coercion does not expose. Representable local fields and items \
         remain while the unevaluated assertion is omitted.",
    ),
    (
        "union-validation-delegated",
        "General JSON Schema anyOf and type unions define input-value unions. GraphQL has no \
         input union type, so a non-finite union is carried through the caller-named fallback \
         scalar and branch validation is delegated.",
    ),
    (
        "unique-items-validation-dropped",
        "JSON Schema uniqueItems compares runtime array values for equality. GraphQL list input \
         coercion cannot require pairwise-distinct elements, so item validation remains while \
         uniqueness is omitted.",
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
            note: Cow::Borrowed(note),
            location: None,
        })
        .collect()
}

/// Build the static contract rows for the
/// `("json-schema", "pydantic-v2")` projection profile.
fn json_schema_pydantic_entries() -> Vec<LossEntry> {
    JSON_SCHEMA_PYDANTIC_PROFILE
        .iter()
        .map(|&(code, note)| LossEntry {
            code: Cow::Borrowed(code),
            from: Cow::Borrowed("json-schema"),
            to: Cow::Borrowed("pydantic-v2"),
            note: Cow::Borrowed(note),
            location: None,
        })
        .collect()
}

/// Build the static contract rows for the
/// `("json-schema", "linkml-1.11")` projection profile.
fn json_schema_linkml_entries() -> Vec<LossEntry> {
    JSON_SCHEMA_LINKML_PROFILE
        .iter()
        .map(|&(code, note)| LossEntry {
            code: Cow::Borrowed(code),
            from: Cow::Borrowed("json-schema"),
            to: Cow::Borrowed("linkml-1.11"),
            note: Cow::Borrowed(note),
            location: None,
        })
        .collect()
}

/// Build the static contract rows for the
/// `("json-schema", "typescript-7.0")` projection profile.
fn json_schema_typescript_entries() -> Vec<LossEntry> {
    JSON_SCHEMA_TYPESCRIPT_PROFILE
        .iter()
        .map(|&(code, note)| LossEntry {
            code: Cow::Borrowed(code),
            from: Cow::Borrowed("json-schema"),
            to: Cow::Borrowed("typescript-7.0"),
            note: Cow::Borrowed(note),
            location: None,
        })
        .collect()
}

/// Build the static contract rows for the
/// `("json-schema", "graphql-september-2025")` projection profile.
fn json_schema_graphql_entries() -> Vec<LossEntry> {
    JSON_SCHEMA_GRAPHQL_PROFILE
        .iter()
        .map(|&(code, note)| LossEntry {
            code: Cow::Borrowed(code),
            from: Cow::Borrowed("json-schema"),
            to: Cow::Borrowed("graphql-september-2025"),
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
/// [`gts_to_rdf_loss_ledger`]), RDF↔OKF direction ledgers
/// ([`rdf_to_okf_loss_ledger`] / [`okf_to_rdf_loss_ledger`]), RDF↔LPG
/// ([`rdf_to_lpg_loss_ledger`] / [`lpg_to_rdf_loss_ledger`]), RDF→OBO Graphs
/// ([`rdf_to_obo_graphs_loss_ledger`]), RDF→SKOS ([`rdf_to_skos_loss_ledger`]), plus
/// [`transcode_and_shapes_entries`] (the full
/// syntax/projection transcode matrix over `SYNTAX_CODECS` × `(SYNTAX_CODECS ∪
/// PROJECTION_CODECS)` and the non-syntax shapes pair `("shacl", "json-schema")`),
/// and the standalone JSON Schema emitter profiles.
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
    entries.extend_from_slice(rdf_to_okf_loss_ledger().entries());
    entries.extend_from_slice(okf_to_rdf_loss_ledger().entries());
    entries.extend_from_slice(rdf_to_lpg_loss_ledger().entries());
    entries.extend_from_slice(lpg_to_rdf_loss_ledger().entries());
    entries.extend_from_slice(rdf_to_obo_graphs_loss_ledger().entries());
    entries.extend_from_slice(rdf_to_skos_loss_ledger().entries());
    entries.extend(transcode_and_shapes_entries());
    entries.extend(json_schema_pydantic_entries());
    entries.extend(json_schema_linkml_entries());
    entries.extend(json_schema_typescript_entries());
    entries.extend(json_schema_graphql_entries());
    entries
}

/// The enumerable loss registry as deterministic JSON: every `(from, to)` pair
/// [`registered_pairs`] reports — the RDF↔GTS directions, every non-identity
/// syntax/projection transcode pair, the shapes projection, and schema-language
/// emitter profiles — rendered from `registry_entries` sorted by
/// `(from, to, code)`.
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
/// directions, graph/tabular/view profiles, every non-identity syntax/projection
/// transcode pair, the shapes projection, and schema-language emitter profiles.
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
//   this is also the exact definition the rendered `"intentional"` JSON field
//   derives from (see [`render`]) — `intentional` is never stored, only
//   computed as membership in [`profile_for`].
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
        // `intentional` is not stored — it is 100% derivable from soundness's own
        // definition (see `check_ledger_sound`): a recorded code is intentional
        // iff it is a member of the declared profile for its `(from, to)` pair.
        // Deriving here (rather than trusting a caller-set bool) makes the two
        // impossible to drift apart.
        let intentional = profile_for(&entry.from, &entry.to).contains(entry.code.as_ref());
        push_bool_field(&mut out, "intentional", intentional);
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

    /// `intentional` is not a stored field; it is derived at render time as
    /// membership in [`profile_for`] (see `render`'s `intentional` binding).
    /// This is the falsifiable proof of that derivation: every code either
    /// direction ledger actually contains is, by construction, a member of
    /// its own declared profile — so the derived value renders `true` for
    /// every entry in these two static ledgers — and the rendered JSON
    /// reflects it.
    #[test]
    fn every_recorded_loss_is_intentional() {
        for entry in rdf_to_gts_loss_ledger().entries() {
            assert!(
                profile_for(&entry.from, &entry.to).contains(entry.code.as_ref()),
                "{} not in its own declared profile",
                entry.code
            );
        }
        for entry in gts_to_rdf_loss_ledger().entries() {
            assert!(
                profile_for(&entry.from, &entry.to).contains(entry.code.as_ref()),
                "{} not in its own declared profile",
                entry.code
            );
        }
        // And the rendered JSON must reflect the derivation: every entry in
        // the combined RDF<->GTS matrix renders `"intentional": true`.
        let json = rdf_gts_loss_matrix_json();
        assert_eq!(
            json.matches("\"intentional\": true").count(),
            EXPECTED_CODES.len(),
            "every in-profile code must render intentional: true: {json}"
        );
        assert_eq!(
            json.matches("\"intentional\": false").count(),
            0,
            "no code in these static ledgers is out-of-profile: {json}"
        );
    }

    /// The falsifiable-teeth half of the derivation: an entry whose code is
    /// deliberately OUTSIDE its own `(from, to)` profile must render
    /// `"intentional": false`. Driving this through the real production
    /// `render`/`rdf_gts_loss_matrix_json` path would require inventing a
    /// bogus static contract entry (there is no natural out-of-profile
    /// contract entry — that is the point of the registry), so this exercises
    /// the render helper directly with a synthetic runtime entry instead,
    /// mirroring `check_ledger_sound_flags_out_of_profile_code` below.
    #[test]
    fn render_computes_intentional_false_for_out_of_profile_code() {
        let entry = LossEntry {
            code: Cow::Borrowed("not-a-real-code"),
            from: Cow::Borrowed("shacl"),
            to: Cow::Borrowed("json-schema"),
            note: Cow::Borrowed("synthetic entry for the intentional-derivation test"),
            location: None,
        };
        let json = render(&[entry], false);
        assert!(
            json.contains("\"intentional\": false"),
            "an out-of-profile code must render intentional: false: {json}"
        );
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
    fn transcode_matrix_includes_json_schema_pydantic_pair() {
        let json = loss_matrix_json();
        assert!(json.contains("\"from\": \"json-schema\""));
        assert!(json.contains("\"to\": \"pydantic-v2\""));
        for (code, _) in JSON_SCHEMA_PYDANTIC_PROFILE {
            assert!(
                json.contains(&format!("\"code\": \"{code}\"")),
                "transcode-loss-matrix.json missing Pydantic-profile code `{code}`"
            );
        }
    }

    #[test]
    fn transcode_matrix_includes_json_schema_linkml_pair() {
        let json = loss_matrix_json();
        assert!(json.contains("\"from\": \"json-schema\""));
        assert!(json.contains("\"to\": \"linkml-1.11\""));
        for (code, _) in JSON_SCHEMA_LINKML_PROFILE {
            assert!(
                json.contains(&format!("\"code\": \"{code}\"")),
                "transcode-loss-matrix.json missing LinkML-profile code `{code}`"
            );
        }
    }

    #[test]
    fn transcode_matrix_includes_json_schema_typescript_pair() {
        let json = loss_matrix_json();
        assert!(json.contains("\"from\": \"json-schema\""));
        assert!(json.contains("\"to\": \"typescript-7.0\""));
        for (code, _) in JSON_SCHEMA_TYPESCRIPT_PROFILE {
            assert!(
                json.contains(&format!("\"code\": \"{code}\"")),
                "transcode-loss-matrix.json missing TypeScript-profile code `{code}`"
            );
        }
    }

    #[test]
    fn transcode_matrix_includes_json_schema_graphql_pair() {
        let json = loss_matrix_json();
        assert!(json.contains("\"from\": \"json-schema\""));
        assert!(json.contains("\"to\": \"graphql-september-2025\""));
        for (code, _) in JSON_SCHEMA_GRAPHQL_PROFILE {
            assert!(
                json.contains(&format!("\"code\": \"{code}\"")),
                "transcode-loss-matrix.json missing GraphQL-profile code `{code}`"
            );
        }
    }

    #[test]
    fn graph_and_tabular_projection_contracts_are_closed_and_registered() {
        let contracts = [
            (
                "rdf-1.2-dataset",
                "lpg",
                rdf_to_lpg_loss_ledger(),
                RDF_LPG_PROFILE,
            ),
            (
                "lpg",
                "rdf-1.2-dataset",
                lpg_to_rdf_loss_ledger(),
                LPG_RDF_PROFILE,
            ),
            (
                "rdf-1.2-dataset",
                "obo-graphs-0.3.2",
                rdf_to_obo_graphs_loss_ledger(),
                RDF_OBO_GRAPHS_PROFILE,
            ),
            (
                "rdf-1.2-dataset",
                "skos",
                rdf_to_skos_loss_ledger(),
                RDF_SKOS_PROFILE,
            ),
        ];

        for (from, to, ledger, declared) in contracts {
            let expected: BTreeSet<&str> = declared.iter().map(|(code, _)| *code).collect();
            assert_eq!(ledger.entries().len(), expected.len());
            assert_eq!(profile_for(from, to), expected);
            for entry in ledger.entries() {
                assert_eq!(entry.from, from);
                assert_eq!(entry.to, to);
                assert!(entry.location.is_none());
            }
            assert_ledger_sound(&ledger, from, to);
        }
    }

    #[test]
    fn transcode_matrix_includes_graph_and_tabular_projection_pairs() {
        let json = loss_matrix_json();
        for (from, to, profile) in [
            ("rdf-1.2-dataset", "lpg", RDF_LPG_PROFILE),
            ("lpg", "rdf-1.2-dataset", LPG_RDF_PROFILE),
            (
                "rdf-1.2-dataset",
                "obo-graphs-0.3.2",
                RDF_OBO_GRAPHS_PROFILE,
            ),
            ("rdf-1.2-dataset", "skos", RDF_SKOS_PROFILE),
        ] {
            assert!(json.contains(&format!("\"from\": \"{from}\"")));
            assert!(json.contains(&format!("\"to\": \"{to}\"")));
            for (code, _) in profile {
                assert!(
                    json.contains(&format!("\"code\": \"{code}\"")),
                    "transcode matrix missing {from}->{to} code {code}"
                );
            }
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

    #[test]
    fn registered_pairs_includes_json_schema_pydantic() {
        assert!(
            registered_pairs().any(|(from, to)| from == "json-schema" && to == "pydantic-v2"),
            "registered_pairs() must include (\"json-schema\", \"pydantic-v2\")"
        );
    }

    #[test]
    fn registered_pairs_includes_json_schema_linkml() {
        assert!(
            registered_pairs().any(|(from, to)| from == "json-schema" && to == "linkml-1.11"),
            "registered_pairs() must include (\"json-schema\", \"linkml-1.11\")"
        );
    }

    #[test]
    fn registered_pairs_includes_json_schema_typescript() {
        assert!(
            registered_pairs().any(|(from, to)| from == "json-schema" && to == "typescript-7.0"),
            "registered_pairs() must include (\"json-schema\", \"typescript-7.0\")"
        );
    }

    #[test]
    fn registered_pairs_includes_json_schema_graphql() {
        assert!(
            registered_pairs()
                .any(|(from, to)| { from == "json-schema" && to == "graphql-september-2025" })
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
    fn profile_for_json_schema_pydantic_is_closed() {
        let profile = profile_for("json-schema", "pydantic-v2");
        let expected: BTreeSet<&str> = JSON_SCHEMA_PYDANTIC_PROFILE
            .iter()
            .map(|(code, _)| *code)
            .collect();
        assert_eq!(profile, expected);
    }

    #[test]
    fn profile_for_json_schema_linkml_is_closed() {
        let profile = profile_for("json-schema", "linkml-1.11");
        let expected: BTreeSet<&str> = JSON_SCHEMA_LINKML_PROFILE
            .iter()
            .map(|(code, _)| *code)
            .collect();
        assert_eq!(profile, expected);
    }

    #[test]
    fn profile_for_json_schema_typescript_is_closed() {
        let profile = profile_for("json-schema", "typescript-7.0");
        let expected: BTreeSet<&str> = JSON_SCHEMA_TYPESCRIPT_PROFILE
            .iter()
            .map(|(code, _)| *code)
            .collect();
        assert_eq!(profile, expected);
    }

    #[test]
    fn profile_for_json_schema_graphql_is_closed() {
        let profile = profile_for("json-schema", "graphql-september-2025");
        let expected: BTreeSet<&str> = JSON_SCHEMA_GRAPHQL_PROFILE
            .iter()
            .map(|(code, _)| *code)
            .collect();
        assert_eq!(profile, expected);
    }

    #[test]
    fn profile_for_trig_turtle_includes_named_graph_dropped() {
        let profile = profile_for("trig", "turtle");
        assert!(
            profile.contains("named-graph-dropped"),
            "profile_for(\"trig\", \"turtle\") missing named-graph-dropped: {profile:?}"
        );
    }

    #[test]
    fn okf_direction_profiles_are_closed_and_registered() {
        let write_profile = profile_for("rdf-1.2-dataset", "okf");
        assert_eq!(
            write_profile,
            BTreeSet::from([
                "named-graph-dropped",
                "okf-annotation-dropped",
                "okf-non-profile-quad-dropped",
                "okf-reifier-dropped",
            ])
        );
        assert_eq!(
            profile_for("okf", "rdf-1.2-dataset"),
            BTreeSet::from(["okf-navigation-page-dropped"])
        );
        assert!(check_ledger_sound(&rdf_to_okf_loss_ledger(), "rdf-1.2-dataset", "okf").is_ok());
        assert!(check_ledger_sound(&okf_to_rdf_loss_ledger(), "okf", "rdf-1.2-dataset").is_ok());
    }

    /// Build a hand-rolled, owned runtime [`LossEntry`] for the mechanical
    /// helper tests below — no `RdfLocation`, since these tests exercise only
    /// `code`/`from`/`to`, never a subject.
    fn owned_entry(code: &str, from: &str, to: &str) -> LossEntry {
        LossEntry {
            code: Cow::Owned(code.to_owned()),
            from: Cow::Owned(from.to_owned()),
            to: Cow::Owned(to.to_owned()),
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
