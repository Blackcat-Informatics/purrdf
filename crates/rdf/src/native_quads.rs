// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native [`RdfQuad`] ⇄ [`RdfDataset`] conversions.
//!
//! A consumer that already holds (or wants) a flat owned-[`RdfQuad`] stream can fold it
//! into the frozen IR (or un-fold the IR back into the source-faithful quad stream).
//! The fold routes through the SAME shared `fold_statement_layer` the text codecs use,
//! so the RDF 1.2 statement layer (`rdf:reifies` reifiers + annotations) is reconstructed
//! identically and the two paths can never drift.

use std::sync::Arc;

use crate::native_codecs::parse::{fold_statement_layer, FoldNode, FoldRow};
use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm};

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Freeze several independently-parsed native [`RdfQuad`] streams into ONE validated
/// [`RdfDataset`], folding the RDF 1.2 statement layer, with blank nodes
/// **standardized apart** per source.
///
/// This is the sanctioned multi-source loader: source at index `i` interns its blank
/// nodes under blank scope `i`, so two sources that each minted `_:b0` stay DISTINCT
/// after the merge instead of silently collapsing into one node. Source 0 uses
/// [`BlankScope::DEFAULT`] (which is `BlankScope(0)`), so a single-source load renders
/// bytes identically to the pre-standardize-apart behavior.
///
/// Routes through the SAME `fold_statement_layer` helper the text codecs use (a
/// `rdf:reifies` triple-term object becomes a reifier binding and a reifier subject's
/// other triples become annotations), mapping each native [`RdfQuad`] into the
/// source-agnostic [`FoldRow`] form. Blanks nested inside quoted triples are scoped too
/// (the builder's scoped interner recurses).
///
/// # Errors
/// Returns the diagnostic string if the folded quads fail dataset validation.
pub fn dataset_from_quad_sources(sources: &[&[RdfQuad]]) -> Result<Arc<RdfDataset>, String> {
    let total: usize = sources.iter().map(|source| source.len()).sum();
    let mut builder = RdfDatasetBuilder::new();
    let mut rows: Vec<FoldRow> = Vec::with_capacity(total);
    for (index, quads) in sources.iter().enumerate() {
        let scope = BlankScope(index as u32);
        for quad in *quads {
            let subject = builder.intern_owned_term_scoped(&quad.subject, scope);
            let is_reifies = quad.predicate == RDF_REIFIES;
            let predicate = builder.intern_iri(&quad.predicate);
            let object = match &quad.object {
                RdfTerm::Triple(triple) => {
                    let s = builder.intern_owned_term_scoped(&triple.subject, scope);
                    let p = builder.intern_iri(&triple.predicate);
                    let o = builder.intern_owned_term_scoped(&triple.object, scope);
                    FoldNode::Triple { s, p, o }
                }
                other => FoldNode::Term(builder.intern_owned_term_scoped(other, scope)),
            };
            let graph = quad
                .graph_name
                .as_ref()
                .map(|g| builder.intern_owned_term_scoped(g, scope));
            rows.push(FoldRow {
                subject,
                is_reifies,
                predicate,
                object,
                graph,
            });
        }
    }

    fold_statement_layer(&mut builder, rows).map_err(|e| e.to_string())?;
    builder.freeze().map_err(|e| e.to_string())
}

/// Freeze already-built native [`RdfQuad`]s into a validated [`RdfDataset`], folding the
/// RDF 1.2 statement layer.
///
/// The single-source convenience over [`dataset_from_quad_sources`] (the sanctioned
/// multi-source loader). Because there is exactly one source, every blank node shares
/// [`BlankScope::DEFAULT`] — correct, since a single parse already minted unique labels.
/// A caller MERGING independently-parsed sources must use [`dataset_from_quad_sources`]
/// so their blanks standardize apart rather than collapse.
///
/// # Errors
/// Returns the diagnostic string if the folded quads fail dataset validation.
pub fn dataset_from_quads(quads: &[RdfQuad]) -> Result<Arc<RdfDataset>, String> {
    dataset_from_quad_sources(&[quads])
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

/// Freeze several independently-parsed flat owned-[`RdfQuad`] streams into ONE dataset
/// WITHOUT folding the RDF 1.2 statement layer (every quad — including a `rdf:reifies`
/// triple-term row — stays a plain quad), with blank nodes **standardized apart** per
/// source.
///
/// The un-folded twin of [`dataset_from_quad_sources`] and the sanctioned multi-source
/// flat loader: source at index `i` interns its blanks under blank scope `i` via
/// [`RdfDatasetBuilder::push_owned_quad_scoped`], so two sources that each minted `_:b0`
/// stay DISTINCT after the merge. Source 0 uses [`BlankScope::DEFAULT`] (`BlankScope(0)`),
/// so a single-source load stays byte-identical to the prior flat canonical path.
///
/// # Errors
/// Returns the diagnostic string if the quads fail dataset validation.
pub fn flat_dataset_from_quad_sources(sources: &[&[RdfQuad]]) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    for (index, quads) in sources.iter().enumerate() {
        let scope = BlankScope(index as u32);
        for quad in *quads {
            builder.push_owned_quad_scoped(quad, scope);
        }
    }
    builder.freeze().map_err(|e| e.to_string())
}

