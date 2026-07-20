// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Validation-scoped asserted-subclass class membership.
//!
//! [`ClassMembershipView`] exposes the SHACL instance relation as a read-only
//! [`DatasetView`]. The effective relation is the asserted default-graph
//! `rdf:type` set plus unique virtual `(subject, rdf:type, superclass)` rows
//! reachable through one or more asserted default-graph `rdfs:subClassOf`
//! edges. No subclass row or other RDFS/OWL consequence is virtualized.

use std::sync::{Arc, OnceLock};

use ::purrdf::ir::QuadProbePlan;
use ::purrdf::{
    DatasetView, FastMap, FastSet, GraphMatch, QuadIds, QuadRef, RdfDataset, RdfStoreCapabilities,
    SmallVec, TermId, TermRef, TermValue,
};

use crate::model::{rdf, rdfs};

#[cfg(test)]
thread_local! {
    static THREAD_INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// One asserted type row, sorted by `(class, subject)` while the compact index
/// is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TypeRow {
    class: TermId,
    subject: TermId,
}

/// The compact ranges owned by one exact asserted type.
#[derive(Debug)]
struct TypedClass {
    class: TermId,
    subject_start: usize,
    subject_end: usize,
    ancestor_start: usize,
    ancestor_end: usize,
}

/// Exact asserted types whose subjects derive one superclass membership.
#[derive(Debug)]
struct SuperclassSources {
    class: TermId,
    source_start: usize,
    source_end: usize,
}

/// Frozen compact state shared by every probe over one dataset.
#[derive(Debug)]
struct ClassMembershipIndex {
    typed_classes: Box<[TypedClass]>,
    subjects: Box<[TermId]>,
    ancestors: Box<[TermId]>,
    superclasses: Box<[SuperclassSources]>,
    source_classes: Box<[u32]>,
    virtual_upper_bound: usize,
    fingerprint: u64,
}

impl ClassMembershipIndex {
    fn typed_class(&self, class: TermId) -> Option<(usize, &TypedClass)> {
        self.typed_classes
            .binary_search_by_key(&class, |entry| entry.class)
            .ok()
            .map(|index| (index, &self.typed_classes[index]))
    }

    fn subjects(&self, class: &TypedClass) -> &[TermId] {
        &self.subjects[class.subject_start..class.subject_end]
    }

    fn ancestors(&self, class: &TypedClass) -> &[TermId] {
        &self.ancestors[class.ancestor_start..class.ancestor_end]
    }

    fn proper_ancestors(&self, class: TermId) -> &[TermId] {
        self.typed_class(class)
            .map_or(&[], |(_, entry)| self.ancestors(entry))
    }

    fn direct_subjects(&self, class: TermId) -> &[TermId] {
        self.typed_class(class)
            .map_or(&[], |(_, entry)| self.subjects(entry))
    }

    fn source_class_indexes(&self, superclass: TermId) -> &[u32] {
        let Ok(index) = self
            .superclasses
            .binary_search_by_key(&superclass, |entry| entry.class)
        else {
            return &[];
        };
        let entry = &self.superclasses[index];
        &self.source_classes[entry.source_start..entry.source_end]
    }
}

/// Lazily initialized state. Cloned views over the same `Arc<RdfDataset>` share
/// this cell, so prepared validation and parallel workers build the index once.
#[derive(Debug, Default)]
struct SharedIndex {
    index: OnceLock<Option<ClassMembershipIndex>>,
    #[cfg(test)]
    builds: std::sync::atomic::AtomicUsize,
}

/// A validation-scoped view over a frozen dataset.
#[derive(Debug, Clone)]
pub(crate) struct ClassMembershipView {
    base: Arc<RdfDataset>,
    rdf_type: Option<TermId>,
    subclass_of: Option<TermId>,
    shared: Arc<SharedIndex>,
}

impl ClassMembershipView {
    pub(crate) fn new(base: Arc<RdfDataset>) -> Self {
        let rdf_type = base.term_id_by_iri(rdf::TYPE);
        let subclass_of = base.term_id_by_iri(rdfs::SUB_CLASS_OF);
        Self {
            base,
            rdf_type,
            subclass_of,
            shared: Arc::new(SharedIndex::default()),
        }
    }

    /// Build the immutable index at the preparation boundary.
    pub(crate) fn prepare(&self) {
        let _ = self.index();
    }

    /// The frozen asserted dataset wrapped by this view.
    #[inline]
    #[cfg(test)]
    pub(crate) fn base(&self) -> &Arc<RdfDataset> {
        &self.base
    }

    /// Whether `subject` is directly or transitively an asserted SHACL instance
    /// of `class` in the default data graph.
    pub(crate) fn is_instance(&self, subject: TermId, class: TermId) -> bool {
        let Some(rdf_type) = self.rdf_type else {
            return false;
        };
        if self
            .base
            .quads_for_pattern(
                Some(subject),
                Some(rdf_type),
                Some(class),
                GraphMatch::Default,
            )
            .next()
            .is_some()
        {
            return true;
        }
        self.has_derived_membership(subject, class)
    }

