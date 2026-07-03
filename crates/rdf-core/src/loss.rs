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

/// In-band machine code: a `CONSTRUCT` whose `WHERE` bound an RDF-1.2 reifier (via
/// an `rdf:reifies` triple pattern) but whose template drops that reifier — the
/// reification layer is lost at the projection. Declared in-band on the output
/// graph (a `logic:ProjectionLoss` node), NOT in this compile-time ledger.
pub const LOSS_REIFIER_LAYER_DROPPED: &str = "reifier-layer-dropped";

/// In-band machine code: a dropped reifier (see [`LOSS_REIFIER_LAYER_DROPPED`])
/// that ALSO carried annotation triples in the `WHERE` — those annotations are lost
/// too. Emitted in addition to the reifier-layer code, never alone.
pub const LOSS_ANNOTATION_LAYER_DROPPED: &str = "annotation-layer-dropped";

/// In-band machine code: a dropped, annotated reifier (see
/// [`LOSS_ANNOTATION_LAYER_DROPPED`]) where one of the dropped annotation
/// predicates was `purrdf:accordingTo` — the standpoint scope is lost. Emitted in
/// addition to the annotation-layer code, never alone.
pub const LOSS_STANDPOINT_SCOPE_DROPPED: &str = "standpoint-scope-dropped";

/// One enumerated, intentional conversion loss between two representations.
///
/// Entries are `&'static` because the ledger is a compiled-in contract, not
/// runtime data: every code is a stable, reviewed promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LossEntry {
    /// Stable machine code, kebab-case (e.g. `direction-dropped`).
    pub code: &'static str,
    /// Source representation (e.g. `"rdf-1.2-dataset"`).
    pub from: &'static str,
    /// Target representation (e.g. `"gts"`).
    pub to: &'static str,
    /// `true` = a known, accepted conversion loss (the only kind this ledger
    /// records). A `false` value would mark an *unintentional* loss, which the
    /// fidelity gate treats as a bug rather than a documented contract.
    pub intentional: bool,
    /// Human-readable explanation of what is dropped and why.
    pub note: &'static str,
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
    /// Build a ledger from arbitrary entries, sorting by `code` for determinism.
    ///
    /// Panics on a duplicate `code`: the ledger is a compiled-in contract and a
    /// collision is a programming error (hard-fail, per the no-optionality
    /// doctrine), not a runtime condition to tolerate.
    fn from_entries(mut entries: Vec<LossEntry>) -> Self {
        entries.sort_by(|a, b| a.code.cmp(b.code));
        for pair in entries.windows(2) {
            assert_ne!(
                pair[0].code, pair[1].code,
                "duplicate loss code `{}` in ledger",
                pair[0].code
            );
        }
        Self { entries }
    }

    /// The ledger entries, sorted by `code`.
    pub fn entries(&self) -> &[LossEntry] {
        &self.entries
    }

    /// `true` when no losses are recorded. Fidelity is asserted only here.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render the ledger as deterministic JSON: a sorted-by-code array of
    /// objects, 2-space indented, with a trailing newline.
    pub fn render_json(&self) -> String {
        render_entries(&self.entries)
    }
}

/// The intentional losses incurred projecting the RDF 1.2 dataset IR → GTS.
pub fn rdf_to_gts_loss_ledger() -> LossLedger {
    LossLedger::from_entries(vec![LossEntry {
        code: "blob-bytes-absent",
        from: "rdf-1.2-dataset",
        to: "gts",
        intentional: true,
        note: "Blob payloads are preserved as content-addressed references (the blob_id digest \
               plus the origin file id), never materialized into the RDF IR, which must stay \
               value-light for arbitrarily large payloads (e.g. multi-terabyte data dumps). A \
               destination GTS carries the reference; the payload bytes are streamed \
               origin->destination on demand (deferred materialization).",
    }])
}

/// The intentional losses incurred reading GTS → the RDF 1.2 dataset IR.
pub fn gts_to_rdf_loss_ledger() -> LossLedger {
    LossLedger::from_entries(vec![LossEntry {
        code: "bnode-scope-flatten",
        from: "gts",
        to: "rdf-1.2-dataset",
        intentional: true,
        note: "`purrdf_gts::reader::read()` folds all segments into one term table, collapsing \
               per-segment blank-node scope; the distinct scopes are recovered only via the \
               streaming-event importer.",
    }])
}

