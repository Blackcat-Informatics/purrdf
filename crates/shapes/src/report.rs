// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL validation report types and serialization.
//!
//! [`ValidationReport`] is the in-memory representation of a SHACL report
//! graph. `to_ntriples()` emits a canonical N-Triples serialization using
//! oxigraph's own serializer (avoiding hand-rolled literal escaping).
//! `tuples_from_ntriples()` round-trips back to the same tuple set for testing.

use std::collections::BTreeSet;

use ::purrdf::FastSet;
use ::purrdf::RdfDatasetBuilder;
use ::purrdf::provenance::Attribution;
use ::purrdf::{RdfQuad, RdfTerm, SerializeGraph, serialize_dataset};

use ::purrdf::RdfDataset;

use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, sh, xsd};
#[cfg(test)]
use crate::term::Literal;
use crate::term::{NamedNode, Term};

// ── Severity ──────────────────────────────────────────────────────────────────

/// SHACL result severity levels, ordered from most to least severe.
///
/// SHACL permits ANY IRI as an `sh:severity` value (spec §2.1.5); the three
/// `sh:` severities are only the built-in defaults. A custom severity IRI is
/// carried verbatim in [`Severity::Other`] so validation reports preserve it
/// (W3C `core/misc/severity-002`) instead of coercing it to `sh:Violation`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// `sh:Violation` — the most severe level.
    Violation,
    /// `sh:Warning`.
    Warning,
    /// `sh:Info` — the least severe level.
    Info,
    /// Any other severity IRI, preserved verbatim.
    Other(NamedNode),
}

impl Severity {
    /// The IRI string for this severity level.
    pub fn iri(&self) -> &str {
        match self {
            Self::Violation => sh::VIOLATION,
            Self::Warning => sh::WARNING,
            Self::Info => sh::INFO,
            Self::Other(iri) => iri.as_str(),
        }
    }

    /// Parse one of the three built-in `sh:` severities from its IRI string,
    /// returning `None` if unrecognised (use [`Severity::Other`] to carry a
    /// custom severity IRI).
    pub fn from_iri(s: &str) -> Option<Self> {
        match s {
            "http://www.w3.org/ns/shacl#Violation" => Some(Self::Violation),
            "http://www.w3.org/ns/shacl#Warning" => Some(Self::Warning),
            "http://www.w3.org/ns/shacl#Info" => Some(Self::Info),
            _ => None,
        }
    }
}

// ── ValidationResult ─────────────────────────────────────────────────────────

/// A single SHACL validation result (`sh:ValidationResult`).
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The focus node that violated the constraint.
    pub focus_node: Term,
    /// The result path, if the violation is path-scoped.
    ///
    /// A plain predicate path is its IRI term; a COMPLEX path (inverse,
    /// sequence, alternative, closure) is a deterministic blank node whose
    /// structure is carried in [`ValidationResult::path_structure`] and emitted
    /// into the report graph by [`ValidationReport::to_ntriples`].
    pub result_path: Option<Term>,
    /// The full SHACL path behind a complex `result_path` blank node, so the
    /// report serialization can emit the spec-mandated path structure
    /// (`[ sh:inversePath … ]`, sequence lists, …). `None` when the result path
    /// is absent or a plain predicate IRI.
    pub path_structure: Option<crate::shapes::Path>,
    /// The offending value at the focus node, if applicable.
    pub value: Option<Term>,
    /// The constraint component that produced this result.
    pub source_constraint_component: NamedNode,
    /// The shape that sourced this result.
    pub source_shape: Term,
    /// The severity of this result.
    pub severity: Severity,
    /// An optional human-readable message.
    pub message: Option<String>,
    /// PurRDF graph-box roles attached to the source shape, if any.
    pub source_box_roles: Vec<NamedNode>,
    /// PurRDF graph-box roles attached to the result path/predicate, if any.
    pub path_box_roles: Vec<NamedNode>,
    /// Deterministic union of source/path/component roles relevant to this result.
    pub result_box_roles: Vec<NamedNode>,
    /// Structured slice attributions for this result (§9 / S5).
    ///
    /// Records which compilation units (identified by their runtime `UnitId`,
    /// resolved to public slice IRIs at the serialization boundary) played which
    /// roles in producing this result. An empty vec means no attribution context
    /// is available (e.g. in legacy or unit-test scenarios).
    pub attributions: Vec<Attribution>,
}

