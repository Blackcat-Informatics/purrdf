// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The COLD certify surface for a shapes graph: every judgement PurRDF can make about
//! a shapes graph before any data is validated against it, in one deterministic report.
//!
//! Loading a shapes graph ([`crate::shapes::from_dataset_with_config_and_graph`]) is the
//! HOT path: it hard-fails on the first construct it cannot evaluate faithfully — an
//! unknown `sh:`/`shnex:` term (the [census](crate::spec::census)), an ill-typed
//! parameter value, an unresolved or duplicate function definition (the
//! [linker](crate::spec)) — and it deliberately does not validate the graph against the
//! W3C's `shacl-shacl.ttl`, because a load pays for that on every validation. [`lint`]
//! is where that price is paid once, on request. It reports seven sections:
//!
//! 1. **load** — the loader's own verdict: accepted, or the refusal it raised.
//! 2. **shacl-shacl** — every result of validating the shapes graph, as DATA, against
//!    the vendored `shacl-shacl.ttl` (`crates/shapes/spec/shacl-shacl.ttl`), the W3C's
//!    "SHACL shapes graph to validate SHACL shapes graphs".
//! 3. **functions** — which implementation every node-expression function call binds
//!    to ([`Shapes::function_resolution`]), when the load succeeded.
//! 4. **validators** — every validator the shapes graph declares for a built-in
//!    constraint component, which the native implementation supersedes and never runs
//!    ([`crate::validator_alternatives`]), when the load succeeded. They are reported,
//!    never findings: the component's semantics are the specification's either way.
//! 5. **unexecuted** — every query the shapes graph declares that violates a pre-binding
//!    restriction (SHACL 1.2 SPARQL Extensions, Appendix A) and that nothing executes,
//!    when the load succeeded: a validator of a built-in component, a validator of a
//!    custom component no use selects, a `sh:SPARQLFunction` nothing calls. Appendix A
//!    requires a failure only for a query "executed with pre-bound variables", so the
//!    load accepts these, and one that does execute refuses the load
//!    ([`ShapesError::Prebinding`]); each unexecuted one is a FINDING here, so the
//!    violation is not silenced. A declaration that violates a SYNTAX rule is never
//!    listed here: it refuses the load ([`ShapesError::IllFormed`]), and the report
//!    carries that refusal in `load`.
//! 6. **diagnostics** — every syntax rule whose "SHOULD" PurRDF applies as a MANDATORY
//!    DIAGNOSTIC rather than a refusal ([`MANDATORY_DIAGNOSTIC_RULES`]): Appendix A's
//!    `in-minListLength` and `xone-minListLength` ("Each such list SHOULD have at least
//!    one member"). The approved W3C tests `core/node/in-002`, `in-003`, `xone-002` and
//!    `xone-003` validate a shapes graph with an empty `sh:in` or `sh:xone` list, so the
//!    load accepts it and a validation report states it well-formed. The rule constrains
//!    the document's author, and this section is where the author is told: every empty
//!    list is listed by rule id and shape, whether or not the load succeeded, and each is
//!    a FINDING.
//! 7. **unanchored-imports** — every `owl:imports` triple of the shapes graph's closure
//!    that is NOT an import, because its subject is no anchor of the document it occurs in
//!    ([`purrdf_core::imports::ImportMap::unanchored_imports`]): not the IRI the document
//!    was read or imported under, not an ontology header, not a shapes graph, and not a
//!    node versioning one of those. Such a triple is data — SHACL 1.2 Core §6.2 enacts no
//!    `owl:imports` of a data graph, and the kernel's one import rule follows none off a
//!    node whose only graph role is `sh:DataGraph` — so no document was looked for. The
//!    section is informational, never a finding: the triple is well-formed RDF the author
//!    may mean as data (the W3C test `sparql/component/validator-001` does). It is listed
//!    so an author who meant an import sees that it is not one. The `shacl-shacl` section
//!    covers the case the W3C flags: `shsh:DataGraphImportsShape` reports, at severity
//!    `sh:Info`, a `sh:DataGraph` that uses `owl:imports` without the type `owl:Ontology`,
//!    and that result is a finding like every other.
//!
//! # Where `shacl-shacl.ttl` lags SHACL 1.2 Core
//!
//! The vendored `shacl-shacl.ttl` predates several SHACL 1.2 Core relaxations, so it
//! flags some graphs the specification makes well-formed — `sh:closed sh:ByTypes`, a
//! list-valued `sh:nodeKind`, a path-valued `sh:equals`. Counting those as findings would
//! refuse valid shapes graphs. [`SHACL_SHACL_SUPERSEDED`] names each such rule with the
//! SHACL 1.2 Core sentence that makes the graph well-formed; a `shacl-shacl` result it
//! covers is reported, marked `superseded`, and is not a finding — but ONLY when the
//! loader accepted the graph. The ledger's premise is that the loader already agrees
//! with the specification there (the shacl-shacl differential oracle,
//! `tests/shacl_shacl_differential.rs`, pins exactly that against every corpus shapes
//! graph and hundreds of mutants); over a graph the loader refused, every result counts.
//!
//! `shacl-shacl.ttl` itself warns about an empty `sh:in` or `sh:xone` list
//! (`sh:minListLength` on the `sh:in` / `sh:xone` path). That result is the same defect a
//! diagnostic states, so it is listed marked `diagnosed RULE` and not counted twice: the
//! diagnostic, which names the rule id, is the finding.
//!
//! A report is CLEAN exactly when the loader accepted the graph, every `shacl-shacl`
//! result is superseded or diagnosed (whatever its severity: an `sh:Info` result counts), no
//! unexecuted query violates a pre-binding restriction, and no mandatory diagnostic
//! applies. `unanchored-imports` never affects it.
//!
//! # An incomplete `owl:imports` closure is not a report
//!
//! A shapes graph IS its `owl:imports` closure (see [`crate::imports`]), so [`lint`]
//! resolves the closure against the caller's import table first and certifies the MERGED
//! graph: an imported document's shapes are loaded and validated against `shacl-shacl.ttl`
//! like the importing document's. A closure that is not in hand is refused with
//! [`ShapesError::Imports`] — the same refusal every validation entry point raises — and
//! never folded into the `load` section: a report about the importing document alone would
//! certify a shapes graph nobody asked about, and would say `clean` about a graph that
//! validation refuses.

