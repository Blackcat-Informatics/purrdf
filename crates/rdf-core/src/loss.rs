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
//! by code; the rendered matrix is committed at `generated/rdf-loss-matrix.json`
//! and a drift gate in this module's tests re-derives and compares it.
//!
//! [`LossEntry`]/[`LossLedger`] serve two disciplines (see [`LossLedger::contract`]
//! and [`LossLedger::record`]): a compile-time **contract** (the static ledgers
//! and the transcode matrix in this module) and a runtime **record** (an actual
//! conversion accumulating located losses as it runs). [`registered_pairs`] and
//! [`profile_for`] enumerate the closed set of `(from, to)` pairs and loss codes
//! known across BOTH disciplines, including the non-syntax `shacl`→`json-schema`
//! shapes projection.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

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

    /// Render the ledger as deterministic JSON: entries sorted by
    /// `(from, to, code, location)`, 2-space indented, with a trailing
    /// newline. A `location` field is emitted per entry when present.
    pub fn render_json(&self) -> String {
        let mut sorted = self.entries.clone();
        sorted.sort_by(|a, b| {
            a.from
                .cmp(&b.from)
                .then_with(|| a.to.cmp(&b.to))
                .then_with(|| a.code.cmp(&b.code))
                .then_with(|| a.location.cmp(&b.location))
        });
        render(&sorted, true)
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
pub fn loss_matrix_json() -> String {
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

/// The full transcode loss matrix as deterministic JSON.
///
/// Iterates all `(from ∈ SYNTAX_CODECS) × (to ∈ SYNTAX_CODECS ∪
/// PROJECTION_CODECS)` pairs, skips identity pairs, collects every non-empty
/// [`LossEntry`], sorts by `(from, to, code)`, and renders via [`render`].
/// Unlike a single [`LossLedger::contract`], codes are NOT assumed unique here
/// — the same code recurs for different `(from, to)` pairs.
///
/// The rendered output is committed at `generated/transcode-loss-matrix.json`.
pub fn transcode_loss_matrix_json() -> String {
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

    // Sort by (from, to, code) for full determinism.
    entries.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.code.cmp(&b.code))
    });

    render(&entries, false)
}

// ── Enumerable loss registry ─────────────────────────────────────────────────

/// The SHACL → JSON Schema/OpenAPI shapes projection's closed loss profile:
/// the exact construct labels `shapes::json_schema::Ctx::record`/the
/// value-vocabulary `LossRecord` builders use (`crates/shapes/src/json_schema.rs`).
/// `sh:sparql` / `sh:expression` (SHACL-AF constraints), property-level
/// `sh:not`, and a `rdfs:range` clash with a value-vocabulary projection are
/// recorded via `Ctx::record`; `sh:SPARQLTarget`-targeted shapes (`Target::Sparql`
/// in `crates/shapes/src/shapes.rs`) have no `$def` equivalent and are excluded
/// from the compiled schema entirely; `value-vocabulary` covers an
/// enum-with-no-members projection and `value-vocabulary member` a dropped
/// blank-node enum member. Mirrored here (rather than depended-on from this
/// crate) because `purrdf-core` never depends on the `shapes` crate.
const SHACL_JSON_SCHEMA_PROFILE: &[&str] = &[
    "rdfs:range",
    "sh:SPARQLTarget",
    "sh:expression",
    "sh:not",
    "sh:sparql",
    "value-vocabulary",
    "value-vocabulary member",
];

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

/// The single source of truth backing [`registered_pairs`] and [`profile_for`]:
/// every `(from, to)` pair this crate knows a loss profile for, mapped to the
/// closed set of codes that pair's conversion may drop.
///
/// Built from the RDF↔GTS direction ledgers, the full syntax/projection
/// transcode matrix ([`pair_loss_ledger`] over `SYNTAX_CODECS` ×
/// `(SYNTAX_CODECS ∪ PROJECTION_CODECS)`), and the non-syntax shapes pair
/// `("shacl", "json-schema")` ([`SHACL_JSON_SCHEMA_PROFILE`]).
fn registry() -> BTreeMap<(&'static str, &'static str), BTreeSet<&'static str>> {
    let mut table: BTreeMap<(&'static str, &'static str), BTreeSet<&'static str>> = BTreeMap::new();

    let record = |table: &mut BTreeMap<(&'static str, &'static str), BTreeSet<&'static str>>,
                  entry: &LossEntry| {
        table
            .entry((static_str(&entry.from), static_str(&entry.to)))
            .or_default()
            .insert(static_str(&entry.code));
    };

    for entry in rdf_to_gts_loss_ledger().entries() {
        record(&mut table, entry);
    }
    for entry in gts_to_rdf_loss_ledger().entries() {
        record(&mut table, entry);
    }

    let all_targets: Vec<&str> = SYNTAX_CODECS
        .iter()
        .chain(PROJECTION_CODECS.iter())
        .copied()
        .collect();
    for &from in SYNTAX_CODECS {
        for &to in &all_targets {
            if from == to {
                continue;
            }
            for entry in pair_loss_ledger(from, to).entries() {
                record(&mut table, entry);
            }
        }
    }

    table.insert(
        ("shacl", "json-schema"),
        SHACL_JSON_SCHEMA_PROFILE.iter().copied().collect(),
    );

    table
}

/// Every `(from, to)` pair with a registered loss profile: the RDF↔GTS
/// directions, every non-identity syntax/projection transcode pair, and the
/// `("shacl", "json-schema")` shapes projection.
pub fn registered_pairs() -> impl Iterator<Item = (&'static str, &'static str)> {
    registry().into_keys()
}

/// The closed set of loss codes a `from -> to` conversion may drop, per the
/// enumerable registry.
///
/// Empty when the pair is unregistered (an identity pair, or one this crate
/// has no declared profile for) — never a panic, since callers may probe
/// arbitrary pairs to check whether a profile is known.
pub fn profile_for(from: &str, to: &str) -> BTreeSet<&'static str> {
    registry()
        .into_iter()
        .find(|((f, t), _)| *f == from && *t == to)
        .map(|(_, codes)| codes)
        .unwrap_or_default()
}

/// Render already-sorted entries to deterministic JSON: 2-space indent, fixed
/// field order, trailing newline.
///
/// Hand-rolled to avoid pulling serde into the kernel rlib (the crate does not
/// depend on it). Codes are NOT assumed unique — callers sort first; this
/// function only renders. `emit_location` gates the `location` field: the
/// static-contract renders ([`loss_matrix_json`], [`transcode_loss_matrix_json`])
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
    /// `direction-dropped` was retired by  (`Term.direction`) and
    /// `multi-reifier-collapsed` by  (reifier-id-keyed
    /// `Graph.reifiers`); both now round-trip losslessly.
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
        assert_eq!(loss_matrix_json(), loss_matrix_json());
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
        let json = loss_matrix_json();
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
            loss_matrix_json(),
            "generated/rdf-loss-matrix.json is stale; regenerate it from loss_matrix_json()"
        );
    }

    #[test]
    fn transcode_loss_matrix_deterministic() {
        assert_eq!(transcode_loss_matrix_json(), transcode_loss_matrix_json());
    }

    #[test]
    fn transcode_matrix_has_not_drifted() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../generated/transcode-loss-matrix.json");
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("generated file missing: {}", path.display()));
        assert_eq!(
            on_disk,
            transcode_loss_matrix_json(),
            "transcode-loss-matrix.json has drifted"
        );
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
    fn loss_matrix_json_is_deterministic_across_calls() {
        assert_eq!(loss_matrix_json(), loss_matrix_json());
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
}
