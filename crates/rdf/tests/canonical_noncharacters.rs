// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! U+FFFE and U+FFFF in a literal, and DEL and C1 controls in an IRI, through
//! the canonical entry points.
//!
//! The 5.0 canonical document spells them as `UCHAR` (N-Quads 1.2 section 3: a
//! character outside the XML 1.1 `Char` production), and its digest is over
//! binary lines and is the same from every entry point. The retained 4.x
//! `purrdf-rdfc12` output (profile version 3) writes them raw, as it always did,
//! so a digest recorded over that output recomputes: the frozen vectors in
//! `tests/vectors/rdfc12_v3_noncharacters.txt` are the answers of the
//! `rust-v4.0.2` crates, and a disagreement is a defect in this path, never a
//! reason to edit a vector.

use purrdf_core::ContentDigest;
use purrdf_core::{
    CANON_CORPUS_POLICY, CanonicalizationControl, try_canonical_digest_view,
    try_canonicalize_digest_view,
};
use purrdf_rdf::rdfc12::{
    CanonHash, RDFC_V2_CORPUS_POLICY, try_canonical_sha256_view, try_canonicalize_flat_view,
    try_canonicalize_graph_view, try_canonicalize_view, try_canonicalize_with,
    try_flat_blake3_view, try_flat_digest_view, try_graph_digest_view,
};
use purrdf_rdf::{TermValue, parse_dataset};
use purrdf_testkit::vectors::{VectorFile, decode_str, encode_bytes};

/// The `rust-v4.0.2` overlay and flat documents and digests of datasets holding
/// U+FFFE, U+FFFF and their neighbours.
const RDFC12_V3_VECTORS: &str = include_str!("vectors/rdfc12_v3_noncharacters.txt");

const SOURCE: &str =
    "<http://example.org/s> <http://example.org/p> \"a\\uFFFEb\\uFFFFc\\u0085\" .\n";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

#[test]
fn the_document_escapes_the_noncharacters_and_every_digest_entry_agrees() {
    let dataset = parse_dataset(SOURCE.as_bytes(), "application/n-quads", None).expect("parses");
    let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
    let (canonical, digest) =
        try_canonicalize_digest_view(&dataset, &mut control).expect("canonicalizes");
    assert_eq!(
        canonical.nquads(),
        "<http://example.org/s> <http://example.org/p> \"a\\uFFFEb\\uFFFFc\u{85}\" .\n"
    );

    let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
    let alone = try_canonical_digest_view(&dataset, &mut control).expect("digest");
    assert_eq!(alone.as_bytes(), digest.as_bytes());

    // The spelled line parses back to the same literal.
    let reparsed =
        parse_dataset(canonical.nquads().as_bytes(), "application/n-quads", None).expect("reads");
    let literal = TermValue::typed_literal("a\u{fffe}b\u{ffff}c\u{85}", XSD_STRING);
    assert!(dataset.term_id_by_value(&literal).is_some());
    assert!(reparsed.term_id_by_value(&literal).is_some());
}

/// Each digest entry of the retained 4.x path is the hash of its own document
/// for a dataset holding the noncharacters, so no entry spells them differently.
#[test]
fn every_4x_digest_entry_equals_the_hash_of_its_document() {
    let source = format!(
        "{SOURCE}<http://example.org/s> <http://example.org/p> \"x\\uFFFEy\" <http://example.org/g> .\n"
    );
    let dataset = parse_dataset(source.as_bytes(), "application/n-quads", None).expect("parses");
    let control = || CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
    let hash = CanonHash::Sha256;

    let overlay = try_canonicalize_with(&dataset, hash, &mut control()).expect("overlay");
    assert!(overlay.nquads().contains('\u{fffe}'));
    let digest = try_canonical_sha256_view(&*dataset, hash, &mut control()).expect("digest");
    assert_eq!(digest, ContentDigest::of(overlay.nquads().as_bytes()));

    let graph = "http://example.org/g";
    let document =
        try_canonicalize_graph_view(&*dataset, graph, hash, &mut control()).expect("graph");
    let digest = try_graph_digest_view(&*dataset, graph, &mut control()).expect("graph digest");
    assert_eq!(digest, ContentDigest::of(document.nquads().as_bytes()));

    let flat = try_canonicalize_flat_view(&*dataset, hash, &mut control()).expect("flat");
    assert!(flat.nquads().contains('\u{fffe}'));
    let digest = try_flat_digest_view(&*dataset, hash, &mut control()).expect("flat digest");
    assert_eq!(digest, ContentDigest::of(flat.nquads().as_bytes()));
    let streamed = try_flat_blake3_view(&*dataset, hash, &mut control()).expect("flat blake3");
    assert_eq!(
        streamed,
        *purrdf_hash::blake3::hash(flat.nquads().as_bytes()).as_bytes()
    );
}

