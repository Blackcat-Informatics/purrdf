// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native FnO (W3C Function Ontology) typed model + serializer.
//!
//! This is the PyO3-free Rust FnO model and serializer (#848). It carries a single,
//! fully-resolved **catalog** of the PURRDF projection-function surface
//! (`generated/projections/functions.fno.ttl`) into an owned RDF quad set and
//! serializes it to N-Triples text.
//!
//! ## The seam
//!
//! Unlike the transitional design, the [`FnoCatalog`] is now BUILT IN RUST — the
//! oxigraph-free FnO correspondence lowering
//! (`crates/logic-compile/src/projections/fno.rs`) discovers the
//! projection functions + cells from the slice framework + the repo DSL tree,
//! reads each input predicate's ontology `rdfs:range` (the fail-closed untyped-param
//! guard), scans the projection cells, sorts + aggregates the var bindings, mints
//! every IRI, and computes the deterministic mapping/param/return blank-node
//! *labels*. It hands the finished [`FnoCatalog`] here; this module emits the EXACT
//! triples the old rdflib body produced. The Python side re-parses the emitted text
//! into a fresh rdflib `Graph` (so the downstream lints + the Turtle writer are
//! unchanged).
//!
//! Parity is **graph isomorphism** (the rdflib writer normalizes blank-node
//! ordering on the final write, and the drift gate compares canonical quad sets),
//! so the native text need not be byte-identical — only the same triple set. In
//! particular the `fno:expects` / `fno:returns` `rdf:List` cells are anonymous in
//! both worlds, so their blank-node labels are immaterial; the mapping/param/return
//! subjects reuse the slice-emitter-computed labels purely so the model is
//! self-documenting.

use crate::{turtle, RdfLiteral, RdfQuad, RdfTerm};

// --------------------------------------------------------------------------- //
// Vocabulary
// --------------------------------------------------------------------------- //

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
const RDFS_SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";

const SKOS_DEFINITION: &str = "http://www.w3.org/2004/02/skos/core#definition";

const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";

const DCTERMS_IS_PART_OF: &str = "http://purl.org/dc/terms/isPartOf";
const DCTERMS_FORMAT: &str = "http://purl.org/dc/terms/format";

const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// The `https://w3id.org/function/ontology#` (fno) namespace.
const FNO: &str = "https://w3id.org/function/ontology#";
/// The `https://w3id.org/function/vocabulary/mapping#` (fnom) namespace.
const FNOM: &str = "https://w3id.org/function/vocabulary/mapping#";
/// The PURRDF namespace (`purrdf:ProjectionFunction`).
///
/// `pub` so the `purrdf-slice` FnO emitter can populate
/// [`FnFunction::kind_types`] with the projection-function type for the
/// `functions.fno.ttl` path (preserving its byte-identical output).
pub const PURRDF_PROJECTION_FUNCTION: &str =
    "https://blackcatinformatics.ca/purrdf/ProjectionFunction";

/// The PURRDF-internal language tag every localizable literal carries until the
/// projection boundary retags it to public BCP-47.
const X_PURRDF_ENGLISH: &str = "x-purrdf-english";

// --------------------------------------------------------------------------- //
// Catalog model (built in Rust by the purrdf-slice FnO emitter)
// --------------------------------------------------------------------------- //

/// The fully-resolved FnO catalog the `purrdf-slice` emitter assembles and
/// serializes here.
///
/// Every IRI / label / type / blank-node label is already derived by the lowering;
/// this struct is a pure data carrier — no ontology reads, no minting, no sorting
/// happens here (that all stays in `crates/logic-compile/src/projections/fno.rs`).
#[derive(Debug, Clone)]
pub struct FnoCatalog {
    /// The root ontology IRI the document `dcterms:isPartOf`.
    pub ontology_iri: String,
    /// The FnO document node IRI (`<ontology_iri>/projections/functions`).
    pub document_iri: String,
    /// The document's `rdfs:label` (a `@x-purrdf-english` literal).
    pub doc_label: String,
    /// The document's `rdfs:comment` banner (a `@x-purrdf-english` literal).
    pub banner: String,
    /// The function nodes, already sorted by the emitter.
    pub functions: Vec<FnFunction>,
    /// The globally-deduped parameter nodes, in first-use order.
    pub params: Vec<FnParam>,
    /// The deduped implementation nodes (one per profile `.rq`), first-seen order.
    pub implementations: Vec<FnImpl>,
    /// The fno:Mapping nodes (one per (function, profile)).
    pub mappings: Vec<FnMapping>,
}