use std::fmt::{self, Write as _};
use std::sync::Arc;

use ::purrdf::RdfDataset;

use crate::data::GraphFilter;
use crate::engine::validate_dataset_as_document;
use crate::error::{PrebindingViolation, ShapesError};
use crate::function_resolution::FunctionResolution;
use crate::imports::{ShapesImports, resolve_shapes_imports};
use crate::model::{BoxRoleVocab, rdf, sh};
use crate::shapes::{
    Shapes, alternative_validators, from_dataset_with_base, from_resolved_dataset_with_unexecuted,
};
use crate::term::{Term, term_value_to_native};
use crate::validator_alternatives::AlternativeValidator;

/// The W3C shapes graph for shapes graphs, vendored byte-exact.
const SHACL_SHACL: &str = include_str!("../spec/shacl-shacl.ttl");

/// One rule by which the vendored `shacl-shacl.ttl` flags a shapes graph SHACL 1.2 Core
/// makes well-formed. See the [module docs](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Supersession {
    /// The rule's stable name, e.g. `closed-by-types`.
    pub name: &'static str,
    /// The `shacl-shacl.ttl` results it covers, each `(constraint component local name,
    /// result path, source shape)`; the path and shape are N-Triples IRIs, and an empty
    /// string matches an absent path or a blank-node shape.
    pub covers: &'static [(&'static str, &'static str, &'static str)],
    /// The SHACL 1.2 Core sentence that makes the flagged graph well-formed.
    pub citation: &'static str,
}

impl Supersession {
    /// Whether this rule covers a `shacl-shacl.ttl` result with `component`, `path` and
    /// `source_shape`.
    #[must_use]
    pub fn covers_result(&self, component: &str, path: Option<&Term>, source_shape: &Term) -> bool {
        self.covers_rendered(
            component,
            &path.map_or_else(String::new, named),
            &named(source_shape),
        )
    }

    /// [`Self::covers_result`] over an already-rendered result: `path` and
    /// `source_shape` are N-Triples IRIs, or the empty string for an absent path or a
    /// blank-node shape.
    #[must_use]
    pub fn covers_rendered(&self, component: &str, path: &str, source_shape: &str) -> bool {
        self.covers.iter().any(|(c, p, s)| {
            component
                .rsplit_once('#')
                .is_some_and(|(_, local)| local == *c)
                && path == *p
                && source_shape == *s
        })
    }
}

/// A term's N-Triples spelling when it is an IRI, and the empty string otherwise.
fn named(term: &Term) -> String {
    match term {
        Term::NamedNode(_) => term.to_string(),
        _ => String::new(),
    }
}