impl ValidationResult {
    /// The focus node's value as a plain string: the IRI for a named node, the label
    /// for a blank node, the lexical form for a literal, and the canonical rendering for
    /// a triple term.
    ///
    /// This lets a consumer render the focus without naming the (oxigraph) [`Term`] type
    /// in its own surface — it is the exact value-extraction the PurRDF
    /// scoreboard's `shacl_term_to_str` performed.
    #[must_use]
    pub fn focus_value(&self) -> String {
        match &self.focus_node {
            Term::NamedNode(n) => n.as_str().to_owned(),
            Term::BlankNode(b) => b.clone(),
            Term::Literal(l) => l.value().to_owned(),
            Term::Triple(_) => self.focus_node.to_string(),
        }
    }

    /// Apply optional PurRDF graph-box role metadata to this result.
    pub fn apply_box_roles(&mut self, source_roles: &[NamedNode], path_roles: &[NamedNode]) {
        self.source_box_roles = dedup_roles(source_roles);
        self.path_box_roles = dedup_roles(path_roles);
        let mut merged = self.source_box_roles.clone();
        merged.extend(self.path_box_roles.iter().cloned());
        self.result_box_roles = dedup_roles(&merged);
    }
}

fn dedup_roles(roles: &[NamedNode]) -> Vec<NamedNode> {
    let mut out = roles.to_vec();
    out.sort_unstable();
    out.dedup();
    out
}

// ── ValidationReport ─────────────────────────────────────────────────────────

/// A SHACL validation report (`sh:ValidationReport`).
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Whether the data graph conforms to the shapes graph.
    pub conforms: bool,
    /// Individual violation/warning/info results.
    pub results: Vec<ValidationResult>,
}

/// The tuple type used for deterministic comparison of result sets.
///
/// `(focus, path, value, component, source_shape, severity)`
pub type ResultTuple = (
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    Severity,
);

