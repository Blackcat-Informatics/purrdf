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
//! is where that price is paid once, on request. It reports four sections:
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
//! A report is CLEAN exactly when the loader accepted the graph and every `shacl-shacl`
//! result is superseded.
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

use std::fmt::Write as _;
use std::sync::Arc;

use ::purrdf::RdfDataset;

use crate::engine::validate_dataset_with_shapes_graph;
use crate::error::ShapesError;
use crate::function_resolution::FunctionResolution;
use crate::imports::{ShapesImports, resolve_shapes_imports};
use crate::model::BoxRoleVocab;
use crate::shapes::{
    Shapes, alternative_validators, from_dataset_with_base, from_resolved_dataset,
};
use crate::term::Term;
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

    /// How many findings the report carries: one for a load refusal, plus every
    /// `shacl-shacl.ttl` result no [`Supersession`] covers.
    #[must_use]
    pub fn findings(&self) -> usize {
        usize::from(self.load_error.is_some())
            + self
                .shacl_shacl
                .iter()
                .filter(|result| result.superseded.is_none())
                .count()
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
    /// result COMPONENT focus F path P value V shape S severity SEV [superseded NAME]
    ///   message TEXT                   (one per result message)
    /// functions N|unavailable
    /// call BINDING FUNCTION in OWNER
    /// validators N|unavailable
    /// alternative COMPONENT ATTACHMENT VALIDATOR LANGUAGE superseded-by-native
    /// findings N
    /// clean true|false
    /// ```
    ///
    /// Terms are N-Triples 1.2; an absent path or value is `-`. `BINDING` is
    /// [`FunctionBinding::label`](crate::function_resolution::FunctionBinding::label);
    /// `functions unavailable` means the loader refused the graph. `LANGUAGE` is
    /// [`ValidatorLanguage::label`](crate::validator_alternatives::ValidatorLanguage::label);
    /// `validators unavailable` means the loader refused the graph. Every list is in the
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
    let dataset = &resolved.dataset;
    let loaded = from_resolved_dataset(
        dataset,
        None,
        &resolved.prefixes,
        box_role_vocab,
        shapes_graph,
    );
    let oracle = shacl_shacl()?;
    let report = validate_dataset_with_shapes_graph(dataset, &oracle, None)
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
            }
        })
        .collect();
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
    let (load_error, functions, alternatives) = match loaded {
        Ok(shapes) => {
            // The load that just succeeded parsed the same registry, so this cannot
            // refuse; were it to, the report says so rather than listing nothing.
            match alternative_validators(dataset, &resolved.prefixes) {
                Ok(alternatives) => (None, Some(shapes.function_resolution()), Some(alternatives)),
                Err(error) => (Some(error), None, None),
            }
        }
        Err(error) => (Some(error), None, None),
    };
    Ok(LintReport {
        load_error,
        shacl_shacl,
        functions,
        alternatives,
    })
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