    /// Every direct or derived instance of `class`, sorted within the asserted
    /// and virtual portions and duplicate-free across them.
    pub(crate) fn instances_of(&self, class: TermId) -> impl Iterator<Item = TermId> + '_ {
        self.rdf_type
            .into_iter()
            .flat_map(move |rdf_type| {
                self.base
                    .quads_for_pattern(None, Some(rdf_type), Some(class), GraphMatch::Default)
                    .map(|quad| quad.s)
            })
            .chain(self.derived_subjects(class))
    }

    fn index(&self) -> Option<&ClassMembershipIndex> {
        let (Some(rdf_type), Some(subclass_of)) = (self.rdf_type, self.subclass_of) else {
            return None;
        };
        self.shared
            .index
            .get_or_init(|| {
                #[cfg(test)]
                {
                    self.shared
                        .builds
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    THREAD_INDEX_BUILDS.with(|builds| builds.set(builds.get() + 1));
                }
                build_index(&self.base, rdf_type, subclass_of)
            })
            .as_ref()
    }

    fn is_derived(&self, subject: TermId, class: TermId) -> bool {
        let Some(rdf_type) = self.rdf_type else {
            return false;
        };
        if self
            .base
            .quads_for_pattern(
                Some(subject),
                Some(rdf_type),
                Some(class),
                GraphMatch::Default,
            )
            .next()
            .is_some()
        {
            return false;
        }
        self.has_derived_membership(subject, class)
    }

    fn has_derived_membership(&self, subject: TermId, class: TermId) -> bool {
        let (Some(rdf_type), Some(index)) = (self.rdf_type, self.index()) else {
            return false;
        };
        self.base
            .quads_for_pattern(Some(subject), Some(rdf_type), None, GraphMatch::Default)
            .any(|quad| index.proper_ancestors(quad.o).binary_search(&class).is_ok())
    }

    fn derived_subjects(&self, class: TermId) -> MergedSubjects<'_> {
        self.index().map_or_else(MergedSubjects::empty, |index| {
            MergedSubjects::new(index, class)
        })
    }

    fn derived_types(&self, subject: TermId) -> MergedTypes<'_> {
        let (Some(rdf_type), Some(index)) = (self.rdf_type, self.index()) else {
            return MergedTypes::empty();
        };
        let mut direct: SmallVec<[TermId; 4]> = self
            .base
            .quads_for_pattern(Some(subject), Some(rdf_type), None, GraphMatch::Default)
            .map(|quad| quad.o)
            .collect();
        direct.sort_unstable();
        direct.dedup();
        MergedTypes::new(index, direct)
    }

    fn derived_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> DerivedPattern<'_> {
        let Some(rdf_type) = self.rdf_type else {
            return DerivedPattern::Empty;
        };
        if matches!(g, GraphMatch::Named(_)) || p.is_some_and(|bound| bound != rdf_type) {
            return DerivedPattern::Empty;
        }
        match (s, o) {
            (Some(subject), Some(class)) => DerivedPattern::One(
                self.is_derived(subject, class)
                    .then_some(QuadIds {
                        s: subject,
                        p: rdf_type,
                        o: class,
                        g: None,
                    })
                    .into_iter(),
            ),
            (Some(subject), None) => DerivedPattern::Types {
                subject,
                rdf_type,
                types: self.derived_types(subject),
            },
            (None, Some(class)) => DerivedPattern::Subjects {
                class,
                rdf_type,
                subjects: self.derived_subjects(class),
            },
            (None, None) => {
                self.index()
                    .map_or(DerivedPattern::Empty, |index| DerivedPattern::All {
                        rdf_type,
                        rows: AllDerived::new(index),
                    })
            }
        }
    }

    fn derived_cardinality_upper_bound(&self, s: Option<TermId>, o: Option<TermId>) -> usize {
        let Some(index) = self.index() else {
            return 0;
        };
        match (s, o) {
            (Some(subject), Some(class)) => usize::from(self.is_derived(subject, class)),
            (Some(subject), None) => {
                let Some(rdf_type) = self.rdf_type else {
                    return 0;
                };
                self.base
                    .quads_for_pattern(Some(subject), Some(rdf_type), None, GraphMatch::Default)
                    .map(|quad| index.proper_ancestors(quad.o).len())
                    .fold(0usize, usize::saturating_add)
            }
            (None, Some(class)) => index
                .source_class_indexes(class)
                .iter()
                .map(|&source| {
                    let source = usize::try_from(source).expect("u32 class index fits usize");
                    index.subjects(&index.typed_classes[source]).len()
                })
                .fold(0usize, usize::saturating_add),
            (None, None) => index.virtual_upper_bound,
        }
    }

    #[cfg(test)]
    pub(crate) fn build_count(&self) -> usize {
        self.shared
            .builds
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn dimensions(&self) -> [usize; 6] {
        self.index().map_or([0; 6], |index| {
            [
                index.typed_classes.len(),
                index.subjects.len(),
                index.ancestors.len(),
                index.superclasses.len(),
                index.source_classes.len(),
                index.virtual_upper_bound,
            ]
        })
    }
}

#[cfg(test)]
pub(crate) fn reset_thread_index_builds() {
    THREAD_INDEX_BUILDS.with(|builds| builds.set(0));
}

#[cfg(test)]
pub(crate) fn thread_index_builds() -> usize {
    THREAD_INDEX_BUILDS.with(std::cell::Cell::get)
}