impl ValidationReport {
    /// Emit the report as N-Triples text using the native purrdf codec.
    ///
    /// The report is built into an in-memory `Store` as quads in the default
    /// graph, folded back to the IR (`dataset_from_store`), then serialised via
    /// the native codec. This avoids hand-rolling literal escaping and carries no
    /// oxigraph `io` dependency. The `DefaultGraph` selection on the
    /// `application/n-quads` codec emits graphless rows (i.e. N-Triples) and is
    /// byte-lenient on language tags, matching the legacy oxigraph serializer.
    pub fn to_ntriples(&self) -> String {
        let mut builder = RdfDatasetBuilder::new();
        // Complex-path structure roots already emitted (keyed by root label).
        let mut emitted_paths: FastSet<String> = FastSet::default();

        let report_subj = RdfTerm::blank_node("report");

        // _:report rdf:type sh:ValidationReport
        push_triple(
            &mut builder,
            report_subj.clone(),
            rdf::TYPE,
            RdfTerm::iri(sh::VALIDATION_REPORT),
        );

        // _:report sh:conforms "true"^^xsd:boolean (or false)
        push_triple(
            &mut builder,
            report_subj.clone(),
            sh::CONFORMS,
            RdfTerm::Literal(::purrdf::RdfLiteral::typed(
                if self.conforms { "true" } else { "false" },
                xsd::BOOLEAN,
            )),
        );

        for (i, r) in self.results.iter().enumerate() {
            let result_subj = RdfTerm::blank_node(format!("r{i}"));

            // _:report sh:result _:r{i}
            push_triple(
                &mut builder,
                report_subj.clone(),
                sh::RESULT,
                result_subj.clone(),
            );

            // _:r{i} rdf:type sh:ValidationResult
            push_triple(
                &mut builder,
                result_subj.clone(),
                rdf::TYPE,
                RdfTerm::iri(sh::VALIDATION_RESULT),
            );

            // sh:focusNode
            push_triple(
                &mut builder,
                result_subj.clone(),
                sh::FOCUS_NODE,
                r.focus_node.to_rdf_term(),
            );

            // sh:resultSeverity
            push_triple(
                &mut builder,
                result_subj.clone(),
                sh::RESULT_SEVERITY,
                RdfTerm::iri(r.severity.iri()),
            );

            // sh:sourceConstraintComponent
            push_triple(
                &mut builder,
                result_subj.clone(),
                sh::SOURCE_CONSTRAINT_COMPONENT,
                RdfTerm::iri(r.source_constraint_component.as_str()),
            );

            // sh:sourceShape
            push_triple(
                &mut builder,
                result_subj.clone(),
                sh::SOURCE_SHAPE,
                r.source_shape.to_rdf_term(),
            );

            // sh:resultPath (optional). A complex path is a blank node; its
            // full SHACL path structure is emitted once per distinct root label
            // (two results sharing a path share the structure bnodes).
            if let Some(path) = &r.result_path {
                push_triple(
                    &mut builder,
                    result_subj.clone(),
                    sh::RESULT_PATH,
                    path.to_rdf_term(),
                );
                if let (Term::BlankNode(label), Some(structure)) = (path, &r.path_structure)
                    && emitted_paths.insert(label.clone())
                {
                    emit_path_structure(&mut builder, label, structure);
                }
            }

            // sh:value (optional)
            if let Some(value) = &r.value {
                push_triple(
                    &mut builder,
                    result_subj.clone(),
                    sh::VALUE,
                    value.to_rdf_term(),
                );
            }

            // sh:resultMessage (optional plain string literal)
            if let Some(msg) = &r.message {
                push_triple(
                    &mut builder,
                    result_subj.clone(),
                    sh::RESULT_MESSAGE,
                    RdfTerm::Literal(::purrdf::RdfLiteral::simple(msg.as_str())),
                );
            }
        }

        let dataset = builder.freeze().expect("report quads freeze into the IR");
        let buf = serialize_dataset(
            &dataset,
            "application/n-quads",
            SerializeGraph::DefaultGraph,
        )
        .expect("native N-Triples serialisation of report quads is infallible");
        String::from_utf8(buf).expect("native N-Triples output is valid UTF-8")
    }

    /// Return the result set as a [`BTreeSet`] of [`ResultTuple`]s for
    /// deterministic equality comparison in tests and conformance checks.
    pub fn result_tuples(&self) -> BTreeSet<ResultTuple> {
        self.results
            .iter()
            .map(|r| {
                (
                    r.focus_node.to_string(),
                    r.result_path.as_ref().map(ToString::to_string),
                    r.value.as_ref().map(ToString::to_string),
                    r.source_constraint_component.to_string(),
                    r.source_shape.to_string(),
                    r.severity.clone(),
                )
            })
            .collect()
    }
}

// ── Builder helpers ───────────────────────────────────────────────────────────

/// Push a triple (default graph) into the report dataset builder.
fn push_triple(
    builder: &mut RdfDatasetBuilder,
    subject: RdfTerm,
    predicate: &str,
    object: RdfTerm,
) {
    builder.push_owned_quad(&RdfQuad::new(subject, predicate, object));
}

// ── SHACL path-structure serialization ────────────────────────────────────────

/// Emit the RDF structure of a COMPLEX SHACL path rooted at blank node `label`
/// (SHACL §2.3.1 path syntax: `[ sh:inversePath … ]`, sequence lists,
/// `[ sh:alternativePath ( … ) ]`, and the three closure forms). Interior blank
/// nodes are labelled `{label}-{n}` with a per-root counter, so the emission is
/// deterministic for a given path.
fn emit_path_structure(builder: &mut RdfDatasetBuilder, label: &str, path: &crate::shapes::Path) {
    let mut counter = 0usize;
    emit_path_node(builder, label, path, label, &mut counter);
}