/// Every rule by which the vendored `shacl-shacl.ttl` lags SHACL 1.2 Core. See the
/// [module docs](self).
pub const SHACL_SHACL_SUPERSEDED: &[Supersession] = &[
    Supersession {
        name: "closed-by-types",
        covers: &[(
            "DatatypeConstraintComponent",
            "<http://www.w3.org/ns/shacl#closed>",
            "",
        )],
        citation: "SHACL 1.2 Core §7.9.1: \"The values of sh:closed in a shape are literals \
                   with datatype xsd:boolean or the IRI sh:ByTypes.\" — shacl-shacl.ttl still \
                   requires an xsd:boolean (closed-datatype)",
    },
    Supersession {
        name: "list-valued-node-kind",
        covers: &[(
            "InConstraintComponent",
            "<http://www.w3.org/ns/shacl#nodeKind>",
            "",
        )],
        citation: "SHACL 1.2 Core §4.1.3: \"The value of sh:nodeKind in a shape is either an \
                   IRI or a blank node that is a well-formed SHACL list where all members are \
                   IRIs.\" — shacl-shacl.ttl still requires one of the six SHACL 1.0 node-kind \
                   IRIs",
    },
    Supersession {
        name: "path-valued-property-pair",
        covers: &[
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#equals>",
                "",
            ),
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#disjoint>",
                "",
            ),
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#lessThan>",
                "",
            ),
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#lessThanOrEquals>",
                "",
            ),
        ],
        citation: "SHACL 1.2 Core §7.6.1: \"The values of sh:equals in a shape are well-formed \
                   SHACL property paths.\" — and §7.6.2, §7.6.4 and §7.6.5 say the same of \
                   sh:disjoint, sh:lessThan and sh:lessThanOrEquals. shacl-shacl.ttl still \
                   requires an IRI for all four (equals-nodeKind, disjoint-nodeKind, \
                   lessThan-nodeKind, lessThanOrEquals-nodeKind)",
    },
    Supersession {
        name: "node-expression-target-node",
        covers: &[(
            "NodeKindConstraintComponent",
            "<http://www.w3.org/ns/shacl#targetNode>",
            "",
        )],
        citation: "SHACL 1.2 Core, \"Node targets\": \"Each value of sh:targetNode in a shape \
                   is a well-formed node expression.\" A blank node that is the subject of no \
                   triple is the empty node expression, and one carrying sh:select is a SPARQL \
                   node expression whose output nodes are the targets; shacl-shacl.ttl still \
                   requires an IRI or a literal",
    },
    Supersession {
        name: "sequence-path-with-other-values",
        covers: &[(
            "XoneConstraintComponent",
            "",
            "<http://www.w3.org/ns/shacl-shacl#ShapeShape>",
        )],
        citation: "SHACL 1.2 Core §2.3.1: \"An inverse path is a blank node that is the \
                   subject of exactly one triple in G.\" A sequence-path node that also carries \
                   sh:inversePath is a sequence path, and the sh:inversePath value is no path \
                   of it. shacl-shacl.ttl's path walk follows sh:inversePath from every path \
                   node, judges the one-member list there as a path, and so fails the property \
                   shape's sh:node shsh:PathShape — which surfaces as the sh:xone of \
                   shsh:ShapeShape",
    },
];

/// One result of validating a shapes graph against `shacl-shacl.ttl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaclShaclResult {
    /// The shapes-graph node the result is about.
    pub focus: Term,
    /// The result path, when the result has one.
    pub path: Option<Term>,
    /// The offending value, when the result has one.
    pub value: Option<Term>,
    /// The constraint component IRI.
    pub component: String,
    /// The `shacl-shacl.ttl` shape that produced the result.
    pub source_shape: Term,
    /// The severity IRI.
    pub severity: String,
    /// The result messages' lexical forms, in the report's canonical order.
    pub messages: Vec<String>,
    /// The [`Supersession`] that covers the result, when the loader accepted the graph
    /// and SHACL 1.2 Core makes what `shacl-shacl.ttl` flags well-formed.
    pub superseded: Option<&'static Supersession>,
    /// The rule id of the [`MandatoryDiagnostic`] that states this same defect, when the
    /// result is `shacl-shacl.ttl`'s `sh:minListLength` warning on an empty `sh:in` or
    /// `sh:xone` list. The diagnostic is the finding; the result is listed beside it and
    /// not counted a second time.
    pub diagnosed: Option<&'static str>,
}

/// One `owl:imports` triple of the shapes graph's closure that is not an import. See the
/// [module docs](self).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnanchoredImport {
    /// The document it occurs in: `None` for the shapes document itself, or the IRI a
    /// supplied document was imported under.
    pub document: Option<String>,
    /// The triple's subject: no anchor of `document`.
    pub subject: Term,
    /// The triple's object.
    pub object: Term,
}

/// The syntax rules PurRDF applies as mandatory diagnostics: `(rule id, list parameter)`.
/// Each is Appendix A's "Each such list SHOULD have at least one member" for the
/// parameter's list. See the [module docs](self).
pub const MANDATORY_DIAGNOSTIC_RULES: &[(&str, &str)] = &[
    ("in-minListLength", sh::IN),
    ("xone-minListLength", sh::XONE),
];

