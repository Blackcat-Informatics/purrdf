// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL validation report types and serialization.
//!
//! [`ValidationReport`] is the in-memory representation of a SHACL report
//! graph. `to_dataset()` materializes that graph as a frozen PurRDF
//! [`RdfDataset`] — the report's PRIMAL RDF form — and `to_ntriples()` is a
//! serialization of exactly that dataset. A caller who wants any other syntax
//! takes `to_dataset()` straight to `serialize_dataset`, with no N-Triples
//! parse round-trip in between.
//! `tuples_from_ntriples()` round-trips back to the same tuple set for testing.

use std::collections::BTreeSet;
use std::sync::Arc;

use ::purrdf::FastSet;
use ::purrdf::RdfDatasetBuilder;
use ::purrdf::provenance::Attribution;
use ::purrdf::{RdfQuad, RdfTerm, SerializeGraph, serialize_dataset};

use ::purrdf::RdfDataset;

use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, sh, xsd};
use crate::term::{Literal, NamedNode, Term};

// ── Severity ──────────────────────────────────────────────────────────────────

/// SHACL result severity levels, ordered from most to least severe.
///
/// SHACL 1.2 Core, "Declaring the Severity of a Shape or Constraint", names five built-in levels — "SHACL includes the IRIs listed in the table
/// below to represent severities": `sh:Trace` ("A trace message that is not a
/// constraint violation"), `sh:Debug` ("A debug message that is not a constraint
/// violation"), `sh:Info` ("A non-critical constraint violation indicating an
/// informative message"), `sh:Warning` ("A non-critical constraint violation
/// indicating a warning") and `sh:Violation` ("A constraint violation"). "Any IRI
/// can be used as a severity", so a custom severity IRI is carried verbatim in
/// [`Severity::Other`] and reports preserve it (W3C `core/misc/severity-002`)
/// instead of coercing it to `sh:Violation`.
///
/// Whether a level blocks conformance is NOT a property of the level: it is
/// decided by the validation request's [`ConformanceDisallows`] set.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// `sh:Violation` — the most severe level.
    Violation,
    /// `sh:Warning`.
    Warning,
    /// `sh:Info`.
    Info,
    /// `sh:Debug` — "a debug message that is not a constraint violation".
    Debug,
    /// `sh:Trace` — the least severe built-in level, "a trace message that is not
    /// a constraint violation".
    Trace,
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
            Self::Debug => sh::DEBUG,
            Self::Trace => sh::TRACE,
            Self::Other(iri) => iri.as_str(),
        }
    }

    /// Parse one of the five built-in `sh:` severities from its IRI string,
    /// returning `None` if unrecognised (use [`Severity::from_iri_open`] to carry
    /// a custom severity IRI).
    pub fn from_iri(s: &str) -> Option<Self> {
        match s {
            sh::VIOLATION => Some(Self::Violation),
            sh::WARNING => Some(Self::Warning),
            sh::INFO => Some(Self::Info),
            sh::DEBUG => Some(Self::Debug),
            sh::TRACE => Some(Self::Trace),
            _ => None,
        }
    }

    /// The severity an IRI names: a built-in level for one of the five `sh:`
    /// severity IRIs, and [`Severity::Other`] carrying the IRI verbatim for any
    /// other ("Any IRI can be used as a severity").
    #[must_use]
    pub fn from_iri_open(s: &str) -> Self {
        Self::from_iri(s).unwrap_or_else(|| Self::Other(NamedNode::from(s)))
    }

    /// This level's bit in a [`ConformanceDisallows`] built-in mask, `None` for a
    /// custom IRI.
    const fn builtin_bit(&self) -> Option<u8> {
        match self {
            Self::Violation => Some(1),
            Self::Warning => Some(1 << 1),
            Self::Info => Some(1 << 2),
            Self::Debug => Some(1 << 3),
            Self::Trace => Some(1 << 4),
            Self::Other(_) => None,
        }
    }
}

// ── The conformance-disallow set ─────────────────────────────────────────────

/// The five built-in levels, in [`Severity`] order.
const BUILTIN_SEVERITIES: [Severity; 5] = [
    Severity::Violation,
    Severity::Warning,
    Severity::Info,
    Severity::Debug,
    Severity::Trace,
];

/// The built-in mask of the default set: `sh:Violation`, `sh:Warning`, `sh:Info`.
const DEFAULT_DISALLOW_MASK: u8 = 0b111;

/// The set of disallowed severity levels a validation checks conformance against.
///
/// SHACL 1.2 Core, "Conformance Checking": "A focus node conforms to a shape
/// if and only if the set of result of the validation of the focus node against
/// the shape does not contain any validation results with a severity level of the
/// set of disallowed levels and no failure has been reported by it. The set of
/// disallowed severity levels is defined as the objects of triples with predicate
/// sh:conformanceDisallows and the validation report as subject. If the
/// validation report contains no such triples, sh:Violation, sh:Warning, and
/// sh:Info are set as defaults."
///
/// The "Conformance-Disallow Set" section makes it the engine's to choose: "The conformance-disallow set is
/// defined by the validation engine. A validation engine MAY provide mechanisms
/// to customize this set." This type is that mechanism. It is a parameter of the
/// validation REQUEST ([`crate::engine::ValidationOptions`]), never read from the
/// shapes graph, and the same set decides both the report's `sh:conforms` and
/// every nested conformance check (`sh:node`, `sh:not`, `sh:and`, `sh:or`,
/// `sh:xone`, `sh:qualifiedValueShape`, …) the run performs — "all
/// shape-expecting constraint parameters of SHACL Core rely on conformance
/// checking" with that one definition.
///
/// # The empty set is refused
///
/// The report ECHOES the set ([`ValidationReport::to_dataset`]), and a report
/// with no `sh:conformanceDisallows` triple means the DEFAULT set. So an empty set
/// has no report that states it: the report it produced would be read back as
/// the default set, contradicting the `sh:conforms` it carries. It is refused at
/// construction rather than emitted as a report that means something else.
///
/// The representation is a bit mask over the five built-in levels plus the sorted
/// custom IRIs, so the default set — and every set without a custom level —
/// costs no allocation to build, clone or consult.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConformanceDisallows {
    /// One bit per built-in level ([`Severity::builtin_bit`]).
    builtin: u8,
    /// The custom severity IRIs, sorted and de-duplicated.
    others: Box<[NamedNode]>,
}