/// A cursor into one canonically sorted subject slice.
#[derive(Debug, Clone, Copy)]
struct SubjectCursor<'a> {
    values: &'a [TermId],
    position: usize,
}

/// Merge the asserted subjects of every exact type below one superclass.
#[derive(Debug)]
struct MergedSubjects<'a> {
    cursors: SmallVec<[SubjectCursor<'a>; 4]>,
    directly_asserted: &'a [TermId],
}

impl<'a> MergedSubjects<'a> {
    fn empty() -> Self {
        Self {
            cursors: SmallVec::new(),
            directly_asserted: &[],
        }
    }

    fn new(index: &'a ClassMembershipIndex, class: TermId) -> Self {
        let mut cursors = SmallVec::new();
        for &source in index.source_class_indexes(class) {
            let source = usize::try_from(source).expect("u32 class index fits usize");
            cursors.push(SubjectCursor {
                values: index.subjects(&index.typed_classes[source]),
                position: 0,
            });
        }
        Self {
            cursors,
            directly_asserted: index.direct_subjects(class),
        }
    }
}

impl Iterator for MergedSubjects<'_> {
    type Item = TermId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self
                .cursors
                .iter()
                .filter_map(|cursor| cursor.values.get(cursor.position).copied())
                .min()?;
            for cursor in &mut self.cursors {
                while cursor.values.get(cursor.position) == Some(&next) {
                    cursor.position += 1;
                }
            }
            if self.directly_asserted.binary_search(&next).is_err() {
                return Some(next);
            }
        }
    }
}

/// A cursor into one exact type's proper-ancestor slice.
#[derive(Debug, Clone, Copy)]
struct TypeCursor<'a> {
    values: &'a [TermId],
    position: usize,
}

/// Merge the proper ancestors of every exact type asserted for one subject.
#[derive(Debug)]
struct MergedTypes<'a> {
    cursors: SmallVec<[TypeCursor<'a>; 4]>,
    directly_asserted: SmallVec<[TermId; 4]>,
}

impl<'a> MergedTypes<'a> {
    fn empty() -> Self {
        Self {
            cursors: SmallVec::new(),
            directly_asserted: SmallVec::new(),
        }
    }

    fn new(index: &'a ClassMembershipIndex, directly_asserted: SmallVec<[TermId; 4]>) -> Self {
        let cursors = directly_asserted
            .iter()
            .map(|&class| TypeCursor {
                values: index.proper_ancestors(class),
                position: 0,
            })
            .collect();
        Self {
            cursors,
            directly_asserted,
        }
    }
}

impl Iterator for MergedTypes<'_> {
    type Item = TermId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self
                .cursors
                .iter()
                .filter_map(|cursor| cursor.values.get(cursor.position).copied())
                .min()?;
            for cursor in &mut self.cursors {
                while cursor.values.get(cursor.position) == Some(&next) {
                    cursor.position += 1;
                }
            }
            if self.directly_asserted.binary_search(&next).is_err() {
                return Some(next);
            }
        }
    }
}

/// Full deterministic virtual-row scan, grouped by superclass then subject.
#[derive(Debug)]
struct AllDerived<'a> {
    index: &'a ClassMembershipIndex,
    superclass: usize,
    current: MergedSubjects<'a>,
}

impl<'a> AllDerived<'a> {
    fn new(index: &'a ClassMembershipIndex) -> Self {
        Self {
            index,
            superclass: 0,
            current: MergedSubjects::empty(),
        }
    }
}

impl Iterator for AllDerived<'_> {
    type Item = (TermId, TermId);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(subject) = self.current.next() {
                let class = self.index.superclasses[self.superclass - 1].class;
                return Some((subject, class));
            }
            let superclass = self.index.superclasses.get(self.superclass)?;
            self.superclass += 1;
            self.current = MergedSubjects::new(self.index, superclass.class);
        }
    }
}

/// The virtual portion of one pattern probe.
#[derive(Debug)]
enum DerivedPattern<'a> {
    Empty,
    One(std::option::IntoIter<QuadIds>),
    Types {
        subject: TermId,
        rdf_type: TermId,
        types: MergedTypes<'a>,
    },
    Subjects {
        class: TermId,
        rdf_type: TermId,
        subjects: MergedSubjects<'a>,
    },
    All {
        rdf_type: TermId,
        rows: AllDerived<'a>,
    },
}

impl Iterator for DerivedPattern<'_> {
    type Item = QuadIds;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::One(row) => row.next(),
            Self::Types {
                subject,
                rdf_type,
                types,
            } => types.next().map(|class| QuadIds {
                s: *subject,
                p: *rdf_type,
                o: class,
                g: None,
            }),
            Self::Subjects {
                class,
                rdf_type,
                subjects,
            } => subjects.next().map(|subject| QuadIds {
                s: subject,
                p: *rdf_type,
                o: *class,
                g: None,
            }),
            Self::All { rdf_type, rows } => rows.next().map(|(subject, class)| QuadIds {
                s: subject,
                p: *rdf_type,
                o: class,
                g: None,
            }),
        }
    }
}