/// One `fno:Function` node (always typed `fno:Function`; any additional
/// `rdf:type` IRIs — e.g. `purrdf:ProjectionFunction` for the projection catalog —
/// come from [`FnFunction::kind_types`]).
#[derive(Debug, Clone)]
pub struct FnFunction {
    pub iri: String,
    /// `rdfs:label` (`@x-purrdf-english`).
    pub label: String,
    /// `skos:definition` (`@x-purrdf-english`), omitted when empty.
    pub description: Option<String>,
    /// Extra `rdf:type` IRIs emitted IN ADDITION to `fno:Function`, in vec order.
    /// The projection-function builder sets `[PURRDF_PROJECTION_FUNCTION]`;
    /// primitives (e.g. the list functions) leave this empty so they are
    /// `fno:Function` ONLY.
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
#[derive(Debug, Clone)]
pub struct FnOutput {
    pub iri: String,
    /// `fno:predicate` — the PURRDF predicate the output realises. `None` for
    /// primitives (the list functions bind no data predicate).
    pub predicate: Option<String>,
    /// `fno:type` — the output's range IRI.
    pub r#type: String,
    /// `rdfs:label` (`@x-purrdf-english`), emitted when `Some`.
    pub label: Option<String>,
    /// `skos:definition` (`@x-purrdf-english`), emitted when `Some`.
    pub description: Option<String>,
}

/// One globally-deduped `fno:Parameter` node.
#[derive(Debug, Clone)]
pub struct FnParam {
    pub iri: String,
    /// `fno:predicate` — the source PURRDF predicate. `None` for primitives (the
    /// list functions bind no data predicate).
    pub predicate: Option<String>,
    /// `fno:type` — the predicate's ontology `rdfs:range` (the fail-closed type).
    pub r#type: String,
    /// `fno:required` (an `xsd:boolean` literal).
    pub required: bool,
    /// `rdfs:label` (`@x-purrdf-english`), emitted when `Some`.
    pub label: Option<String>,
    /// `skos:definition` (`@x-purrdf-english`), emitted when `Some`.
    pub description: Option<String>,
}

/// One `fno:Implementation` node (one per profile `.rq`).
#[derive(Debug, Clone)]
pub struct FnImpl {
    pub iri: String,
    /// `dcterms:format` — a plain string literal (`"application/sparql-query"`).
    pub format: String,
    /// `rdfs:seeAlso` — the profile's `.rq` query.
    pub see_also: String,
}

/// One `fno:Mapping` node linking a function to one profile's implementation.
#[derive(Debug, Clone)]
pub struct FnMapping {
    /// The deterministic blank-node label (`mapping-<fn_local>-<profile>`).
    pub bnode_label: String,
    /// `rdfs:label` (`@x-purrdf-english`).
    pub label: String,
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
#[derive(Debug, Clone)]
pub struct FnParamMapping {
    /// The deterministic blank-node label.
    pub bnode_label: String,
    /// `rdfs:label` (`@x-purrdf-english`).
    pub label: String,
    /// `fnom:functionParameter` — the parameter IRI.
    pub function_parameter: String,
    /// `fnom:implementationProperty` — the SPARQL var name (a plain string).
    pub implementation_property: String,
}

/// One `fnom:DefaultReturnMapping` (the function output ↦ a SPARQL variable).
#[derive(Debug, Clone)]
pub struct FnReturnMapping {
    /// The deterministic blank-node label.
    pub bnode_label: String,
    /// `rdfs:label` (`@x-purrdf-english`).
    pub label: String,
    /// `fnom:functionOutput` — the function's output node IRI.
    pub function_output: String,
    /// `fnom:implementationProperty` — the SPARQL var name (a plain string).
    pub implementation_property: String,
}

// --------------------------------------------------------------------------- //
// Serialization
// --------------------------------------------------------------------------- //

/// Sanitize a label into a blank-node id: every non-alphanumeric character becomes
/// `_`, prefixed with `n_`. This keeps the mapping/param/return subjects
/// self-documenting; isomorphism does not require the exact labels.
fn bnode(label: &str) -> RdfTerm {
    let safe: String = label
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    RdfTerm::blank_node(format!("n_{safe}"))
}

/// A `@x-purrdf-english` language-tagged literal term.
fn en(text: &str) -> RdfTerm {
    RdfTerm::literal(RdfLiteral::language_tagged(
        text.to_owned(),
        X_PURRDF_ENGLISH,
    ))
}

/// A plain (no datatype, no language) string literal term — the form the rdflib
/// emitter produced for `dcterms:format` and `fnom:implementationProperty`.
fn plain(text: &str) -> RdfTerm {
    RdfTerm::literal(RdfLiteral::simple(text.to_owned()))
}

/// An `xsd:boolean` literal term (`"true"`/`"false"`), matching rdflib's
/// `Literal(bool)` lexical form.
fn boolean(value: bool) -> RdfTerm {
    RdfTerm::literal(RdfLiteral::typed(
        if value { "true" } else { "false" },
        XSD_BOOLEAN,
    ))
}

/// Build the typed model's quads in the EXACT shape `emit_fno` / `_emit_fnom`
/// produced — the same triple set, datatypes, and language tags.
///
/// The `fno:expects` / `fno:returns` lists are emitted as proper
/// `rdf:first`/`rdf:rest`/`rdf:nil` chains; the list cells are anonymous blank
/// nodes (isomorphism ignores their labels) minted deterministically from the
/// list subject + a stable suffix so re-running is reproducible.
pub fn to_quads(catalog: &FnoCatalog) -> Vec<RdfQuad> {
    let mut quads: Vec<RdfQuad> = Vec::new();

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
        en(&catalog.doc_label),
    ));
    quads.push(RdfQuad::new(
        doc.clone(),
        DCTERMS_IS_PART_OF,
        RdfTerm::iri(&catalog.ontology_iri),
    ));
    quads.push(RdfQuad::new(doc, RDFS_COMMENT, en(&catalog.banner)));

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
        quads.push(RdfQuad::new(fn_iri.clone(), RDFS_LABEL, en(&func.label)));
        if let Some(description) = &func.description {
            quads.push(RdfQuad::new(
                fn_iri.clone(),
                SKOS_DEFINITION,
                en(description),
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
            "expects",
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
            quads.push(RdfQuad::new(out.clone(), RDFS_LABEL, en(label)));
        }
        if let Some(description) = &func.output.description {
            quads.push(RdfQuad::new(out.clone(), SKOS_DEFINITION, en(description)));
        }
        attach_list(
            &mut quads,
            &fn_iri,
            &format!("{FNO}returns"),
            std::slice::from_ref(&out),
            "returns",
        );
    }

    // ── Parameters (globally deduped, first-use order) ─────────────────────
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
            boolean(param.required),
        ));
        if let Some(label) = &param.label {
            quads.push(RdfQuad::new(p.clone(), RDFS_LABEL, en(label)));
        }
        if let Some(description) = &param.description {
            quads.push(RdfQuad::new(p, SKOS_DEFINITION, en(description)));
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
        let m = bnode(&mapping.bnode_label);
        quads.push(RdfQuad::new(
            m.clone(),
            RDF_TYPE,
            RdfTerm::iri(format!("{FNO}Mapping")),
        ));
        quads.push(RdfQuad::new(m.clone(), RDFS_LABEL, en(&mapping.label)));
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
            let p = bnode(&pmap.bnode_label);
            quads.push(RdfQuad::new(
                p.clone(),
                RDF_TYPE,
                RdfTerm::iri(format!("{FNOM}PropertyParameterMapping")),
            ));
            quads.push(RdfQuad::new(p.clone(), RDFS_LABEL, en(&pmap.label)));
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
            let r = bnode(&rmap.bnode_label);
            quads.push(RdfQuad::new(
                r.clone(),
                RDF_TYPE,
                RdfTerm::iri(format!("{FNOM}DefaultReturnMapping")),
            ));
            quads.push(RdfQuad::new(r.clone(), RDFS_LABEL, en(&rmap.label)));
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
/// list-cell blank nodes deterministically from the subject IRI + `tag` + index.
///
/// An empty list links straight to `rdf:nil` (rdflib's `Collection` does the
/// same). The cell labels are immaterial to the isomorphism gate; they are derived
/// reproducibly only so re-running yields a stable document.
fn attach_list(
    quads: &mut Vec<RdfQuad>,
    subject: &RdfTerm,
    predicate: &str,
    items: &[RdfTerm],
    tag: &str,
) {
    if items.is_empty() {
        quads.push(RdfQuad::new(
            subject.clone(),
            predicate,
            RdfTerm::iri(RDF_NIL),
        ));
        return;
    }
    let subj_id = match subject {
        RdfTerm::Iri(iri) => iri.as_str(),
        RdfTerm::BlankNode(label) => label.as_str(),
        _ => "list",
    };
    let safe: String = subj_id
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let cells: Vec<RdfTerm> = (0..items.len())
        .map(|i| RdfTerm::blank_node(format!("l_{safe}_{tag}_{i}")))
        .collect();
    quads.push(RdfQuad::new(subject.clone(), predicate, cells[0].clone()));
    for (i, item) in items.iter().enumerate() {
        quads.push(RdfQuad::new(cells[i].clone(), RDF_FIRST, item.clone()));
        let rest = if i + 1 < items.len() {
            cells[i + 1].clone()
        } else {
            RdfTerm::iri(RDF_NIL)
        };
        quads.push(RdfQuad::new(cells[i].clone(), RDF_REST, rest));
    }
}

/// Serialize a [`FnoCatalog`]'s typed model to N-Triples text (the rdflib-parseable
/// form the Python side re-parses).
///
/// The returned text is full-IRI N-Triples (no `@prefix` block); the Python caller
/// parses it back into a fresh rdflib `Graph` and the rdflib Turtle writer is the
/// byte-stability layer.
pub fn to_ntriples(catalog: &FnoCatalog) -> String {
    to_quads(catalog).iter().map(turtle::emit_quad).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-built catalog: one function with one required + one optional param,
    /// one profile implementation, and one mapping with two param-var bindings and
    /// one return-var binding.
    fn sample_catalog() -> FnoCatalog {
        FnoCatalog {
            ontology_iri: "https://blackcatinformatics.ca/purrdf".to_owned(),
            document_iri: "https://blackcatinformatics.ca/purrdf/projections/functions".to_owned(),
            doc_label: "PURRDF projection functions (FnO)".to_owned(),
            banner: "GENERATED — DO NOT EDIT.".to_owned(),
            functions: vec![FnFunction {
                iri: "https://blackcatinformatics.ca/purrdf/fnDemo".to_owned(),
                label: "demo function".to_owned(),
                description: Some("a demo".to_owned()),
                kind_types: vec![PURRDF_PROJECTION_FUNCTION.to_owned()],
                see_also: Some(
                    "https://blackcatinformatics.ca/purrdf/queries/projections/demo.rq".to_owned(),
                ),
                expects: vec![
                    "https://blackcatinformatics.ca/purrdf/paramFoo".to_owned(),
                    "https://blackcatinformatics.ca/purrdf/paramBar".to_owned(),
                ],
                output: FnOutput {
                    iri: "https://blackcatinformatics.ca/purrdf/outDemo".to_owned(),
                    predicate: Some("https://blackcatinformatics.ca/purrdf/fullName".to_owned()),
                    r#type: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    label: None,
                    description: None,
                },
            }],
            params: vec![
                FnParam {
                    iri: "https://blackcatinformatics.ca/purrdf/paramFoo".to_owned(),
                    predicate: Some("https://blackcatinformatics.ca/purrdf/foo".to_owned()),
                    r#type: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    required: true,
                    label: None,
                    description: None,
                },
                FnParam {
                    iri: "https://blackcatinformatics.ca/purrdf/paramBar".to_owned(),
                    predicate: Some("https://blackcatinformatics.ca/purrdf/bar".to_owned()),
                    r#type: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    required: false,
                    label: None,
                    description: None,
                },
            ],
            implementations: vec![FnImpl {
                iri: "https://blackcatinformatics.ca/purrdf/implDemo".to_owned(),
                format: "application/sparql-query".to_owned(),
                see_also: "https://blackcatinformatics.ca/purrdf/queries/projections/demo.rq"
                    .to_owned(),
            }],
            mappings: vec![FnMapping {
                bnode_label: "mapping-fnDemo-demo".to_owned(),
                label: "fnDemo → demo (FnO mapping)".to_owned(),
                function: "https://blackcatinformatics.ca/purrdf/fnDemo".to_owned(),
                implementation: "https://blackcatinformatics.ca/purrdf/implDemo".to_owned(),
                parameter_mappings: vec![
                    FnParamMapping {
                        bnode_label: "param-fnDemo-demo-paramFoo-foo".to_owned(),
                        label: "paramFoo ↦ ?foo".to_owned(),
                        function_parameter: "https://blackcatinformatics.ca/purrdf/paramFoo"
                            .to_owned(),
                        implementation_property: "foo".to_owned(),
                    },
                    FnParamMapping {
                        bnode_label: "param-fnDemo-demo-paramBar-bar".to_owned(),
                        label: "paramBar ↦ ?bar".to_owned(),
                        function_parameter: "https://blackcatinformatics.ca/purrdf/paramBar"
                            .to_owned(),
                        implementation_property: "bar".to_owned(),
                    },
                ],
                return_mappings: vec![FnReturnMapping {
                    bnode_label: "return-fnDemo-demo-out".to_owned(),
                    label: "fnDemo output ↦ ?out".to_owned(),
                    function_output: "https://blackcatinformatics.ca/purrdf/outDemo".to_owned(),
                    implementation_property: "out".to_owned(),
                }],
            }],
        }
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
        let purrdf_pf = RdfTerm::iri(PURRDF_PROJECTION_FUNCTION);
        assert!(has_obj(&quads, RDF_TYPE, &fno_function));
        assert!(has_obj(&quads, RDF_TYPE, &purrdf_pf));
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
        let t = RdfTerm::literal(RdfLiteral::typed("true", XSD_BOOLEAN));
        let f = RdfTerm::literal(RdfLiteral::typed("false", XSD_BOOLEAN));
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
        let foo = RdfTerm::iri("https://blackcatinformatics.ca/purrdf/paramFoo");
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
        let bar = RdfTerm::iri("https://blackcatinformatics.ca/purrdf/paramBar");
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
        let out = RdfTerm::iri("https://blackcatinformatics.ca/purrdf/outDemo");
        // head rdf:first out ; head rdf:rest rdf:nil.
        assert!(quads
            .iter()
            .any(|q| q.subject == head && q.predicate == RDF_FIRST && q.object == out));
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
        let mapping = bnode("mapping-fnDemo-demo");
        assert!(quads.iter().any(|q| q.subject == mapping
            && q.predicate == RDF_TYPE
            && q.object == RdfTerm::iri(format!("{FNO}Mapping"))));
    }

    #[test]
    fn to_ntriples_emits_the_document_node() {
        let empty = FnoCatalog {
            ontology_iri: "https://blackcatinformatics.ca/purrdf".to_owned(),
            document_iri: "https://blackcatinformatics.ca/purrdf/projections/functions".to_owned(),
            doc_label: "PURRDF projection functions (FnO)".to_owned(),
            banner: "GENERATED — DO NOT EDIT.".to_owned(),
            functions: vec![],
            params: vec![],
            implementations: vec![],
            mappings: vec![],
        };
        let text = to_ntriples(&empty);
        // The document node typing is always present.
        assert!(text.contains(&format!("<{OWL_ONTOLOGY}>")));
        assert!(text.contains(X_PURRDF_ENGLISH));
    }
}