impl Default for ConformanceDisallows {
    /// `sh:Violation`, `sh:Warning` and `sh:Info` — the set SHACL 1.2 Core names
    /// for a report that declares none.
    fn default() -> Self {
        Self {
            builtin: DEFAULT_DISALLOW_MASK,
            others: Box::new([]),
        }
    }
}

impl ConformanceDisallows {
    /// A set holding exactly `levels`.
    ///
    /// # Errors
    ///
    /// Returns an error when `levels` is empty: see the type docs for why an empty
    /// set cannot be echoed truthfully.
    pub fn new(levels: impl IntoIterator<Item = Severity>) -> Result<Self, String> {
        let mut builtin = 0u8;
        let mut others: Vec<NamedNode> = Vec::new();
        for level in levels {
            match level.builtin_bit() {
                Some(bit) => builtin |= bit,
                None => {
                    if let Severity::Other(iri) = level {
                        others.push(iri);
                    }
                }
            }
        }
        if builtin == 0 && others.is_empty() {
            return Err(
                "the conformance-disallow set is empty; a validation report with no \
                 sh:conformanceDisallows triple means the DEFAULT set (sh:Violation, sh:Warning, \
                 sh:Info), so an empty set has no report that states it — name at least one \
                 severity"
                    .to_owned(),
            );
        }
        others.sort_unstable();
        others.dedup();
        Ok(Self {
            builtin,
            others: others.into_boxed_slice(),
        })
    }

    /// A set holding the severity levels the IRIs name (a built-in level for an
    /// `sh:` severity IRI, [`Severity::Other`] for any other).
    ///
    /// # Errors
    ///
    /// Returns an error when `iris` is empty, or when an entry is not an absolute
    /// IRI ("All values of sh:conformanceDisallows MUST be IRIs", SHACL 1.2 Core,
    /// "Conformance-Disallow Set").
    pub fn from_iris<I, S>(iris: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut levels = Vec::new();
        for iri in iris {
            let iri = iri.as_ref();
            let absolute = purrdf_iri::parse(iri).is_ok_and(|parsed| parsed.has_scheme());
            if !absolute {
                return Err(format!(
                    "sh:conformanceDisallows value {iri:?} is not an absolute IRI; all values of \
                     sh:conformanceDisallows must be IRIs"
                ));
            }
            levels.push(Severity::from_iri_open(iri));
        }
        Self::new(levels)
    }

    /// Whether a result of `severity` blocks conformance.
    #[inline]
    #[must_use]
    pub fn contains(&self, severity: &Severity) -> bool {
        match severity.builtin_bit() {
            Some(bit) => self.builtin & bit != 0,
            None => match severity {
                Severity::Other(iri) => self.others.binary_search(iri).is_ok(),
                _ => false,
            },
        }
    }

    /// Whether this is the default set (`sh:Violation`, `sh:Warning`, `sh:Info`).
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.builtin == DEFAULT_DISALLOW_MASK && self.others.is_empty()
    }

    /// The levels in the set, in [`Severity`] order: the built-in levels most
    /// severe first, then the custom IRIs in IRI order.
    #[must_use]
    pub fn levels(&self) -> Vec<Severity> {
        BUILTIN_SEVERITIES
            .iter()
            .filter(|level| self.contains(level))
            .cloned()
            .chain(self.others.iter().cloned().map(Severity::Other))
            .collect()
    }

    /// The set as IRI strings, in [`Self::levels`] order.
    #[must_use]
    pub fn iris(&self) -> Vec<String> {
        self.levels()
            .iter()
            .map(|level| level.iri().to_owned())
            .collect()
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
    /// The result's messages (`sh:resultMessage`), each an RDF 1.2 literal that
    /// keeps its datatype, language tag and base direction, in the canonical order
    /// [`canonical_messages`] gives. Empty when the result has no message.
    ///
    /// SHACL 1.2 Core, "Declaring Messages for a Shape or Constraint": "If a shape
    /// has at least one value for sh:message in the shapes graph, then all
    /// validation results produced as a result of the shape will have exactly
    /// these messages as their value of sh:resultMessage, i.e. the values will be
    /// copied from the shapes graph into the results graph." So every message is
    /// carried — `"Too many characters"@en` beside `"Zu viele Zeichen"@de` — never
    /// one chosen from several.
    pub messages: Vec<Literal>,
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
    /// The nested results that detail this one (`sh:detail`, SHACL 1.2 Core
    /// §3.6.2.7), in a deterministic order: for `sh:memberShape`, the results of
    /// each list member that does not conform to the member shape; for
    /// `sh:uniqueMembers`, one result per duplicated member. Empty for every
    /// other component.
    pub details: Vec<Self>,
    /// The result's SHACL-SPARQL result annotations: `(annotation property,
    /// value)` pairs the SPARQL-based constraint or validator that produced it
    /// declares with `sh:resultAnnotation`, copied from the solution's binding of
    /// the annotation's variable or, when it is unbound, from its
    /// `sh:annotationValue` defaults (SHACL 1.2 SPARQL Extensions, "Annotation
    /// Properties"). Sorted by property IRI, then by the value's N-Triples
    /// rendering, without duplicates; empty for every other result.
    pub annotations: Vec<(NamedNode, Term)>,
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
    ///
    /// The box-role feature is caller-configured and inactive when no vocabulary
    /// supplies one, so the overwhelmingly common call has two empty inputs and
    /// three empty outputs; that case clears in place and touches the allocator
    /// not at all.
    pub fn apply_box_roles(&mut self, source_roles: &[NamedNode], path_roles: &[NamedNode]) {
        if source_roles.is_empty() && path_roles.is_empty() {
            self.source_box_roles.clear();
            self.path_box_roles.clear();
            self.result_box_roles.clear();
            return;
        }
        self.source_box_roles = dedup_roles(source_roles);
        self.path_box_roles = dedup_roles(path_roles);
        // The union is built straight from the two deduped halves — a separate
        // `merged` copy would only be sorted and deduped again.
        let mut result_roles =
            Vec::with_capacity(self.source_box_roles.len() + self.path_box_roles.len());
        result_roles.extend_from_slice(&self.source_box_roles);
        result_roles.extend_from_slice(&self.path_box_roles);
        result_roles.sort_unstable();
        result_roles.dedup();
        self.result_box_roles = result_roles;
    }
}

