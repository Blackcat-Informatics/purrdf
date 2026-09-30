// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Slice-analysis graph emitter.
//!
//! Serialises computed dependency edges into the `<vocab>graph/slice-analysis`
//! named graph (the graph IRI and every emitted term derive from the caller's
//! [`SliceVocab`]). The authored manifests are NEVER modified.
//!
//! # Two-pass attestation
//!
//! 1. **Pass 1**: `OwnershipAnalyzer::analyze()` derives edges.
//! 2. **Pass 2**: [`emit_analysis_graph`] serialises those edges, stamping each
//!    with: bundle content-ID, toolchain stamp, and evidence.
//!
//! # Self-attestation guard
//!
//! [`emit_analysis_graph`] hard-fails when `authored_input_text` contains the
//! analysis graph IRI — the analysis graph must not be consumed as its own input.

use std::fmt::Write as _;

use purrdf_lex::literal_escape::{self, Carrier};
use purrdf_lex::term_syntax;

use crate::cache::ToolchainContext;
use crate::ownership::{DependencyEdge, ReconciliationStatus, SliceIri};
use crate::vocab::SliceVocab;

// The emitted vocabulary — the analysis graph IRI (`{ns}graph/slice-analysis`),
// `computedSliceDependency`, `dependencyStatus`, `dependencyEvidence`,
// `computedProfileMembership`, `termCoverage`, and the provenance predicates —
// all derive from the caller's [`SliceVocab`] (see its accessor methods).

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors produced during analysis graph emission.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalysisError {
    /// The authored input contains the self-attestation guard IRI, which would
    /// allow the analysis graph to be consumed as its own input.
    SelfAttestationViolation {
        /// The forbidden IRI found in the authored input.
        found: String,
    },
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfAttestationViolation { found } => write!(
                f,
                "self-attestation guard violation: authored input contains \
                 the analysis graph IRI {found:?}; the analysis graph must \
                 not be consumed as its own input"
            ),
        }
    }
}

impl std::error::Error for AnalysisError {}

// ── Tier-based forbidden-edge detection ──────────────────────────────────────

/// Whether a computed dependency edge is architecturally forbidden under the
/// tier model (RFC §10 / Principle 16).
///
/// An extension slice MUST NOT depend on another extension slice.  A core
/// slice MUST NOT depend on an extension slice (core must never depend on
/// optional extensions).
///
/// `from_tier` and `to_tier` are numeric priorities: 0 = core, 1 = extension,
/// 2+ = unknown/domain.
pub fn is_forbidden_edge(from_tier: u8, to_tier: u8) -> bool {
    match (from_tier, to_tier) {
        // Core depending on extension: forbidden.
        (0, 1) => true,
        // Extension depending on another extension: forbidden (Principle 16).
        (1, 1) => true,
        _ => false,
    }
}

// ── Evidence summarizer ───────────────────────────────────────────────────────

fn edge_evidence_summary(edge: &DependencyEdge) -> String {
    if edge.evidence.is_empty() {
        return String::from("no evidence (synthetic stale or forbidden edge)");
    }
    let mut parts = Vec::new();
    for ev in edge.evidence.iter().take(5) {
        parts.push(format!(
            "{}→{}[{}]",
            ev.referenced_term.as_str(),
            ev.from_artifact.logical_path,
            ev.from_artifact.raw_digest.get(..8).unwrap_or("?")
        ));
    }
    if edge.evidence.len() > 5 {
        parts.push(format!("…and {} more", edge.evidence.len() - 5));
    }
    parts.join("; ")
}

// ── Bundle content-ID ─────────────────────────────────────────────────────────

/// Compute the bundle content-ID from the raw digests of all authored
/// artifacts.  The digests are sorted for determinism, then hashed together.
pub fn bundle_content_id(raw_digests: &[&str]) -> String {
    use sha2::{Digest as _, Sha256};
    let mut sorted: Vec<&str> = raw_digests.to_vec();
    sorted.sort_unstable();
    let mut h = Sha256::new();
    for d in sorted {
        h.update(d.as_bytes());
        h.update(b"\n");
    }
    let digest = h.finalize();
    format!("{}", purrdf_hash::hex::Lower(&digest))
}

