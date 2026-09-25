// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parse identity of a shapes graph: the caller-supplied inputs that decided
//! what the parsed [`Shapes`](crate::shapes::Shapes) actually mean, and the
//! obtaining identity of a preparation: which route produced the
//! [`PreparedShapes`](crate::engine::PreparedShapes) a report came out of.
//!
//! The two are deliberately separate facts recorded by separate producers.
//! [`ParseProvenance`] answers *what did this shapes graph MEAN?* and travels
//! inside a prepared product, because changing any of it changes the shapes.
//! [`ValidatorProvenance`] answers *where did this preparation COME FROM?* and
//! travels nowhere: it is a property of one restore in one process, not of the
//! declarative model, so it is never encoded into a product (see
//! [`ValidatorProvenance`] for why putting it there would be wrong as well as
//! expensive).
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
//! the document wrote them in: the text entry point hands on the Turtle codec's own
//! record ([`crate::text_ingest::TurtleDocument::prefixes`]), which resolves
//! last-writer-wins duplicates and therefore emits prefix-sorted pairs, while a caller
//! entering at
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

use std::fmt;

use purrdf_core::artifact::Identity;

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

/// Where a [`PreparedShapes`](crate::engine::PreparedShapes) came from: parsed in
/// this process, or restored from a named prepared product.
///
/// # Why an authenticated artifact needs this
///
/// A prepared product answers *may this process execute these bytes?* — its stage
/// id, profile and input binding are checked before any of it reaches a validator.
/// It does not answer the question a consumer has AFTERWARDS, looking at a report:
/// *which artifact produced this?* A deployment that restores a product from a
/// cache directory, a CI job that fetches one from a registry and a test that packs
/// one inline all hand the engine the same `PreparedShapes` type, and the report
/// they produce is identical prose either way. Without an accessor the only way to
/// attribute a verdict to an artifact is for the caller to remember which file it
/// opened — which is precisely the "an identity supplied alongside a value is a
/// claim, not a fact" forgery [`ParseProvenance`] exists to prevent, arriving one
/// layer up.
///
/// # Total by construction, not by convention
///
/// There is no `Unknown` arm and no `Option`. Every expression in this crate that
/// builds a `PreparedShapes` states its provenance at the construction site:
/// [`PreparedShapes::new`](crate::engine::PreparedShapes::new) — the only public
/// constructor, and the one every parse-side entry point funnels through — records
/// [`Parsed`](Self::Parsed), and the codec's two admission seams record
/// [`Restored`](Self::Restored) with the identity the product declares. An
/// "unknown" arm would mean "a construction path forgot", which is a defect to fix
/// rather than a value to report, and offering somewhere to put it is how the
/// forgetting becomes permanent.
///
/// # Not part of the product
///
/// A prepared product carries the declarative model and the inputs that decided
/// what it means. It does NOT carry this, and must not: the route by which a
/// preparation was obtained is a fact about one restore in one process, so encoding
/// it would put a value in the artifact that is false for every reader except the
/// one that wrote it. It would also change what the model census digests, moving the
/// preparation stage id — a model-meaning digest — for a change that alters no
/// model meaning at all, invalidating every product ever written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorProvenance {
    /// The shape tree was analyzed directly, from a [`Shapes`](crate::shapes::Shapes)
    /// this process holds.
    ///
    /// Carries nothing, deliberately. The facts about the parse behind those shapes
    /// are already reachable — and reachable in exactly one spelling — through
    /// [`PreparedShapes::shapes`](crate::engine::PreparedShapes::shapes) and
    /// [`Shapes::provenance`](crate::shapes::Shapes::provenance). Copying them here
    /// would create a second copy of one fact for the two to drift apart on, which
    /// is the failure [`ParseProvenance`]'s module docs describe and not a
    /// convenience worth paying for.
    Parsed,
    /// The preparation was restored from a prepared product.
    Restored {
        /// The input binding the product declares — the ordered, labelled
        /// components and the SHA-256 digest over them. The digest is the value
        /// that NAMES the artifact: it is what
        /// `purrdf shacl explain` prints on its `identity-digest` line and what
        /// the bound restores require, so an attributed report and the artifact it
        /// came from are compared on one spelling rather than two.
        identity: Identity,
        /// Which of the codec's two restore seams produced it — see
        /// [`ProductRestore`], because the identity above means something
        /// materially different on each.
        restore: ProductRestore,
    },
}

impl ValidatorProvenance {
    /// The product identity this preparation was restored from, or `None` when it
    /// was parsed in this process.
    ///
    /// The `Option` is a statement about the WORLD, not about this crate's
    /// bookkeeping: a preparation that was never restored from an artifact has no
    /// artifact to name, and inventing an empty identity for it would be minting a
    /// fact. That is the opposite of an "unknown" arm, which would mean a restore
    /// happened and nothing recorded which one.
    #[must_use]
    pub fn product_identity(&self) -> Option<&Identity> {
        match self {
            Self::Parsed => None,
            Self::Restored { identity, .. } => Some(identity),
        }
    }
}

/// Which of the prepared-product codec's two restore seams produced a preparation.
///
/// The distinction is not bookkeeping: it decides what the identity beside it has
/// been PROVEN to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductRestore {
    /// The memo was admitted: the product's profile, preparation stage id and
    /// complete input binding were checked against the executing environment
    /// before the preparation existed. The identity beside this arm is therefore a
    /// verified statement about this process as well as a name for the artifact.
    Admitted,
    /// The preparation was re-derived from the shapes dataset the product carries,
    /// on the forward-compatibility path a reader takes when it meets a preparation
    /// stage it does not know.
    ///
    /// The identity beside this arm still names the artifact — it is decoded from
    /// the product's own identity region, which the envelope's digests
    /// authenticate — but it was deliberately NOT checked against this environment,
    /// because every component a rebuild could compare against is a claim made by a
    /// build whose model is not this one's, and checking them would refuse exactly
    /// the products the rebuild path exists to rescue. What binds a rebuilt
    /// preparation is the derivation from that authenticated dataset.
    Rebuilt,
}

impl fmt::Display for ValidatorProvenance {
    /// The ONE rendering of a provenance, shared by every host that shows it.
    ///
    /// A single whitespace-separated token, or that token and the artifact's
    /// identity digest as 64 lowercase hexadecimal digits — the same spelling
    /// `purrdf shacl explain` prints on its `identity-digest` line and
    /// `parse_identity_digest` reads back, so a value scraped off a validation
    /// receipt can be handed straight to a bound restore without editing.
    ///
    /// `parsed`, `restored-admitted <digest>` or `restored-rebuilt <digest>`. The
    /// two restore tokens are distinct rather than one `restored` because the
    /// difference between "checked against this environment" and "re-derived
    /// without that check" is the whole content of [`ProductRestore`], and a
    /// rendering that flattened it would report the stronger claim for both.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parsed => f.write_str("parsed"),
            Self::Restored { identity, restore } => {
                f.write_str(match restore {
                    ProductRestore::Admitted => "restored-admitted ",
                    ProductRestore::Rebuilt => "restored-rebuilt ",
                })?;
                for byte in identity.digest() {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }
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
            crate::text_ingest::parse_turtle_document(SHAPES_TTL, None)
                .expect("fixture parses")
                .prefixes
                .as_slice(),
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
            &crate::text_ingest::parse_turtle_document(SHAPES_TTL, None)
                .expect("fixture parses")
                .prefixes,
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
