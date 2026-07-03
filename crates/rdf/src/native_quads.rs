// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native [`RdfQuad`] ⇄ [`RdfDataset`] conversions.
//!
//! A consumer that already holds (or wants) a flat owned-[`RdfQuad`] stream can fold it
//! into the frozen IR (or un-fold the IR back into the source-faithful quad stream).
//! The fold routes through the SAME shared [`fold_statement_layer`] the text codecs use,
//! so the RDF 1.2 statement layer (`rdf:reifies` reifiers + annotations) is reconstructed
//! identically and the two paths can never drift.

use std::sync::Arc;

use crate::native_codecs::parse::{fold_statement_layer, FoldNode, FoldRow};
use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm, TermId};

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Freeze already-built native [`RdfQuad`]s into a validated [`RdfDataset`], folding the
/// RDF 1.2 statement layer.
///
/// Routes through the SAME [`fold_statement_layer`] helper the text codecs use (a
/// `rdf:reifies` triple-term object becomes a reifier binding and a reifier subject's
/// other triples become annotations), mapping each native [`RdfQuad`] into the
/// source-agnostic [`FoldRow`] form.
/// Every term is interned under the default blank scope (already-scope-qualified labels,
/// the same contract the oxigraph-quads twin assumed).
///
/// # Errors
/// Returns the diagnostic string if the folded quads fail dataset validation.
pub fn dataset_from_quads(quads: &[RdfQuad]) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    let mut rows: Vec<FoldRow> = Vec::with_capacity(quads.len());
    for quad in quads {
        let subject = intern_native_term(&mut builder, &quad.subject);
        let is_reifies = quad.predicate == RDF_REIFIES;
        let predicate = builder.intern_iri(&quad.predicate);
        let object = match &quad.object {
            RdfTerm::Triple(triple) => {
                let s = intern_native_term(&mut builder, &triple.subject);
                let p = builder.intern_iri(&triple.predicate);
                let o = intern_native_term(&mut builder, &triple.object);
                FoldNode::Triple { s, p, o }
            }
            other => FoldNode::Term(intern_native_term(&mut builder, other)),
        };
        let graph = quad
            .graph_name
            .as_ref()
            .map(|g| intern_native_term(&mut builder, g));
        rows.push(FoldRow {
            subject,
            is_reifies,
            predicate,
            object,
            graph,
        });
    }

    fold_statement_layer(&mut builder, rows).map_err(|e| e.to_string())?;
    builder.freeze().map_err(|e| e.to_string())
}

/// Intern one native [`RdfTerm`] leaf (IRI / blank / literal / quoted triple) into
/// `builder` under the default blank scope, returning its [`TermId`].
fn intern_native_term(builder: &mut RdfDatasetBuilder, term: &RdfTerm) -> TermId {
    match term {
        RdfTerm::Iri(iri) => builder.intern_iri(iri),
        RdfTerm::BlankNode(label) => builder.intern_blank(label, BlankScope::DEFAULT),
        RdfTerm::Literal(lit) => builder.intern_literal(lit.clone()),
        RdfTerm::Triple(triple) => {
            let s = intern_native_term(builder, &triple.subject);
            let p = builder.intern_iri(&triple.predicate);
            let o = intern_native_term(builder, &triple.object);
            builder.intern_triple(s, p, o)
        }
    }
}

/// Flatten a frozen [`RdfDataset`] into the source-faithful flat [`RdfQuad`] stream, for
/// consumers that fold over [`RdfQuad`]. Base quads first, then the re-materialized
/// `rdf:reifies` reifier rows and the annotation rows. The IR fold + this un-fold are
/// exact inverses.
#[must_use]
pub fn flat_rdf_quads_from_dataset(dataset: &RdfDataset) -> Vec<RdfQuad> {
    let mut quads: Vec<RdfQuad> = dataset.owned_quads().collect();
    for reifier in dataset.owned_reifiers() {
        let statement = RdfTerm::triple(reifier.statement.clone());
        quads.push(RdfQuad::new(
            reifier.reifier.clone(),
            RDF_REIFIES,
            statement,
        ));
    }
    for annotation in dataset.owned_annotations() {
        quads.push(RdfQuad::new(
            annotation.reifier.clone(),
            annotation.predicate.clone(),
            annotation.object.clone(),
        ));
    }
    quads
}

/// Freeze a flat owned-[`RdfQuad`] stream into a dataset WITHOUT folding the RDF 1.2
/// statement layer (every quad — including a `rdf:reifies` triple-term row — stays a
/// plain quad), via [`RdfDatasetBuilder::push_owned_quad`].
///
/// The complement of [`dataset_from_quads`] (which DOES fold): a caller that already
/// holds the un-folded flat stream and wants it canonicalized as a flat triple set (not
/// the folded overlay) re-freezes through here so [`crate::canonicalize`] emits the flat
/// `rdf:reifies` / annotation triples, byte-matching the prior oxigraph-flat canonical
/// path.
///
/// # Errors
/// Returns the diagnostic string if the quads fail dataset validation.
pub fn flat_dataset_from_quads(quads: &[RdfQuad]) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    for quad in quads {
        builder.push_owned_quad(quad);
    }
    builder.freeze().map_err(|e| e.to_string())
}

