// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-subject subgraph extraction — the **Symmetric Concise Bounded Description**
//! (SCBD) of a resource.
//!
//! The documentation site exports each term's (and each slice's) RDF in every
//! serialization format. To do that it needs the subgraph that *describes* a subject.
//! A plain CBD (outgoing triples + forward blank closure) would under-represent the
//! very thing PurRDF exists to showcase: the **incoming** links — `skos:exactMatch`
//! targets, `rdfs:subPropertyOf`/`subClassOf` children, authority back-references. So
//! `describe` returns the **symmetric** CBD:
//!
//! 1. every triple where the subject is the **subject** (outgoing), and
//! 2. every triple where the subject is the **object** (incoming), and
//! 3. the transitive **blank-node** closure on both directions (a definition hung off
//!    a blank restriction surfaces in full), and
//! 4. the RDF-1.2 statement-layer **reifiers** whose reified triple's subject *or*
//!    object lies in the closure, together with their annotations.
//!
//! Named-node endpoints do **not** expand (that would pull in the whole graph); only
//! blank nodes do. Reification is standpoint-scoped and carries no graph dimension, so
//! reifiers are selected by reified-triple membership, never by graph.
//!
//! The extracted subgraph is a fresh, structurally valid [`RdfDataset`] that can be
//! handed straight to the `native_codecs` serializers (Turtle / N-Triples / N-Quads /
//! TriG / RDF-XML) and the JSON-LD serializer — the one serialization seam.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

use crate::{
    DatasetView, QuadIds, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, RdfTerm,
    RdfTriple, TermId, TermRef, TermValue,
};

/// Resolve a term id to the owned [`RdfTerm`] model through the [`DatasetView`] read
/// path (recursively for triple terms) — the id-agnostic twin of
/// `RdfDataset::to_owned_term`, so the extractor rebuilds a describing subgraph over
/// any backend whose id type is [`TermId`].
fn owned_term<D: DatasetView>(dataset: &D, id: D::Id) -> RdfTerm {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => RdfTerm::iri(iri),
        TermRef::Blank { label, scope } => RdfTerm::blank_node(scope.qualify_label(label)),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype_iri = match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!("literal datatype must resolve to an IRI, got {other:?}"),
            };
            RdfTerm::literal(RdfLiteral {
                lexical_form: lexical.to_owned(),
                datatype: Some(datatype_iri),
                language: language.map(str::to_owned),
                direction,
            })
        }
        TermRef::Triple { s, p, o } => {
            let subject = owned_term(dataset, s);
            let predicate = match dataset.resolve(p) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!("triple predicate must resolve to an IRI, got {other:?}"),
            };
            let object = owned_term(dataset, o);
            RdfTerm::triple(RdfTriple::new(subject, predicate, object))
        }
    }
}

/// A view id → the quads touching it as subject/object (the endpoint adjacency).
type EndpointQuads<I> = BTreeMap<I, Vec<QuadIds<I>>>;

/// A view id → a list of `(id, id)` bindings (reifier/triple or annotation `(p, o)`).
type EndpointPairs<I> = BTreeMap<I, Vec<(I, I)>>;

/// A reusable extractor: it builds the subject/object adjacency and the reifier
/// endpoint index **once**, so extracting the SCBD of many subjects (one per exported
/// term/slice) is cheap — each extraction is a bounded graph walk over the index, not
/// a full re-scan of the dataset.
///
/// Generic over the read view `D` (any [`DatasetView`]), so the SCBD walk runs over the
/// concrete [`RdfDataset`] or any other backend id width unchanged; the extracted
/// subgraph is always a fresh [`RdfDataset`].
#[derive(Debug)]
pub struct Describer<'a, D: DatasetView = RdfDataset> {
    dataset: &'a D,
    /// term id → the quads that touch it as subject or object.
    by_endpoint: EndpointQuads<D::Id>,
    /// term id → the `(reifier, triple-term)` bindings whose reified triple has this
    /// id as its subject or object.
    reifiers_by_endpoint: EndpointPairs<D::Id>,
    /// reifier id → the `(p, o)` annotation bindings hung off that reifier.
    annotations_by_reifier: EndpointPairs<D::Id>,
}

