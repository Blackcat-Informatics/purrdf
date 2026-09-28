// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native OWL axiom-annotation ↔ RDF 1.2 statement codec — the **lead** writer.
//!
//! Mirrors the two SPARQL CONSTRUCT codecs (`queries/codecs/rdf12-project.rq`
//! and `rdf12-to-owl.rq`) as pure structural folds over the purrdf model, so
//! the RDF 1.2 statement lead artifact (`generated/statements/purrdf.rdf12.ttl`)
//! is produced with **no Apache Jena, no Docker, and no SPARQL engine**. The native
//! [`parse_dataset`] codec only *parses* the input
//! Turtle into the IR; the projection itself is a fold over native RDF quads (the
//! IR flattened back to a flat quad stream) into RDF 1.2 triple terms.
//!
//! Both emitters write full-IRI Turtle (no prefix compaction); the drift gate
//! compares RDFC-1.0 canonical quad sets (graph isomorphism), so banners and
//! prefixes are immaterial — the triple/reifier/annotation *structure* is what
//! must round-trip losslessly, verified by construction — never a silent drop.
//!
//! The RDF 1.2 triple-term form `<<( s p o )>>` is emitted (matching the SPARQL
//! codecs and what oxigraph/Jena both parse), not the RDF-star `<< s p o >>`
//! shorthand the reasoning-closure emitter uses.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::native_quads::flat_rdf_quads_from_dataset;
use crate::{
    NativeRdfFormat, RdfDiagnostic, RdfLiteral, RdfQuad, RdfTerm, RdfTriple, parse_dataset,
};

const OWL_AXIOM: &str = "http://www.w3.org/2002/07/owl#Axiom";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const OWL_ANNOTATED_SOURCE: &str = "http://www.w3.org/2002/07/owl#annotatedSource";
const OWL_ANNOTATED_PROPERTY: &str = "http://www.w3.org/2002/07/owl#annotatedProperty";
const OWL_ANNOTATED_TARGET: &str = "http://www.w3.org/2002/07/owl#annotatedTarget";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Parse a Turtle document (incl. RDF 1.2 triple terms) into model quads.
///
/// Uses the native [`parse_dataset`](crate::parse_dataset) codec into the IR, NOT a
/// `Store`: the `Store` canonicalizes typed-literal lexical forms (`+00:00` → `Z`,
/// `0.70` → `0.7`), but a faithful codec must round-trip literals byte-for-byte so
/// the inversion is a pure serialization-shape change with no value churn into the
/// GTS bundle and the other derived artifacts. The IR is then flattened back to the
/// source-faithful flat quad stream (base quads + the `rdf:reifies` / annotation
/// rows the projection folds over) via [`flat_rdf_quads_from_dataset`].
fn parse_quads(ttl: &str) -> Result<Vec<RdfQuad>, RdfDiagnostic> {
    let dataset = parse_dataset(ttl.as_bytes(), NativeRdfFormat::Turtle.media_type(), None)
        .map_err(|e| RdfDiagnostic::error("statements-turtle-parse", e.to_string()))?;
    Ok(flat_rdf_quads_from_dataset(&dataset))
}

/// Drop a redundant `^^xsd:string` datatype so a simple literal serializes bare.
///
/// oxigraph (RDF 1.1) types a bare `"x"` as `xsd:string`; the authored OWL graph
/// (rdflib) keeps it as a plain literal, and rdflib's isomorphism treats the two
/// as distinct. Jena and oxigraph both emit `xsd:string` literals bare — match
/// that so the round-trip proof against the authored OWL holds. Triple-term
/// components are normalized too.
///
/// A triple term is rebuilt bottom-up over a work list: its subject, then its
/// object, each normalized fully before the next, then the triple with its
/// predicate unchanged.
fn simplify_term(term: &RdfTerm) -> RdfTerm {
    enum Step<'t> {
        Term(&'t RdfTerm),
        Triple(&'t str),
    }
    let mut steps: Vec<Step<'_>> = vec![Step::Term(term)];
    let mut simplified: Vec<RdfTerm> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Term(RdfTerm::Literal(literal))
                if literal.language.is_none()
                    && literal.datatype.as_deref() == Some(XSD_STRING) =>
            {
                simplified.push(RdfTerm::literal(RdfLiteral::simple(
                    literal.lexical_form.clone(),
                )));
            }
            Step::Term(RdfTerm::Triple(triple)) => steps.extend([
                Step::Triple(&triple.predicate),
                Step::Term(&triple.object),
                Step::Term(&triple.subject),
            ]),
            Step::Term(other) => simplified.push(other.clone()),
            Step::Triple(predicate) => {
                let object = simplified
                    .pop()
                    .expect("a triple term's object is normalized");
                let subject = simplified
                    .pop()
                    .expect("a triple term's subject is normalized");
                simplified.push(RdfTerm::triple(RdfTriple::new(
                    subject,
                    predicate.to_owned(),
                    object,
                )));
            }
        }
    }
    simplified
        .pop()
        .expect("the term's own normalization is the last one made")
}

