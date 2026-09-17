// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The parse identity of a shapes graph: the caller-supplied inputs that decided
//! what the parsed [`Shapes`](crate::shapes::Shapes) actually mean.
//!
//! # Why the parse inputs are retained rather than re-supplied
//!
//! Parsing a shapes graph is not a pure function of its RDF. The base resolves
//! every relative IRI reference in the source, the document's `@prefix` map is the
//! fallback `PREFIX` header baked into each SHACL-AF query body, the box-role
//! vocabulary decides whether role annotations are collected at all, and the
//! shapes-graph IRI is the name under which SHACL-SPARQL sees the shapes. Change
//! any one of them and the same bytes parse into different shapes. Until now those
//! four values were consumed by the parser and dropped, so a `Shapes` in hand could
//! not answer the question "what was I parsed from?" — only the caller who happened
//! to still be holding the arguments could, and only by remembering.
//!
//! The rejected alternative was to have the consumer that needs the identity take
//! it as parameters. It compiles, it is less code, and it is precisely the forgery
//! this type exists to prevent: an identity supplied alongside a value is a
//! *claim* about that value, not a fact about it, and nothing checks the two agree.
//! A caller that serialized a `Shapes` while passing the prefix map of a different
//! document would produce an artifact that is internally consistent, verifies, and
//! describes shapes nobody parsed.
//!
//! The failure this prevents is that silent mismatch surviving a round trip. With
//! the inputs recorded by the parser itself, re-deriving a shapes graph from its
//! retained dataset plus its retained provenance reproduces the original byte for
//! byte — including the `PREFIX` headers already baked into its query text — so a
//! divergence is a detectable difference rather than an invisible one.
//!
//! # Order is preserved, not normalized
//!
//! [`ParseProvenance::doc_prefixes`] hands back the pairs in exactly the order the
//! parser received them, whatever that order is. It is NOT necessarily the order
//! the document wrote them in: the text entry point folds the document scan through
//! [`crate::text_ingest::extract_prefixes`], which resolves last-writer-wins
//! duplicates and therefore emits prefix-sorted pairs, while a caller entering at
//! [`crate::shapes::from_dataset_with_prefixes`] supplies whatever order it built.
//!
//! Re-sorting here would be the wrong layer either way. The list the parser
//! consumed is the fact — it is the list that produced the `PREFIX` headers baked
//! into the query text — and any canonical ordering a serializer wants is a
//! property of that serializer's encoding, applied at encode time where it can be
//! stated and tested. Normalizing at capture would destroy the observation and
//! leave nothing able to reproduce it.
//!
//! Nothing here touches the filesystem, a clock, a thread, or randomness; it holds
//! caller-supplied strings and stays `wasm32-unknown-unknown` compatible.

use crate::model::BoxRoleVocab;

/// The caller-supplied inputs a shapes graph was parsed with.
///
/// Captured by the parser at the moment it used them, so the values are what the
/// parse actually saw rather than what a later caller believes it passed. Every
/// field is optional-or-empty in the honest sense: an entry point that genuinely
/// had no value records the absence instead of a fabricated default, because
/// PurRDF mints no vocabulary IRIs and an invented base would resolve relative
/// references nobody wrote.
///
/// Read it through [`Shapes::provenance`](crate::shapes::Shapes::provenance).
#[derive(Debug, Clone, Default)]
pub struct ParseProvenance {
    /// The RFC-3986 §5.1.2 base the source document's relative IRI references
    /// were resolved against, when the caller supplied one.
    base: Option<String>,
    /// The document `@prefix` fallback map (prefix → namespace) the parse used,
    /// in the order the parser received it.
    doc_prefixes: Vec<(String, String)>,
    /// The caller-supplied box-role vocabulary in force for the parse; `None`
    /// means the box-role feature was inactive.
    box_role_vocab: Option<BoxRoleVocab>,
    /// The named-graph IRI under which the shapes dataset is exposed to
    /// SHACL-SPARQL queries, when the caller named one.
    shapes_graph: Option<String>,
}

impl ParseProvenance {
    /// Record the inputs of one parse.
    ///
    /// `pub(crate)` on purpose: the point of this type is that only the parser
    /// that consumed the values can state them. A public constructor would hand
    /// back the ability to assert an identity for shapes that were never parsed
    /// that way, which is the whole failure mode.
    pub(crate) fn new(
        base: Option<String>,
        doc_prefixes: Vec<(String, String)>,
        box_role_vocab: Option<BoxRoleVocab>,
        shapes_graph: Option<String>,
    ) -> Self {
        Self {
            base,
            doc_prefixes,
            box_role_vocab,
            shapes_graph,
        }
    }