/// Freeze a flat owned-[`RdfQuad`] stream into a dataset WITHOUT folding the RDF 1.2
/// statement layer (every quad — including a `rdf:reifies` triple-term row — stays a
/// plain quad).
///
/// The single-source convenience over [`flat_dataset_from_quad_sources`] (the sanctioned
/// multi-source flat loader). Because there is exactly one source, every blank node
/// shares [`BlankScope::DEFAULT`] — correct, since a single parse already minted unique
/// labels. A caller MERGING independently-parsed sources must use
/// [`flat_dataset_from_quad_sources`] so their blanks standardize apart rather than
/// collapse.
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
    flat_dataset_from_quad_sources(&[quads])
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

    const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

    /// Parse a Turtle string and flatten it to the source-faithful owned quad stream.
    fn turtle_quads(ttl: &str) -> Vec<RdfQuad> {
        let ds = crate::parse_dataset(ttl.as_bytes(), "text/turtle", None).expect("parse turtle");
        flat_rdf_quads_from_dataset(&ds)
    }

    /// The set of DISTINCT blank-node subjects that are `<subj> a owl:Restriction`.
    fn blank_restriction_subjects(quads: &[RdfQuad]) -> std::collections::BTreeSet<String> {
        quads
            .iter()
            .filter_map(
                |quad| match (&quad.subject, quad.predicate.as_str(), &quad.object) {
                    (RdfTerm::BlankNode(label), RDF_TYPE, RdfTerm::Iri(iri))
                        if iri == OWL_RESTRICTION =>
                    {
                        Some(label.clone())
                    }
                    _ => None,
                },
            )
            .collect()
    }

    // Two Turtle sources, each carrying exactly one anonymous `owl:Restriction` the
    // parser labels `_:b0`. Merged, they must stay two distinct nodes.
    const RESTRICTION_A: &str = concat!(
        "@prefix ex: <https://example.org/> .\n",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        "ex:A rdfs:subClassOf [ a owl:Restriction ; ",
        "owl:onProperty ex:p ; owl:someValuesFrom ex:C ] .\n",
    );
    const RESTRICTION_B: &str = concat!(
        "@prefix ex: <https://example.org/> .\n",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        "ex:B rdfs:subClassOf [ a owl:Restriction ; ",
        "owl:onProperty ex:p ; owl:someValuesFrom ex:D ] .\n",
    );

    /// Core fix: merging two independently-parsed sources via
    /// [`flat_dataset_from_quad_sources`] standardizes their blanks apart, so two
    /// `_:b0`-labeled restrictions stay DISTINCT — whereas the naive single-source
    /// concatenation collapses them into one, the bug this fix removes.
    #[test]
    fn multi_source_flat_freeze_standardizes_blanks_apart() {
        let a = turtle_quads(RESTRICTION_A);
        let b = turtle_quads(RESTRICTION_B);

        // Standardize-apart merge: two distinct blank restriction subjects.
        let merged = flat_dataset_from_quad_sources(&[&a, &b]).expect("multi-source flat freeze");
        let merged_flat = flat_rdf_quads_from_dataset(&merged);
        assert_eq!(
            blank_restriction_subjects(&merged_flat).len(),
            2,
            "two independently-minted _:b0 restrictions must remain distinct after a \
             standardize-apart merge"
        );

        // Naive single-source concatenation collapses both `_:b0` into one node —
        // locked in as the contrast the fix removes.
        let concatenated = [a, b].concat();
        let collapsed = flat_dataset_from_quads(&concatenated).expect("single-source flat freeze");
        let collapsed_flat = flat_rdf_quads_from_dataset(&collapsed);
        assert_eq!(
            blank_restriction_subjects(&collapsed_flat).len(),
            1,
            "concatenating two sources as ONE parse collapses the shared _:b0 label"
        );
    }

    /// The folding twin: [`dataset_from_quad_sources`] threads a per-source scope through
    /// the RDF 1.2 statement-layer fold, so the same two restrictions stay distinct and
    /// the fold still succeeds.
    #[test]
    fn multi_source_folded_freeze_standardizes_blanks_apart() {
        let a = turtle_quads(RESTRICTION_A);
        let b = turtle_quads(RESTRICTION_B);

        let merged = dataset_from_quad_sources(&[&a, &b]).expect("multi-source folded freeze");
        let merged_flat = flat_rdf_quads_from_dataset(&merged);
        assert_eq!(
            blank_restriction_subjects(&merged_flat).len(),
            2,
            "the folding path must also standardize blanks apart per source"
        );
    }

    /// Canonical byte-lock (scope 0): the single-source `flat_dataset_from_quads` and the
    /// one-element `flat_dataset_from_quad_sources(&[&q])` it delegates to produce
    /// datasets with byte-identical canonical flat N-Quads — confirming source 0's
    /// `BlankScope::DEFAULT` renders bare labels exactly as before the refactor.
    #[test]
    fn single_source_canonical_bytes_unchanged_across_delegate() {
        const TRIG: &str = r#"
@prefix ex: <https://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:s ex:p ex:o .
ex:s ex:friend [ ex:name "anon" ] .
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9"^^xsd:decimal .
ex:g {
  ex:a ex:b ex:c .
}
"#;
        let ir = crate::parse_dataset(TRIG.as_bytes(), "application/trig", None).expect("parse");
        let quads = flat_rdf_quads_from_dataset(&ir);

        let via_single = flat_dataset_from_quads(&quads).expect("single-source freeze");
        let via_sources =
            flat_dataset_from_quad_sources(&[&quads]).expect("one-element sources freeze");

        let canon_single = canonical_flat_nquads(&via_single).expect("canon single");
        let canon_sources = canonical_flat_nquads(&via_sources).expect("canon sources");
        assert_eq!(
            canon_single, canon_sources,
            "the 1-element delegate must not change the single-source canonical bytes"
        );
    }
}