/// Serialize a term to Turtle, normalizing simple literals first. Total: the
/// kernel emitter escapes a blank-node label outside the Turtle
/// `BLANK_NODE_LABEL` alphabet into it rather than writing an unparsable
/// document.
fn emit(term: &RdfTerm) -> String {
    crate::emit_term(&simplify_term(term))
}

/// Render a term for sort keys and diagnostics (deterministic, infallible —
/// this is report/key identity, not document egress).
fn display(term: &RdfTerm) -> String {
    crate::display_term(&simplify_term(term))
}

/// Emit an RDF 1.2 triple term `<<( <s> <p> <o> )>>`.
fn emit_triple_term(subject: &RdfTerm, predicate: &str, object: &RdfTerm) -> String {
    format!("<<( {} <{}> {} )>>", emit(subject), predicate, emit(object))
}

/// Require an IRI term (predicates must be IRIs).
fn require_iri(term: &RdfTerm, context: &str) -> Result<String, RdfDiagnostic> {
    match term {
        RdfTerm::Iri(iri) => Ok(iri.clone()),
        other => Err(RdfDiagnostic::error(
            "statements-non-iri",
            format!("{context} must be an IRI, got {:?}", other.kind()),
        )),
    }
}

/// Set a structural slot exactly once.
///
/// A duplicate triple carrying the *same* object is the idempotent re-assertion of
/// one triple (RDF is a set) and is accepted. A duplicate carrying a *different*
/// object means two contradictory structural triples share one subject — a corrupt
/// input the codec must **reject**, never silently last-write-win — conflicting
/// structure is a hard failure, not an option to paper over.
fn set_once_or_error<T: PartialEq>(
    slot: &mut Option<T>,
    value: T,
    subject: &RdfTerm,
    field: &str,
) -> Result<(), RdfDiagnostic> {
    if let Some(existing) = slot {
        if *existing != value {
            return Err(RdfDiagnostic::error(
                "statements-conflicting-structural",
                format!(
                    "{} has conflicting {field} triples (one subject carries two different values)",
                    crate::display_term(subject)
                ),
            ));
        }
    } else {
        *slot = Some(value);
    }
    Ok(())
}

/// Accumulated facts for one `owl:Axiom` subject during projection.
#[derive(Default)]
struct AxiomAccum {
    subject: Option<RdfTerm>,
    is_axiom: bool,
    source: Option<RdfTerm>,
    property: Option<RdfTerm>,
    target: Option<RdfTerm>,
    annotations: Vec<(String, RdfTerm)>,
}