/// Allocate the next interior blank-node label under `root`.
fn next_path_label(root: &str, counter: &mut usize) -> String {
    let label = format!("{root}-{counter}");
    *counter += 1;
    label
}

/// The RDF term standing for a sub-path: a plain predicate inlines as its IRI;
/// a composite sub-path becomes a fresh blank node whose structure is emitted
/// recursively.
fn path_object(
    builder: &mut RdfDatasetBuilder,
    path: &crate::shapes::Path,
    root: &str,
    counter: &mut usize,
) -> RdfTerm {
    if let crate::shapes::Path::Predicate(p) = path {
        return RdfTerm::iri(p.as_str());
    }
    let label = next_path_label(root, counter);
    emit_path_node(builder, &label, path, root, counter);
    RdfTerm::blank_node(label)
}

/// Emit the structure triples for the composite path node `label`.
fn emit_path_node(
    builder: &mut RdfDatasetBuilder,
    label: &str,
    path: &crate::shapes::Path,
    root: &str,
    counter: &mut usize,
) {
    use crate::shapes::Path;
    match path {
        // A plain predicate is always inlined by `path_object`; a predicate
        // root never reaches here (`path_structure` is only set for complex paths).
        Path::Predicate(_) => {}
        Path::Inverse(inner) => {
            let object = path_object(builder, inner, root, counter);
            push_triple(
                builder,
                RdfTerm::blank_node(label),
                sh::INVERSE_PATH,
                object,
            );
        }
        Path::ZeroOrMore(inner) => {
            let object = path_object(builder, inner, root, counter);
            push_triple(
                builder,
                RdfTerm::blank_node(label),
                sh::ZERO_OR_MORE_PATH,
                object,
            );
        }
        Path::OneOrMore(inner) => {
            let object = path_object(builder, inner, root, counter);
            push_triple(
                builder,
                RdfTerm::blank_node(label),
                sh::ONE_OR_MORE_PATH,
                object,
            );
        }
        Path::ZeroOrOne(inner) => {
            let object = path_object(builder, inner, root, counter);
            push_triple(
                builder,
                RdfTerm::blank_node(label),
                sh::ZERO_OR_ONE_PATH,
                object,
            );
        }
        // A sequence path IS the RDF list (the list head sits in path position).
        Path::Sequence(parts) => {
            emit_path_list(builder, label, parts, root, counter);
        }
        // An alternative path wraps its list under sh:alternativePath.
        Path::Alternative(parts) => {
            let head = next_path_label(root, counter);
            push_triple(
                builder,
                RdfTerm::blank_node(label),
                sh::ALTERNATIVE_PATH,
                RdfTerm::blank_node(head.clone()),
            );
            emit_path_list(builder, &head, parts, root, counter);
        }
    }
}

