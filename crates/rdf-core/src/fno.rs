// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native Function Ontology catalogs and deterministic RDF emission.
//!
//! [`FnoCatalog::try_new`] admits caller-owned [`FnoCatalogInput`] after checking
//! RDF terms, node descriptions and references. Set-valued fields are canonical;
//! `fno:expects` retains its list order. Mapping nodes retain native blank scopes.
//! The writer uses the shared RDF list builder, term conversion and text emitter.

use crate::collections::{ListVocab, build_rdf_list};
use crate::{BlankScope, RdfLiteral, RdfQuad, RdfTerm, TermValue, turtle};
use std::collections::BTreeSet;

mod admission;
mod inspection;
mod read;
mod read_error;
#[cfg(test)]
mod read_tests;
pub use admission::{FnoAdmissionError, FnoAdmissionReason};
pub use read_error::{FnoFieldKind, FnoReadError, FnoStructureKind};

// --------------------------------------------------------------------------- //
// Vocabulary
// --------------------------------------------------------------------------- //

use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;

use purrdf_iri::vocab::rdfs::COMMENT as RDFS_COMMENT;
use purrdf_iri::vocab::rdfs::LABEL as RDFS_LABEL;
use purrdf_iri::vocab::rdfs::SEE_ALSO as RDFS_SEE_ALSO;

use purrdf_iri::vocab::skos::DEFINITION as SKOS_DEFINITION;

use purrdf_iri::vocab::owl::ONTOLOGY as OWL_ONTOLOGY;

const DCTERMS_IS_PART_OF: &str = "http://purl.org/dc/terms/isPartOf";
const DCTERMS_FORMAT: &str = "http://purl.org/dc/terms/format";

/// The `https://w3id.org/function/ontology#` (fno) namespace.
const FNO: &str = "https://w3id.org/function/ontology#";
/// The `https://w3id.org/function/vocabulary/mapping#` (fnom) namespace.
const FNOM: &str = "https://w3id.org/function/vocabulary/mapping#";

/// Derive a consumer ontology's `ProjectionFunction` class IRI from its
/// ontology IRI: `<ontology_iri>/ProjectionFunction` (a trailing `/` on the
/// ontology IRI is normalized away first).
///
/// The projection-function class is CONSUMER vocabulary, not PurRDF's: PurRDF
/// does not supply a consumer ontology namespace.
/// Emitters that type their functions as projection functions populate
/// [`FnFunction::kind_types`] with this derived IRI (or any other class IRI of
/// their own choosing).
#[must_use]
pub fn projection_function_class(ontology_iri: &str) -> String {
    format!("{}/ProjectionFunction", ontology_iri.trim_end_matches('/'))
}

// --------------------------------------------------------------------------- //
// Catalog model (built in Rust by the purrdf-slice FnO emitter)
// --------------------------------------------------------------------------- //

/// Caller-owned fields admitted together into a canonical [`FnoCatalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoCatalogInput {
    /// The root ontology IRI the document `dcterms:isPartOf`.
    pub ontology_iri: String,
    /// The FnO document node IRI (`<ontology_iri>/projections/functions`).
    pub document_iri: String,
    /// The document's original `rdfs:label` literal.
    pub doc_label: TermValue,
    /// The document's original `rdfs:comment` literal.
    pub banner: TermValue,
    /// Function descriptions, canonicalized by IRI at admission.
    pub functions: Vec<FnFunction>,
    /// Parameter descriptions, canonicalized by IRI at admission.
    pub params: Vec<FnParam>,
    /// Implementation descriptions, canonicalized by IRI at admission.
    pub implementations: Vec<FnImpl>,
    /// The fno:Mapping nodes (one per (function, profile)).
    pub mappings: Vec<FnMapping>,
}

/// An immutable catalog with canonical set order and validated references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoCatalog {
    data: FnoCatalogInput,
}

impl FnoCatalog {
    /// Admit complete catalog data, preserving the original input on refusal.
    ///
    /// # Errors
    /// Refuses malformed RDF values, conflicting descriptions and unresolved
    /// references. Indistinguishable duplicate facts are canonicalized once valid.
    pub fn try_new(mut data: FnoCatalogInput) -> Result<Self, FnoAdmissionError> {
        let shared = match admission::validate(&data) {
            Ok(shared) => shared,
            Err(reason) => {
                return Err(FnoAdmissionError {
                    input: Box::new(data),
                    reason,
                });
            }
        };
        admission::canonicalize(&mut data, &shared);
        Ok(Self { data })
    }

    /// Borrow every admitted field without permitting mutation of invariants.
    #[must_use]
    pub const fn as_input(&self) -> &FnoCatalogInput {
        &self.data
    }

    /// The consumer's `ProjectionFunction` class IRI, derived from this
    /// catalog's [`FnoCatalogInput::ontology_iri`] via
    /// [`projection_function_class`].
    #[must_use]
    pub fn projection_function_class(&self) -> String {
        projection_function_class(&self.data.ontology_iri)
    }
}