/// Project the OWL axiom-annotation downcast → the RDF 1.2 / RDF* lead form.
///
/// Mirrors `rdf12-project.rq`: each `owl:Axiom` (with
/// `owl:annotatedSource`/`Property`/`Target` + annotations) becomes the asserted
/// base triple plus a reifier `<<( s p o )>>` carrying the annotations.
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] on a Turtle parse error, a malformed axiom
/// (missing source/property/target), or a non-IRI annotated property.
pub fn project_owl_to_rdf12(owl_ttl: &str) -> Result<String, RdfDiagnostic> {
    let quads = parse_quads(owl_ttl)?;

    let mut axioms: BTreeMap<String, AxiomAccum> = BTreeMap::new();
    for quad in &quads {
        let acc = axioms.entry(display(&quad.subject)).or_default();
        if acc.subject.is_none() {
            acc.subject = Some(quad.subject.clone());
        }
        match quad.predicate.as_str() {
            RDF_TYPE => {
                if matches!(&quad.object, RdfTerm::Iri(iri) if iri == OWL_AXIOM) {
                    acc.is_axiom = true;
                }
                // Other rdf:type values are filtered out (NOT IN rdf:type).
            }
            OWL_ANNOTATED_SOURCE => set_once_or_error(
                &mut acc.source,
                quad.object.clone(),
                &quad.subject,
                "owl:annotatedSource",
            )?,
            OWL_ANNOTATED_PROPERTY => set_once_or_error(
                &mut acc.property,
                quad.object.clone(),
                &quad.subject,
                "owl:annotatedProperty",
            )?,
            OWL_ANNOTATED_TARGET => set_once_or_error(
                &mut acc.target,
                quad.object.clone(),
                &quad.subject,
                "owl:annotatedTarget",
            )?,
            other => acc
                .annotations
                .push((other.to_owned(), quad.object.clone())),
        }
    }

    let mut out = String::new();
    for acc in axioms.values() {
        if !acc.is_axiom {
            continue;
        }
        let subject = acc.subject.as_ref().expect("subject seen for accumulator");
        let missing = |what: &str| {
            RdfDiagnostic::error(
                "statements-malformed-axiom",
                format!("owl:Axiom {} lacks {what}", display(subject)),
            )
        };
        let source = acc
            .source
            .as_ref()
            .ok_or_else(|| missing("owl:annotatedSource"))?;
        let property = acc
            .property
            .as_ref()
            .ok_or_else(|| missing("owl:annotatedProperty"))?;
        let target = acc
            .target
            .as_ref()
            .ok_or_else(|| missing("owl:annotatedTarget"))?;
        let property_iri = require_iri(property, "owl:annotatedProperty")?;

        // ?s ?p ?o .
        let _ = writeln!(
            out,
            "{} <{}> {} .",
            emit(source),
            property_iri,
            emit(target)
        );

        // ?axiom rdf:reifies <<( ?s ?p ?o )>> ; ?annProp ?annVal ; … .
        let mut annotations: Vec<(String, String)> = acc
            .annotations
            .iter()
            .map(|(predicate, object)| (predicate.clone(), emit(object)))
            .collect();
        annotations.sort();
        let mut line = format!(
            "{} <{}> {}",
            emit(subject),
            RDF_REIFIES,
            emit_triple_term(source, &property_iri, target)
        );
        for (predicate, object) in &annotations {
            let _ = write!(line, " ;\n   <{predicate}> {object}");
        }
        line.push_str(" .\n");
        out.push_str(&line);
    }
    Ok(out)
}

/// Accumulated facts for one reifier subject during normalization.
struct ReifierAccum {
    subject: RdfTerm,
    reified: Option<Box<RdfTriple>>,
    annotations: Vec<(String, RdfTerm)>,
}