/// The canonical order of a result's messages: by lexical form, then language
/// tag, then base direction, then datatype — so two literals that differ only in
/// their tag (or in being `rdf:HTML` rather than `xsd:string`) are ordered and
/// kept apart, and equal ones collapse.
#[must_use]
pub fn canonical_messages(mut messages: Vec<Literal>) -> Vec<Literal> {
    messages.sort_by(|a, b| message_key(a).cmp(&message_key(b)));
    messages.dedup();
    messages
}

/// The sort key of one message literal — see [`canonical_messages`].
fn message_key(message: &Literal) -> (&str, Option<&str>, Option<u8>, &str) {
    (
        message.value(),
        message.language(),
        message.direction().map(|direction| match direction {
            ::purrdf::RdfTextDirection::Ltr => 0,
            ::purrdf::RdfTextDirection::Rtl => 1,
        }),
        message.datatype_str(),
    )
}

/// A deterministic textual key for a result's messages, for total sort orders.
pub(crate) fn messages_sort_key(messages: &[Literal]) -> String {
    messages
        .iter()
        .map(|m| Term::Literal(m.clone()).to_string())
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

/// A deterministic textual key for a result's annotations, for total sort orders.
/// Allocates nothing for the common result, which has none.
pub(crate) fn annotations_sort_key(annotations: &[(NamedNode, Term)]) -> String {
    annotations
        .iter()
        .map(|(property, value)| format!("{property}\u{1e}{value}"))
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

fn dedup_roles(roles: &[NamedNode]) -> Vec<NamedNode> {
    if roles.is_empty() {
        return Vec::new();
    }
    let mut out = roles.to_vec();
    out.sort_unstable();
    out.dedup();
    out
}

// ── ValidationReport ─────────────────────────────────────────────────────────

/// A SHACL validation report (`sh:ValidationReport`).
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Whether the data graph conforms to the shapes graph: no result's severity
    /// is in [`Self::conformance_disallows`] (and no failure was reported, which
    /// is an `Err` rather than a report).
    pub conforms: bool,
    /// Individual violation/warning/info/debug/trace results.
    pub results: Vec<ValidationResult>,
    /// The conformance-disallow set `conforms` was judged against, which the
    /// report graph echoes as `sh:conformanceDisallows` (see
    /// [`Self::to_dataset`]).
    pub conformance_disallows: ConformanceDisallows,
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
    /// The report of `results` judged against `disallows`: it conforms iff no
    /// result's severity is in the set (SHACL 1.2 Core, "Conformance-Disallow
    /// Set": "Presence of any sh:ValidationResult with a severity level in the set
    /// of disallowed severity levels MUST result in a sh:conforms value of false
    /// on the associated sh:ValidationReport instance").
    #[must_use]
    pub fn from_results(results: Vec<ValidationResult>, disallows: ConformanceDisallows) -> Self {
        let conforms = !results
            .iter()
            .any(|result| disallows.contains(&result.severity));
        Self {
            conforms,
            results,
            conformance_disallows: disallows,
        }
    }

    /// Materialize the report graph as a frozen PurRDF [`RdfDataset`].
    ///
    /// This is the report's primal RDF form: the quads are built straight from
    /// the report's own [`Term`] values into an [`RdfDatasetBuilder`] and frozen.
    /// [`ValidationReport::to_ntriples`] is a *serialization of this dataset*, so
    /// rendering a report in any other syntax is
    ///
    /// ```no_run
    /// # use purrdf_shapes::report::ValidationReport;
    /// # use purrdf::{SerializeGraph, serialize_dataset};
    /// # fn f(report: &ValidationReport) -> Result<Vec<u8>, purrdf::RdfDiagnostic> {
    /// serialize_dataset(&report.to_dataset(), "text/turtle", SerializeGraph::DefaultGraph)
    /// # }
    /// ```
    ///
    /// rather than a `to_ntriples()` → `parse_dataset()` round-trip. Avoiding
    /// that round-trip is not merely a speed-up: the direct path carries every
    /// RDF 1.2 term the report holds (a triple-term focus node or value included)
    /// with the report's own blank-node labels, instead of whatever survives a
    /// text grammar and gets relabelled by a parser.
    ///
    /// Everything the report can express lives in the default graph; the returned
    /// dataset declares no named graphs and populates no reifier/annotation side
    /// table.
    ///
    /// The blank nodes the report MINTS (the report node, one per result, and the
    /// interior nodes of a complex `sh:path`) are guaranteed distinct from every
    /// blank node the report CARRIES: a data graph is free to contain `_:r0`, and
    /// the minted nodes step into a reserved label namespace when it does.
    #[must_use]
    pub fn to_dataset(&self) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        // Complex-path structure roots already emitted (keyed by root label).
        let mut emitted_paths: FastSet<String> = FastSet::default();

        // Reserve a label namespace for the nodes this function invents, so a
        // data graph that happens to use `_:report` or `_:r0` cannot be fused
        // with the report's own structure. Empty (and so byte-identical to a
        // report with no such data) unless the report really does carry a
        // colliding label.
        let mint = mint_prefix(self);
        let report_subj = RdfTerm::blank_node(format!("{mint}report"));

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

        // _:report sh:conformanceDisallows <level> — the set `sh:conforms` was
        // judged against. "If no values are present in the results graph for the
        // property sh:conformanceDisallows, then a default set MUST be used,
        // comprised of sh:Violation, sh:Warning, and sh:Info", so the default set
        // is echoed by stating nothing: that is exactly what a reader recovers from
        // it, and every report under the default set keeps its bytes. Any other
        // set is echoed level by level, in `ConformanceDisallows::levels` order.
        if !self.conformance_disallows.is_default() {
            for level in self.conformance_disallows.levels() {
                push_triple(
                    &mut builder,
                    report_subj.clone(),
                    sh::CONFORMANCE_DISALLOWS,
                    RdfTerm::iri(level.iri()),
                );
            }
        }

        for (i, r) in self.results.iter().enumerate() {
            let label = format!("{mint}r{i}");
            // _:report sh:result _:r{i}
            push_triple(
                &mut builder,
                report_subj.clone(),
                sh::RESULT,
                RdfTerm::blank_node(label.clone()),
            );
            emit_result(&mut builder, &label, r, &mint, &mut emitted_paths);
        }

        // `freeze` only rejects structural violations (out-of-range term ids, a
        // literal subject, a non-IRI predicate/graph name, reifier/annotation
        // targets that are not triple terms, triple-term cycles) — none of which
        // this builder can produce: every quad above is pushed with a fixed IRI
        // predicate and a well-formed subject/object built from validated report
        // data. Blank-node LABEL content is never checked here; it is opaque to
        // the IR and is escaped, never rejected, at codec egress (see
        // `to_ntriples`).
        builder.freeze().expect("report quads freeze into the IR")
    }

    /// Emit the report as N-Triples text using the native purrdf codec.
    ///
    /// The report graph is materialized by [`ValidationReport::to_dataset`] and
    /// serialized from there, so the text and the dataset are the same graph by
    /// construction. This avoids hand-rolling literal escaping and carries no
    /// oxigraph `io` dependency. The `DefaultGraph` selection on the
    /// `application/n-quads` codec emits graphless rows (i.e. N-Triples) and is
    /// byte-lenient on language tags, matching the legacy oxigraph serializer.
    #[must_use]
    pub fn to_ntriples(&self) -> String {
        let dataset = self.to_dataset();
        // `serialize_dataset` is `Result` for two reasons that both provably do
        // not apply here: (1) `classify(media_type)` can reject an unknown media
        // type, but `"application/n-quads"` is a constant, always-valid literal;
        // (2) the interner can fail on a reifier target that is not a triple term
        // or a literal datatype that is not an IRI, but this dataset never
        // populates the reifier/annotation side table and every literal is built
        // from `RdfLiteral::typed`/`simple`, which always carry an IRI datatype.
        // A blank label illegal in the N-Triples `BLANK_NODE_LABEL` grammar
        // (e.g. `a×b`, a C0 control) is escaped at intern time, never refused —
        // see `to_ntriples_survives_hostile_blank_labels_in_focus_nodes` below,
        // which proves this claim against exactly that hostile input.
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

// ── Minted blank-node labels ──────────────────────────────────────────────────

/// The prefix every blank node the report MINTS carries, so a minted node can
/// never be the same node as one the report CARRIES.
///
/// # The bug this closes
///
/// The report invents `_:report`, `_:r0`, `_:r1`, … and the interior nodes of a
/// complex `sh:path`. Blank-node labels reaching the report from the data or
/// shapes graph are ordinary opaque strings — a data graph is entirely free to
/// contain `_:r0` — and a blank label at [`::purrdf::BlankScope::DEFAULT`] passes
/// through the IR verbatim. Validating such a graph used to emit
///
/// ```text
/// _:r0 a sh:ValidationResult ; sh:focusNode _:r0 .
/// ```
///
/// fusing the validation result with the node it is reporting on: the report
/// asserted that a `sh:ValidationResult` was an instance of the data's class, and
/// a consumer following `sh:focusNode` landed back on the result. Nothing was
/// dropped and nothing failed — two distinct nodes silently became one.
///
/// # The rule
///
/// Return `""` when no carried label collides with a minted one, which is the
/// overwhelmingly common case and keeps the emitted bytes byte-identical to
/// before. Otherwise return the shortest `_`-run that NO carried label starts
/// with; prefixing every minted label with it puts the minted nodes in a label
/// namespace the carried labels provably do not reach. `_` is `PN_CHARS_U`, so
/// the marked label is a legal `BLANK_NODE_LABEL` as written and the text codecs
/// never have to escape it.
///
/// The search is bounded, not merely terminating: only a carried label at least
/// `k` bytes long can start with a `k`-long run, so a run one byte longer than
/// the longest carried label is always free.
///
/// Linear in the report: the complex-path roots are collected once, and each
/// carried label is then checked with a constant number of hash lookups.
fn mint_prefix(report: &ValidationReport) -> String {
    let carried = carried_blank_labels(report);
    let roots = minted_path_roots(report);
    if !carried
        .iter()
        .any(|label| collides_with_minted(label, report.results.len(), &roots))
    {
        return String::new();
    }
    let ceiling = carried.iter().map(|label| label.len()).max().unwrap_or(0) + 1;
    // `1..` starts past the empty prefix just rejected; `ceiling` is free by
    // construction, so `find` always succeeds.
    (1..=ceiling)
        .map(|k| "_".repeat(k))
        .find(|mark| !carried.iter().any(|label| label.starts_with(mark.as_str())))
        .expect("a run longer than every carried label is always free")
}

/// Every blank-node label the report CARRIES — the ones that arrive from the data
/// or shapes graph rather than being invented here.
///
/// A complex path's root label is excluded: it is minted by
/// [`crate::path::path_to_term`], not carried, and it takes the mint prefix along
/// with everything else minted (see [`minted_path_roots`]).
fn carried_blank_labels(report: &ValidationReport) -> FastSet<&str> {
    let mut labels = FastSet::default();
    for r in &report.results {
        collect_result_blank_labels(r, &mut labels);
    }
    labels
}

/// [`carried_blank_labels`] for one result and every result nested under it.
fn collect_result_blank_labels<'a>(r: &'a ValidationResult, labels: &mut FastSet<&'a str>) {
    collect_blank_labels(&r.focus_node, labels);
    collect_blank_labels(&r.source_shape, labels);
    if let Some(value) = &r.value {
        collect_blank_labels(value, labels);
    }
    // A blank `result_path` with no `path_structure` is not a minted complex
    // path root (see `ValidationResult::result_path`), so it is carried.
    if let (Some(path), None) = (&r.result_path, &r.path_structure) {
        collect_blank_labels(path, labels);
    }
    // An annotation value is a solution binding, so it may be a data blank node.
    for (_, value) in &r.annotations {
        collect_blank_labels(value, labels);
    }
    for detail in &r.details {
        collect_result_blank_labels(detail, labels);
    }
}

/// The root labels of the report's complex paths — blank nodes the report MINTS
/// (a blank `result_path` paired with a `path_structure`), collected once so the
/// collision check is a hash lookup per carried label rather than a rescan of
/// every result.
fn minted_path_roots(report: &ValidationReport) -> FastSet<&str> {
    let mut roots = FastSet::default();
    for r in &report.results {
        collect_minted_path_roots(r, &mut roots);
    }
    roots
}

/// [`minted_path_roots`] for one result and every result nested under it.
fn collect_minted_path_roots<'a>(r: &'a ValidationResult, roots: &mut FastSet<&'a str>) {
    if let (Some(Term::BlankNode(root)), Some(_)) = (&r.result_path, &r.path_structure) {
        roots.insert(root.as_str());
    }
    for detail in &r.details {
        collect_minted_path_roots(detail, roots);
    }
}