/// One mandatory diagnostic: a shape whose `sh:in` or `sh:xone` list is empty. See the
/// [module docs](self).
///
/// Every run reports it, not only [`lint`]: a loaded shapes graph carries its diagnostics
/// ([`Shapes::mandatory_diagnostics`]) and every validation report states them
/// ([`crate::report::ValidationReport::diagnostics`]) beside its results, never as a
/// `sh:ValidationResult` — the shapes graph is well-formed and the verdict is unchanged —
/// and a rules or entailment run surfaces them the same way. Its [`fmt::Display`] is
/// `RULE SHAPE`, the text every host renders after `diagnostic `.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MandatoryDiagnostic {
    /// The syntax rule's id, `in-minListLength` or `xone-minListLength`.
    pub rule: &'static str,
    /// The shape whose list is empty.
    pub shape: Term,
}

impl MandatoryDiagnostic {
    /// The list parameter whose list is empty: `sh:in` or `sh:xone`.
    #[must_use]
    pub fn parameter(&self) -> &'static str {
        MANDATORY_DIAGNOSTIC_RULES
            .iter()
            .find(|(rule, _)| *rule == self.rule)
            .map_or("", |(_, parameter)| *parameter)
    }

    /// The diagnostic as a sentence naming the shape, the rule and the specification's
    /// text — the message a SARIF notification carries.
    #[must_use]
    pub fn message(&self) -> String {
        let local = self.parameter().rsplit('#').next().unwrap_or_default();
        format!(
            "shape {} has an empty sh:{local} list; SHACL 1.2 Core, Appendix A, syntax rule \
             {}: \"Each such list SHOULD have at least one member\". The shapes graph is \
             well-formed and the report is unaffected: the diagnostic is the shapes graph \
             author's, not the data graph's",
            self.shape, self.rule
        )
    }
}

impl fmt::Display for MandatoryDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.rule, self.shape)
    }
}

/// The whole cold-certify report for one shapes graph. See the [module docs](self).
#[derive(Debug, Clone)]
pub struct LintReport {
    /// The loader's refusal, or `None` when it accepted the graph.
    load_error: Option<String>,
    /// Every `shacl-shacl.ttl` result, in a deterministic order.
    shacl_shacl: Vec<ShaclShaclResult>,
    /// The call-site bindings, when the loader accepted the graph.
    functions: Option<FunctionResolution>,
    /// The validators declared for built-ins, when the loader accepted the graph.
    alternatives: Option<Vec<AlternativeValidator>>,
    /// The pre-binding violations of the queries nothing executes, when the loader
    /// accepted the graph.
    unexecuted: Option<Vec<PrebindingViolation>>,
    /// Every empty `sh:in` / `sh:xone` list, by rule id and shape.
    diagnostics: Vec<MandatoryDiagnostic>,
    /// Every `owl:imports` triple of the closure that is not an import.
    unanchored_imports: Vec<UnanchoredImport>,
}

impl LintReport {
    /// The loader's refusal, or `None` when it accepted the graph.
    #[must_use]
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// Every `shacl-shacl.ttl` result, superseded ones included, ordered by component,
    /// focus, path, value and source shape.
    #[must_use]
    pub fn shacl_shacl(&self) -> &[ShaclShaclResult] {
        &self.shacl_shacl
    }

    /// Which implementation every node-expression function call binds to, or `None`
    /// when the loader refused the graph (a refused graph binds nothing).
    #[must_use]
    pub fn function_resolution(&self) -> Option<&FunctionResolution> {
        self.functions.as_ref()
    }

    /// Every validator the shapes graph declares for a built-in component — superseded
    /// by the native implementation — sorted, or `None` when the loader refused the
    /// graph. Never findings; see the [module docs](self).
    #[must_use]
    pub fn alternative_validators(&self) -> Option<&[AlternativeValidator]> {
        self.alternatives.as_deref()
    }

    /// Every pre-binding violation of a query nothing executes, sorted, or `None` when
    /// the loader refused the graph. Each is a finding; see the [module docs](self).
    #[must_use]
    pub fn unexecuted(&self) -> Option<&[PrebindingViolation]> {
        self.unexecuted.as_deref()
    }

    /// Every mandatory diagnostic — one per shape with an empty `sh:in` or `sh:xone`
    /// list — ordered by rule id, then shape. Reported whether or not the loader accepted
    /// the graph; each is a finding. See the [module docs](self).
    #[must_use]
    pub fn diagnostics(&self) -> &[MandatoryDiagnostic] {
        &self.diagnostics
    }