/// One `fno:Function` node (always typed `fno:Function`; any additional
/// `rdf:type` IRIs — e.g. the consumer's projection-function class for the
/// projection catalog — come from [`FnFunction::kind_types`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnFunction {
    /// The function node's IRI.
    pub iri: String,
    /// Original `rdfs:label` literal, including its language and direction.
    pub label: TermValue,
    /// Original `skos:definition` literal, omitted only when `None`.
    pub description: Option<TermValue>,
    /// Extra `rdf:type` IRIs in canonical order after admission.
    /// The projection-function builder sets its consumer's projection-function
    /// class (see [`projection_function_class`]); primitives (e.g. the list
    /// functions) leave this empty so they are `fno:Function` ONLY.
    pub kind_types: Vec<String>,
    /// `rdfs:seeAlso` (e.g. a projection function's `.rq` query), omitted when
    /// `None`. Primitives with no related resource leave it unset rather than
    /// pointing at a dummy or process-flow target.
    pub see_also: Option<String>,
    /// Ordered `fno:expects` parameter IRIs (required first, then optional).
    pub expects: Vec<String>,
    /// The single `fno:Output` node (`fno:returns` is a one-element list).
    pub output: FnOutput,
}

/// The `fno:Output` node of a function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnOutput {
    /// The output node's IRI.
    pub iri: String,
    /// `fno:predicate` — the PurRDF predicate the output realises. `None` for
    /// primitives (the list functions bind no data predicate).
    pub predicate: Option<String>,
    /// `fno:type` — the output's range IRI.
    pub r#type: String,
    /// Original `rdfs:label` literal, emitted when `Some`.
    pub label: Option<TermValue>,
    /// Original `skos:definition` literal, emitted when `Some`.
    pub description: Option<TermValue>,
}

/// One globally-deduped `fno:Parameter` node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnParam {
    /// The parameter node's IRI.
    pub iri: String,
    /// `fno:predicate` — the source PurRDF predicate. `None` for primitives (the
    /// list functions bind no data predicate).
    pub predicate: Option<String>,
    /// `fno:type` — the predicate's ontology `rdfs:range` (the fail-closed type).
    pub r#type: String,
    /// `fno:required` (an `xsd:boolean` literal).
    pub required: bool,
    /// Original `rdfs:label` literal, emitted when `Some`.
    pub label: Option<TermValue>,
    /// Original `skos:definition` literal, emitted when `Some`.
    pub description: Option<TermValue>,
}

/// One `fno:Implementation` node (one per profile `.rq`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnImpl {
    /// The implementation node's IRI.
    pub iri: String,
    /// `dcterms:format` — a plain string literal (`"application/sparql-query"`).
    pub format: String,
    /// `rdfs:seeAlso` — the profile's `.rq` query.
    pub see_also: String,
}

/// One `fno:Mapping` node linking a function to one profile's implementation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnMapping {
    /// Native mapping identity, including its document scope.
    pub node: FnBlank,
    /// Original `rdfs:label` literal.
    pub label: TermValue,
    /// `fno:function` — the mapped function IRI.
    pub function: String,
    /// `fno:implementation` — the implementation IRI.
    pub implementation: String,
    /// `fno:parameterMapping` nodes (a `fnom:PropertyParameterMapping` each).
    pub parameter_mappings: Vec<FnParamMapping>,
    /// `fno:returnMapping` nodes (a `fnom:DefaultReturnMapping` each).
    pub return_mappings: Vec<FnReturnMapping>,
}

/// One `fnom:PropertyParameterMapping` (a parameter ↦ a SPARQL variable).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnParamMapping {
    /// Native mapping identity, including its document scope.
    pub node: FnBlank,
    /// Original `rdfs:label` literal.
    pub label: TermValue,
    /// `fnom:functionParameter` — the parameter IRI.
    pub function_parameter: String,
    /// `fnom:implementationProperty` — the SPARQL var name (a plain string).
    pub implementation_property: String,
}

/// One `fnom:DefaultReturnMapping` (the function output ↦ a SPARQL variable).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnReturnMapping {
    /// Native mapping identity, including its document scope.
    pub node: FnBlank,
    /// Original `rdfs:label` literal.
    pub label: TermValue,
    /// `fnom:functionOutput` — the function's output node IRI.
    pub function_output: String,
    /// `fnom:implementationProperty` — the SPARQL var name (a plain string).
    pub implementation_property: String,
}

// --------------------------------------------------------------------------- //
// Serialization
// --------------------------------------------------------------------------- //

/// The native blank identity of a mapping or one of its child mappings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnBlank {
    /// Original label, without a concrete-syntax prefix or encoding.
    pub label: String,
    /// Native document scope; equal labels in different scopes remain distinct.
    pub scope: BlankScope,
}