/// The combined RDF↔GTS matrix as a single deterministic, sorted-by-code JSON
/// array — the body of the generated `generated/rdf-loss-matrix.json` artifact.
pub fn loss_matrix_json() -> String {
    let mut entries: Vec<LossEntry> = Vec::new();
    entries.extend_from_slice(rdf_to_gts_loss_ledger().entries());
    entries.extend_from_slice(gts_to_rdf_loss_ledger().entries());
    LossLedger::from_entries(entries).render_json()
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
/// Panics on any name not found in [`SYNTAX_CODECS`] or [`PROJECTION_CODECS`].
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

    let mut entries: Vec<LossEntry> = Vec::new();

    if is_projection(to) {
        if supports_quads(from) {
            entries.push(LossEntry {
                code: "named-graph-dropped",
                from,
                to,
                intentional: true,
                note: "The target syntax has no named-graph construct; quads are folded into the \
                       default graph and graph names are dropped.",
            });
        }
        let proj_entry = match to {
            "owl-dl" => LossEntry {
                code: "owl-dl-projection",
                from,
                to,
                intentional: true,
                note: "Projection to OWL 2 DL: rules and constructs outside the decidable DL \
                       fragment are dropped; the result is a sound view.",
            },
            "owl-el" => LossEntry {
                code: "owl-el-projection",
                from,
                to,
                intentional: true,
                note: "Projection to the OWL 2 EL profile: constructs outside EL are dropped; \
                       the result is a sound, PTIME-decidable view.",
            },
            "datalog" => LossEntry {
                code: "datalog-projection",
                from,
                to,
                intentional: true,
                note: "Projection to Datalog: non-rule axioms and existentials outside the \
                       Datalog fragment are dropped.",
            },
            "n3" => LossEntry {
                code: "n3-projection",
                from,
                to,
                intentional: true,
                note: "Projection to Notation3 rules: validation-only and non-rule constructs \
                       are dropped.",
            },
            "nemo" => LossEntry {
                code: "nemo-projection",
                from,
                to,
                intentional: true,
                note: "Projection to Nemo existential rules: constructs outside the supported \
                       rule fragment are dropped.",
            },
            "gufo" => LossEntry {
                code: "gufo-projection",
                from,
                to,
                intentional: true,
                note: "Projection to gUFO foundational classes: structure without a gUFO \
                       correspondence is dropped.",
            },
            "canonical-rdf12" => LossEntry {
                code: "canonical-rdf12-projection",
                from,
                to,
                intentional: true,
                note: "Projection to the canonical RDF-1.2 logic form: non-logic RDF structure \
                       is dropped.",
            },
            _ => unreachable!("unhandled projection codec `{to}`"),
        };
        entries.push(proj_entry);
    } else {
        // to is a syntax codec
        if supports_quads(from) && !supports_quads(to) {
            entries.push(LossEntry {
                code: "named-graph-dropped",
                from,
                to,
                intentional: true,
                note: "The target syntax has no named-graph construct; quads are folded into the \
                       default graph and graph names are dropped.",
            });
        }
        if supports_stars(from) && !supports_stars(to) {
            let star_entry = match to {
                "rdfxml" => LossEntry {
                    code: "rdf12-star-unrepresentable",
                    from,
                    to,
                    intentional: true,
                    note: "RDF/XML has no triple-term (RDF-1.2 quoted triple) syntax; reifying \
                           triples and their annotations are dropped.",
                },
                "jsonld" => LossEntry {
                    code: "rdf12-star-jsonld-rejected",
                    from,
                    to,
                    intentional: true,
                    note: "The JSON-LD 1.1 serializer rejects RDF-1.2 quoted triples; reifying \
                           triples and their annotations are dropped (use jsonld-star or \
                           yaml-ld-star to retain them).",
                },
                _ => unreachable!(
                    "supports_stars(from)=true but supports_stars({to})=false for unhandled target"
                ),
            };
            entries.push(star_entry);
        }
    }

    LossLedger::from_entries(entries)
}

/// Render a slice of [`LossEntry`] values to deterministic JSON.
///
/// Unlike [`render_entries`] this function does NOT assume codes are unique —
/// the transcode matrix can have the same code for different (from, to) pairs.
/// Entries are rendered in the order supplied; callers must pre-sort.
fn render_entries_sorted_by_pair(entries: &[LossEntry]) -> String {
    let mut out = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        out.push_str("  {\n");
        push_field(&mut out, "code", entry.code, false);
        push_field(&mut out, "from", entry.from, false);
        push_field(&mut out, "to", entry.to, false);
        push_bool_field(&mut out, "intentional", entry.intentional);
        push_field(&mut out, "note", entry.note, true);
        out.push_str("  }");
        if i + 1 < entries.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

/// The full transcode loss matrix as deterministic JSON.
///
/// Iterates all `(from ∈ SYNTAX_CODECS) × (to ∈ SYNTAX_CODECS ∪
/// PROJECTION_CODECS)` pairs, skips identity pairs, collects every non-empty
/// [`LossEntry`], sorts by `(from, to, code)`, and renders via
/// [`render_entries_sorted_by_pair`].
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
            .cmp(b.from)
            .then(a.to.cmp(b.to))
            .then(a.code.cmp(b.code))
    });

    render_entries_sorted_by_pair(&entries)
}

/// Render a sorted slice of entries to deterministic JSON.
///
/// Hand-rolled to avoid pulling serde into the kernel rlib (the crate does not
/// depend on it). Fields are emitted in a fixed order; strings are JSON-escaped.
fn render_entries(entries: &[LossEntry]) -> String {
    let mut out = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        out.push_str("  {\n");
        push_field(&mut out, "code", entry.code, false);
        push_field(&mut out, "from", entry.from, false);
        push_field(&mut out, "to", entry.to, false);
        push_bool_field(&mut out, "intentional", entry.intentional);
        push_field(&mut out, "note", entry.note, true);
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
        assert!(LossLedger::from_entries(vec![]).is_empty());
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
        assert!(trig_to_turtle
            .entries()
            .iter()
            .any(|e| e.code == "named-graph-dropped"));

        let nquads_to_rdfxml = pair_loss_ledger("nquads", "rdfxml");
        assert!(nquads_to_rdfxml
            .entries()
            .iter()
            .any(|e| e.code == "named-graph-dropped"));
        assert!(nquads_to_rdfxml
            .entries()
            .iter()
            .any(|e| e.code == "rdf12-star-unrepresentable"));

        let turtle_to_jsonld = pair_loss_ledger("turtle", "jsonld");
        assert!(turtle_to_jsonld
            .entries()
            .iter()
            .any(|e| e.code == "rdf12-star-jsonld-rejected"));
    }

    #[test]
    #[should_panic(expected = "projection")]
    fn pair_loss_panics_on_projection_source() {
        let _ = pair_loss_ledger("owl-dl", "turtle");
    }
}