    /// Every `owl:imports` triple of the shapes graph's closure whose subject is no anchor of
    /// the document it occurs in: the shapes document's first, then each imported
    /// document's in the order the closure reached it, each in document order. Never
    /// findings; see the [module docs](self).
    #[must_use]
    pub fn unanchored_imports(&self) -> &[UnanchoredImport] {
        &self.unanchored_imports
    }

    /// How many findings the report carries: one for a load refusal, plus every
    /// `shacl-shacl.ttl` result no [`Supersession`] covers and no diagnostic states, plus every unexecuted query
    /// that violates a pre-binding restriction, plus every mandatory diagnostic.
    #[must_use]
    pub fn findings(&self) -> usize {
        usize::from(self.load_error.is_some())
            + self
                .shacl_shacl
                .iter()
                .filter(|result| result.superseded.is_none() && result.diagnosed.is_none())
                .count()
            + self.unexecuted.as_ref().map_or(0, Vec::len)
            + self.diagnostics.len()
    }

    /// Whether the report carries no finding.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings() == 0
    }

    /// The report as deterministic line-oriented text — the rendering every host
    /// prints:
    ///
    /// ```text
    /// load accepted|refused
    ///   error LINE                     (one per line of the refusal, when refused)
    /// shacl-shacl N
    /// result COMPONENT focus F path P value V shape S severity SEV [superseded NAME] [diagnosed RULE]
    ///   message TEXT                   (one per result message)
    /// functions N|unavailable
    /// call BINDING FUNCTION in OWNER
    /// validators N|unavailable
    /// alternative COMPONENT ATTACHMENT VALIDATOR LANGUAGE superseded-by-native
    /// unexecuted N|unavailable
    /// violation DECLARATION
    ///   error LINE                     (one per line of the violated restriction)
    /// diagnostics N
    /// diagnostic RULE SHAPE
    /// unanchored-imports N
    /// unanchored SUBJECT OBJECT document -|<IRI>
    /// findings N
    /// clean true|false
    /// ```
    ///
    /// Terms are N-Triples 1.2; an absent path or value is `-`. `BINDING` is
    /// [`FunctionBinding::label`](crate::function_resolution::FunctionBinding::label);
    /// `functions unavailable` means the loader refused the graph. `LANGUAGE` is
    /// [`ValidatorLanguage::label`](crate::validator_alternatives::ValidatorLanguage::label);
    /// `validators unavailable` means the loader refused the graph, and so does
    /// `unexecuted unavailable`. `diagnostics` is always present: `RULE` is
    /// `in-minListLength` or `xone-minListLength`. `unanchored-imports` is always present (the closure is
    /// resolved before the load); its `document` is `-` for the shapes document itself and
    /// the import IRI for an imported one. Every list is in the
    /// order its accessor documents, so the text is a pure function of the shapes graph.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        match &self.load_error {
            None => out.push_str("load accepted\n"),
            Some(error) => {
                out.push_str("load refused\n");
                for line in error.lines() {
                    let _ = writeln!(out, "  error {line}");
                }
            }
        }
        let _ = writeln!(out, "shacl-shacl {}", self.shacl_shacl.len());
        for result in &self.shacl_shacl {
            let optional =
                |term: Option<&Term>| term.map_or_else(|| "-".to_owned(), ToString::to_string);
            let _ = write!(
                out,
                "result <{}> focus {} path {} value {} shape {} severity <{}>",
                result.component,
                result.focus,
                optional(result.path.as_ref()),
                optional(result.value.as_ref()),
                result.source_shape,
                result.severity,
            );
            if let Some(rule) = result.superseded {
                let _ = write!(out, " superseded {}", rule.name);
            }
            if let Some(rule) = result.diagnosed {
                let _ = write!(out, " diagnosed {rule}");
            }
            out.push('\n');
            for message in &result.messages {
                let _ = writeln!(out, "  message {}", message.replace('\n', " "));
            }
        }
        match &self.functions {
            None => out.push_str("functions unavailable\n"),
            Some(functions) => {
                let _ = writeln!(out, "functions {}", functions.sites().count());
                for site in functions.sites() {
                    let _ = writeln!(
                        out,
                        "call {} <{}> in {}",
                        site.binding.label(),
                        site.function,
                        site.owner
                    );
                }
            }
        }
        match &self.alternatives {
            None => out.push_str("validators unavailable\n"),
            Some(alternatives) => {
                let _ = writeln!(out, "validators {}", alternatives.len());
                for alternative in alternatives {
                    let _ = writeln!(
                        out,
                        "alternative <{}> <{}> {} {} superseded-by-native",
                        alternative.component,
                        alternative.attachment,
                        alternative.validator,
                        alternative.language.label(),
                    );
                }
            }
        }
        match &self.unexecuted {
            None => out.push_str("unexecuted unavailable\n"),
            Some(unexecuted) => {
                let _ = writeln!(out, "unexecuted {}", unexecuted.len());
                for violation in unexecuted {
                    let _ = writeln!(out, "violation {}", violation.declaration());
                    for line in violation.message().lines() {
                        let _ = writeln!(out, "  error {line}");
                    }
                }
            }
        }
        let _ = writeln!(out, "diagnostics {}", self.diagnostics.len());
        for diagnostic in &self.diagnostics {
            let _ = writeln!(out, "diagnostic {diagnostic}");
        }
        let _ = writeln!(out, "unanchored-imports {}", self.unanchored_imports.len());
        for entry in &self.unanchored_imports {
            let _ = writeln!(
                out,
                "unanchored {} {} document {}",
                entry.subject,
                entry.object,
                entry
                    .document
                    .as_ref()
                    .map_or_else(|| "-".to_owned(), |iri| format!("<{iri}>")),
            );
        }
        let _ = writeln!(out, "findings {}", self.findings());
        let _ = writeln!(out, "clean {}", self.is_clean());
        out
    }
}