/// A digest the retained path recorded in 4.x recomputes, U+FFFE and U+FFFF
/// included: the overlay and flat documents and both digests equal the frozen
/// `rust-v4.0.2` answers byte for byte.
#[test]
fn the_retained_4x_path_recomputes_the_frozen_4_0_2_answers() {
    let control = || CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
    let hash = CanonHash::Sha256;
    let replayed = VectorFile::load(RDFC12_V3_VECTORS)
        .replay(1, |input| {
            let source = decode_str(input[0]).expect("an encoded N-Quads input");
            let dataset =
                parse_dataset(source.as_bytes(), "application/n-quads", None).expect("parses");
            let overlay = try_canonicalize_with(&dataset, hash, &mut control()).expect("overlay");
            let digest =
                try_canonical_sha256_view(&*dataset, hash, &mut control()).expect("digest");
            let flat = try_canonicalize_flat_view(&*dataset, hash, &mut control()).expect("flat");
            let flat_digest =
                try_flat_digest_view(&*dataset, hash, &mut control()).expect("flat digest");
            vec![
                encode_bytes(overlay.nquads().as_bytes()),
                digest.to_hex(),
                encode_bytes(flat.nquads().as_bytes()),
                flat_digest.to_hex(),
            ]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, 5);
}

/// The `rust-v4.0.2` overlay and flat documents and digests of views whose IRIs
/// hold DEL and C1 controls, and of a neighbour that needs no escape.
const RDFC12_V3_IRI_VECTORS: &str = include_str!("vectors/rdfc12_v3_iri_controls.txt");

mod raw_iri_view {
    use purrdf_core::{
        DatasetView, FallibleDatasetView, GraphMatch, NoopReservation, OwnedWorkspaceReservation,
        QuadIds, RdfDataset, RdfStoreCapabilities, TermId, TermRef, TermValue, ViewOperationStatus,
    };
    use std::convert::Infallible;
    use std::sync::Arc;

    /// A view over a frozen dataset whose IRIs may hold DEL and C1 controls, which
    /// `freeze` refuses. The source text writes each such scalar as `~HH~` (two
    /// upper-case hex digits) inside an IRI; the view serves the scalar itself.
    pub(crate) struct RawIriView {
        inner: Arc<RdfDataset>,
        /// The served IRI text of each term that held a `~HH~` placeholder, by dense index.
        served: Vec<Option<String>>,
    }

    /// `iri` with each `~HH~` replaced by the scalar U+00HH, or `None` when it holds none.
    fn serve_scalars(iri: &str) -> Option<String> {
        let mut out = String::new();
        let mut rest = iri;
        let mut changed = false;
        while let Some(at) = rest.find('~') {
            let tail = &rest[at + 1..];
            let scalar = tail
                .get(..3)
                .filter(|t| t.as_bytes()[2] == b'~')
                .and_then(|t| u32::from_str_radix(&t[..2], 16).ok())
                .and_then(char::from_u32);
            let Some(scalar) = scalar else {
                out.push_str(&rest[..=at]);
                rest = tail;
                continue;
            };
            out.push_str(&rest[..at]);
            out.push(scalar);
            rest = &tail[3..];
            changed = true;
        }
        out.push_str(rest);
        changed.then_some(out)
    }

    impl RawIriView {
        pub(crate) fn new(inner: Arc<RdfDataset>) -> Self {
            let mut served = vec![None; usize::try_from(inner.term_count()).expect("small")];
            let mut work: Vec<TermId> = Vec::new();
            for q in inner
                .quads()
                .chain(inner.reifier_quads())
                .chain(inner.annotation_quads())
            {
                work.extend([q.s, q.p, q.o]);
                work.extend(q.g);
            }
            while let Some(id) = work.pop() {
                match inner.as_ref().resolve(id) {
                    TermRef::Iri(iri) => served[id.index()] = serve_scalars(iri),
                    TermRef::Literal { datatype, .. } => work.push(datatype),
                    TermRef::Triple { s, p, o } => work.extend([s, p, o]),
                    TermRef::Blank { .. } => {}
                }
            }
            Self { inner, served }
        }
    }

    impl DatasetView for RawIriView {
        type Id = TermId;
        type ReadError = Infallible;
        type TermGuard<'a> = TermRef<'a, TermId>;
        type ProbePlan = ();

        fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.quads()
        }
        fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.reifier_quads()
        }
        fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.annotation_quads()
        }
        fn resolve(&self, id: TermId) -> Result<TermRef<'_, TermId>, Infallible> {
            Ok(match &self.served[id.index()] {
                Some(iri) => TermRef::Iri(iri),
                None => self.inner.as_ref().resolve(id),
            })
        }
        fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Infallible> {
            Ok(self.inner.as_ref().term_id_by_value(value))
        }
        fn capabilities(&self) -> RdfStoreCapabilities {
            self.inner.capabilities()
        }
        fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch) {}
        fn quads_for_pattern_with_plan(
            &self,
            (): &(),
            s: Option<TermId>,
            p: Option<TermId>,
            o: Option<TermId>,
            g: GraphMatch,
        ) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.quads_for_pattern(s, p, o, g)
        }
        fn term_count(&self) -> u64 {
            self.inner.term_count()
        }
        fn reserve_owned_workspace(
            &self,
            _: u64,
        ) -> Option<Result<OwnedWorkspaceReservation<Infallible>, Infallible>> {
            Some(Ok(Box::new(NoopReservation::<Infallible>::default())
                as OwnedWorkspaceReservation<Infallible>))
        }
    }

    impl FallibleDatasetView for RawIriView {
        type Error = Infallible;
        type Evidence = ();
        fn operation_status(&self) -> ViewOperationStatus<Infallible, ()> {
            ViewOperationStatus::Ready { evidence: () }
        }
    }
}
use raw_iri_view::RawIriView;