// ── Analysis graph output ─────────────────────────────────────────────────────

/// `value` as a Turtle `IRIREF`, spelled by [`term_syntax::write_iri`].
fn iri(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    term_syntax::write_iri(value, &mut out);
    out
}

/// `value` as an explicitly typed `"…"^^xsd:string` Turtle literal, the body
/// escaped by the RDF 1.2 canonical literal escaper ([`Carrier::Canonical`]).
/// The `xsd:` prefix is the one the analysis-graph preamble declares.
fn string_literal(value: &str) -> String {
    format!(
        "\"{}\"^^xsd:string",
        literal_escape::escape(value, Carrier::Canonical)
    )
}

/// The emitted analysis graph: Turtle body text plus metadata.
#[derive(Debug, Clone)]
pub struct AnalysisGraph {
    /// Serialised Turtle for the named graph body (without a GRAPH wrapper).
    /// Every triple is implicitly in `<vocab>graph/slice-analysis`.
    pub turtle_body: String,
    /// Bundle content-ID stamped into every triple's provenance.
    pub bundle_content_id: String,
    /// Toolchain context used when computing this analysis.
    pub toolchain: ToolchainContext,
    /// Total dependency edges serialised.
    pub edge_count: usize,
    /// Edges classified as forbidden (tier violation).
    pub forbidden_count: usize,
}