impl DatasetView for ClassMembershipView {
    type Id = TermId;
    type ProbePlan = QuadProbePlan;

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.base
            .quads()
            .chain(self.derived_for_pattern(None, None, None, GraphMatch::Any))
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        self.quads().map(|quad| QuadRef {
            s: self.base.resolve(quad.s),
            p: self.base.resolve(quad.p),
            o: self.base.resolve(quad.o),
            g: quad.g.map(|graph| self.base.resolve(graph)),
        })
    }

    #[inline]
    fn resolve(&self, id: TermId) -> TermRef<'_> {
        self.base.resolve(id)
    }

    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.base
            .quads_for_pattern(s, p, o, g)
            .chain(self.derived_for_pattern(s, p, o, g))
    }

    #[inline]
    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        self.base.term_id_by_value(value)
    }

    #[inline]
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.base.capabilities()
    }

    fn len_hint(&self) -> Option<usize> {
        if self.rdf_type.is_none() || self.subclass_of.is_none() || self.index().is_none() {
            Some(self.base.quad_count())
        } else {
            None
        }
    }

    #[inline]
    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch,
    ) -> QuadProbePlan {
        RdfDataset::probe_plan(s_bound, p_bound, o_bound, g)
    }

    fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.base
            .quads_for_pattern_with_plan(plan, s, p, o, g)
            .chain(self.derived_for_pattern(s, p, o, g))
    }

    fn cardinality_estimate(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> usize {
        let asserted = self.base.cardinality_estimate(s, p, o, g);
        let can_match_virtual = !matches!(g, GraphMatch::Named(_))
            && self
                .rdf_type
                .is_some_and(|rdf_type| p.is_none_or(|bound| bound == rdf_type));
        if !can_match_virtual {
            return asserted;
        }
        asserted.saturating_add(self.derived_cardinality_upper_bound(s, o))
    }

    #[inline]
    fn term_count(&self) -> usize {
        self.base.term_count()
    }

    fn stats_fingerprint(&self) -> u64 {
        const VIEW_TAG: u64 = 0x5348_4143_4c54_5950;
        let base = self.base.stats_fingerprint() ^ VIEW_TAG;
        self.index().map_or(base, |index| base ^ index.fingerprint)
    }

    #[inline]
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.base.reifier_quads()
    }

    #[inline]
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.base.annotation_quads()
    }

    #[inline]
    fn annotations_of_with_graph(
        &self,
        reifier: TermId,
    ) -> impl Iterator<Item = (TermId, TermId, Option<TermId>)> + '_ {
        self.base.annotations_of_with_graph(reifier)
    }

    #[inline]
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        self.base.named_graphs()
    }
}

fn build_index(
    dataset: &RdfDataset,
    rdf_type: TermId,
    subclass_of: TermId,
) -> Option<ClassMembershipIndex> {
    let mut parents: FastMap<TermId, Vec<TermId>> = FastMap::default();
    let mut asserted_type_classes = FastSet::default();
    let mut previous_type = None;
    for quad in dataset.quads() {
        if quad.g.is_some() {
            continue;
        }
        if quad.p == rdf_type {
            if previous_type != Some(quad.o) {
                asserted_type_classes.insert(quad.o);
                previous_type = Some(quad.o);
            }
        } else if quad.p == subclass_of {
            parents.entry(quad.s).or_default().push(quad.o);
        }
    }
    if parents.is_empty() {
        return None;
    }
    for direct in parents.values_mut() {
        direct.sort_unstable();
        direct.dedup();
    }

    // If no asserted type can take even the first subclass step, there can be no
    // virtual membership and no exact-type index is retained. Once one can,
    // retain every exact type: direct ancestor subjects are required to suppress
    // direct-plus-derived duplicates during object-bound enumeration.
    let has_virtual_source = asserted_type_classes
        .iter()
        .any(|class| parents.contains_key(class));
    if !has_virtual_source {
        return None;
    }
    let mut rows: Vec<_> = dataset
        .quads()
        .filter(|quad| quad.g.is_none() && quad.p == rdf_type)
        .map(|quad| TypeRow {
            class: quad.o,
            subject: quad.s,
        })
        .collect();
    rows.sort_unstable();
    rows.dedup();

    let mut subjects = Vec::with_capacity(rows.len());
    let mut ancestors = Vec::new();
    let mut typed_classes = Vec::new();
    let mut marks = vec![0u32; dataset.term_count()];
    let mut generation = 0u32;
    let mut frontier = Vec::new();
    let mut position = 0usize;
    let mut virtual_upper_bound = 0usize;
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;

    while position < rows.len() {
        let class = rows[position].class;
        let subject_start = subjects.len();
        while position < rows.len() && rows[position].class == class {
            subjects.push(rows[position].subject);
            fingerprint = fingerprint_mix(fingerprint, rows[position].subject.index());
            position += 1;
        }
        let subject_end = subjects.len();

        generation = generation.checked_add(1).unwrap_or_else(|| {
            marks.fill(0);
            1
        });
        marks[class.index()] = generation;
        frontier.clear();
        frontier.push(class);
        let ancestor_start = ancestors.len();
        while let Some(child) = frontier.pop() {
            if let Some(direct) = parents.get(&child) {
                for &parent in direct {
                    if marks[parent.index()] != generation {
                        marks[parent.index()] = generation;
                        ancestors.push(parent);
                        frontier.push(parent);
                    }
                }
            }
        }
        // The generation marks admit each reachable class once, so sorting is
        // sufficient to freeze a canonical proper-ancestor slice.
        ancestors[ancestor_start..].sort_unstable();
        let ancestor_end = ancestors.len();
        for &ancestor in &ancestors[ancestor_start..ancestor_end] {
            fingerprint = fingerprint_mix(fingerprint, ancestor.index());
        }
        virtual_upper_bound = virtual_upper_bound.saturating_add(
            (subject_end - subject_start).saturating_mul(ancestor_end - ancestor_start),
        );
        typed_classes.push(TypedClass {
            class,
            subject_start,
            subject_end,
            ancestor_start,
            ancestor_end,
        });
    }

    if virtual_upper_bound == 0 {
        return None;
    }

    let mut source_pairs = Vec::new();
    for (source, class) in typed_classes.iter().enumerate() {
        let source = u32::try_from(source).expect("term table bounds typed classes to u32");
        for &superclass in &ancestors[class.ancestor_start..class.ancestor_end] {
            source_pairs.push((superclass, source));
        }
    }
    source_pairs.sort_unstable();
    source_pairs.dedup();

    let mut source_classes = Vec::with_capacity(source_pairs.len());
    let mut superclasses = Vec::new();
    let mut source_position = 0usize;
    while source_position < source_pairs.len() {
        let class = source_pairs[source_position].0;
        let source_start = source_classes.len();
        while source_position < source_pairs.len() && source_pairs[source_position].0 == class {
            source_classes.push(source_pairs[source_position].1);
            source_position += 1;
        }
        superclasses.push(SuperclassSources {
            class,
            source_start,
            source_end: source_classes.len(),
        });
    }

    Some(ClassMembershipIndex {
        typed_classes: typed_classes.into_boxed_slice(),
        subjects: subjects.into_boxed_slice(),
        ancestors: ancestors.into_boxed_slice(),
        superclasses: superclasses.into_boxed_slice(),
        source_classes: source_classes.into_boxed_slice(),
        virtual_upper_bound,
        fingerprint,
    })
}