/// Emit an RDF collection of sub-paths with `head_label` as the first cell.
fn emit_path_list(
    builder: &mut RdfDatasetBuilder,
    head_label: &str,
    parts: &[crate::shapes::Path],
    root: &str,
    counter: &mut usize,
) {
    let mut cell = head_label.to_owned();
    for (i, part) in parts.iter().enumerate() {
        let object = path_object(builder, part, root, counter);
        push_triple(
            builder,
            RdfTerm::blank_node(cell.clone()),
            rdf::FIRST,
            object,
        );
        if i + 1 == parts.len() {
            push_triple(
                builder,
                RdfTerm::blank_node(cell.clone()),
                rdf::REST,
                RdfTerm::iri(rdf::NIL),
            );
        } else {
            let next = next_path_label(root, counter);
            push_triple(
                builder,
                RdfTerm::blank_node(cell),
                rdf::REST,
                RdfTerm::blank_node(next.clone()),
            );
            cell = next;
        }
    }
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

/// Extract the `sh:conforms` boolean from an N-Triples SHACL report string.
///
/// # Errors
///
/// Returns an error string if the N-Triples cannot be parsed.
pub fn conforms_from_ntriples(nt: &str) -> Result<bool, String> {
    let data = dataset_from_ntriples(nt)?;
    Ok(conforms_from_dataset(&data).unwrap_or(true))
}

/// Extract a `BTreeSet<ResultTuple>` from an N-Triples SHACL report string.
///
/// # Errors
///
/// Returns an error string if the N-Triples cannot be parsed.
pub fn tuples_from_ntriples(nt: &str) -> Result<BTreeSet<ResultTuple>, String> {
    let data = dataset_from_ntriples(nt)?;
    Ok(tuples_from_dataset(&data))
}

/// Parse a SHACL report N-Triples string into a query-able frozen [`RdfDataset`]
/// via the native purrdf codec — no oxigraph.
fn dataset_from_ntriples(nt: &str) -> Result<std::sync::Arc<RdfDataset>, String> {
    ::purrdf::parse_dataset(nt.as_bytes(), "application/n-triples", None)
        .map_err(|e| format!("N-Triples parse error: {e}"))
}

/// Walk a SHACL report dataset and extract result tuples.
///
/// Finds all `?r rdf:type sh:ValidationResult` nodes and reads their mandatory and
/// optional predicates, building the same tuple shape as
/// [`ValidationReport::result_tuples`].
pub fn tuples_from_dataset(data: &RdfDataset) -> BTreeSet<ResultTuple> {
    let result_nodes = subjects_typed(data, sh::VALIDATION_RESULT);

    let mut tuples = BTreeSet::new();

    for result_node in result_nodes {
        let focus = object_string(data, &result_node, sh::FOCUS_NODE).unwrap_or_default();
        let path = object_string(data, &result_node, sh::RESULT_PATH);
        let value = object_string(data, &result_node, sh::VALUE);
        let component =
            object_string(data, &result_node, sh::SOURCE_CONSTRAINT_COMPONENT).unwrap_or_default();
        let source_shape = object_string(data, &result_node, sh::SOURCE_SHAPE).unwrap_or_default();
        let severity_iri =
            object_string(data, &result_node, sh::RESULT_SEVERITY).unwrap_or_default();

        // Parse severity from the IRI string (strip angle brackets the term
        // rendering adds for a NamedNode). A non-built-in severity IRI is
        // preserved verbatim (Severity::Other); a missing severity defaults to
        // sh:Violation.
        let sev_str = severity_iri.trim_matches(|c| c == '<' || c == '>');
        let severity = if sev_str.is_empty() {
            Severity::Violation
        } else {
            Severity::from_iri(sev_str).unwrap_or_else(|| Severity::Other(NamedNode::from(sev_str)))
        };

        tuples.insert((focus, path, value, component, source_shape, severity));
    }

    tuples
}

/// Extract the `sh:conforms` boolean from a report dataset, if present.
pub fn conforms_from_dataset(data: &RdfDataset) -> Option<bool> {
    let report_node = subjects_typed(data, sh::VALIDATION_REPORT)
        .into_iter()
        .next()?;
    let raw = object_string(data, &report_node, sh::CONFORMS)?;
    // The boolean literal renders as `"true"^^<xsd:boolean>` (typed).
    match raw.as_str() {
        s if s.starts_with("\"true\"") => Some(true),
        s if s.starts_with("\"false\"") => Some(false),
        _ => None,
    }
}

// ── Internal query helpers ────────────────────────────────────────────────────

/// All subjects of `(?, rdf:type, class_iri)` in the report dataset.
fn subjects_typed(data: &RdfDataset, class_iri: &str) -> Vec<Term> {
    let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
    let class = Term::NamedNode(NamedNode::from(class_iri));
    native_quads(
        data,
        None,
        Some(&rdf_type),
        Some(&class),
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(subject, _, _)| subject)
    .collect()
}

/// Return the first object of `(subj, pred, ?)` as a `Term::to_string()` string.
fn object_string(data: &RdfDataset, subj: &Term, pred: &str) -> Option<String> {
    let predicate = Term::NamedNode(NamedNode::from(pred));
    native_quads(
        data,
        Some(subj),
        Some(&predicate),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .next()
    .map(|(_, _, object)| object.to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result() -> ValidationResult {
        ValidationResult {
            focus_node: Term::NamedNode(NamedNode::new_unchecked("http://example.org/focusA")),
            result_path: Some(Term::NamedNode(NamedNode::new_unchecked(
                "http://example.org/predP",
            ))),
            path_structure: None,
            value: Some(Term::Literal(Literal::new_simple_literal("bad value"))),
            source_constraint_component: NamedNode::new_unchecked(
                "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
            ),
            source_shape: Term::NamedNode(NamedNode::new_unchecked("http://example.org/ShapeA")),
            severity: Severity::Violation,
            message: Some("must have at least one value".to_owned()),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
        }
    }

    #[test]
    fn report_round_trip_with_one_result() {
        let report = ValidationReport {
            conforms: false,
            results: vec![make_result()],
        };

        let nt = report.to_ntriples();
        assert!(!nt.is_empty(), "N-Triples output must not be empty");

        let parsed =
            tuples_from_ntriples(&nt).expect("N-Triples from to_ntriples() must parse cleanly");
        let expected = report.result_tuples();

        assert_eq!(
            parsed, expected,
            "round-trip tuples must match original tuples"
        );
    }

    #[test]
    fn empty_conforming_report_round_trips() {
        let report = ValidationReport {
            conforms: true,
            results: vec![],
        };

        let nt = report.to_ntriples();

        // conforms=true must appear in the N-Triples
        assert!(
            nt.contains("true"),
            "N-Triples must contain 'true' for sh:conforms"
        );

        let parsed =
            tuples_from_ntriples(&nt).expect("N-Triples from empty report must parse cleanly");
        assert!(parsed.is_empty(), "empty report must produce zero tuples");

        // Check conforms_from_dataset directly
        let data = dataset_from_ntriples(&nt).unwrap();
        assert_eq!(conforms_from_dataset(&data), Some(true));
    }

    #[test]
    fn conforms_from_ntriples_true() {
        let nt = "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/shacl#ValidationReport> .\n\
                  _:r <http://www.w3.org/ns/shacl#conforms> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n";
        assert!(
            conforms_from_ntriples(nt).expect("must parse"),
            "conforming report must return true"
        );
    }

    #[test]
    fn conforms_from_ntriples_false() {
        let nt = "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/shacl#ValidationReport> .\n\
                  _:r <http://www.w3.org/ns/shacl#conforms> \"false\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n";
        assert!(
            !conforms_from_ntriples(nt).expect("must parse"),
            "violating report must return false"
        );
    }

    #[test]
    fn conforms_from_ntriples_parse_error() {
        let bad = "not valid ntriples @@@\n";
        assert!(
            conforms_from_ntriples(bad).is_err(),
            "invalid N-Triples must return Err"
        );
    }

    #[test]
    fn conforms_from_ntriples_no_report_node_defaults_true() {
        // Empty graph has no sh:ValidationReport → unwrap_or(true)
        let nt = "";
        assert!(
            conforms_from_ntriples(nt).expect("empty must parse"),
            "missing sh:conforms defaults to true"
        );
    }

    #[test]
    fn severity_iri_round_trip() {
        for sev in [Severity::Violation, Severity::Warning, Severity::Info] {
            let iri = sev.iri().to_owned();
            let parsed = Severity::from_iri(&iri);
            assert_eq!(
                parsed.as_ref(),
                Some(&sev),
                "from_iri(iri()) must round-trip for {sev:?}"
            );
        }
    }

    #[test]
    fn severity_from_iri_unknown_returns_none() {
        assert!(Severity::from_iri("http://example.org/Unknown").is_none());
    }

    // ── S5 attribution tests ──────────────────────────────────────────────────

    /// Test 1: Cross-slice SHACL distinct roles.
    ///
    /// A SHACL result where the SHAPE is owned by slice A and the FOCUS NODE
    /// data is asserted by slice B. The result records TWO attributions:
    /// `ShapeOwner = A`, `FocusOrigin = B` — distinct units with distinct roles.
    #[test]
    fn cross_slice_shacl_distinct_roles() {
        use ::purrdf::provenance::{Attribution, AttributionRole, UnitInterner};

        let mut interner = UnitInterner::new();
        let unit_a = interner.intern("https://example.org/slices/core/shapes"); // slice A — owns the shape
        let unit_b = interner.intern("https://example.org/slices/ext/data"); // slice B — asserts the focus node

        let mut result = make_result();
        // Apply two attributions with different roles and different units.
        result.attributions = vec![
            Attribution {
                unit: unit_a,
                role: AttributionRole::ShapeOwner,
                evidence: Some("slices/core/epistemics/shapes.ttl".to_owned()),
            },
            Attribution {
                unit: unit_b,
                role: AttributionRole::FocusOrigin,
                evidence: Some("http://example.org/focusA".to_owned()),
            },
        ];

        assert_eq!(result.attributions.len(), 2, "must carry two attributions");

        // The two attributions must reference different units.
        assert_ne!(
            result.attributions[0].unit, result.attributions[1].unit,
            "shape-owner and focus-origin units must be distinct (cross-slice)"
        );

        // The roles must be distinct.
        assert_ne!(
            result.attributions[0].role, result.attributions[1].role,
            "roles must differ"
        );
        assert_eq!(result.attributions[0].role, AttributionRole::ShapeOwner);
        assert_eq!(result.attributions[1].role, AttributionRole::FocusOrigin);
    }

    /// Test 2: Absence-based violation (`sh:minCount`) attribution.
    ///
    /// A `minCount` violation has NO offending data quad — there is no value to
    /// attribute. The result still carries EvaluationScope + ShapeOwner
    /// attributions. `AssertionOrigin` / `FocusOrigin` / `ValueOrigin` are NOT
    /// required (and not asserted here).
    #[test]
    fn absence_based_violation_carries_scope_and_shape_attributions() {
        use ::purrdf::provenance::{Attribution, AttributionRole, UnitInterner};

        let mut interner = UnitInterner::new();
        let unit_shape = interner.intern("https://example.org/slices/core/shapes"); // owns the sh:minCount shape
        let unit_scope = interner.intern("https://example.org/slices/core/profile"); // defines the evaluation scope

        let min_count_result = ValidationResult {
            focus_node: Term::NamedNode(NamedNode::new_unchecked("http://example.org/subjectX")),
            // minCount: no result_path (not path-scoped here for simplicity).
            result_path: None,
            path_structure: None,
            // No offending value — absence-based.
            value: None,
            source_constraint_component: NamedNode::new_unchecked(
                "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
            ),
            source_shape: Term::NamedNode(NamedNode::new_unchecked(
                "http://example.org/RequiredPropertyShape",
            )),
            severity: Severity::Violation,
            message: Some("missing required property".to_owned()),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            // ShapeOwner + EvaluationScope — no AssertionOrigin (nothing was asserted).
            attributions: vec![
                Attribution {
                    unit: unit_shape,
                    role: AttributionRole::ShapeOwner,
                    evidence: None,
                },
                Attribution {
                    unit: unit_scope,
                    role: AttributionRole::EvaluationScope,
                    evidence: None,
                },
            ],
        };

        // No value (absence-based) — this is the critical invariant.
        assert!(
            min_count_result.value.is_none(),
            "minCount violation must have no offending value"
        );
        // No AssertionOrigin — nothing was asserted.
        assert!(
            !min_count_result
                .attributions
                .iter()
                .any(|a| a.role == AttributionRole::AssertionOrigin),
            "absence-based violation must not carry AssertionOrigin"
        );
        // Shape owner is present.
        assert!(
            min_count_result
                .attributions
                .iter()
                .any(|a| a.role == AttributionRole::ShapeOwner),
            "must carry ShapeOwner attribution"
        );
        // Evaluation scope is present.
        assert!(
            min_count_result
                .attributions
                .iter()
                .any(|a| a.role == AttributionRole::EvaluationScope),
            "must carry EvaluationScope attribution"
        );
    }
}