/// Emit the computed dependency edges as the `<vocab>graph/slice-analysis`
/// named graph.
///
/// # Arguments
///
/// - `vocab` — the caller's slice vocabulary; every emitted term and the
///   analysis-graph IRI derive from it.
/// - `edges` — computed edges from [`crate::ownership::OwnershipAnalyzer::analyze`].
/// - `authored_input_text` — text of the authored input checked for the
///   self-attestation guard (may be `""` when the caller has no text form).
/// - `all_raw_digests` — SHA-256 digests of every authored artifact in scope
///   (drives the bundle content-ID).
/// - `toolchain` — compiler/profile version stamped into provenance.
/// - `tier_of` — maps a slice IRI to numeric tier (0=core, 1=extension, 2=unknown).
/// - `term_count_of` — maps a slice IRI to its term count (for `<vocab>termCoverage`).
///
/// # Errors
///
/// Returns [`AnalysisError::SelfAttestationViolation`] if `authored_input_text`
/// contains the vocab's analysis-graph IRI.
pub fn emit_analysis_graph(
    vocab: &SliceVocab,
    edges: &[DependencyEdge],
    authored_input_text: &str,
    all_raw_digests: &[&str],
    toolchain: &ToolchainContext,
    tier_of: impl Fn(&SliceIri) -> u8,
    term_count_of: impl Fn(&SliceIri) -> usize,
) -> Result<AnalysisGraph, AnalysisError> {
    let analysis_graph_iri = vocab.analysis_graph_iri();
    let ns = vocab.ns();
    let prefix = vocab.prefix_name();
    let term_coverage = vocab.term_coverage();
    let computed_slice_dependency = vocab.computed_slice_dependency();
    let dependency_status = vocab.dependency_status();
    let dependency_evidence = vocab.dependency_evidence();

    // ── Self-attestation guard ────────────────────────────────────────────────
    if authored_input_text.contains(&analysis_graph_iri) {
        return Err(AnalysisError::SelfAttestationViolation {
            found: analysis_graph_iri,
        });
    }

    let content_id = bundle_content_id(all_raw_digests);
    let mut body = String::new();
    let mut forbidden_count = 0usize;

    // Turtle preamble.
    let xsd_ns = iri(purrdf_core::datatype::XSD_NS);
    let rdfs_ns = iri(purrdf_iri::vocab::rdfs::NS);
    writeln!(body, "@prefix {prefix}: {} .", iri(ns)).unwrap();
    writeln!(body, "@prefix xsd:   {xsd_ns} .").unwrap();
    writeln!(body, "@prefix rdfs:  {rdfs_ns} .").unwrap();
    writeln!(body).unwrap();

    let term = |local: &str| iri(&format!("{ns}{local}"));
    let graph = iri(&analysis_graph_iri);
    let content_id_literal = string_literal(&content_id);
    let compiler_literal = string_literal(&toolchain.compiler_version);
    let profile_literal = string_literal(&toolchain.reasoning_profile);
    let provenance = |body: &mut String| {
        writeln!(
            body,
            "    {} {content_id_literal} ;",
            term("bundleContentId")
        )
        .unwrap();
        writeln!(
            body,
            "    {} {compiler_literal} ;",
            term("toolchainCompiler")
        )
        .unwrap();
        writeln!(body, "    {} {profile_literal} .", term("toolchainProfile")).unwrap();
        writeln!(body).unwrap();
    };

    // Graph-level provenance node. This is generated A-Box instance data folded
    // into the bundle, not vocabulary surface: it carries a human label, its own
    // named-graph provenance anchor, and the assertional `<vocab>boxABox` role so
    // it satisfies the assertional-tier validation contract (no `skos:definition`).
    writeln!(body, "{graph}").unwrap();
    writeln!(body, "    a {} ;", term("SliceAnalysisGraph")).unwrap();
    writeln!(body, "    rdfs:label \"Slice analysis graph\" ;").unwrap();
    writeln!(body, "    rdfs:isDefinedBy {graph} ;").unwrap();
    writeln!(body, "    {} {} ;", term("graphBoxRole"), term("boxABox")).unwrap();
    provenance(&mut body);

    // Per-slice term-coverage triples (unique slices, sorted for determinism).
    let mut all_slices: Vec<SliceIri> = edges
        .iter()
        .flat_map(|e| [e.from_slice.clone(), e.to_slice.clone()])
        .collect();
    all_slices.sort_unstable();
    all_slices.dedup();

    let term_coverage = iri(&term_coverage);
    for slice_iri in &all_slices {
        let count = term_count_of(slice_iri);
        writeln!(body, "{}", iri(slice_iri)).unwrap();
        writeln!(body, "    {term_coverage} \"{count}\"^^xsd:integer .").unwrap();
        writeln!(body).unwrap();
    }

    // Per-edge blank nodes.
    let computed_slice_dependency = iri(&computed_slice_dependency);
    let dependency_status = iri(&dependency_status);
    let dependency_evidence = iri(&dependency_evidence);
    for (i, edge) in edges.iter().enumerate() {
        let from_tier = tier_of(&edge.from_slice);
        let to_tier = tier_of(&edge.to_slice);

        let effective_status = if is_forbidden_edge(from_tier, to_tier) {
            forbidden_count += 1;
            ReconciliationStatus::Forbidden
        } else {
            edge.reconciliation
        };

        let evidence_str = edge_evidence_summary(edge);
        let kind_str = format!("{:?}", edge.edge_kind);

        writeln!(body, "_:dep{i}").unwrap();
        writeln!(body, "    a {computed_slice_dependency} ;").unwrap();
        writeln!(
            body,
            "    {dependency_status} {} ;",
            string_literal(effective_status.token())
        )
        .unwrap();
        writeln!(
            body,
            "    {} {} ;",
            term("dependencyFromSlice"),
            iri(&edge.from_slice)
        )
        .unwrap();
        writeln!(
            body,
            "    {} {} ;",
            term("dependencyToSlice"),
            iri(&edge.to_slice)
        )
        .unwrap();
        writeln!(
            body,
            "    {} {} ;",
            term("dependencyEdgeKind"),
            string_literal(&kind_str)
        )
        .unwrap();
        writeln!(
            body,
            "    {dependency_evidence} {} ;",
            string_literal(&evidence_str)
        )
        .unwrap();
        provenance(&mut body);
    }

    Ok(AnalysisGraph {
        turtle_body: body,
        bundle_content_id: content_id,
        toolchain: toolchain.clone(),
        edge_count: edges.len(),
        forbidden_count,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ownership::{DependencyEdge, EdgeKind, ReconciliationStatus};

    fn tc() -> ToolchainContext {
        ToolchainContext::new("purrdf-logic v1", "el")
    }

    /// Pure fixtures use a caller-supplied example.org vocabulary.
    fn vocab() -> SliceVocab {
        SliceVocab::for_namespace("https://example.org/vocab/")
    }

    fn make_edge(
        from: &str,
        to: &str,
        kind: EdgeKind,
        status: ReconciliationStatus,
    ) -> DependencyEdge {
        DependencyEdge {
            from_slice: from.to_string(),
            to_slice: to.to_string(),
            edge_kind: kind,
            evidence: Vec::new(),
            reconciliation: status,
        }
    }

    #[test]
    fn no_authored_overwrite() {
        // The emitter returns a NEW graph string; the authored input is unchanged.
        let authored = "@prefix vocab: <https://example.org/vocab/> .\n\
                        vocab:sliceA vocab:sliceDependsOn vocab:sliceB .\n";
        let authored_orig = authored;
        let edges = vec![make_edge(
            "https://example.org/vocab/sliceA",
            "https://example.org/vocab/sliceB",
            EdgeKind::Ontology,
            ReconciliationStatus::Matched,
        )];
        let result =
            emit_analysis_graph(&vocab(), &edges, authored, &[], &tc(), |_| 0, |_| 5).unwrap();
        // Authored string is a static &str — prove it is structurally unchanged.
        assert_eq!(authored, authored_orig);
        // The output contains the analysis graph IRI, not sliceDependsOn.
        assert!(result.turtle_body.contains(&vocab().analysis_graph_iri()));
        assert!(!result.turtle_body.contains("sliceDependsOn"));
    }

    #[test]
    fn status_coverage_all_variants() {
        let edges = vec![
            make_edge(
                "https://example.org/vocab/sliceA",
                "https://example.org/vocab/sliceB",
                EdgeKind::Ontology,
                ReconciliationStatus::Matched,
            ),
            make_edge(
                "https://example.org/vocab/sliceC",
                "https://example.org/vocab/sliceB",
                EdgeKind::Shape,
                ReconciliationStatus::Undeclared,
            ),
            make_edge(
                "https://example.org/vocab/sliceD",
                "https://example.org/vocab/sliceB",
                EdgeKind::Ontology,
                ReconciliationStatus::Stale,
            ),
        ];
        let digests = ["abc123", "def456"];
        let result =
            emit_analysis_graph(&vocab(), &edges, "", &digests, &tc(), |_| 2, |_| 3).unwrap();

        assert!(result.turtle_body.contains("\"matched\""));
        assert!(result.turtle_body.contains("\"undeclared\""));
        assert!(result.turtle_body.contains("\"stale\""));
        assert_eq!(result.edge_count, 3);

        // Deterministic bundle content-ID.
        let id2 = emit_analysis_graph(&vocab(), &edges, "", &digests, &tc(), |_| 2, |_| 3)
            .unwrap()
            .bundle_content_id;
        assert_eq!(result.bundle_content_id, id2);
    }

    #[test]
    fn forbidden_status_emitted_for_tier_violations() {
        // core (0) → extension (1): forbidden.
        let edges = vec![make_edge(
            "https://example.org/vocab/coreSlice",
            "https://example.org/vocab/extSlice",
            EdgeKind::Ontology,
            ReconciliationStatus::Matched,
        )];
        let tier_of = |iri: &SliceIri| -> u8 { u8::from(!iri.ends_with("coreSlice")) };
        let result = emit_analysis_graph(&vocab(), &edges, "", &[], &tc(), tier_of, |_| 0).unwrap();
        assert!(result.turtle_body.contains("\"forbidden\""));
        assert_eq!(result.forbidden_count, 1);
    }

    #[test]
    fn self_attestation_guard_fires() {
        let graph_iri = vocab().analysis_graph_iri();
        let poisoned = format!(
            "@prefix vocab: <https://example.org/vocab/> .\n\
             <{graph_iri}> a vocab:SliceAnalysisGraph .\n"
        );
        let res = emit_analysis_graph(&vocab(), &[], &poisoned, &[], &tc(), |_| 2, |_| 0);
        assert!(matches!(
            res,
            Err(AnalysisError::SelfAttestationViolation { .. })
        ));
    }

    #[test]
    fn term_coverage_in_output() {
        let edges = vec![make_edge(
            "https://example.org/vocab/sliceA",
            "https://example.org/vocab/sliceB",
            EdgeKind::Ontology,
            ReconciliationStatus::Matched,
        )];
        let term_count = |iri: &SliceIri| -> usize { if iri.ends_with("sliceA") { 7 } else { 3 } };
        let result =
            emit_analysis_graph(&vocab(), &edges, "", &[], &tc(), |_| 2, term_count).unwrap();
        assert!(result.turtle_body.contains(&vocab().term_coverage()));
        assert!(
            result.turtle_body.contains("\"7\"^^xsd:integer")
                || result.turtle_body.contains("\"3\"^^xsd:integer")
        );
    }

    /// Verify that the Turtle body emitted by `emit_analysis_graph` is
    /// syntactically valid Turtle by re-parsing it with the native Turtle
    /// parser.  This catches any `^^`-suffix bugs where the lexical form is
    /// not quoted (e.g. `7^^xsd:integer` instead of `"7"^^xsd:integer`).
    #[test]
    fn emitted_turtle_is_valid() {
        let edges = vec![
            make_edge(
                "https://example.org/vocab/sliceA",
                "https://example.org/vocab/sliceB",
                EdgeKind::Ontology,
                ReconciliationStatus::Matched,
            ),
            make_edge(
                "https://example.org/vocab/sliceC",
                "https://example.org/vocab/sliceB",
                EdgeKind::Shape,
                ReconciliationStatus::Undeclared,
            ),
        ];
        let term_count = |iri: &SliceIri| -> usize {
            if iri.ends_with("sliceA") {
                7
            } else if iri.ends_with("sliceC") {
                0
            } else {
                3
            }
        };
        let result = emit_analysis_graph(
            &vocab(),
            &edges,
            "",
            &["abc", "def"],
            &tc(),
            |_| 2,
            term_count,
        )
        .unwrap();

        // Drive the native codec over the emitted body.  Any Turtle syntax
        // error (including an un-quoted typed-literal like `7^^xsd:integer`)
        // will produce a parse error and `parse_dataset` returns an Err that
        // panics here with a descriptive message including the offending source.
        let turtle = &result.turtle_body;
        // `None` is the correct base here and not a discarded one: this body was just
        // emitted in memory by `emit_analysis_graph`, was never retrieved from anywhere,
        // and mints only absolute IRIs — so there is no retrieval IRI to supply and
        // nothing relative for one to resolve. A relative IRI appearing here would be an
        // emitter bug, and the hard `iri-relative-no-base` failure is how this test
        // would catch it.
        let dataset =
            purrdf::parse_dataset(turtle.as_bytes(), "text/turtle", None).unwrap_or_else(|e| {
                panic!("emitted Turtle is not valid:\n{e}\n\n--- emitted body ---\n{turtle}")
            });

        // Sanity: at least the graph-level provenance node + per-slice + per-edge triples.
        let triple_count = dataset.quad_count();
        assert!(
            triple_count >= 4,
            "expected at least 4 triples, got {triple_count}: \n{turtle}"
        );
    }

    /// A toolchain string holding a quote, a backslash, a newline, a C0 control, an
    /// accented letter and a combining mark is emitted as a Turtle literal that
    /// parses back to exactly that string. Rust `Debug` spelling wrote the control
    /// and the combining mark as `\u{1}` / `\u{301}`, which is not Turtle.
    #[test]
    fn a_toolchain_string_needing_escapes_is_valid_turtle_and_round_trips() {
        let compiler = "say \"hi\" \\ path\nnext\u{1} caf\u{e9} e\u{301}";
        let toolchain = ToolchainContext::new(compiler, "el");
        let result = emit_analysis_graph(&vocab(), &[], "", &[], &toolchain, |_| 2, |_| 0)
            .expect("the graph is emitted");
        let turtle = &result.turtle_body;
        let dataset = crate::rdf_query::Dataset::parse_turtle(turtle.as_bytes(), None, "analysis")
            .unwrap_or_else(|e| panic!("emitted Turtle is not valid:\n{e}\n\n{turtle}"));
        let read = dataset
            .object_literal(
                &vocab().analysis_graph_iri(),
                "https://example.org/vocab/toolchainCompiler",
            )
            .expect("the query runs");
        assert_eq!(read.as_deref(), Some(compiler));
    }

    /// Plain ASCII input is spelled exactly as before: the whole body is pinned.
    #[test]
    fn a_plain_ascii_analysis_graph_is_byte_identical() {
        let edges = vec![make_edge(
            "https://example.org/vocab/sliceA",
            "https://example.org/vocab/sliceB",
            EdgeKind::Ontology,
            ReconciliationStatus::Matched,
        )];
        let result = emit_analysis_graph(&vocab(), &edges, "", &["abc"], &tc(), |_| 2, |_| 4)
            .expect("the graph is emitted");
        let id = &result.bundle_content_id;
        let v = "https://example.org/vocab/";
        let graph = vocab().analysis_graph_iri();
        let provenance = format!(
            "    <{v}bundleContentId> \"{id}\"^^xsd:string ;\n    \
             <{v}toolchainCompiler> \"purrdf-logic v1\"^^xsd:string ;\n    \
             <{v}toolchainProfile> \"el\"^^xsd:string .\n\n"
        );
        let expected = format!(
            "@prefix {prefix}: <{v}> .\n\
             @prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .\n\
             @prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .\n\n\
             <{graph}>\n    a <{v}SliceAnalysisGraph> ;\n    \
             rdfs:label \"Slice analysis graph\" ;\n    rdfs:isDefinedBy <{graph}> ;\n    \
             <{v}graphBoxRole> <{v}boxABox> ;\n{provenance}\
             <{v}sliceA>\n    <{coverage}> \"4\"^^xsd:integer .\n\n\
             <{v}sliceB>\n    <{coverage}> \"4\"^^xsd:integer .\n\n\
             _:dep0\n    a <{dependency}> ;\n    <{status}> \"matched\"^^xsd:string ;\n    \
             <{v}dependencyFromSlice> <{v}sliceA> ;\n    \
             <{v}dependencyToSlice> <{v}sliceB> ;\n    \
             <{v}dependencyEdgeKind> \"Ontology\"^^xsd:string ;\n    \
             <{evidence}> \"no evidence (synthetic stale or forbidden edge)\"^^xsd:string ;\n\
             {provenance}",
            prefix = vocab().prefix_name(),
            coverage = vocab().term_coverage(),
            dependency = vocab().computed_slice_dependency(),
            status = vocab().dependency_status(),
            evidence = vocab().dependency_evidence(),
        );
        assert_eq!(result.turtle_body, expected);
    }

    #[test]
    fn bundle_content_id_order_invariant() {
        let id1 = bundle_content_id(&["aaa", "bbb", "ccc"]);
        let id2 = bundle_content_id(&["ccc", "bbb", "aaa"]);
        assert_eq!(id1, id2);
    }

    /// GOLDEN: pins `bundle_content_id`'s literal output over a fixed input.
    ///
    /// This is the only place in `purrdf-slice` a real SHA-256 hex string is pinned. Every
    /// other cache-key test in this crate is relational (`assert_eq!`/`assert_ne!` between
    /// two computed keys), which cannot detect a change to the ENCODING — uppercase hex, a
    /// truncated digest, or an added prefix would still satisfy every relational test.
    /// `bundle_content_id` is the right call site to pin: it produces the
    /// `bundleContentId` stamped into every emitted provenance triple, and it is reachable
    /// from the Python bindings, so a user sees this exact string.
    ///
    /// The golden below is `sha256("alpha\nbeta\n")` — i.e. each of the two (already
    /// sorted) inputs, followed by a newline, concatenated and hashed — computed
    /// independently with `sha256sum` outside this crate. A change to this literal is a
    /// user-visible content-ID change and must not be "fixed" by updating the literal
    /// without understanding why the encoding moved.
    #[test]
    fn bundle_content_id_is_pinned_sha256_hex() {
        let id = bundle_content_id(&["alpha", "beta"]);
        assert_eq!(
            id,
            "e49c81e2d2f84e259d40e2fb8192f3bcd198b355184845d76d8f58807d0d78ee"
        );
    }

    #[test]
    fn is_forbidden_edge_logic() {
        assert!(is_forbidden_edge(0, 1)); // core → extension
        assert!(is_forbidden_edge(1, 1)); // extension → extension
        assert!(!is_forbidden_edge(0, 0)); // core → core: ok
        assert!(!is_forbidden_edge(1, 0)); // extension → core: ok
        assert!(!is_forbidden_edge(2, 2)); // unknown → unknown: not forbidden
    }
}