#[inline]
fn fingerprint_mix(state: u64, value: usize) -> u64 {
    state
        .wrapping_mul(0x0000_0100_0000_01b3)
        .wrapping_add(value as u64)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use ::purrdf::{BlankScope, RdfDatasetBuilder};
    use proptest::prelude::*;

    const EX: &str = "https://example.org/class-membership/";

    fn fixture() -> (ClassMembershipView, [TermId; 8]) {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri(rdf::TYPE);
        let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
        let sub = builder.intern_iri(&format!("{EX}Sub"));
        let mid = builder.intern_iri(&format!("{EX}Mid"));
        let top = builder.intern_iri(&format!("{EX}Top"));
        let other = builder.intern_iri(&format!("{EX}Other"));
        let alice = builder.intern_iri(&format!("{EX}alice"));
        let bob = builder.intern_iri(&format!("{EX}bob"));
        builder.push_quad(sub, subclass, mid, None);
        builder.push_quad(mid, subclass, top, None);
        builder.push_quad(alice, rdf_type, sub, None);
        builder.push_quad(alice, rdf_type, top, None);
        builder.push_quad(bob, rdf_type, other, None);
        let dataset = builder.freeze().expect("fixture freezes");
        (
            ClassMembershipView::new(dataset),
            [rdf_type, subclass, sub, mid, top, other, alice, bob],
        )
    }

    #[test]
    fn bound_membership_is_transitive_unique_and_built_once() {
        let (view, [rdf_type, _, sub, mid, top, _, alice, _]) = fixture();
        assert!(view.is_instance(alice, sub));
        assert!(view.is_instance(alice, mid));
        assert!(view.is_instance(alice, top));
        assert_eq!(view.build_count(), 1);

        assert_eq!(
            view.quads_for_pattern(Some(alice), Some(rdf_type), Some(top), GraphMatch::Default)
                .count(),
            1,
            "direct-plus-derived type stays unique"
        );
        assert_eq!(view.build_count(), 1);
    }

    #[test]
    fn parallel_probes_initialize_the_shared_index_once() {
        use rayon::prelude::*;

        let (view, [_, _, _, mid, _, _, alice, _]) = fixture();
        (0..256usize).into_par_iter().for_each(|_| {
            assert!(view.is_instance(alice, mid));
        });
        assert_eq!(view.build_count(), 1);
    }

    #[test]
    fn variable_patterns_return_only_the_effective_type_relation() {
        let (view, [rdf_type, subclass, sub, mid, top, other, alice, bob]) = fixture();
        let alice_types: Vec<_> = view
            .quads_for_pattern(Some(alice), Some(rdf_type), None, GraphMatch::Default)
            .map(|quad| quad.o)
            .collect();
        assert_eq!(alice_types, vec![sub, top, mid]);

        let mid_instances: Vec<_> = view.instances_of(mid).collect();
        assert_eq!(mid_instances, vec![alice]);

        assert_eq!(
            view.quads_for_pattern(None, Some(subclass), None, GraphMatch::Default)
                .count(),
            2,
            "subclass rows themselves are never virtualized"
        );
        assert_eq!(
            view.quads_for_pattern(Some(bob), Some(rdf_type), Some(other), GraphMatch::Default,)
                .count(),
            1
        );
    }

    #[test]
    fn cycles_terminate_and_named_graphs_do_not_contribute() {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri(rdf::TYPE);
        let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
        let a = builder.intern_iri(&format!("{EX}A"));
        let b = builder.intern_iri(&format!("{EX}B"));
        let hidden = builder.intern_iri(&format!("{EX}Hidden"));
        let subject = builder.intern_iri(&format!("{EX}subject"));
        let graph = builder.intern_iri(&format!("{EX}shapes"));
        builder.push_quad(a, subclass, b, None);
        builder.push_quad(b, subclass, a, None);
        builder.push_quad(a, subclass, a, None);
        builder.push_quad(a, subclass, hidden, Some(graph));
        builder.push_quad(subject, rdf_type, a, None);
        let view = ClassMembershipView::new(builder.freeze().expect("fixture freezes"));

        assert!(view.is_instance(subject, a));
        assert!(view.is_instance(subject, b));
        assert!(!view.is_instance(subject, hidden));
        assert_eq!(
            view.quads_for_pattern(Some(subject), Some(rdf_type), None, GraphMatch::Default,)
                .count(),
            2
        );
        assert_eq!(view.named_graphs().collect::<Vec<_>>(), vec![graph]);
    }

    #[test]
    fn unrelated_rdfs_and_owl_terms_never_create_membership() {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri(rdf::TYPE);
        let person = builder.intern_iri(&format!("{EX}Person"));
        let other = builder.intern_iri(&format!("{EX}Other"));
        let alice = builder.intern_iri(&format!("{EX}alice"));
        let bob = builder.intern_iri(&format!("{EX}bob"));
        let property = builder.intern_iri(&format!("{EX}property"));
        let value = builder.intern_iri(&format!("{EX}value"));
        let domain = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#domain");
        let range = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#range");
        let subproperty = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#subPropertyOf");
        let equivalent = builder.intern_iri("http://www.w3.org/2002/07/owl#equivalentClass");
        builder.push_quad(property, domain, person, None);
        builder.push_quad(property, range, person, None);
        builder.push_quad(property, subproperty, rdf_type, None);
        builder.push_quad(other, equivalent, person, None);
        builder.push_quad(alice, property, value, None);
        builder.push_quad(bob, rdf_type, other, None);
        let view = ClassMembershipView::new(builder.freeze().expect("fixture freezes"));

        assert!(!view.is_instance(alice, person));
        assert!(!view.is_instance(value, person));
        assert!(!view.is_instance(bob, person));
        assert_eq!(view.quads().count(), 6);
    }

    #[test]
    fn no_hierarchy_is_an_identity_view() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri(&format!("{EX}subject"));
        let predicate = builder.intern_iri(&format!("{EX}predicate"));
        let object = builder.intern_iri(&format!("{EX}object"));
        builder.push_quad(subject, predicate, object, None);
        let dataset = builder.freeze().expect("fixture freezes");
        let view = ClassMembershipView::new(Arc::clone(&dataset));
        assert_eq!(
            view.quads().collect::<Vec<_>>(),
            dataset.quads().collect::<Vec<_>>()
        );
        assert_eq!(view.len_hint(), Some(1));
        assert_eq!(view.build_count(), 0, "missing vocabulary skips the scan");
    }

    #[test]
    fn direct_root_types_do_not_retain_an_empty_index() {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri(rdf::TYPE);
        let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
        let child = builder.intern_iri(&format!("{EX}Child"));
        let root = builder.intern_iri(&format!("{EX}Root"));
        let subject = builder.intern_iri(&format!("{EX}subject"));
        builder.push_quad(child, subclass, root, None);
        builder.push_quad(subject, rdf_type, root, None);
        let dataset = builder.freeze().expect("fixture freezes");
        let view = ClassMembershipView::new(Arc::clone(&dataset));

        view.prepare();
        assert_eq!(view.dimensions(), [0; 6]);
        assert_eq!(
            view.quads().collect::<Vec<_>>(),
            dataset.quads().collect::<Vec<_>>()
        );
        assert_eq!(view.len_hint(), Some(dataset.quad_count()));
        assert_eq!(view.build_count(), 1);
    }

    #[test]
    fn rdf12_side_tables_are_forwarded_verbatim() {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri(&format!("{EX}s"));
        let p = builder.intern_iri(&format!("{EX}p"));
        let o = builder.intern_iri(&format!("{EX}o"));
        let triple = builder.intern_triple(s, p, o);
        let reifier = builder.intern_iri(&format!("{EX}r"));
        let annotation_p = builder.intern_iri(&format!("{EX}source"));
        let annotation_o = builder.intern_literal(::purrdf::RdfLiteral::simple("test"));
        builder.push_reifier(reifier, triple);
        builder.push_annotation(reifier, annotation_p, annotation_o);
        let dataset = builder.freeze().expect("fixture freezes");
        let view = ClassMembershipView::new(Arc::clone(&dataset));
        assert_eq!(
            view.reifier_quads().collect::<Vec<_>>(),
            dataset.reifier_quads().collect::<Vec<_>>()
        );
        assert_eq!(
            view.annotation_quads().collect::<Vec<_>>(),
            dataset.annotation_quads().collect::<Vec<_>>()
        );
        assert_eq!(view.resolve(triple), dataset.resolve(triple));
    }

    #[test]
    fn blank_resources_and_triple_term_classes_participate_in_membership() {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri(rdf::TYPE);
        let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
        let base_s = builder.intern_iri(&format!("{EX}embedded-subject"));
        let base_p = builder.intern_iri(&format!("{EX}embedded-predicate"));
        let base_o = builder.intern_iri(&format!("{EX}embedded-object"));
        let iri_subject = builder.intern_iri(&format!("{EX}iri-subject"));
        let triple_class = builder.intern_triple(base_o, base_p, base_s);
        let blank_subject = builder.intern_blank("subject", BlankScope::DEFAULT);
        let blank_class = builder.intern_blank("class", BlankScope::DEFAULT);
        let leaf = builder.intern_iri(&format!("{EX}Leaf"));
        builder.push_quad(leaf, subclass, triple_class, None);
        builder.push_quad(leaf, subclass, blank_class, None);
        builder.push_quad(iri_subject, rdf_type, leaf, None);
        builder.push_quad(blank_subject, rdf_type, leaf, None);
        let view = ClassMembershipView::new(builder.freeze().expect("fixture freezes"));

        assert!(view.is_instance(iri_subject, triple_class));
        assert!(view.is_instance(blank_subject, blank_class));
        assert_eq!(
            view.quads_for_pattern(
                Some(iri_subject),
                Some(rdf_type),
                Some(triple_class),
                GraphMatch::Default,
            )
            .count(),
            1
        );
        assert_eq!(
            view.quads_for_pattern(
                Some(blank_subject),
                Some(rdf_type),
                Some(blank_class),
                GraphMatch::Default,
            )
            .count(),
            1
        );

        let targets = crate::sparql::eval_target_view(
            &view,
            "SELECT ?this WHERE { ?this a ?class . FILTER(isTriple(?class)) }",
            &[],
        )
        .expect("SPARQL sees the virtual triple-term class");
        assert_eq!(
            targets,
            vec![
                crate::term::term_id_to_native(view.base(), iri_subject),
                crate::term::term_id_to_native(view.base(), blank_subject),
            ]
        );
    }

    #[test]
    fn diamond_derivations_and_insertion_order_are_unique_and_deterministic() {
        fn build(reverse: bool) -> ClassMembershipView {
            let mut builder = RdfDatasetBuilder::new();
            let rdf_type = builder.intern_iri(rdf::TYPE);
            let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
            let leaf = builder.intern_iri(&format!("{EX}Leaf"));
            let left = builder.intern_iri(&format!("{EX}Left"));
            let right = builder.intern_iri(&format!("{EX}Right"));
            let top = builder.intern_iri(&format!("{EX}Top"));
            let subject = builder.intern_iri(&format!("{EX}subject"));
            let mut rows = vec![
                (leaf, subclass, left),
                (leaf, subclass, right),
                (left, subclass, top),
                (right, subclass, top),
                (subject, rdf_type, leaf),
                (subject, rdf_type, top),
            ];
            if reverse {
                rows.reverse();
            }
            for (s, p, o) in rows {
                builder.push_quad(s, p, o, None);
            }
            ClassMembershipView::new(builder.freeze().expect("fixture freezes"))
        }

        let forward = build(false);
        let reverse = build(true);
        let forward_rows: Vec<_> = forward.quads().collect();
        let reverse_rows: Vec<_> = reverse.quads().collect();
        assert_eq!(forward_rows, reverse_rows);
        assert_eq!(forward_rows, forward.quads().collect::<Vec<_>>());

        let rdf_type = forward
            .term_id_by_value(&TermValue::iri(rdf::TYPE))
            .expect("rdf:type is interned");
        let top = forward
            .term_id_by_value(&TermValue::iri(format!("{EX}Top")))
            .expect("top is interned");
        let subject = forward
            .term_id_by_value(&TermValue::iri(format!("{EX}subject")))
            .expect("subject is interned");
        assert_eq!(
            forward
                .quads_for_pattern(
                    Some(subject),
                    Some(rdf_type),
                    Some(top),
                    GraphMatch::Default,
                )
                .count(),
            1
        );
    }

    fn row_key(quad: QuadIds) -> (usize, usize, usize, Option<usize>) {
        (
            quad.s.index(),
            quad.p.index(),
            quad.o.index(),
            quad.g.map(TermId::index),
        )
    }

    fn eager_oracle(dataset: &RdfDataset, rdf_type: TermId, subclass: TermId) -> Vec<QuadIds> {
        let mut parents: BTreeMap<TermId, BTreeSet<TermId>> = BTreeMap::new();
        let mut asserted_types = BTreeSet::new();
        let mut rows: Vec<_> = dataset.quads().collect();
        for quad in dataset.quads().filter(|quad| quad.g.is_none()) {
            if quad.p == rdf_type {
                asserted_types.insert((quad.s, quad.o));
            } else if quad.p == subclass {
                parents.entry(quad.s).or_default().insert(quad.o);
            }
        }

        let mut virtual_rows = BTreeSet::new();
        for &(subject, exact_class) in &asserted_types {
            let mut seen = BTreeSet::from([exact_class]);
            let mut frontier = vec![exact_class];
            while let Some(class) = frontier.pop() {
                for &parent in parents.get(&class).into_iter().flatten() {
                    if seen.insert(parent) {
                        frontier.push(parent);
                        if !asserted_types.contains(&(subject, parent)) {
                            virtual_rows.insert((subject, parent));
                        }
                    }
                }
            }
        }
        rows.extend(virtual_rows.into_iter().map(|(s, o)| QuadIds {
            s,
            p: rdf_type,
            o,
            g: None,
        }));
        rows
    }

    fn matches_pattern(
        quad: QuadIds,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> bool {
        s.is_none_or(|bound| quad.s == bound)
            && p.is_none_or(|bound| quad.p == bound)
            && o.is_none_or(|bound| quad.o == bound)
            && g.matches(quad.g)
    }

    fn property_config() -> ProptestConfig {
        ProptestConfig {
            cases: 96,
            failure_persistence: None,
            ..ProptestConfig::default()
        }
    }

    proptest! {
        #![proptest_config(property_config())]

        #[test]
        fn every_probe_shape_matches_an_eager_oracle(
            class_count in 1usize..=5,
            subject_count in 1usize..=4,
            subclass_bits in proptest::collection::vec(any::<bool>(), 25),
            type_bits in proptest::collection::vec(any::<bool>(), 20),
            named_subclass_bits in proptest::collection::vec(any::<bool>(), 25),
            named_type_bits in proptest::collection::vec(any::<bool>(), 20),
            reverse in any::<bool>(),
        ) {
            let mut builder = RdfDatasetBuilder::new();
            let rdf_type = builder.intern_iri(rdf::TYPE);
            let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
            let other_predicate = builder.intern_iri(&format!("{EX}other"));
            let graph = builder.intern_iri(&format!("{EX}named"));
            let classes: Vec<_> = (0..class_count)
                .map(|index| builder.intern_iri(&format!("{EX}class-{index}")))
                .collect();
            let subjects: Vec<_> = (0..subject_count)
                .map(|index| builder.intern_iri(&format!("{EX}subject-{index}")))
                .collect();
            builder.declare_named_graph(graph);

            let mut rows = Vec::new();
            for child in 0..class_count {
                for parent in 0..class_count {
                    let bit = child * 5 + parent;
                    if subclass_bits[bit] {
                        rows.push((classes[child], subclass, classes[parent], None));
                    }
                    if named_subclass_bits[bit] {
                        rows.push((classes[child], subclass, classes[parent], Some(graph)));
                    }
                }
            }
            for (subject_index, &subject) in subjects.iter().enumerate() {
                for (class_index, &class) in classes.iter().enumerate() {
                    let bit = subject_index * 5 + class_index;
                    if type_bits[bit] {
                        rows.push((subject, rdf_type, class, None));
                    }
                    if named_type_bits[bit] {
                        rows.push((subject, rdf_type, class, Some(graph)));
                    }
                }
                rows.push((subject, other_predicate, classes[0], None));
            }
            if reverse {
                rows.reverse();
            }
            for (s, p, o, g) in rows {
                builder.push_quad(s, p, o, g);
            }

            let dataset = builder.freeze().expect("generated dataset freezes");
            let oracle = eager_oracle(&dataset, rdf_type, subclass);
            let view = ClassMembershipView::new(Arc::clone(&dataset));
            view.prepare();
            assert!(Arc::ptr_eq(view.base(), &dataset));

            let subject_choices = [None, Some(subjects[0]), Some(classes[0])];
            let predicate_choices = [None, Some(rdf_type), Some(other_predicate)];
            let object_choices = [None, Some(classes[0]), Some(subjects[0])];
            let graph_choices = [GraphMatch::Any, GraphMatch::Default, GraphMatch::Named(graph)];

            for s in subject_choices {
                for p in predicate_choices {
                    for o in object_choices {
                        for g in graph_choices {
                            let actual: Vec<_> = view.quads_for_pattern(s, p, o, g).collect();
                            let repeated: Vec<_> = view.quads_for_pattern(s, p, o, g).collect();
                            assert_eq!(actual, repeated, "probe ordering must be deterministic");

                            let plan = view.probe_plan(s.is_some(), p.is_some(), o.is_some(), g);
                            let planned: Vec<_> = view
                                .quads_for_pattern_with_plan(&plan, s, p, o, g)
                                .collect();
                            assert_eq!(actual, planned, "planned and ordinary probes diverged");

                            let mut actual_keys: Vec<_> = actual.iter().copied().map(row_key).collect();
                            let actual_len = actual_keys.len();
                            actual_keys.sort_unstable();
                            actual_keys.dedup();
                            assert_eq!(actual_len, actual_keys.len(), "probe emitted a duplicate row");

                            let mut expected: Vec<_> = oracle
                                .iter()
                                .copied()
                                .filter(|quad| matches_pattern(*quad, s, p, o, g))
                                .map(row_key)
                                .collect();
                            expected.sort_unstable();
                            expected.dedup();
                            assert_eq!(actual_keys, expected, "probe differed from eager closure");
                            assert!(
                                view.cardinality_estimate(s, p, o, g) >= actual_len,
                                "cardinality estimate must be a sound upper bound"
                            );
                        }
                    }
                }
            }
        }
    }
}