/// Normalize the RDF 1.2 / RDF* lead form → the OWL axiom-annotation normal form.
///
/// Mirrors `rdf12-to-owl.rq` (the round-trip inverse): each
/// `?reifier rdf:reifies <<( s p o )>>` + annotations becomes the asserted base
/// triple plus the plain `owl:Axiom` reification rdflib can parse (rdflib cannot
/// parse RDF 1.2 triple terms — this normal form is what the isomorphism check
/// compares against the authored OWL graph).
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] on a Turtle parse error or an `rdf:reifies` whose
/// object is not a triple term.
pub fn normalize_rdf12_to_owl(rdf12_ttl: &str) -> Result<String, RdfDiagnostic> {
    let quads = parse_quads(rdf12_ttl)?;

    let mut by_subject: BTreeMap<String, ReifierAccum> = BTreeMap::new();
    for quad in &quads {
        let key = display(&quad.subject);
        let acc = by_subject.entry(key).or_insert_with(|| ReifierAccum {
            subject: quad.subject.clone(),
            reified: None,
            annotations: Vec::new(),
        });
        match quad.predicate.as_str() {
            RDF_REIFIES => match &quad.object {
                RdfTerm::Triple(triple) => set_once_or_error(
                    &mut acc.reified,
                    triple.clone(),
                    &quad.subject,
                    "rdf:reifies",
                )?,
                other => {
                    return Err(RdfDiagnostic::error(
                        "statements-reifies-non-triple",
                        format!(
                            "rdf:reifies object must be a triple term, got {:?}",
                            other.kind()
                        ),
                    ));
                }
            },
            RDF_TYPE => {} // filtered (NOT IN rdf:type)
            other => acc
                .annotations
                .push((other.to_owned(), quad.object.clone())),
        }
    }

    let mut out = String::new();
    for acc in by_subject.values() {
        // Non-reifier subjects (bare base triples) are reconstructed from the
        // triple terms, exactly as rdf12-to-owl.rq does — so they are dropped here.
        let Some(reified) = &acc.reified else {
            continue;
        };
        let (s, p, o) = (&reified.subject, &reified.predicate, &reified.object);

        // ?s ?p ?o .
        let _ = writeln!(out, "{} <{}> {} .", emit(s), p, emit(o));

        // ?reifier a owl:Axiom ; owl:annotated* … ; ?annProp ?annVal .
        let mut properties: Vec<(String, String)> = vec![
            (RDF_TYPE.to_owned(), format!("<{OWL_AXIOM}>")),
            (OWL_ANNOTATED_SOURCE.to_owned(), emit(s)),
            (OWL_ANNOTATED_PROPERTY.to_owned(), format!("<{p}>")),
            (OWL_ANNOTATED_TARGET.to_owned(), emit(o)),
        ];
        for (predicate, object) in &acc.annotations {
            properties.push((predicate.clone(), emit(object)));
        }
        properties.sort();
        let mut line = emit(&acc.subject);
        for (index, (predicate, object)) in properties.iter().enumerate() {
            let sep = if index == 0 { " " } else { " ;\n   " };
            let _ = write!(line, "{sep}<{predicate}> {object}");
        }
        line.push_str(" .\n");
        out.push_str(&line);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const OWL: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ex: <https://example.org/vocab/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:s ex:p ex:o .
ex:ax a owl:Axiom ;
    owl:annotatedSource ex:s ;
    owl:annotatedProperty ex:p ;
    owl:annotatedTarget ex:o ;
    ex:accordingTo ex:analyst ;
    ex:confidence "0.9"^^xsd:decimal .
"#;

    /// Canonical default-graph quad set for isomorphism comparison (named nodes
    /// only, so set equality is exact).
    fn quad_set(ttl: &str) -> BTreeSet<String> {
        parse_quads(ttl)
            .expect("parse")
            .iter()
            .map(crate::emit_quad)
            .collect()
    }

    #[test]
    fn project_emits_rdf12_triple_term() {
        let rdf12 = project_owl_to_rdf12(OWL).expect("project");
        assert!(
            rdf12.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( "),
            "expected an rdf:reifies triple term:\n{rdf12}"
        );
        // The base triple is asserted alongside the reifier.
        assert!(rdf12.contains(
            "<https://example.org/vocab/s> <https://example.org/vocab/p> <https://example.org/vocab/o> ."
        ));
    }

    #[test]
    fn round_trip_is_isomorphic_to_authored_owl() {
        let rdf12 = project_owl_to_rdf12(OWL).expect("project");
        let owl_back = normalize_rdf12_to_owl(&rdf12).expect("normalize");
        assert_eq!(
            quad_set(&owl_back),
            quad_set(OWL),
            "round-trip must reproduce the authored OWL graph"
        );
    }

    #[test]
    fn projection_is_deterministic() {
        let a = project_owl_to_rdf12(OWL).expect("project a");
        let b = project_owl_to_rdf12(OWL).expect("project b");
        assert_eq!(a, b, "projection must be byte-deterministic");
    }

    #[test]
    fn dropped_annotation_breaks_the_round_trip() {
        // Same axiom, but without the confidence annotation.
        const OWL_MISSING: &str = r"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ex: <https://example.org/vocab/> .

ex:s ex:p ex:o .
ex:ax a owl:Axiom ;
    owl:annotatedSource ex:s ;
    owl:annotatedProperty ex:p ;
    owl:annotatedTarget ex:o ;
    ex:accordingTo ex:analyst .
";
        // The faithful round-trip carries the confidence, so it must NOT match a
        // graph that lost it (proves annotations flow through, not silently dropped).
        let rdf12 = project_owl_to_rdf12(OWL).expect("project");
        let owl_back = normalize_rdf12_to_owl(&rdf12).expect("normalize");
        assert_ne!(quad_set(&owl_back), quad_set(OWL_MISSING));
    }

    #[test]
    fn conflicting_annotated_source_is_rejected() {
        // Two DIFFERENT owl:annotatedSource for one axiom: a corrupt input the
        // codec must hard-fail on, never silently last-write-win.
        const OWL_CONFLICT: &str = r"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ex: <https://example.org/vocab/> .

ex:s ex:p ex:o .
ex:ax a owl:Axiom ;
    owl:annotatedSource ex:s ;
    owl:annotatedSource ex:other ;
    owl:annotatedProperty ex:p ;
    owl:annotatedTarget ex:o .
";
        let err = project_owl_to_rdf12(OWL_CONFLICT)
            .expect_err("conflicting owl:annotatedSource must hard-fail, not be silently dropped");
        assert_eq!(err.code, "statements-conflicting-structural");
    }

    #[test]
    fn duplicate_identical_source_is_idempotent() {
        // The SAME triple repeated is set membership, not a conflict — accepted.
        const OWL_DUP: &str = r"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ex: <https://example.org/vocab/> .

ex:s ex:p ex:o .
ex:ax a owl:Axiom ;
    owl:annotatedSource ex:s ;
    owl:annotatedSource ex:s ;
    owl:annotatedProperty ex:p ;
    owl:annotatedTarget ex:o .
";
        project_owl_to_rdf12(OWL_DUP)
            .expect("an identical duplicate structural triple is accepted");
    }

    #[test]
    fn conflicting_reifies_is_rejected() {
        // Two DIFFERENT rdf:reifies triple terms for one reifier subject are
        // legal RDF 1.2 (`rdf:reifies` is not functional) and parse cleanly, but
        // they CANNOT be downcast to OWL: `owl:annotatedSource`/`Property`/
        // `Target` describe exactly one statement per axiom resource, so the
        // downcast hard-fails at its own `set_once_or_error` guard rather than
        // silently last-write-winning. The refusal belongs to the OWL
        // vocabulary's functional properties, not to `rdf:reifies`.
        const RDF12_CONFLICT: &str = r"
@prefix ex: <https://example.org/vocab/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:s ex:p ex:o .
ex:ax rdf:reifies <<( ex:s ex:p ex:o )>> ;
    rdf:reifies <<( ex:s ex:p ex:other )>> .
";
        let err = normalize_rdf12_to_owl(RDF12_CONFLICT)
            .expect_err("conflicting rdf:reifies must hard-fail, not be silently dropped");
        assert_eq!(err.code, "statements-conflicting-structural");
        assert!(err.to_string().contains("conflicting"), "{err:?}");
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! Simple-literal normalization against its recursive reference, and at a hundred
    //! thousand levels on a 128 KiB thread.

    use purrdf_core::{RdfLiteral, RdfTerm, RdfTriple, TermValue};

    use super::{XSD_STRING, simplify_term};

    /// The recursive reference of [`simplify_term`].
    fn reference(term: &RdfTerm) -> RdfTerm {
        match term {
            RdfTerm::Literal(literal)
                if literal.language.is_none()
                    && literal.datatype.as_deref() == Some(XSD_STRING) =>
            {
                RdfTerm::literal(RdfLiteral::simple(literal.lexical_form.clone()))
            }
            RdfTerm::Triple(triple) => RdfTerm::triple(RdfTriple::new(
                reference(&triple.subject),
                triple.predicate.clone(),
                reference(&triple.object),
            )),
            other => other.clone(),
        }
    }

    /// A test-only owned-model twin of a generated term value whose predicates are IRIs.
    fn owned(value: &TermValue) -> RdfTerm {
        match value {
            TermValue::Triple { s, p, o } => {
                let TermValue::Iri(predicate) = &**p else {
                    unreachable!("the generator was asked for IRI predicates")
                };
                RdfTerm::triple(RdfTriple::new(owned(s), predicate.clone(), owned(o)))
            }
            TermValue::Iri(iri) => RdfTerm::iri(iri.clone()),
            TermValue::Blank { label, .. } => RdfTerm::blank_node(label.clone()),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => RdfTerm::literal(RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            }),
        }
    }

    /// The work-list normalization answers every generated term exactly as the recursive
    /// reference does, nested triple terms included.
    #[test]
    fn simplification_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::test_rng::term_value(
                &mut state,
                &mut budget,
                purrdf_core::test_rng::TermShape::IriPredicates,
            );
            nested += usize::from(budget < 7);
            let term = owned(&value);
            assert_eq!(simplify_term(&term), reference(&term), "seed {seed}");
        }
        assert!(nested > 0, "some generated term nests a triple term in one");
    }

    /// A triple term a hundred thousand levels deep, its innermost subject an
    /// `xsd:string` literal, is normalized on a thread whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_simplified_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut term = RdfTerm::literal(RdfLiteral {
                    lexical_form: "x".to_owned(),
                    datatype: Some(XSD_STRING.to_owned()),
                    language: None,
                    direction: None,
                });
                for _ in 0..LEVELS {
                    term = RdfTerm::triple(RdfTriple::new(
                        term,
                        "http://example.org/p",
                        RdfTerm::iri("http://example.org/o"),
                    ));
                }
                let mut simplified = simplify_term(&term);
                // The owned model's derived drop descends once per level, so both chains
                // are taken apart one level at a time, the innermost subject checked.
                let mut levels = 0;
                while let RdfTerm::Triple(triple) = simplified {
                    simplified = triple.subject;
                    levels += 1;
                }
                assert_eq!(levels, LEVELS);
                assert_eq!(simplified, RdfTerm::literal(RdfLiteral::simple("x")));
                while let RdfTerm::Triple(triple) = term {
                    term = triple.subject;
                }
            })
            .expect("the thread starts")
            .join()
            .expect("the normalization did not overflow the thread's stack");
    }
}