/// Add every blank-node label reachable from `term`, descending through RDF 1.2
/// triple terms (a quoted triple's own subject/object are carried nodes too).
fn collect_blank_labels<'a>(term: &'a Term, labels: &mut FastSet<&'a str>) {
    match term {
        Term::BlankNode(label) => {
            labels.insert(label.as_str());
        }
        Term::Triple(t) => {
            collect_blank_labels(&t.subject, labels);
            collect_blank_labels(&t.object, labels);
        }
        Term::NamedNode(_) | Term::Literal(_) => {}
    }
}

/// Whether `label` is one of the labels the report would mint under the EMPTY
/// prefix: the report node, one of the `result_count` result nodes, a complex
/// path's root, or an interior node of one of the report's complex paths
/// (`{root}-{n}`, `n` a decimal counter — see [`next_path_label`]).
fn collides_with_minted(label: &str, result_count: usize, roots: &FastSet<&str>) -> bool {
    if label == "report" || roots.contains(label) {
        return true;
    }
    // A result node is `r{i}`, and a detail node nested under it
    // `r{i}d{j}d{k}…`: every such label is minted when `i` names a result.
    if let Some(rest) = label.strip_prefix('r') {
        let mut parts = rest.split('d');
        if let Some(index) = parts.next()
            && let Ok(index) = index.parse::<usize>()
            && index < result_count
            && parts.all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        {
            return true;
        }
    }
    // An interior node is `{root}-{n}`; the counter carries no `-`, so the LAST
    // `-` is the one that separates it from the root.
    label
        .rsplit_once('-')
        .is_some_and(|(stem, counter)| counter.parse::<usize>().is_ok() && roots.contains(stem))
}