    /// The base the source document's relative IRI references were resolved
    /// against, or `None` when the caller supplied none (an in-document `@base`
    /// may still have established one, and with neither a relative reference is a
    /// hard parse error).
    #[must_use]
    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }

    /// The document `@prefix` fallback map the parse used, in the parser's own
    /// order (see the [module docs](self) — this is not re-sorted here).
    ///
    /// Feed these straight back into
    /// [`from_dataset_with_prefixes`](crate::shapes::from_dataset_with_prefixes)
    /// together with [`Shapes::dataset`](crate::shapes::Shapes::dataset) to
    /// re-derive the identical shapes, query text included.
    #[must_use]
    pub fn doc_prefixes(&self) -> &[(String, String)] {
        &self.doc_prefixes
    }

    /// The box-role vocabulary the parse ran under; `None` = feature inactive.
    #[must_use]
    pub fn box_role_vocab(&self) -> Option<&BoxRoleVocab> {
        self.box_role_vocab.as_ref()
    }

    /// The named-graph IRI under which the shapes dataset is exposed to
    /// SHACL-SPARQL queries, when one was named.
    #[must_use]
    pub fn shapes_graph(&self) -> Option<&str> {
        self.shapes_graph.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::parse_shapes;
    use crate::model::BoxRoleVocab;
    use crate::shapes::{
        Constraint, Shapes, from_dataset_with_config_and_graph, from_dataset_with_prefixes,
    };

    /// A shapes document whose SHACL-SPARQL constraint relies on the document's
    /// own `@prefix` declarations — the pySHACL-compatible fallback that makes the
    /// prefix map part of the parse result rather than a formatting detail.
    const SHAPES_TTL: &str = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:sparql [
                a sh:SPARQLConstraint ;
                sh:message "every person needs a name" ;
                sh:select "SELECT $this WHERE { $this ex:name ?name . FILTER (?name = 'x'^^xsd:string) }" ;
            ] .
    "#;

    /// Every `sh:select` text retained on a parsed shapes graph, in shape and
    /// constraint order — the surface the document prefix map is baked into.
    fn select_texts(shapes: &Shapes) -> Vec<String> {
        shapes
            .node_shapes
            .iter()
            .flat_map(|shape| &shape.constraints)
            .filter_map(|constraint| match constraint {
                Constraint::Sparql { select, .. } => Some(select.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn provenance_round_trips_prefixes_and_base() {
        let shapes = parse_shapes(SHAPES_TTL, Some("https://example.org/base")).expect("parse");
        let provenance = shapes.provenance();

        assert_eq!(provenance.base(), Some("https://example.org/base"));
        assert!(
            provenance.box_role_vocab().is_none(),
            "no box-role vocabulary was supplied, so none may be reported"
        );
        assert_eq!(provenance.shapes_graph(), None);

        let declared: Vec<(&str, &str)> = provenance
            .doc_prefixes()
            .iter()
            .map(|(prefix, namespace)| (prefix.as_str(), namespace.as_str()))
            .collect();
        assert_eq!(
            declared,
            vec![
                ("ex", "http://example.org/ns#"),
                ("sh", "http://www.w3.org/ns/shacl#"),
                ("xsd", "http://www.w3.org/2001/XMLSchema#"),
            ],
            "every @prefix the document declared is retained"
        );
        // …and retained VERBATIM: element for element, in the producer's own
        // order. The text entry point folds duplicates through a map and so hands
        // over prefix-sorted pairs; the point of this assertion is that the
        // provenance is that exact list and not a re-sorted copy of it, because a
        // re-sorted copy is a different list that nothing could invert.
        assert_eq!(
            provenance.doc_prefixes(),
            crate::text_ingest::extract_prefixes(SHAPES_TTL).as_slice(),
            "the retained map is the parser's own list, unreordered"
        );
    }

    #[test]
    fn provenance_reconstructs_identical_shapes() {
        let shapes = parse_shapes(SHAPES_TTL, Some("https://example.org/base")).expect("parse");
        let original = select_texts(&shapes);
        assert!(
            !original.is_empty(),
            "the fixture must retain at least one sh:select, or the property is vacuous"
        );
        assert!(
            original
                .iter()
                .all(|select| select.starts_with("PREFIX ex: <http://example.org/ns#>")),
            "the document prefix map must actually be baked into the query text, or this test \
             would pass for shapes that never consulted it: {original:?}"
        );

        let rederived =
            from_dataset_with_prefixes(shapes.dataset(), shapes.provenance().doc_prefixes())
                .expect("re-parse from the retained dataset and provenance");

        assert_eq!(
            select_texts(&rederived),
            original,
            "the retained dataset plus the retained prefix map must reproduce the query text \
             byte for byte, or a product built from them would execute different queries"
        );
    }

    /// The two configuration inputs the dataset entry points carry — the box-role
    /// vocabulary and the shapes-graph IRI — reach the provenance as given. Both
    /// change what validation does, so a product that failed to record them would
    /// restore shapes that validate differently than the ones prepared.
    #[test]
    fn provenance_records_the_dataset_entry_point_configuration() {
        let dataset =
            crate::text_ingest::parse_turtle_to_dataset(SHAPES_TTL, None).expect("Turtle parses");
        let vocab = BoxRoleVocab::for_namespace("http://example.org/roles#");
        let shapes = from_dataset_with_config_and_graph(
            &dataset,
            &crate::text_ingest::extract_prefixes(SHAPES_TTL),
            Some(vocab.clone()),
            Some("http://example.org/shapes-graph".to_owned()),
        )
        .expect("parse");

        assert_eq!(shapes.provenance().box_role_vocab(), Some(&vocab));
        assert_eq!(
            shapes.provenance().shapes_graph(),
            Some("http://example.org/shapes-graph")
        );
        assert_eq!(
            shapes.provenance().base(),
            None,
            "a dataset entered here is already resolved and no base was supplied, so none may \
             be invented"
        );
    }

    #[test]
    fn shapes_default_has_empty_provenance() {
        let shapes = Shapes::default();
        let provenance = shapes.provenance();

        assert_eq!(provenance.base(), None);
        assert_eq!(provenance.doc_prefixes(), []);
        assert!(provenance.box_role_vocab().is_none());
        assert_eq!(provenance.shapes_graph(), None);
    }
}