impl<'a, D: DatasetView> Describer<'a, D> {
    /// Build the adjacency indices over `dataset`.
    #[must_use]
    pub fn new(dataset: &'a D) -> Self {
        let mut by_endpoint: EndpointQuads<D::Id> = BTreeMap::new();
        for q in dataset.quads() {
            by_endpoint.entry(q.s).or_default().push(q);
            // Avoid double-listing a reflexive `s p s` quad under the same key.
            if q.o != q.s {
                by_endpoint.entry(q.o).or_default().push(q);
            }
        }

        let mut reifiers_by_endpoint: EndpointPairs<D::Id> = BTreeMap::new();
        for (reifier, triple) in dataset.reifier_quads().map(|q| (q.s, q.o)) {
            if let TermRef::Triple { s, p: _, o } = dataset.resolve(triple) {
                reifiers_by_endpoint
                    .entry(s)
                    .or_default()
                    .push((reifier, triple));
                if o != s {
                    reifiers_by_endpoint
                        .entry(o)
                        .or_default()
                        .push((reifier, triple));
                }
            }
        }

        let mut annotations_by_reifier: EndpointPairs<D::Id> = BTreeMap::new();
        for (reifier, p, o) in dataset.annotation_quads().map(|q| (q.s, q.p, q.o)) {
            annotations_by_reifier
                .entry(reifier)
                .or_default()
                .push((p, o));
        }

        Self {
            dataset,
            by_endpoint,
            reifiers_by_endpoint,
            annotations_by_reifier,
        }
    }

    /// The SCBD of the IRI `subject`, or an **empty** dataset if the dataset contains
    /// no such subject. (An absent subject is not an error — a term may legitimately
    /// carry no asserted or incoming triples.)
    ///
    /// # Errors
    /// Propagates a freeze diagnostic if the extracted subgraph is somehow invalid
    /// (it never should be, being a subset of an already-valid dataset).
    pub fn describe_iri(&self, subject: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let seed = self.dataset.term_id_by_value(&TermValue::iri(subject));
        self.describe_seeds(seed.into_iter().collect())
    }