/// Emit one validation result rooted at the blank node `label`, and — through
/// `sh:detail` — every result nested under it, each at `{label}d{j}`.
fn emit_result(
    builder: &mut RdfDatasetBuilder,
    label: &str,
    r: &ValidationResult,
    mint: &str,
    emitted_paths: &mut FastSet<String>,
) {
    let result_subj = RdfTerm::blank_node(label.to_owned());

    // _:r rdf:type sh:ValidationResult
    push_triple(
        builder,
        result_subj.clone(),
        rdf::TYPE,
        RdfTerm::iri(sh::VALIDATION_RESULT),
    );

    // sh:focusNode
    push_triple(
        builder,
        result_subj.clone(),
        sh::FOCUS_NODE,
        r.focus_node.to_rdf_term(),
    );

    // sh:resultSeverity
    push_triple(
        builder,
        result_subj.clone(),
        sh::RESULT_SEVERITY,
        RdfTerm::iri(r.severity.iri()),
    );

    // sh:sourceConstraintComponent
    push_triple(
        builder,
        result_subj.clone(),
        sh::SOURCE_CONSTRAINT_COMPONENT,
        RdfTerm::iri(r.source_constraint_component.as_str()),
    );

    // sh:sourceShape
    push_triple(
        builder,
        result_subj.clone(),
        sh::SOURCE_SHAPE,
        r.source_shape.to_rdf_term(),
    );

    // sh:resultPath (optional). A complex path is a blank node the report MINTS
    // (see `path::path_to_term`), so it carries the mint prefix like every other
    // minted node; its full SHACL path structure is emitted once per distinct
    // root label (two results sharing a path share the structure bnodes).
    if let Some(path) = &r.result_path {
        let root = match (path, &r.path_structure) {
            (Term::BlankNode(root_label), Some(_)) => Some(format!("{mint}{root_label}")),
            _ => None,
        };
        push_triple(
            builder,
            result_subj.clone(),
            sh::RESULT_PATH,
            root.clone()
                .map_or_else(|| path.to_rdf_term(), RdfTerm::blank_node),
        );
        if let (Some(root), Some(structure)) = (root, &r.path_structure)
            && emitted_paths.insert(root.clone())
        {
            emit_path_structure(builder, &root, structure);
        }
    }

    // sh:value (optional)
    if let Some(value) = &r.value {
        push_triple(builder, result_subj.clone(), sh::VALUE, value.to_rdf_term());
    }

    // sh:resultMessage — one per message, each literal as it was declared
    // (language tag, base direction and datatype preserved), in canonical order.
    for msg in &r.messages {
        push_triple(
            builder,
            result_subj.clone(),
            sh::RESULT_MESSAGE,
            Term::Literal(msg.clone()).to_rdf_term(),
        );
    }

    // SHACL-SPARQL result annotations: each `(property, value)` the constraint's
    // `sh:resultAnnotation`s produced for this result, in canonical order.
    for (property, value) in &r.annotations {
        push_triple(
            builder,
            result_subj.clone(),
            property.as_str(),
            value.to_rdf_term(),
        );
    }

    // sh:detail (optional nested results), in the result's own detail order.
    for (j, detail) in r.details.iter().enumerate() {
        let detail_label = format!("{label}d{j}");
        push_triple(
            builder,
            result_subj.clone(),
            sh::DETAIL,
            RdfTerm::blank_node(detail_label.clone()),
        );
        emit_result(builder, &detail_label, detail, mint, emitted_paths);
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
fn dataset_from_ntriples(nt: &str) -> Result<Arc<RdfDataset>, String> {
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

/// The conformance-disallow set a report dataset declares: the objects of its
/// `sh:conformanceDisallows` triples, or — when it has none — the default set
/// (SHACL 1.2 Core: "If the validation report contains no such triples,
/// sh:Violation, sh:Warning, and sh:Info are set as defaults").
///
/// # Errors
///
/// Returns an error when a value of `sh:conformanceDisallows` is not an IRI ("All
/// values of sh:conformanceDisallows MUST be IRIs").
pub fn conformance_disallows_from_dataset(
    data: &RdfDataset,
) -> Result<ConformanceDisallows, String> {
    let predicate = Term::NamedNode(NamedNode::from(sh::CONFORMANCE_DISALLOWS));
    let mut levels = Vec::new();
    for report_node in subjects_typed(data, sh::VALIDATION_REPORT) {
        for (_, _, object) in native_quads(
            data,
            Some(&report_node),
            Some(&predicate),
            None,
            GraphFilter::AnyGraph,
        ) {
            let Term::NamedNode(level) = object else {
                return Err(format!(
                    "sh:conformanceDisallows value {object} is not an IRI; all values of \
                     sh:conformanceDisallows must be IRIs"
                ));
            };
            levels.push(Severity::from_iri_open(level.as_str()));
        }
    }
    if levels.is_empty() {
        return Ok(ConformanceDisallows::default());
    }
    ConformanceDisallows::new(levels)
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
            messages: vec![Literal::new_simple_literal("must have at least one value")],
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
            details: vec![],
            annotations: vec![],
        }
    }

    #[test]
    fn report_round_trip_with_one_result() {
        let report = ValidationReport {
            conforms: false,
            results: vec![make_result()],
            conformance_disallows: ConformanceDisallows::default(),
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

    /// Regression: a SHACL validation report whose focus nodes are blank nodes
    /// with labels illegal in EVERY native text syntax — `a×b` (out-of-alphabet
    /// `BLANK_NODE_LABEL` byte) and `bad\u{1f}label` (a C0 control) — must still
    /// serialize to valid, re-parseable N-Triples. Before the blank-label escape-
    /// at-egress fix, `to_ntriples()` panicked (`.expect`) on exactly this input:
    /// the focus node came straight from data PurRDF itself had parsed, so the
    /// SHACL engine could not have rejected it upstream.
    ///
    /// The data graph is built through the `RdfDatasetBuilder` API (never through
    /// N-Triples/Turtle text) because a hostile label like `a×b` is not
    /// expressible in the `BLANK_NODE_LABEL` text grammar in the first place —
    /// exactly the class of value that only round-trips through the IR itself
    /// (e.g. a GTS-backed store, or SHACL-SPARQL `BIND`/`CONSTRUCT` output).
    #[test]
    fn to_ntriples_survives_hostile_blank_labels_in_focus_nodes() {
        use ::purrdf::{RdfQuad, RdfTerm};

        let hostile_labels = ["a\u{d7}b", "bad\u{1f}label"];

        let mut builder = RdfDatasetBuilder::new();
        for label in hostile_labels {
            builder.push_owned_quad(&RdfQuad::new(
                RdfTerm::blank_node(label),
                rdf::TYPE,
                RdfTerm::iri("http://example.org/Thing"),
            ));
        }
        let data = builder
            .freeze()
            .expect("hostile blank labels are opaque IR content, not a structural violation");

        let shapes_ttl = "\
            @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix ex: <http://example.org/> .\n\
            ex:ThingShape a sh:NodeShape ;\n\
                sh:targetClass ex:Thing ;\n\
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .\n";
        let shapes = crate::engine::parse_shapes(shapes_ttl, None).expect("shapes must parse");

        let report = crate::engine::validate_dataset(data.as_ref(), &shapes)
            .expect("validation over a well-formed dataset must not hard-fail");
        assert!(
            !report.conforms,
            "both hostile-labelled focus nodes are missing ex:required"
        );
        assert_eq!(report.results.len(), hostile_labels.len());

        // The actual regression: this must not panic, regardless of how hostile
        // the blank labels embedded in the report are.
        let nt = report.to_ntriples();

        // Prove the claim: re-parse the emitted text as N-Triples. A hostile
        // label that leaked through unescaped would break the grammar here.
        let reparsed = ::purrdf::parse_dataset(nt.as_bytes(), "application/n-triples", None)
            .expect("to_ntriples() output must be valid, re-parseable N-Triples");
        assert!(
            reparsed.quad_count() > 0,
            "re-parsed report must carry the emitted triples"
        );

        // The result tuples must still round-trip identically through text.
        let parsed =
            tuples_from_ntriples(&nt).expect("N-Triples from to_ntriples() must parse cleanly");
        assert_eq!(
            parsed,
            report.result_tuples(),
            "round-trip tuples must match original tuples even with hostile blank labels"
        );
    }

    /// Every [`::purrdf::TermId`] in `dataset`'s dense term table.
    fn term_ids(dataset: &RdfDataset) -> impl Iterator<Item = ::purrdf::TermId> {
        let count = u32::try_from(dataset.term_count()).expect("report term table fits in u32");
        (0..count).map(::purrdf::TermId::from_index)
    }

    /// A deliberately hostile report for the `to_dataset()` ≡ `to_ntriples()`
    /// equivalence proof: five results spanning all four severity kinds
    /// (including a caller-minted `Severity::Other` IRI), an IRI / a blank node /
    /// an RDF 1.2 triple term in focus position, a plain-IRI path and a COMPLEX
    /// path (whose structure blank nodes are emitted once and SHARED by two
    /// results), typed / language-tagged / blank-node / triple-term values, and
    /// results with and without messages.
    fn hostile_report() -> ValidationReport {
        let ex = |local: &str| NamedNode::new_unchecked(format!("http://example.org/{local}"));
        let component =
            NamedNode::new_unchecked("http://www.w3.org/ns/shacl#MinCountConstraintComponent");
        // A complex path shared by two results: [ sh:alternativePath ( ex:a [ sh:inversePath ex:b ] ) ].
        let complex = crate::shapes::Path::Alternative(vec![
            crate::shapes::Path::Predicate(ex("a")),
            crate::shapes::Path::Inverse(Box::new(crate::shapes::Path::Predicate(ex("b")))),
        ]);
        // The quoted triple that appears both as a focus node and as a value.
        let quoted = Term::Triple(Box::new(crate::term::Triple::new(
            Term::NamedNode(ex("s")),
            ex("p"),
            Term::Literal(Literal::new_simple_literal("quoted object")),
        )));

        let base = ValidationResult {
            focus_node: Term::NamedNode(ex("focusA")),
            result_path: Some(Term::NamedNode(ex("predP"))),
            path_structure: None,
            value: None,
            source_constraint_component: component,
            source_shape: Term::NamedNode(ex("ShapeA")),
            severity: Severity::Violation,
            messages: vec![],
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
            details: vec![],
            annotations: vec![],
        };

        let mut results = Vec::new();

        // 1. IRI focus, plain predicate path, typed literal value, message.
        let mut r = base.clone();
        r.value = Some(Term::Literal(Literal::new_typed_literal("-3", ex("Count"))));
        r.messages = vec![Literal::new_simple_literal("must have at least one value")];
        results.push(r);

        // 2. Blank-node focus, complex path (structure emitted), warning,
        //    language-tagged value.
        let mut r = base.clone();
        r.focus_node = Term::blank("focusB");
        r.result_path = Some(Term::blank("path0"));
        r.path_structure = Some(complex.clone());
        r.severity = Severity::Warning;
        r.value = Some(Term::Literal(
            Literal::new_language_tagged_literal_unchecked("valeur", "fr"),
        ));
        r.messages = vec![Literal::new_simple_literal("langue")];
        results.push(r);

        // 3. The SAME complex path on a second result — the structure must be
        //    emitted exactly once and shared, on both the direct and text paths.
        let mut r = base.clone();
        r.focus_node = Term::blank("focusC");
        r.result_path = Some(Term::blank("path0"));
        r.path_structure = Some(complex);
        r.severity = Severity::Info;
        r.value = Some(Term::blank("valueC"));
        results.push(r);

        // 4. RDF 1.2: a triple term in BOTH focus and value position.
        let mut r = base.clone();
        r.focus_node = quoted.clone();
        r.result_path = None;
        r.value = Some(quoted);
        r.severity = Severity::Other(ex("Advisory"));
        r.messages = vec![Literal::new_simple_literal("statement-level result")];
        results.push(r);

        // 5. No path, no value, no message — the minimal result.
        let mut r = base;
        r.focus_node = Term::NamedNode(ex("focusE"));
        r.result_path = None;
        results.push(r);

        ValidationReport {
            conforms: false,
            results,
            conformance_disallows: ConformanceDisallows::default(),
        }
    }

    /// `to_dataset()` must be the report's primal RDF form — the graph a caller
    /// gets from it is EXACTLY the graph they would have got by serializing to
    /// N-Triples and parsing that text back, minus the round-trip.
    ///
    /// The comparison is RDFC-1.0 canonical, not naive triple-set equality:
    /// blank-node labelling is precisely where a "direct" construction can
    /// silently diverge from a parse (the direct path keeps the report's own
    /// `_:r0` / `_:path0-1` labels; the parser mints its own), so only an
    /// isomorphism-invariant comparison proves the two describe the same graph.
    #[test]
    fn to_dataset_matches_the_ntriples_round_trip_canonically() {
        let report = hostile_report();

        let direct = report.to_dataset();
        let round_tripped = dataset_from_ntriples(&report.to_ntriples())
            .expect("the report's own N-Triples must parse");

        assert_eq!(
            direct.quad_count(),
            round_tripped.quad_count(),
            "the direct dataset must carry exactly the quads the text does"
        );
        assert_eq!(
            ::purrdf::canonicalize(&direct).nquads,
            ::purrdf::canonicalize(&round_tripped).nquads,
            "to_dataset() and parse(to_ntriples()) must be the same graph"
        );
    }

    /// The equivalence assertion above is only worth anything if the fixture it
    /// runs on actually exercises the hard cases. This pins that: the direct
    /// dataset carries all five results, the shared complex path's structure
    /// emitted ONCE, and a real RDF 1.2 triple term.
    #[test]
    fn to_dataset_carries_rdf12_triple_terms_and_shared_path_structure() {
        let report = hostile_report();
        let nt = report.to_ntriples();

        assert_eq!(
            nt.matches("<http://www.w3.org/ns/shacl#result>").count(),
            5,
            "all five results must reach the graph"
        );
        // The shared complex path emits its `sh:alternativePath` root once, not
        // once per referencing result.
        assert_eq!(
            nt.matches("<http://www.w3.org/ns/shacl#alternativePath>")
                .count(),
            1,
            "a complex path shared by two results is emitted once"
        );
        // RDF 1.2 triple term, in the syntax's own quoted-triple form.
        assert!(
            nt.contains("<<("),
            "the triple-term focus node/value must survive as a quoted triple, got:\n{nt}"
        );

        // And the direct dataset holds it as a real triple term, not as text.
        let direct = report.to_dataset();
        assert!(
            term_ids(&direct)
                .any(|id| matches!(direct.resolve(id), ::purrdf::TermRef::Triple { .. })),
            "to_dataset() must materialize the quoted triple as an IR triple term"
        );
    }

    /// The `sh:conforms` boolean and the report node itself survive the direct
    /// path for a CONFORMING report too — the empty-results case is the one a
    /// naive "build from results" implementation drops on the floor.
    #[test]
    fn to_dataset_of_a_conforming_report_matches_the_round_trip() {
        let report = ValidationReport {
            conforms: true,
            results: vec![],
            conformance_disallows: ConformanceDisallows::default(),
        };

        let direct = report.to_dataset();
        let round_tripped = dataset_from_ntriples(&report.to_ntriples()).expect("must parse");

        assert_eq!(
            ::purrdf::canonicalize(&direct).nquads,
            ::purrdf::canonicalize(&round_tripped).nquads
        );
        assert_eq!(conforms_from_dataset(&direct), Some(true));
        assert!(tuples_from_dataset(&direct).is_empty());
    }

    /// Hostile blank labels (`a×b`, a C0 control) are escaped at codec egress but
    /// are NOT escaped in the IR. The direct dataset therefore keeps the label
    /// verbatim where the text round-trip cannot — the two graphs stay isomorphic
    /// (same shape, same everything else), which is what `to_dataset()` promises,
    /// while the direct path additionally preserves the caller's own label.
    #[test]
    fn to_dataset_keeps_hostile_blank_labels_the_text_path_must_escape() {
        let mut report = hostile_report();
        report.results[1].focus_node = Term::blank("a\u{d7}b");

        let direct = report.to_dataset();
        let round_tripped =
            dataset_from_ntriples(&report.to_ntriples()).expect("escaped text must parse");

        // Same graph up to blank labelling…
        assert_eq!(
            ::purrdf::canonicalize(&direct).nquads,
            ::purrdf::canonicalize(&round_tripped).nquads,
            "escaping a blank label must not change the graph"
        );
        // …and the direct path still holds the caller's label unescaped.
        assert!(
            term_ids(&direct).any(|id| matches!(
                direct.resolve(id),
                ::purrdf::TermRef::Blank { label, .. } if label == "a\u{d7}b"
            )),
            "to_dataset() must carry the hostile blank label verbatim"
        );
    }

    /// Regression: a data graph is free to contain a blank node labelled `r0` or
    /// `report`, and those labels pass through the IR verbatim. The report used
    /// to mint the SAME labels for its own structure, emitting
    /// `_:r0 a sh:ValidationResult ; sh:focusNode _:r0` — the result fused with
    /// the node it reports on. Nothing was dropped and nothing failed; two
    /// distinct nodes silently became one.
    #[test]
    fn minted_blank_nodes_never_fuse_with_carried_ones() {
        for hostile in ["r0", "report"] {
            let mut report = ValidationReport {
                conforms: false,
                results: vec![make_result()],
                conformance_disallows: ConformanceDisallows::default(),
            };
            report.results[0].focus_node = Term::blank(hostile);

            let dataset = report.to_dataset();
            let focus = object_string(&dataset, &Term::blank(hostile), sh::FOCUS_NODE);
            assert!(
                focus.is_none(),
                "_:{hostile} is the DATA's node; it must not also be a result node"
            );

            // The focus node still denotes the caller's node, verbatim.
            let results = subjects_typed(&dataset, sh::VALIDATION_RESULT);
            assert_eq!(results.len(), 1, "exactly one result node");
            assert_eq!(
                object_string(&dataset, &results[0], sh::FOCUS_NODE).as_deref(),
                Some(&*format!("_:{hostile}")),
                "the result must still point at the caller's focus node"
            );
            assert_ne!(
                results[0],
                Term::blank(hostile),
                "the result node must be a DIFFERENT node from the focus node"
            );

            // And the report node is likewise distinct from the data's.
            let reports = subjects_typed(&dataset, sh::VALIDATION_REPORT);
            assert_eq!(reports.len(), 1);
            assert_ne!(reports[0], Term::blank(hostile));

            // The mark is grammar-legal: the minted labels reach the text
            // verbatim, never through the codec's escape envelope.
            let nt = report.to_ntriples();
            assert!(
                nt.contains("_:_report ") && nt.contains("_:_r0 "),
                "minted labels must be written as-is under a `_` mark:\n{nt}"
            );
            assert!(
                !nt.contains("purrdfesc"),
                "a `_`-marked label never needs escaping:\n{nt}"
            );
        }
    }

    /// The mirror of the test above, per the over-refusal discipline: a report
    /// whose carried labels do NOT collide must keep minting the plain
    /// `_:report` / `_:r0` labels, so the emitted bytes — and the 70 byte-frozen
    /// corpus reports — are unchanged.
    #[test]
    fn a_non_colliding_report_keeps_its_plain_minted_labels() {
        let mut report = ValidationReport {
            conforms: false,
            results: vec![make_result()],
            conformance_disallows: ConformanceDisallows::default(),
        };
        // `r1` is one past the last result index, and `reports` is not `report`:
        // neither is a label this report mints.
        report.results[0].focus_node = Term::blank("r1");
        report.results[0].value = Some(Term::blank("reports"));

        assert_eq!(mint_prefix(&report), "", "no collision means no prefix");
        let nt = report.to_ntriples();
        assert!(
            nt.contains("_:report "),
            "plain report label retained:\n{nt}"
        );
        assert!(nt.contains("_:r0 "), "plain result label retained:\n{nt}");
    }

    /// A complex path's root and interior nodes are minted too, so a data graph
    /// carrying the root label must not fuse with the path structure either.
    #[test]
    fn minted_path_structure_never_fuses_with_carried_labels() {
        let mut report = hostile_report();
        // `path0` is the complex path's root; `path0-1` one of its interiors.
        report.results[0].focus_node = Term::blank("path0");
        report.results[4].focus_node = Term::blank("path0-1");

        let mint = mint_prefix(&report);
        assert_ne!(mint, "", "a carried root label must force a mint prefix");

        let dataset = report.to_dataset();
        // The carried nodes are still only focus nodes — never path structure.
        for carried in ["path0", "path0-1"] {
            assert!(
                object_string(&dataset, &Term::blank(carried), sh::ALTERNATIVE_PATH).is_none()
                    && object_string(&dataset, &Term::blank(carried), rdf::FIRST).is_none(),
                "_:{carried} is a carried data node, not part of the path structure"
            );
        }
        // …and the graph is still the same one the text path produces.
        assert_eq!(
            ::purrdf::canonicalize(&dataset).nquads,
            ::purrdf::canonicalize(
                &dataset_from_ntriples(&report.to_ntriples()).expect("must parse")
            )
            .nquads
        );
    }

    #[test]
    fn empty_conforming_report_round_trips() {
        let report = ValidationReport {
            conforms: true,
            results: vec![],
            conformance_disallows: ConformanceDisallows::default(),
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
            messages: vec![Literal::new_simple_literal("missing required property")],
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
            details: vec![],
            annotations: vec![],
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