/// Certify the shapes graph `dataset`: load it exactly as validation would, validate it
/// as data against `shacl-shacl.ttl`, and report which implementation every function
/// call binds to. `doc_prefixes`, `box_role_vocab` and `shapes_graph` are the loader's
/// own configuration — see [`from_dataset_with_base`]; `imports` is the shapes graph's
/// `owl:imports` table, with the IRIs the shapes document was read under declared loaded.
///
/// A malformed shapes graph is not an `Err`: its refusal is the report's `load` section.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand
/// or `imports` cannot be used (see the [module documentation](self)); otherwise only when
/// the vendored `shacl-shacl.ttl` itself fails to load or to validate — an internal defect,
/// never a verdict about `dataset`.
pub fn lint(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
    imports: &ShapesImports,
) -> Result<LintReport, ShapesError> {
    let resolved = resolve_shapes_imports(dataset, doc_prefixes, &[], imports)?;
    let unanchored_imports = imports
        .import_map()
        .unanchored_imports(dataset)
        .into_iter()
        .map(|entry| UnanchoredImport {
            document: entry.document,
            subject: term_value_to_native(&entry.subject),
            object: term_value_to_native(&entry.object),
        })
        .collect();
    let dataset = &resolved.dataset;
    let diagnostics = mandatory_diagnostics(dataset);
    let loaded = from_resolved_dataset_with_unexecuted(
        dataset,
        None,
        &resolved.prefixes,
        box_role_vocab,
        shapes_graph,
    )
    .map_err(String::from);
    let oracle = shacl_shacl()?;
    let report = validate_dataset_as_document(dataset, &oracle)
        .map_err(|e| format!("shacl-shacl.ttl failed to validate the shapes graph: {e}"))?;
    let accepted = loaded.is_ok();
    let mut shacl_shacl: Vec<ShaclShaclResult> = report
        .results
        .iter()
        .map(|result| {
            let component = result.source_constraint_component.as_str().to_owned();
            let superseded = if accepted {
                SHACL_SHACL_SUPERSEDED.iter().find(|rule| {
                    rule.covers_result(
                        &component,
                        result.result_path.as_ref(),
                        &result.source_shape,
                    )
                })
            } else {
                None
            };
            ShaclShaclResult {
                focus: result.focus_node.clone(),
                path: result.result_path.clone(),
                value: result.value.clone(),
                component,
                source_shape: result.source_shape.clone(),
                severity: result.severity.iri().to_owned(),
                messages: result
                    .messages
                    .iter()
                    .map(|message| message.value().to_owned())
                    .collect(),
                superseded,
                diagnosed: None,
            }
        })
        .collect();
    for result in &mut shacl_shacl {
        result.diagnosed = diagnosed_by(result, &diagnostics);
    }
    shacl_shacl.sort_by_cached_key(|result| {
        (
            result.component.clone(),
            result.focus.to_string(),
            result.path.as_ref().map(ToString::to_string),
            result.value.as_ref().map(ToString::to_string),
            result.source_shape.to_string(),
        )
    });
    shacl_shacl.dedup();
    let (load_error, functions, alternatives, unexecuted) = match loaded {
        Ok((shapes, unexecuted)) => {
            // The load that just succeeded parsed the same registry, so this cannot
            // refuse; were it to, the report says so rather than listing nothing.
            match alternative_validators(dataset, &resolved.prefixes) {
                Ok(alternatives) => (
                    None,
                    Some(shapes.function_resolution()),
                    Some(alternatives),
                    Some(unexecuted),
                ),
                Err(error) => (Some(error), None, None, None),
            }
        }
        Err(error) => (Some(error), None, None, None),
    };
    Ok(LintReport {
        load_error,
        shacl_shacl,
        functions,
        alternatives,
        unexecuted,
        diagnostics,
        unanchored_imports,
    })
}

