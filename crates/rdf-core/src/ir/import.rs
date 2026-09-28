// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed, memoized transfer from one read view into a native dataset builder.

use core::convert::Infallible;

use super::term_walk::{Nested, try_fold_nested};
use crate::hash::FastMap;
use crate::{DatasetView, RdfDatasetBuilder, TermId, TermRef};

/// Resolve a source term in another native dictionary without an owned term
/// tree. The memo belongs to this exact source/target/scope translation.
///
/// The walk runs over [`try_fold_nested`]'s work list. A term is entered by first
/// consulting `memo`; a triple term's subject, predicate and object are each looked
/// up fully, in that order — every one of them, whether or not an earlier one was
/// found — and the triple is looked up by their answers once all three exist. Every
/// term's answer is recorded in `memo` as soon as it is known.
pub(crate) fn lookup_native_term(
    source: &crate::RdfDataset,
    target: &crate::RdfDataset,
    id: TermId,
    scope: impl Fn(crate::BlankScope) -> Option<crate::BlankScope> + Copy,
    memo: &mut FastMap<TermId, Option<TermId>>,
) -> Option<TermId> {
    let found = try_fold_nested(
        id,
        memo,
        |memo, id| {
            if let Some(found) = memo.get(&id) {
                return Ok::<_, Infallible>(Nested::Leaf(*found));
            }
            let found = match source.resolve(id) {
                TermRef::Iri(iri) => target.term_id_by_iri(iri),
                TermRef::Blank {
                    label,
                    scope: original,
                } => scope(original).and_then(|scope| target.term_id_by_blank(label, scope)),
                TermRef::Literal {
                    lexical,
                    datatype,
                    language,
                    direction,
                } => {
                    let TermRef::Iri(datatype) = source.resolve(datatype) else {
                        unreachable!("native datatype is an IRI")
                    };
                    target.term_id_by_literal(lexical, datatype, language, direction)
                }
                TermRef::Triple { s, p, o } => return Ok(Nested::Triple(s, p, o)),
            };
            memo.insert(id, found);
            Ok(Nested::Leaf(found))
        },
        |memo, id, s, p, o| {
            let found = match (s, p, o) {
                (Some(s), Some(p), Some(o)) => target.term_id_by_triple(s, p, o),
                _ => None,
            };
            memo.insert(id, found);
            Ok(found)
        },
    );
    match found {
        Ok(found) => found,
    }
}

/// Operational counts for a typed import. These describe work performed, never
/// a content identity, correctness certificate, or cache key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DatasetImportStats {
    /// Distinct source terms resolved and interned, including triple components.
    pub terms: usize,
    /// Term requests answered by the import's source-local memo.
    pub reused_terms: usize,
    /// Base quad rows replayed (the builder may deduplicate them).
    pub quads: usize,
    /// Reifier bindings replayed.
    pub reifiers: usize,
    /// Annotation rows replayed.
    pub annotations: usize,
    /// Named graphs declared, including graphs without any rows.
    pub named_graphs: usize,
}

/// An import bound to exactly one immutable source and one destination builder.
///
/// Repeated source IDs reuse the destination ID without constructing an owned
/// term tree. Triple components are transferred once each; blank labels
/// retain their explicit scopes. A fresh importer is required for each source,
/// so equal numeric IDs in unrelated datasets never alias. The destination
/// builder still deduplicates equal RDF values across imports.
///
/// The memo is bounded by the source's referenced term count and released with
/// this importer. This is a materialization boundary; callers that can consume
/// the source view directly should keep doing so. Nothing is published until
/// the destination builder's ordinary validated freeze succeeds.
pub struct DatasetImporter<'builder, 'view, D: DatasetView> {
    builder: &'builder mut RdfDatasetBuilder,
    view: &'view D,
    terms: FastMap<D::Id, TermId>,
    stats: DatasetImportStats,
}