impl FnBlank {
    /// Represent this identity through the existing native term value.
    #[must_use]
    pub fn term_value(&self) -> TermValue {
        TermValue::Blank {
            label: self.label.clone(),
            scope: self.scope,
        }
    }
}

fn owned(value: &TermValue) -> RdfTerm {
    value
        .to_rdf_term()
        .expect("FnO admission permits only leaf terms")
}

/// The profile's plain string for `dcterms:format` and implementation properties.
fn plain(text: &str) -> RdfTerm {
    RdfTerm::literal(RdfLiteral::simple(text.to_owned()))
}

/// Build the admitted model's quads, retaining native term identity and facets.
///
/// The `fno:expects` / `fno:returns` lists are emitted as proper
/// `rdf:first`/`rdf:rest`/`rdf:nil` chains; the list cells are anonymous blank
/// nodes minted by one catalog-wide supply disjoint from every mapping identity.
pub fn to_quads(catalog: &FnoCatalog) -> Vec<RdfQuad> {
    let catalog = catalog.as_input();
    let mut quads: Vec<RdfQuad> = Vec::new();
    let mut cells = ListCells::new(catalog);

    // ── Document node ──────────────────────────────────────────────────────
    let doc = RdfTerm::iri(&catalog.document_iri);
    quads.push(RdfQuad::new(
        doc.clone(),
        RDF_TYPE,
        RdfTerm::iri(OWL_ONTOLOGY),
    ));
    quads.push(RdfQuad::new(
        doc.clone(),
        RDFS_LABEL,
        owned(&catalog.doc_label),
    ));
    quads.push(RdfQuad::new(
        doc.clone(),
        DCTERMS_IS_PART_OF,
        RdfTerm::iri(&catalog.ontology_iri),
    ));
    quads.push(RdfQuad::new(doc, RDFS_COMMENT, owned(&catalog.banner)));

    // ── Functions (+ their output node + expects/returns lists) ────────────
    for func in &catalog.functions {
        let fn_iri = RdfTerm::iri(&func.iri);
        quads.push(RdfQuad::new(
            fn_iri.clone(),
            RDF_TYPE,
            RdfTerm::iri(format!("{FNO}Function")),
        ));
        for kind in &func.kind_types {
            quads.push(RdfQuad::new(fn_iri.clone(), RDF_TYPE, RdfTerm::iri(kind)));
        }
        quads.push(RdfQuad::new(fn_iri.clone(), RDFS_LABEL, owned(&func.label)));
        if let Some(description) = &func.description {
            quads.push(RdfQuad::new(
                fn_iri.clone(),
                SKOS_DEFINITION,
                owned(description),
            ));
        }
        if let Some(see_also) = &func.see_also {
            quads.push(RdfQuad::new(
                fn_iri.clone(),
                RDFS_SEE_ALSO,
                RdfTerm::iri(see_also),
            ));
        }

        // fno:expects — an ordered rdf:List of the parameter IRIs.
        let expects: Vec<RdfTerm> = func.expects.iter().map(RdfTerm::iri).collect();
        attach_list(
            &mut quads,
            &fn_iri,
            &format!("{FNO}expects"),
            &expects,
            &mut cells,
        );

        // The fno:Output node + fno:returns one-element list.
        let out = RdfTerm::iri(&func.output.iri);
        quads.push(RdfQuad::new(
            out.clone(),
            RDF_TYPE,
            RdfTerm::iri(format!("{FNO}Output")),
        ));
        if let Some(predicate) = &func.output.predicate {
            quads.push(RdfQuad::new(
                out.clone(),
                format!("{FNO}predicate"),
                RdfTerm::iri(predicate),
            ));
        }
        quads.push(RdfQuad::new(
            out.clone(),
            format!("{FNO}type"),
            RdfTerm::iri(&func.output.r#type),
        ));
        if let Some(label) = &func.output.label {
            quads.push(RdfQuad::new(out.clone(), RDFS_LABEL, owned(label)));
        }
        if let Some(description) = &func.output.description {
            quads.push(RdfQuad::new(
                out.clone(),
                SKOS_DEFINITION,
                owned(description),
            ));
        }
        attach_list(
            &mut quads,
            &fn_iri,
            &format!("{FNO}returns"),
            std::slice::from_ref(&out),
            &mut cells,
        );
    }

    // ── Parameters (globally deduped, canonical IRI order) ────────────────
    for param in &catalog.params {
        let p = RdfTerm::iri(&param.iri);
        quads.push(RdfQuad::new(
            p.clone(),
            RDF_TYPE,
            RdfTerm::iri(format!("{FNO}Parameter")),
        ));
        if let Some(predicate) = &param.predicate {
            quads.push(RdfQuad::new(
                p.clone(),
                format!("{FNO}predicate"),
                RdfTerm::iri(predicate),
            ));
        }
        quads.push(RdfQuad::new(
            p.clone(),
            format!("{FNO}type"),
            RdfTerm::iri(&param.r#type),
        ));
        quads.push(RdfQuad::new(
            p.clone(),
            format!("{FNO}required"),
            TermValue::boolean(param.required)
                .into_rdf_term()
                .expect("a boolean literal is no triple term"),
        ));
        if let Some(label) = &param.label {
            quads.push(RdfQuad::new(p.clone(), RDFS_LABEL, owned(label)));
        }
        if let Some(description) = &param.description {
            quads.push(RdfQuad::new(p, SKOS_DEFINITION, owned(description)));
        }
    }

    // ── Implementations (one per profile .rq, deduped) ─────────────────────
    for implementation in &catalog.implementations {
        let i = RdfTerm::iri(&implementation.iri);
        quads.push(RdfQuad::new(
            i.clone(),
            RDF_TYPE,
            RdfTerm::iri(format!("{FNO}Implementation")),
        ));
        quads.push(RdfQuad::new(
            i.clone(),
            DCTERMS_FORMAT,
            plain(&implementation.format),
        ));
        quads.push(RdfQuad::new(
            i,
            RDFS_SEE_ALSO,
            RdfTerm::iri(&implementation.see_also),
        ));
    }

    // ── Mappings (fno:Mapping + fnom param/return mappings) ────────────────
    for mapping in &catalog.mappings {
        let m = owned(&mapping.node.term_value());
        quads.push(RdfQuad::new(
            m.clone(),
            RDF_TYPE,
            RdfTerm::iri(format!("{FNO}Mapping")),
        ));
        quads.push(RdfQuad::new(m.clone(), RDFS_LABEL, owned(&mapping.label)));
        quads.push(RdfQuad::new(
            m.clone(),
            format!("{FNO}function"),
            RdfTerm::iri(&mapping.function),
        ));
        quads.push(RdfQuad::new(
            m.clone(),
            format!("{FNO}implementation"),
            RdfTerm::iri(&mapping.implementation),
        ));

        for pmap in &mapping.parameter_mappings {
            let p = owned(&pmap.node.term_value());
            quads.push(RdfQuad::new(
                p.clone(),
                RDF_TYPE,
                RdfTerm::iri(format!("{FNOM}PropertyParameterMapping")),
            ));
            quads.push(RdfQuad::new(p.clone(), RDFS_LABEL, owned(&pmap.label)));
            quads.push(RdfQuad::new(
                p.clone(),
                format!("{FNOM}functionParameter"),
                RdfTerm::iri(&pmap.function_parameter),
            ));
            quads.push(RdfQuad::new(
                p.clone(),
                format!("{FNOM}implementationProperty"),
                plain(&pmap.implementation_property),
            ));
            quads.push(RdfQuad::new(m.clone(), format!("{FNO}parameterMapping"), p));
        }

        for rmap in &mapping.return_mappings {
            let r = owned(&rmap.node.term_value());
            quads.push(RdfQuad::new(
                r.clone(),
                RDF_TYPE,
                RdfTerm::iri(format!("{FNOM}DefaultReturnMapping")),
            ));
            quads.push(RdfQuad::new(r.clone(), RDFS_LABEL, owned(&rmap.label)));
            quads.push(RdfQuad::new(
                r.clone(),
                format!("{FNOM}functionOutput"),
                RdfTerm::iri(&rmap.function_output),
            ));
            quads.push(RdfQuad::new(
                r.clone(),
                format!("{FNOM}implementationProperty"),
                plain(&rmap.implementation_property),
            ));
            quads.push(RdfQuad::new(m.clone(), format!("{FNO}returnMapping"), r));
        }
    }

    quads
}

/// Attach an `rdf:List` of `items` to `subject` via `predicate`, minting anonymous
/// list-cell blank nodes from the catalog's one collision-free supply.
///
/// An empty list links straight to `rdf:nil`. Cell labels are derived
/// reproducibly only so re-running yields a stable document.
fn attach_list(
    quads: &mut Vec<RdfQuad>,
    subject: &RdfTerm,
    predicate: &str,
    items: &[RdfTerm],
    cells: &mut ListCells<'_>,
) {
    // The link precedes the cells, so it is placed once the head is known.
    let link_at = quads.len();
    let head = build_rdf_list(
        items.iter().cloned(),
        &ListVocab::rdf_terms(),
        |_| cells.next(),
        |cell, predicate, object| {
            let RdfTerm::Iri(predicate) = predicate else {
                unreachable!("the list vocabulary is IRIs")
            };
            quads.push(RdfQuad::new(cell, predicate, object));
        },
    );
    quads.insert(link_at, RdfQuad::new(subject.clone(), predicate, head));
}

struct ListCells<'a> {
    occupied: BTreeSet<&'a FnBlank>,
    next: usize,
}

impl<'a> ListCells<'a> {
    fn new(catalog: &'a FnoCatalogInput) -> Self {
        let mut occupied = BTreeSet::new();
        for mapping in &catalog.mappings {
            occupied.insert(&mapping.node);
            occupied.extend(mapping.parameter_mappings.iter().map(|item| &item.node));
            occupied.extend(mapping.return_mappings.iter().map(|item| &item.node));
        }
        Self { occupied, next: 0 }
    }

    fn next(&mut self) -> RdfTerm {
        loop {
            let node = FnBlank {
                label: format!("fno-list-{}", self.next),
                scope: BlankScope::DEFAULT,
            };
            // Every occupied candidate owns at least one allocated byte; a live
            // catalog cannot exhaust the address-space-sized ordinal range.
            self.next += 1;
            // The monotonically increasing supply already separates generated
            // cells; only original mapping identities need collision checks.
            if !self.occupied.contains(&node) {
                return owned(&node.term_value());
            }
        }
    }
}

/// Serialize an admitted catalog to deterministic full-IRI N-Triples.
#[must_use]
pub fn to_ntriples(catalog: &FnoCatalog) -> String {
    to_quads(catalog).iter().map(turtle::emit_quad).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_iri::vocab::rdf::{FIRST as RDF_FIRST, NIL as RDF_NIL, REST as RDF_REST};

    const X_PURRDF_ENGLISH: &str = "x-purrdf-english";

    fn label(value: &str) -> TermValue {
        TermValue::lang_literal(value, X_PURRDF_ENGLISH)
    }

    fn blank(value: &str) -> FnBlank {
        FnBlank {
            label: value.to_owned(),
            scope: BlankScope::DEFAULT,
        }
    }

    /// A hand-built catalog: one function with one required + one optional param,
    /// one profile implementation, and one mapping with two param-var bindings and
    /// one return-var binding.
    pub(super) fn sample_catalog() -> FnoCatalog {
        FnoCatalog::try_new(FnoCatalogInput {
            ontology_iri: "https://example.org/vocab".to_owned(),
            document_iri: "https://example.org/vocab/projections/functions".to_owned(),
            doc_label: label("PurRDF projection functions (FnO)"),
            banner: label("GENERATED — DO NOT EDIT."),
            functions: vec![FnFunction {
                iri: "https://example.org/vocab/fnDemo".to_owned(),
                label: label("demo function"),
                description: Some(label("a demo")),
                kind_types: vec![projection_function_class("https://example.org/vocab")],
                see_also: Some("https://example.org/vocab/queries/projections/demo.rq".to_owned()),
                expects: vec![
                    "https://example.org/vocab/paramFoo".to_owned(),
                    "https://example.org/vocab/paramBar".to_owned(),
                ],
                output: FnOutput {
                    iri: "https://example.org/vocab/outDemo".to_owned(),
                    predicate: Some("https://example.org/vocab/fullName".to_owned()),
                    r#type: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    label: None,
                    description: None,
                },
            }],
            params: vec![
                FnParam {
                    iri: "https://example.org/vocab/paramFoo".to_owned(),
                    predicate: Some("https://example.org/vocab/foo".to_owned()),
                    r#type: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    required: true,
                    label: None,
                    description: None,
                },
                FnParam {
                    iri: "https://example.org/vocab/paramBar".to_owned(),
                    predicate: Some("https://example.org/vocab/bar".to_owned()),
                    r#type: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    required: false,
                    label: None,
                    description: None,
                },
            ],
            implementations: vec![FnImpl {
                iri: "https://example.org/vocab/implDemo".to_owned(),
                format: "application/sparql-query".to_owned(),
                see_also: "https://example.org/vocab/queries/projections/demo.rq".to_owned(),
            }],
            mappings: vec![FnMapping {
                node: blank("mapping-fnDemo-demo"),
                label: label("fnDemo → demo (FnO mapping)"),
                function: "https://example.org/vocab/fnDemo".to_owned(),
                implementation: "https://example.org/vocab/implDemo".to_owned(),
                parameter_mappings: vec![
                    FnParamMapping {
                        node: blank("param-fnDemo-demo-paramFoo-foo"),
                        label: label("paramFoo ↦ ?foo"),
                        function_parameter: "https://example.org/vocab/paramFoo".to_owned(),
                        implementation_property: "foo".to_owned(),
                    },
                    FnParamMapping {
                        node: blank("param-fnDemo-demo-paramBar-bar"),
                        label: label("paramBar ↦ ?bar"),
                        function_parameter: "https://example.org/vocab/paramBar".to_owned(),
                        implementation_property: "bar".to_owned(),
                    },
                ],
                return_mappings: vec![FnReturnMapping {
                    node: blank("return-fnDemo-demo-out"),
                    label: label("fnDemo output ↦ ?out"),
                    function_output: "https://example.org/vocab/outDemo".to_owned(),
                    implementation_property: "out".to_owned(),
                }],
            }],
        })
        .unwrap()
    }

    /// True iff a quad with this `(predicate, object-term)` shape is present.
    fn has_obj(quads: &[RdfQuad], predicate: &str, object: &RdfTerm) -> bool {
        quads
            .iter()
            .any(|q| q.predicate == predicate && &q.object == object)
    }

    #[test]
    fn emits_function_typing_and_label_langstring() {
        let quads = to_quads(&sample_catalog());
        let fno_function = RdfTerm::iri(format!("{FNO}Function"));
        let projection_fn = RdfTerm::iri(projection_function_class("https://example.org/vocab"));
        assert!(has_obj(&quads, RDF_TYPE, &fno_function));
        assert!(has_obj(&quads, RDF_TYPE, &projection_fn));
        // The label is a @x-purrdf-english langString (NOT plain, NOT pre-retagged).
        let label = RdfTerm::literal(RdfLiteral::language_tagged(
            "demo function",
            X_PURRDF_ENGLISH,
        ));
        assert!(has_obj(&quads, RDFS_LABEL, &label));
    }

    #[test]
    fn required_is_an_xsd_boolean_literal() {
        let quads = to_quads(&sample_catalog());
        let t = RdfTerm::literal(RdfLiteral::typed("true", purrdf_xsd::datatype::XSD_BOOLEAN));
        let f = RdfTerm::literal(RdfLiteral::typed(
            "false",
            purrdf_xsd::datatype::XSD_BOOLEAN,
        ));
        assert!(
            has_obj(&quads, &format!("{FNO}required"), &t),
            "required true"
        );
        assert!(
            has_obj(&quads, &format!("{FNO}required"), &f),
            "required false"
        );
    }

    #[test]
    fn expects_is_an_ordered_rdf_list() {
        let quads = to_quads(&sample_catalog());
        // The function points at a list head via fno:expects.
        let head = quads
            .iter()
            .find(|q| q.predicate == format!("{FNO}expects"))
            .map(|q| q.object.clone())
            .expect("fno:expects head");
        assert!(matches!(head, RdfTerm::BlankNode(_)));
        // The head's rdf:first is the required param (Foo, emitted first).
        let foo = RdfTerm::iri("https://example.org/vocab/paramFoo");
        let first = quads
            .iter()
            .find(|q| q.subject == head && q.predicate == RDF_FIRST)
            .map(|q| q.object.clone())
            .expect("rdf:first on head");
        assert_eq!(first, foo);
        // Follow rdf:rest one step → the optional param (Bar), then rdf:nil.
        let rest = quads
            .iter()
            .find(|q| q.subject == head && q.predicate == RDF_REST)
            .map(|q| q.object.clone())
            .expect("rdf:rest on head");
        let bar = RdfTerm::iri("https://example.org/vocab/paramBar");
        assert!(
            has_obj(&quads, RDF_FIRST, &bar) || {
                quads
                    .iter()
                    .any(|q| q.subject == rest && q.predicate == RDF_FIRST && q.object == bar)
            }
        );
        // The list terminates in rdf:nil.
        assert!(has_obj(&quads, RDF_REST, &RdfTerm::iri(RDF_NIL)));
    }

    #[test]
    fn returns_is_a_one_element_list() {
        let quads = to_quads(&sample_catalog());
        let head = quads
            .iter()
            .find(|q| q.predicate == format!("{FNO}returns"))
            .map(|q| q.object.clone())
            .expect("fno:returns head");
        let out = RdfTerm::iri("https://example.org/vocab/outDemo");
        // head rdf:first out ; head rdf:rest rdf:nil.
        assert!(
            quads
                .iter()
                .any(|q| q.subject == head && q.predicate == RDF_FIRST && q.object == out)
        );
        assert!(quads.iter().any(|q| q.subject == head
            && q.predicate == RDF_REST
            && q.object == RdfTerm::iri(RDF_NIL)));
    }

    #[test]
    fn mapping_uses_plain_string_for_implementation_property() {
        let quads = to_quads(&sample_catalog());
        // dcterms:format and fnom:implementationProperty are PLAIN strings.
        let sparql = RdfTerm::literal(RdfLiteral::simple("application/sparql-query"));
        assert!(has_obj(&quads, DCTERMS_FORMAT, &sparql));
        let foo_var = RdfTerm::literal(RdfLiteral::simple("foo"));
        assert!(has_obj(
            &quads,
            &format!("{FNOM}implementationProperty"),
            &foo_var
        ));
        // The mapping subject is the slice-emitter-computed deterministic blank node.
        let mapping = owned(&blank("mapping-fnDemo-demo").term_value());
        assert!(quads.iter().any(|q| q.subject == mapping
            && q.predicate == RDF_TYPE
            && q.object == RdfTerm::iri(format!("{FNO}Mapping"))));
    }

    #[test]
    fn to_ntriples_emits_the_document_node() {
        let empty = FnoCatalog::try_new(FnoCatalogInput {
            ontology_iri: "https://example.org/vocab".to_owned(),
            document_iri: "https://example.org/vocab/projections/functions".to_owned(),
            doc_label: label("PurRDF projection functions (FnO)"),
            banner: label("GENERATED — DO NOT EDIT."),
            functions: vec![],
            params: vec![],
            implementations: vec![],
            mappings: vec![],
        })
        .unwrap();
        let text = to_ntriples(&empty);
        // The document node typing is always present.
        assert!(text.contains(&format!("<{OWL_ONTOLOGY}>")));
        assert!(text.contains(X_PURRDF_ENGLISH));
    }

    /// RDF set facts canonicalize while parameter list order remains significant.
    #[test]
    fn canonical_catalogs_preserve_ordered_expects() {
        let original = sample_catalog();
        let mut input = original.as_input().clone();
        input.params.reverse();
        input.params.push(input.params[0].clone());
        input.mappings[0].parameter_mappings.reverse();
        input.mappings.push(input.mappings[0].clone());
        input.functions[0].kind_types.push(format!("{FNO}Function"));
        input.functions.push(input.functions[0].clone());
        let permuted = FnoCatalog::try_new(input).unwrap();
        assert_eq!(permuted, original);
        assert_eq!(to_quads(&permuted), to_quads(&original));
        let mut input = original.as_input().clone();
        input.functions[0].expects.reverse();
        let reversed = FnoCatalog::try_new(input).unwrap();
        assert_ne!(reversed, original);
        assert_eq!(
            reversed.as_input().functions[0].expects[0],
            "https://example.org/vocab/paramBar"
        );
    }

    /// Compatible parameter/output aliases retain their union of RDF facts.
    #[test]
    fn compatible_roles_share_facts_without_narrowing_the_profile() {
        let mut input = sample_catalog().as_input().clone();
        let parameter = &mut input.params[0];
        parameter.label = Some(label("shared label"));
        let output = &mut input.functions[0].output;
        output.iri.clone_from(&parameter.iri);
        output.predicate.clone_from(&parameter.predicate);
        output.r#type.clone_from(&parameter.r#type);
        output.description = Some(label("shared description"));
        input.mappings[0].return_mappings[0]
            .function_output
            .clone_from(&output.iri);
        let catalog = FnoCatalog::try_new(input.clone()).unwrap();
        let output = &catalog.as_input().functions[0].output;
        let parameter = catalog
            .as_input()
            .params
            .iter()
            .find(|item| item.iri == output.iri)
            .unwrap();
        assert_eq!(output.label, parameter.label);
        assert_eq!(output.description, parameter.description);
        let subject = RdfTerm::iri(&output.iri);
        let quads = to_quads(&catalog);
        for class in ["Parameter", "Output"] {
            assert!(quads.iter().any(|quad| quad.subject == subject
                && quad.predicate == RDF_TYPE
                && quad.object == RdfTerm::iri(format!("{FNO}{class}"))));
        }
        input.functions[0].output.r#type = purrdf_xsd::datatype::XSD_BOOLEAN.to_owned();
        assert!(matches!(
            *FnoCatalog::try_new(input).unwrap_err().reason,
            FnoAdmissionReason::ConflictingNode { .. }
        ));
        assert_eq!(
            FnoCatalog::try_new(catalog.as_input().clone()).unwrap(),
            catalog
        );

        let mut input = sample_catalog().as_input().clone();
        let function_iri = input.functions[0].iri.clone();
        let previous_parameter = input.params[0].iri.clone();
        input.params[0].iri.clone_from(&function_iri);
        input.params[0].label = Some(input.functions[0].label.clone());
        for parameter in &mut input.functions[0].expects {
            if parameter == &previous_parameter {
                parameter.clone_from(&function_iri);
            }
        }
        for mapping in &mut input.mappings {
            for item in &mut mapping.parameter_mappings {
                if item.function_parameter == previous_parameter {
                    item.function_parameter.clone_from(&function_iri);
                }
            }
        }
        input.document_iri.clone_from(&function_iri);
        input.doc_label.clone_from(&input.functions[0].label);
        input.functions[0]
            .kind_types
            .extend([format!("{FNO}Parameter"), OWL_ONTOLOGY.to_owned()]);
        let alias = FnoCatalog::try_new(input).unwrap();
        assert!(
            !alias.as_input().functions[0]
                .kind_types
                .iter()
                .any(|kind| kind == OWL_ONTOLOGY || kind == &format!("{FNO}Parameter"))
        );
        assert_eq!(
            FnoCatalog::try_new(alias.as_input().clone()).unwrap(),
            alias
        );
    }

    /// Caller blank identity and every generated chain remain disjoint.
    #[test]
    fn scoped_mapping_and_function_collisions_are_distinct() {
        let mut input = sample_catalog().as_input().clone();
        let template = input.mappings[0].clone();
        input.mappings.clear();
        for node in [
            blank("a-b"),
            blank("a_b"),
            blank("fno-list-0"),
            FnBlank {
                label: "same".to_owned(),
                scope: BlankScope(1),
            },
            FnBlank {
                label: "same".to_owned(),
                scope: BlankScope(2),
            },
        ] {
            let mut mapping = template.clone();
            mapping.node = node;
            input.mappings.push(mapping);
        }
        for iri in ["https://example.org/f-a", "https://example.org/f_a"] {
            let mut function = input.functions[0].clone();
            function.iri = iri.to_owned();
            input.functions.push(function);
        }
        let catalog = FnoCatalog::try_new(input).unwrap();
        let quads = to_quads(&catalog);
        let subjects: BTreeSet<_> = quads
            .iter()
            .filter(|quad| {
                quad.predicate == RDF_TYPE && quad.object == RdfTerm::iri(format!("{FNO}Mapping"))
            })
            .map(|quad| TermValue::from_rdf_term(&quad.subject))
            .collect();
        assert_eq!(subjects.len(), 5);
        for mapping in &catalog.as_input().mappings {
            assert!(subjects.contains(&mapping.node.term_value()));
            assert!(
                !quads
                    .iter()
                    .any(|quad| TermValue::from_rdf_term(&quad.subject)
                        == mapping.node.term_value()
                        && quad.predicate == RDF_FIRST)
            );
        }
        let heads: BTreeSet<_> = quads
            .iter()
            .filter(|quad| {
                quad.predicate == format!("{FNO}expects")
                    || quad.predicate == format!("{FNO}returns")
            })
            .map(|quad| TermValue::from_rdf_term(&quad.object))
            .collect();
        assert_eq!(heads.len(), 6);
        assert!(heads.is_disjoint(&subjects));
        assert_eq!(quads, to_quads(&catalog));
    }

    /// Native literal owners preserve datatype, language and RDF base direction.
    #[test]
    fn original_literal_facets_survive_emission() {
        let mut input = sample_catalog().as_input().clone();
        input.doc_label = TermValue::simple_literal("plain document label");
        input.functions[0].label = TermValue::Literal {
            lexical_form: "مرحبا".to_owned(),
            datatype: purrdf_iri::vocab::rdf::DIR_LANG_STRING.to_owned(),
            language: Some("AR".to_owned()),
            direction: Some(crate::RdfTextDirection::Rtl),
        };
        input.functions[0].output.description = Some(TermValue::typed_literal(
            "native text",
            "https://example.org/Text",
        ));
        let catalog = FnoCatalog::try_new(input).unwrap();
        let label = &catalog.as_input().functions[0].label;
        assert!(
            matches!(label, TermValue::Literal { language: Some(language), .. } if language == "ar")
        );
        let quads = to_quads(&catalog);
        assert!(
            quads
                .iter()
                .any(|quad| TermValue::from_rdf_term(&quad.object) == *label)
        );
        assert!(
            quads
                .iter()
                .any(|quad| TermValue::from_rdf_term(&quad.object)
                    == TermValue::typed_literal("native text", "https://example.org/Text"))
        );
        assert!(to_ntriples(&catalog).contains("@ar--rtl"));
    }

    /// Each admission boundary retains rejected input and a healthy neighbor.
    #[test]
    fn admission_retains_original_data_and_native_causes() {
        use purrdf_testkit::error_capture::Capture;
        let healthy = sample_catalog();
        for selection in 0..5 {
            let mut input = healthy.as_input().clone();
            match selection {
                0 => input.ontology_iri = "relative".to_owned(),
                1 => input.banner = TermValue::iri("https://example.org/not-literal"),
                2 => {
                    input.banner = TermValue::Literal {
                        lexical_form: "bad facets".to_owned(),
                        datatype: purrdf_xsd::datatype::XSD_STRING.to_owned(),
                        language: None,
                        direction: Some(crate::RdfTextDirection::Rtl),
                    }
                }
                3 => {
                    let mut param = input.params[0].clone();
                    param.required = !param.required;
                    input.params.push(param);
                }
                4 => input.functions[0]
                    .expects
                    .push("https://example.org/missing".to_owned()),
                _ => unreachable!(),
            }
            let pointer = input.document_iri.as_ptr();
            let expected = input.clone();
            let error = FnoCatalog::try_new(input).unwrap_err();
            assert_eq!(error.input.document_iri.as_ptr(), pointer);
            assert_eq!(*error.input, expected);
            let capture = Capture::read(&error).child("reason");
            assert_eq!(
                capture.code,
                [
                    "fno-iri",
                    "fno-expected-literal",
                    "fno-literal",
                    "fno-conflicting-node",
                    "fno-reference"
                ][selection]
            );
            if selection == 0 || selection == 2 {
                assert!(std::error::Error::source(&error.reason).is_some());
            }
            assert_eq!(
                FnoCatalog::try_new(healthy.as_input().clone()).unwrap(),
                healthy
            );
        }
    }
}