    /// The union SCBD of several IRI subjects — the slice-scope export (every subject
    /// the slice module mints, described as one subgraph).
    ///
    /// # Errors
    /// Propagates a freeze diagnostic (see [`describe_iri`](Self::describe_iri)).
    pub fn describe_iris<'s>(
        &self,
        subjects: impl IntoIterator<Item = &'s str>,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let seeds: Vec<D::Id> = subjects
            .into_iter()
            .filter_map(|s| self.dataset.term_id_by_value(&TermValue::iri(s)))
            .collect();
        self.describe_seeds(seeds)
    }

    /// The shared walk: a single BFS from `seeds`, expanding **every** blank node it
    /// can reach — whether that blank surfaces as a quad endpoint, as a reifier id, as
    /// an endpoint of a reified triple, or as an annotation object — then re-intern the
    /// collected quads + statement layer into a fresh dataset.
    ///
    /// Folding the reifier/annotation harvest **into** the frontier loop (rather than
    /// running it once after the quad walk) is what makes the blank-node closure
    /// transitive across the statement layer: a blank reifier's own describing triples
    /// ride along because the reifier id is pushed to the frontier (its quads live in
    /// `by_endpoint`), and a blank annotation object's triples ride along because it is
    /// pushed to the frontier when the annotation is harvested. Otherwise those blanks
    /// dangle — emitted in the subgraph with nothing describing them.
    fn describe_seeds(&self, seeds: Vec<D::Id>) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let mut anchors: BTreeSet<D::Id> = BTreeSet::new();
        let mut frontier: Vec<D::Id> = Vec::new();
        for s in seeds {
            if anchors.insert(s) {
                frontier.push(s);
            }
        }

        let mut quads: HashSet<QuadIds<D::Id>> = HashSet::new();
        let mut reifiers: BTreeSet<(D::Id, D::Id)> = BTreeSet::new();
        let mut annotations: Vec<(D::Id, D::Id, D::Id)> = Vec::new();
        let mut visited_reifiers: HashSet<D::Id> = HashSet::new();

        // Expand a blank endpoint into the frontier; named nodes never expand (that
        // would drag in the entire neighbourhood of the graph).
        macro_rules! expand_blank {
            ($frontier:ident, $anchors:ident, $end:expr) => {{
                let end = $end;
                if self.is_blank(end) && $anchors.insert(end) {
                    $frontier.push(end);
                }
            }};
        }

        while let Some(anchor) = frontier.pop() {
            if let Some(touching) = self.by_endpoint.get(&anchor) {
                for &q in touching {
                    quads.insert(q);
                    expand_blank!(frontier, anchors, q.s);
                    expand_blank!(frontier, anchors, q.o);
                }
            }

            // Reifiers whose reified triple is about this anchor. Selection is by
            // reified-triple endpoint; a named-node reifier is still kept (only blank
            // ids/objects expand).
            if let Some(bindings) = self.reifiers_by_endpoint.get(&anchor) {
                for &b @ (reifier, triple) in bindings {
                    reifiers.insert(b);
                    // Close THIS binding's blank endpoints every time: a reifier may
                    // reify more than one triple, so each triple's blank subject/object
                    // (and a blank reifier id, whose own plain quads live in
                    // `by_endpoint`) must be anchored — `expand_blank!` is idempotent.
                    expand_blank!(frontier, anchors, reifier);
                    if let TermRef::Triple { s, p: _, o } = self.dataset.resolve(triple) {
                        expand_blank!(frontier, anchors, s);
                        expand_blank!(frontier, anchors, o);
                    }
                    // Harvest the reifier's annotations ONCE — they are keyed by the
                    // reifier, not by the individual binding, so only this needs the
                    // visited-guard.
                    if visited_reifiers.insert(reifier)
                        && let Some(annos) = self.annotations_by_reifier.get(&reifier)
                    {
                        for &(p, o) in annos {
                            annotations.push((reifier, p, o));
                            expand_blank!(frontier, anchors, p);
                            expand_blank!(frontier, anchors, o);
                        }
                    }
                }
            }
        }

        // Re-intern the selected quads + statement layer into a fresh dataset. A remap
        // memoizes old-id → new-id so the owned-term round-trip runs once per term.
        // `quads` is a `HashSet` (for dedup during the walk), whose iteration order is
        // randomized — re-interning in that order would make the extracted subgraph's
        // bytes unstable across runs. Sort by the source `(g, s, p, o)` ids first so the
        // output is deterministic (byte-reproducibility).
        let mut ordered: Vec<QuadIds<D::Id>> = quads.into_iter().collect();
        ordered.sort_unstable_by_key(|q| (q.g, q.s, q.p, q.o));
        let mut builder = RdfDatasetBuilder::new();
        let mut remap: BTreeMap<D::Id, TermId> = BTreeMap::new();
        for q in &ordered {
            let s = self.map_id(&mut builder, &mut remap, q.s);
            let p = self.map_id(&mut builder, &mut remap, q.p);
            let o = self.map_id(&mut builder, &mut remap, q.o);
            let g = q.g.map(|g| self.map_id(&mut builder, &mut remap, g));
            builder.push_quad(s, p, o, g);
        }
        for &(reifier, triple) in &reifiers {
            let r = self.map_id(&mut builder, &mut remap, reifier);
            let t = self.map_id(&mut builder, &mut remap, triple);
            builder.push_reifier(r, t);
        }
        for &(reifier, p, o) in &annotations {
            let r = self.map_id(&mut builder, &mut remap, reifier);
            let p = self.map_id(&mut builder, &mut remap, p);
            let o = self.map_id(&mut builder, &mut remap, o);
            builder.push_annotation(r, p, o);
        }

        builder.freeze()
    }

    /// Whether a term id is a blank node in the source dataset.
    fn is_blank(&self, id: D::Id) -> bool {
        matches!(self.dataset.resolve(id), TermRef::Blank { .. })
    }

    /// Intern a source term id into `builder`, memoized. `to_owned_term` recurses
    /// through triple terms, and `intern_owned_term` re-interns them, so quoted
    /// triples inside reifiers rebuild faithfully.
    fn map_id(
        &self,
        builder: &mut RdfDatasetBuilder,
        remap: &mut BTreeMap<D::Id, TermId>,
        old: D::Id,
    ) -> TermId {
        if let Some(&new) = remap.get(&old) {
            return new;
        }
        let owned = owned_term(self.dataset, old);
        let new = builder.intern_owned_term(&owned);
        remap.insert(old, new);
        new
    }
}