/// A digest the retained 4.x path recorded over an IRI holding DEL or a C1
/// control recomputes: the retained path escapes them (as `rust-v4.0.2` did, in
/// the subject, predicate, object, graph, datatype and a triple term), the
/// neighbour that needs no escape is unchanged, and the overlay and flat
/// documents and both digests equal the frozen answers byte for byte.
#[test]
fn the_retained_4x_path_escapes_del_and_c1_in_an_iri_as_4_0_2_did() {
    let control = || CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
    let hash = CanonHash::Sha256;
    let replayed = VectorFile::load(RDFC12_V3_IRI_VECTORS)
        .replay(1, |input| {
            let source = decode_str(input[0]).expect("an encoded N-Quads input");
            let dataset =
                parse_dataset(source.as_bytes(), "application/n-quads", None).expect("parses");
            let view = RawIriView::new(dataset);
            let overlay = try_canonicalize_view(&view, hash, &mut control()).expect("overlay");
            let digest = try_canonical_sha256_view(&view, hash, &mut control()).expect("digest");
            let flat = try_canonicalize_flat_view(&view, hash, &mut control()).expect("flat");
            let flat_digest =
                try_flat_digest_view(&view, hash, &mut control()).expect("flat digest");
            vec![
                encode_bytes(overlay.nquads().as_bytes()),
                digest.to_hex(),
                encode_bytes(flat.nquads().as_bytes()),
                flat_digest.to_hex(),
            ]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, 5);
}