/// The rule id of the diagnostic in `diagnostics` that states the defect `result` flags:
/// `shacl-shacl.ttl`'s `sh:minListLength` result on a shape's `sh:in` or `sh:xone` path,
/// for a shape the diagnostics list under that parameter's rule.
fn diagnosed_by(
    result: &ShaclShaclResult,
    diagnostics: &[MandatoryDiagnostic],
) -> Option<&'static str> {
    if result.component != SH_MIN_LIST_LENGTH_COMPONENT {
        return None;
    }
    let Some(Term::NamedNode(path)) = &result.path else {
        return None;
    };
    let (rule, _) = MANDATORY_DIAGNOSTIC_RULES
        .iter()
        .find(|(_, parameter)| *parameter == path.as_str())?;
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.rule == *rule && diagnostic.shape == result.focus)
        .then_some(*rule)
}

/// `sh:MinListLengthConstraintComponent`, the component `shacl-shacl.ttl` reports an empty
/// `sh:in` or `sh:xone` list under.
const SH_MIN_LIST_LENGTH_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MinListLengthConstraintComponent";

/// Every shape of `dataset` — the shapes graph's whole `owl:imports` closure — with an
/// empty `sh:in` or `sh:xone` list, ordered by rule id and then canonically by shape.
/// Any subject of either predicate is a shape (the predicate is a parameter), so the
/// subject is reported as it stands.
pub(crate) fn mandatory_diagnostics(dataset: &RdfDataset) -> Vec<MandatoryDiagnostic> {
    // Allocation-free for a graph with no empty list: the parameters and `rdf:nil` are
    // resolved by id rather than as owned terms, and nothing is collected until a triple
    // matches. Every shapes-graph assembly — a parse and a product admission alike — pays
    // this, so it must not charge admission a constant for the common case.
    let Some(nil) = dataset.term_id_by_iri(rdf::NIL) else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for &(rule, parameter) in MANDATORY_DIAGNOSTIC_RULES {
        let Some(parameter) = dataset.term_id_by_iri(parameter) else {
            continue;
        };
        let mut shapes: Vec<Term> = crate::data::quads_for_pattern_ids(
            dataset,
            None,
            Some(parameter),
            Some(nil),
            GraphFilter::AnyGraph,
        )
        .map(|quad| crate::term::term_id_to_native(dataset, quad.s))
        .filter(Term::is_subject)
        .collect();
        crate::term::sort_terms_canonical(&mut shapes);
        shapes.dedup();
        diagnostics.extend(
            shapes
                .into_iter()
                .map(|shape| MandatoryDiagnostic { rule, shape }),
        );
    }
    diagnostics
}

/// `shacl-shacl.ttl`, loaded as a shapes graph.
///
/// It `owl:imports <http://www.w3.org/ns/shacl#>`, which it does not itself declare, so it
/// is loaded like any other shapes graph with an import: the vendored `shacl.ttl` — the
/// document whose header declares that ontology — is supplied for it. Merging the SHACL
/// vocabulary into a shapes graph adds no shape and changes no report
/// (`tests/vocabulary_import_invariance.rs`).
fn shacl_shacl() -> Result<Shapes, String> {
    let document = crate::text_ingest::parse_turtle_document(SHACL_SHACL, None)
        .map_err(|errors| format!("shacl-shacl.ttl does not parse: {}", errors.join("; ")))?;
    let mut imports = ShapesImports::new();
    imports
        .insert_turtle(SH_NAMESPACE, SHACL_VOCABULARY)
        .map_err(|e| format!("shacl.ttl does not load as shacl-shacl.ttl's import: {e}"))?;
    from_dataset_with_base(
        &document.dataset,
        None,
        &document.prefixes,
        None,
        None,
        &imports,
    )
    .map_err(|e| format!("shacl-shacl.ttl does not load as a shapes graph: {e}"))
}

/// The ontology IRI `shacl-shacl.ttl` imports.
const SH_NAMESPACE: &str = "http://www.w3.org/ns/shacl#";

/// The W3C SHACL vocabulary, vendored byte-exact: the document that declares
/// [`SH_NAMESPACE`] an ontology.
const SHACL_VOCABULARY: &str = include_str!("../spec/shacl.ttl");

#[cfg(test)]
mod tests {
    use super::{LintReport, lint};
    use crate::imports::ShapesImports;

    const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
        @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        @prefix sh: <http://www.w3.org/ns/shacl#> .\n";

    /// A shape every report below carries, so the graph is never empty.
    const SHAPE: &str = "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:nodeKind sh:IRI .\n";

    const LIB: &str = "http://example.org/lib";