/// One-shot convenience: the SCBD of a single IRI subject in `dataset`.
///
/// For extracting many subjects (the docs export walks every term) build a
/// [`Describer`] once and reuse it — this rebuilds the adjacency index per call.
///
/// # Errors
/// Propagates a freeze diagnostic (see [`Describer::describe_iri`]).
pub fn describe(dataset: &RdfDataset, subject: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    Describer::new(dataset).describe_iri(subject)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RdfLiteral, RdfQuad, RdfTerm};

    const S: &str = "https://e/s";
    const OTHER: &str = "https://e/other";

    fn iri(v: &str) -> RdfTerm {
        RdfTerm::iri(v)
    }

    /// Build a dataset from owned quads (default graph) with an optional reifier +
    /// annotation on the first quad.
    fn dataset(quads: &[RdfQuad]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for q in quads {
            b.push_owned_quad(q);
        }
        b.freeze().expect("freeze test dataset")
    }

    fn triple(s: &str, p: &str, o: RdfTerm) -> RdfQuad {
        RdfQuad::new(iri(s), p.to_string(), o)
    }

    /// The set of `(s, p, o)` IRI/lexical strings in a described subgraph, for terse
    /// membership assertions (blank labels are scope-qualified, so compare by kind).
    fn objects_for(ds: &RdfDataset, subject: &str, predicate: &str) -> Vec<String> {
        let mut out = Vec::new();
        for q in ds.quad_refs() {
            let s = match q.s {
                TermRef::Iri(i) => i.to_string(),
                _ => continue,
            };
            let p = match q.p {
                TermRef::Iri(i) => i.to_string(),
                _ => continue,
            };
            if s == subject
                && p == predicate
                && let TermRef::Iri(o) = q.o
            {
                out.push(o.to_string());
            }
        }
        out.sort();
        out
    }

    #[test]
    fn describes_outgoing_triples() {
        let ds = dataset(&[
            triple(S, "https://e/p", iri("https://e/o1")),
            triple(S, "https://e/p", iri("https://e/o2")),
            triple(OTHER, "https://e/p", iri("https://e/x")),
        ]);
        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            objects_for(&scbd, S, "https://e/p"),
            vec!["https://e/o1".to_string(), "https://e/o2".to_string()]
        );
        // The unrelated OTHER subject's triple must NOT be pulled in.
        assert!(objects_for(&scbd, OTHER, "https://e/p").is_empty());
    }

    #[test]
    fn describes_incoming_triples_symmetrically() {
        // A plain (forward-only) CBD would miss this: OTHER points AT S.
        let ds = dataset(&[triple(OTHER, "https://e/refersTo", iri(S))]);
        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            objects_for(&scbd, OTHER, "https://e/refersTo"),
            vec![S.to_string()],
            "the incoming link OTHER -> S must be present in the symmetric CBD"
        );
    }

    #[test]
    fn named_node_neighbours_do_not_expand() {
        // S -> N, and N -> deep. `deep` must NOT come along: named nodes don't expand.
        let ds = dataset(&[
            triple(S, "https://e/p", iri("https://e/n")),
            triple("https://e/n", "https://e/p", iri("https://e/deep")),
        ]);
        let scbd = describe(&ds, S).unwrap();
        // The N -> deep triple is neither outgoing-from nor incoming-to S, so absent.
        assert!(objects_for(&scbd, "https://e/n", "https://e/p").is_empty());
        assert_eq!(
            objects_for(&scbd, S, "https://e/p"),
            vec!["https://e/n".to_string()]
        );
    }

    #[test]
    fn blank_nodes_expand_transitively() {
        // S -> _:b (restriction) -> onProperty target. The blank closure must bring the
        // blank's own triples along, both hops.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let has = b.intern_iri("https://e/hasRestriction");
        let bnode = b.intern_blank("r1", crate::BlankScope::DEFAULT);
        let on = b.intern_iri("https://e/onProperty");
        let target = b.intern_iri("https://e/target");
        b.push_quad(s, has, bnode, None);
        b.push_quad(bnode, on, target, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        // Both quads survive: S -> _:b and _:b -> target.
        assert_eq!(
            scbd.quad_count(),
            2,
            "blank-node closure must keep both hops"
        );
        // The blank's onProperty edge is present (object is the named target).
        let has_target = scbd.quad_refs().any(|q| {
            matches!(q.p, TermRef::Iri(i) if i == "https://e/onProperty")
                && matches!(q.o, TermRef::Iri(i) if i == "https://e/target")
        });
        assert!(has_target, "the blank node's own triple must be included");
    }

    #[test]
    fn absent_subject_yields_empty() {
        let ds = dataset(&[triple(S, "https://e/p", iri("https://e/o"))]);
        let scbd = describe(&ds, "https://e/nope").unwrap();
        assert_eq!(scbd.quad_count(), 0);
    }

    #[test]
    fn includes_reifiers_about_the_subject() {
        // S p o, with a reifier annotating that statement (a certainty note).
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("stmt1", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, triple_term);
        let certainty = b.intern_iri("https://e/certainty");
        let high = b.intern_literal(RdfLiteral::simple("high"));
        b.push_annotation(reifier, certainty, high);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        // The reifier binding (reifier rdf:reifies << s p o >>) and its annotation ride
        // along because the reified statement's subject is S.
        assert_eq!(
            scbd.reifiers().count(),
            1,
            "the reifier about S must be kept"
        );
        assert_eq!(scbd.annotations().count(), 1, "its annotation must be kept");
    }

    #[test]
    fn slice_scope_unions_subjects() {
        let ds = dataset(&[
            triple(S, "https://e/p", iri("https://e/o1")),
            triple(OTHER, "https://e/p", iri("https://e/o2")),
        ]);
        let d = Describer::new(&ds);
        let scbd = d.describe_iris([S, OTHER]).unwrap();
        assert_eq!(scbd.quad_count(), 2, "both subjects' triples in the union");
    }

    // The serializer round-trip (describe → every `native_codecs` format) lives in
    // `purrdf` (`crates/rdf/tests/describe_serialize.rs`) because the serializers
    // are in that higher crate; `purrdf-core` holds only the extraction itself.

    #[test]
    fn blank_reifier_own_triples_are_closed() {
        // A blank reifier that is ALSO the subject of a plain quad
        // (`_:stmt ex:author ex:alice`). That describing quad must ride along with the
        // reifier — a plain forward walk would drop it, leaving a dangling blank.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("stmt1", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, triple_term);
        // A plain triple describing the blank reifier itself.
        let author = b.intern_iri("https://e/author");
        let alice = b.intern_iri("https://e/alice");
        b.push_quad(reifier, author, alice, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            scbd.reifiers().count(),
            1,
            "the reifier about S must be kept"
        );
        let has_author = scbd.quad_refs().any(|q| {
            matches!(q.s, TermRef::Blank { .. })
                && matches!(q.p, TermRef::Iri(i) if i == "https://e/author")
                && matches!(q.o, TermRef::Iri(i) if i == "https://e/alice")
        });
        assert!(
            has_author,
            "the blank reifier's own describing triple must be included"
        );
    }

    #[test]
    fn blank_annotation_object_is_closed() {
        // S p o, reified; the reifier's `source` annotation points at a blank provenance
        // node that itself carries a triple (`_:prov ex:by ex:agent`). The annotation
        // side-table does not live in `by_endpoint`, so without folding the annotation
        // harvest into the walk that blank would dangle.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("stmt1", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, triple_term);
        let source = b.intern_iri("https://e/source");
        let prov = b.intern_blank("prov", crate::BlankScope::DEFAULT);
        b.push_annotation(reifier, source, prov);
        // The blank provenance node's own describing triple (a plain quad).
        let by = b.intern_iri("https://e/by");
        let agent = b.intern_iri("https://e/agent");
        b.push_quad(prov, by, agent, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            scbd.annotations().count(),
            1,
            "the source annotation must be kept"
        );
        // The blank provenance node's `by agent` triple must survive (no dangling blank).
        let has_by = scbd.quad_refs().any(|q| {
            matches!(q.s, TermRef::Blank { .. })
                && matches!(q.p, TermRef::Iri(i) if i == "https://e/by")
                && matches!(q.o, TermRef::Iri(i) if i == "https://e/agent")
        });
        assert!(
            has_by,
            "the blank annotation object's own triple must be included"
        );
    }

    #[test]
    fn reifier_reifying_multiple_triples_closes_each_binding() {
        // One reifier reifies TWO triples about S. The second reified triple is NOT
        // asserted as a plain quad and its object is a blank with its own triple, so the
        // blank is reachable ONLY through that second binding. Deduplicating the whole
        // binding on the reifier id (rather than only the annotation harvest) would skip
        // closing the second triple's endpoints and drop the blank.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p1 = b.intern_iri("https://e/p1");
        let o1 = b.intern_iri("https://e/o1");
        let p2 = b.intern_iri("https://e/p2");
        let blank = b.intern_blank("bo", crate::BlankScope::DEFAULT);
        // Only the first triple is asserted as a plain quad; the second exists solely as
        // a reified statement.
        b.push_quad(s, p1, o1, None);
        let t1 = b.intern_triple(s, p1, o1);
        let t2 = b.intern_triple(s, p2, blank);
        let reifier = b.intern_blank("stmt", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, t1);
        b.push_reifier(reifier, t2);
        // The blank object of the second reified triple carries its own describing triple.
        let deep = b.intern_iri("https://e/deep");
        let val = b.intern_iri("https://e/val");
        b.push_quad(blank, deep, val, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            scbd.reifiers().count(),
            2,
            "both reified triples must be kept"
        );
        let has_deep = scbd
            .quad_refs()
            .any(|q| matches!(q.p, TermRef::Iri(i) if i == "https://e/deep"));
        assert!(
            has_deep,
            "the blank endpoint of the second reified triple must be closed"
        );
    }
}