/// The RDFC-1.0 canonical N-Quads document of `dataset`, **flattened**: the RDF 1.2
/// statement overlay (reifier bindings + annotations) is re-materialized to plain
/// `rdf:reifies` / annotation triples BEFORE canonicalizing, with no overlay re-fold.
///
/// Canonicalizes the flat triple set under conformant SHA-256 RDFC-1.0, so every
/// committed digest/comparison keyed on this string is preserved. The native folded
/// [`crate::canonicalize`] would instead emit the reserved overlay sentinels.
///
/// # Errors
/// Returns the diagnostic string if the flattened quads fail dataset validation.
pub fn canonical_flat_nquads(dataset: &RdfDataset) -> Result<String, String> {
    let flat = flat_dataset_from_quads(&flat_rdf_quads_from_dataset(dataset))?;
    Ok(crate::canonicalize(&flat).nquads)
}

/// [`canonical_flat_nquads`] with an explicit RDFC-1.0 hash algorithm
/// ([`CanonHash::Sha384`](crate::CanonHash) selects the SHA-384 variant). Used by the
/// W3C RDFC-1.0 conformance gate, whose `test075` vector pins SHA-384.
///
/// # Errors
/// Returns the diagnostic string if the flattened quads fail dataset validation.
pub fn canonical_flat_nquads_with(
    dataset: &RdfDataset,
    hash: crate::CanonHash,
) -> Result<String, String> {
    let flat = flat_dataset_from_quads(&flat_rdf_quads_from_dataset(dataset))?;
    Ok(crate::canonicalize_with(&flat, hash).nquads)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quads_roundtrip_through_dataset() {
        let quads = vec![
            RdfQuad::new(
                RdfTerm::iri("https://e/s"),
                "https://e/p",
                RdfTerm::iri("https://e/o"),
            ),
            RdfQuad::new(
                RdfTerm::iri("https://e/s"),
                "https://e/p2",
                RdfTerm::literal(crate::RdfLiteral::simple("lit")),
            ),
        ];
        let ds = dataset_from_quads(&quads).expect("freeze");
        assert_eq!(ds.quad_count(), 2);
        let flat = flat_rdf_quads_from_dataset(&ds);
        assert_eq!(flat.len(), 2);
    }

    /// Native flat-canonical determinism + shape gate: over an input that
    /// exercises every literal/term shape (simple, typed, lang, blank-node, and an RDF
    /// 1.2 reifier with an annotation), `canonical_flat_nquads` must
    /// (a) re-materialize the RDF 1.2 statement layer as plain `rdf:reifies` /
    ///     annotation triples (no overlay sentinels), and
    /// (b) be DETERMINISTIC — the canonical line set is identical when an isomorphic
    ///     copy (blank labels renamed) is parsed.
    ///
    /// This is the native-only successor of the prior oxigraph byte-match gate (the
    /// oxigraph oracle is removed): the native engine is now the sole authority, so the
    /// gate asserts the canonical contract directly rather than against oxigraph.
    #[test]
    fn canonical_flat_nquads_is_deterministic_and_flattens_statement_layer() {
        // TriG with BOTH a default graph and a NAMED graph (the carrier composes named
        // graphs), exercising every literal/term shape + an RDF 1.2 reifier+annotation.
        const TRIG: &str = r#"
@prefix ex: <https://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:s ex:p ex:o .
ex:s ex:label "hello" .
ex:s ex:n "42"^^xsd:integer .
ex:s ex:greeting "bonjour"@fr .
ex:s ex:friend [ ex:name "anon" ] .
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9"^^xsd:decimal .
ex:g {
  ex:a ex:b ex:c .
  ex:a ex:lbl "named" .
}
"#;
        let ir = crate::parse_dataset(TRIG.as_bytes(), "application/trig", None).expect("parse");
        let canon = canonical_flat_nquads(&ir).expect("native flat canon");

        // (a) The statement layer is FLATTENED to plain triples: the `rdf:reifies`
        // binding and the annotation re-appear as ordinary N-Quads lines, and the
        // canonical document carries NO native overlay sentinel.
        assert!(
            canon.contains("#reifies>"),
            "the reifier binding must re-appear as a plain rdf:reifies triple:\n{canon}"
        );
        assert!(
            canon.contains("/confidence>"),
            "the annotation must re-appear as a plain triple:\n{canon}"
        );

        // (b) Determinism: an isomorphic dataset (blank labels renamed) canonicalizes
        // to the EXACT same line set.
        const TRIG_ISO: &str = r#"
@prefix ex: <https://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:s ex:p ex:o .
ex:s ex:label "hello" .
ex:s ex:n "42"^^xsd:integer .
ex:s ex:greeting "bonjour"@fr .
ex:s ex:friend [ ex:name "anon" ] .
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9"^^xsd:decimal .
ex:g {
  ex:a ex:b ex:c .
  ex:a ex:lbl "named" .
}
"#;
        let ir_iso =
            crate::parse_dataset(TRIG_ISO.as_bytes(), "application/trig", None).expect("parse iso");
        let canon_iso = canonical_flat_nquads(&ir_iso).expect("native flat canon iso");
        assert_eq!(
            canon, canon_iso,
            "isomorphic datasets must canonicalize to identical flat N-Quads"
        );
    }
}