    fn report(shapes: &str, imports: &ShapesImports) -> LintReport {
        let document =
            crate::text_ingest::parse_turtle_document(&format!("{PREFIXES}{SHAPE}{shapes}"), None)
                .expect("turtle");
        lint(&document.dataset, &document.prefixes, None, None, imports).expect("lints")
    }

    /// The `(focus, source shape, severity)` of every `shacl-shacl` result.
    fn shacl_shacl(report: &LintReport) -> Vec<(String, String, String)> {
        report
            .shacl_shacl()
            .iter()
            .map(|result| {
                (
                    result.focus.to_string(),
                    result.source_shape.to_string(),
                    result.severity.clone(),
                )
            })
            .collect()
    }

    /// An `owl:imports` on a node that is no anchor is listed under `unanchored-imports`,
    /// and the section is informational: the finding count and `clean` are exactly the
    /// control's, which lacks the triple. The control lists nothing, so a section that
    /// listed every graph's imports regardless would be observed.
    #[test]
    fn an_unanchored_import_is_listed_and_is_never_a_finding() {
        let control = report("", &ShapesImports::new());
        assert_eq!(control.unanchored_imports(), []);
        assert!(control.render().contains("unanchored-imports 0\n"));

        let listed = report("ex:Other owl:imports ex:Target .\n", &ShapesImports::new());
        assert_eq!(listed.unanchored_imports().len(), 1, "{}", listed.render());
        assert!(
            listed.render().contains(
                "unanchored-imports 1\nunanchored <http://example.org/ns#Other> \
                 <http://example.org/ns#Target> document -\n"
            ),
            "{}",
            listed.render()
        );
        assert_eq!(listed.findings(), control.findings());
        assert_eq!(listed.is_clean(), control.is_clean());
        assert!(listed.is_clean(), "{}", listed.render());
    }

    /// W3C `shsh:DataGraphImportsShape` — "sh:DataGraphs using owl:imports should
    /// explicitly include the type owl:Ontology", severity `sh:Info` — surfaces in the
    /// `shacl-shacl` section and is a finding: an `sh:Info` result is not dropped. The
    /// data-graph node's triple is also listed as unanchored, since it is not an import.
    /// The neighbour types the node `owl:Ontology` too: the shape is satisfied, the import
    /// is followed (and supplied), and nothing is unanchored.
    #[test]
    fn a_data_graph_import_without_an_ontology_type_is_an_info_finding() {
        const SHSH_DATA_GRAPH_IMPORTS: &str =
            "<http://www.w3.org/ns/shacl-shacl#DataGraphImportsShape>";
        const SH_INFO: &str = "http://www.w3.org/ns/shacl#Info";
        let flagged = report(
            &format!("ex:D a sh:DataGraph ; owl:imports <{LIB}> .\n"),
            &ShapesImports::new(),
        );
        assert!(
            shacl_shacl(&flagged).contains(&(
                "<http://example.org/ns#D>".to_owned(),
                SHSH_DATA_GRAPH_IMPORTS.to_owned(),
                SH_INFO.to_owned()
            )),
            "{}",
            flagged.render()
        );
        assert!(!flagged.is_clean(), "{}", flagged.render());
        assert_eq!(flagged.unanchored_imports().len(), 1);

        let mut imports = ShapesImports::new();
        imports.insert_turtle(LIB, "").expect("empty document");
        let typed = report(
            &format!("ex:D a sh:DataGraph , owl:Ontology ; owl:imports <{LIB}> .\n"),
            &imports,
        );
        assert!(
            !shacl_shacl(&typed)
                .iter()
                .any(|(_, shape, _)| shape == SHSH_DATA_GRAPH_IMPORTS),
            "{}",
            typed.render()
        );
        assert_eq!(typed.unanchored_imports(), [], "{}", typed.render());
        assert!(typed.is_clean(), "{}", typed.render());
    }

    /// An imported document's unanchored `owl:imports` is listed under the IRI it was
    /// imported by; the importing document's anchored import is not listed.
    #[test]
    fn an_imported_documents_unanchored_import_names_its_document() {
        let mut imports = ShapesImports::new();
        imports
            .insert_turtle(
                LIB,
                "<http://example.org/ns#LibNode> <http://www.w3.org/2002/07/owl#imports> \
                 <http://example.org/ns#Elsewhere> .\n",
            )
            .expect("turtle");
        let listed = report(
            &format!("ex:G a sh:ShapesGraph ; owl:imports <{LIB}> .\n"),
            &imports,
        );
        assert!(
            listed.render().contains(
                "unanchored-imports 1\nunanchored <http://example.org/ns#LibNode> \
                 <http://example.org/ns#Elsewhere> document <http://example.org/lib>\n"
            ),
            "{}",
            listed.render()
        );
    }
}