impl<D: DatasetView> core::fmt::Debug for DatasetImporter<'_, '_, D> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DatasetImporter")
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl<'builder, 'view, D: DatasetView> DatasetImporter<'builder, 'view, D> {
    /// Bind a new import. No source rows or terms are copied at construction — the
    /// only work done here is RESERVING the destination tables and this import's own
    /// memo against the source's self-reported size.
    ///
    /// Every figure comes from the view seam itself ([`DatasetView::term_count`],
    /// [`DatasetView::term_bytes_hint`] and [`DatasetView::len_hint`]), so they are
    /// the source's own counts rather than a guess, and a view that declines to
    /// answer simply reserves nothing for that table. A replay that overruns any
    /// figure still grows normally; the reserve removes the doubling walk, it does
    /// not bound the import.
    pub fn new(builder: &'builder mut RdfDatasetBuilder, view: &'view D) -> Self {
        let terms = view.term_count();
        builder.reserve_for_replay(
            terms,
            view.term_bytes_hint().unwrap_or(0),
            view.len_hint().unwrap_or(0),
        );
        Self {
            builder,
            view,
            terms: FastMap::with_capacity_and_hasher(terms, crate::hash::FastHasher::default()),
            stats: DatasetImportStats::default(),
        }
    }

    /// Transfer one source-local term, preserving its complete RDF 1.2 value.
    /// The ID must belong to the source passed to [`Self::new`].
    ///
    /// The walk runs over [`try_fold_nested`]'s work list. A term is entered by first
    /// consulting the memo; a triple term's subject, predicate and object are each
    /// transferred fully, in that order, before the triple. Every term is recorded in
    /// the memo as soon as its own id exists.
    pub fn term(&mut self, id: D::Id) -> TermId {
        // The source outlives this importer, so its resolved strings can be handed to
        // the destination interner BORROWED: the interner keys on `&str` and owns the
        // bytes in its own arena, so an owned `RdfLiteral` here would allocate a
        // lexical form, a datatype IRI and a language tag per literal for the interner
        // to immediately copy and drop.
        let view = self.view;
        let mapped = try_fold_nested(
            id,
            self,
            |this, id| {
                if let Some(mapped) = this.terms.get(&id) {
                    this.stats.reused_terms += 1;
                    return Ok::<_, Infallible>(Nested::Leaf(*mapped));
                }
                let mapped = match view.resolve(id) {
                    TermRef::Iri(iri) => this.builder.intern_iri(iri),
                    TermRef::Blank { label, scope } => this.builder.intern_blank(label, scope),
                    TermRef::Literal {
                        lexical,
                        datatype,
                        language,
                        direction,
                    } => {
                        let TermRef::Iri(datatype) = view.resolve(datatype) else {
                            unreachable!("a literal datatype must resolve to an IRI");
                        };
                        this.builder.intern_literal_parts(
                            lexical,
                            Some(datatype),
                            language,
                            direction,
                        )
                    }
                    TermRef::Triple { s, p, o } => return Ok(Nested::Triple(s, p, o)),
                };
                this.record(id, mapped);
                Ok(Nested::Leaf(mapped))
            },
            |this, id, s, p, o| {
                let mapped = this.builder.intern_triple(s, p, o);
                this.record(id, mapped);
                Ok(mapped)
            },
        );
        match mapped {
            Ok(mapped) => mapped,
        }
    }

    /// Record that source term `id` was transferred as `mapped`.
    fn record(&mut self, id: D::Id, mapped: TermId) {
        self.terms.insert(id, mapped);
        self.stats.terms += 1;
    }

    /// Replay the complete RDF surface exposed by the source view, keeping base
    /// quads, reifier bindings and annotations in their respective tables.
    /// Declaration-only named graphs survive even when no quad mentions them.
    ///
    /// `DatasetView` does not expose source locations or non-RDF lookaside
    /// records; owners must carry those sidecars separately. This operation does
    /// not invent or claim to transfer them.
    pub fn append(&mut self) {
        let view = self.view;
        for quad in view.quads() {
            let s = self.term(quad.s);
            let p = self.term(quad.p);
            let o = self.term(quad.o);
            let g = quad.g.map(|g| self.term(g));
            self.builder.push_quad(s, p, o, g);
            self.stats.quads += 1;
        }
        // The RDF 1.2 overlay has no count on the view seam, but its own iterator
        // states one: a backend reading columns off a side-table knows its row count
        // before it yields a row. A source that cannot say reports a zero lower bound
        // and the tables grow as they always did.
        let reifiers = view.reifier_quads();
        self.builder.reserve_reifiers(reifiers.size_hint().0);
        for quad in reifiers {
            let reifier = self.term(quad.s);
            let triple = self.term(quad.o);
            let graph = quad.g.map(|g| self.term(g));
            self.builder.push_reifier_in_graph(reifier, triple, graph);
            self.stats.reifiers += 1;
        }
        let annotations = view.annotation_quads();
        self.builder.reserve_annotations(annotations.size_hint().0);
        for quad in annotations {
            let reifier = self.term(quad.s);
            let predicate = self.term(quad.p);
            let object = self.term(quad.o);
            let graph = quad.g.map(|g| self.term(g));
            self.builder
                .push_annotation_in_graph(reifier, predicate, object, graph);
            self.stats.annotations += 1;
        }
        for graph in view.named_graphs() {
            let graph = self.term(graph);
            self.builder.declare_named_graph(graph);
            self.stats.named_graphs += 1;
        }
    }

    /// Work performed by this import so far.
    #[must_use]
    pub const fn stats(&self) -> DatasetImportStats {
        self.stats
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The cross-dictionary lookup and the typed import against their recursive
    //! references.

    use super::{DatasetImporter, lookup_native_term};
    use crate::backend::TermFactory as _;
    use crate::hash::FastMap;
    use crate::test_rng::TermShape;
    use crate::{DatasetView, RdfDataset, RdfDatasetBuilder, TermId, TermRef, TermValue};

    fn reference_lookup(
        source: &RdfDataset,
        target: &RdfDataset,
        id: TermId,
        memo: &mut FastMap<TermId, Option<TermId>>,
    ) -> Option<TermId> {
        if let Some(found) = memo.get(&id) {
            return *found;
        }
        let found = match source.resolve(id) {
            TermRef::Triple { s, p, o } => {
                let s = reference_lookup(source, target, s, memo);
                let p = reference_lookup(source, target, p, memo);
                let o = reference_lookup(source, target, o, memo);
                match (s, p, o) {
                    (Some(s), Some(p), Some(o)) => target.term_id_by_triple(s, p, o),
                    _ => None,
                }
            }
            _ => {
                let mut leaf = FastMap::default();
                lookup_native_term(source, target, id, Some, &mut leaf)
            }
        };
        memo.insert(id, found);
        found
    }

    fn reference_term<D: DatasetView>(
        importer: &mut DatasetImporter<'_, '_, D>,
        id: D::Id,
    ) -> TermId {
        if let Some(mapped) = importer.terms.get(&id) {
            importer.stats.reused_terms += 1;
            return *mapped;
        }
        let mapped = match importer.view.resolve(id) {
            TermRef::Triple { s, p, o } => {
                let s = reference_term(importer, s);
                let p = reference_term(importer, p);
                let o = reference_term(importer, o);
                importer.builder.intern_triple(s, p, o)
            }
            _ => {
                let mut single = DatasetImporter::new(importer.builder, importer.view);
                single.term(id)
            }
        };
        importer.terms.insert(id, mapped);
        importer.stats.terms += 1;
        mapped
    }

    /// A frozen dataset holding `value` as the object of one quad, and that object's id.
    fn holding(value: &TermValue) -> (std::sync::Arc<RdfDataset>, TermId) {
        let mut builder = RdfDatasetBuilder::new();
        let object = builder.intern_value(value);
        let holder = builder.intern_iri("http://example.org/holder");
        builder.push_quad(holder, holder, object, None);
        let dataset = builder.freeze().expect("a generated term freezes");
        let object = dataset.quads().next().expect("one quad").o;
        (dataset, object)
    }

    /// Every generated stored term is looked up in a dictionary that holds it and in one
    /// that holds another, and imported twice into a fresh builder, exactly as the
    /// recursive references do: the same answers, memos, ids and counts.
    #[test]
    fn the_lookup_and_the_import_agree_with_their_recursive_references() {
        let mut found_any = 0;
        for seed in 0..300_u64 {
            let (mut state, mut budget) = (seed, 8);
            let value = crate::test_rng::term_value(&mut state, &mut budget, TermShape::WellFormed);
            let (mut state, mut budget) = (seed + 9_000, 8);
            let other = crate::test_rng::term_value(&mut state, &mut budget, TermShape::WellFormed);
            let (source, id) = holding(&value);
            for (target, _) in [holding(&value), holding(&other)] {
                let (mut memo, mut expected_memo) = (FastMap::default(), FastMap::default());
                let found = lookup_native_term(&source, &target, id, Some, &mut memo);
                assert_eq!(
                    found,
                    reference_lookup(&source, &target, id, &mut expected_memo),
                    "seed {seed}"
                );
                assert_eq!(memo, expected_memo, "seed {seed}");
                found_any += usize::from(found.is_some());
            }
            let (mut into, mut expected_into) =
                (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            let mut importer = DatasetImporter::new(&mut into, &*source);
            let mut expected = DatasetImporter::new(&mut expected_into, &*source);
            for _ in 0..2 {
                assert_eq!(
                    importer.term(id),
                    reference_term(&mut expected, id),
                    "seed {seed}"
                );
            }
            assert_eq!(importer.stats(), expected.stats(), "seed {seed}");
        }
        assert!(found_any > 0, "some generated term is found");
    }
}
