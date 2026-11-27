// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrow an ordered run from one already-resident graph page.

use super::admission::{LocalPattern, PageAdmission, admit_pattern, candidate_pages_for_stream};
use super::{PageStream, PageTranslation, PagedDataset};
use crate::{
    GlobalTermId, GraphMatch, QuadIds, RdfDataset, StatisticsStream, SubjectOrderedScan, TermId,
};

pub(super) enum Scope<'a> {
    Empty,
    Page {
        data: &'a RdfDataset,
        translation: &'a PageTranslation,
        pattern: LocalPattern,
    },
}

impl PagedDataset {
    pub(super) fn resident_subject_scope(
        &self,
        predicate: Option<GlobalTermId>,
        object: Option<GlobalTermId>,
        graph: GraphMatch<GlobalTermId>,
    ) -> Option<Scope<'_>> {
        if predicate.is_none() || object.is_none() || matches!(graph, GraphMatch::Any) {
            return None;
        }
        let postings = |stream| {
            candidate_pages_for_stream(self.graph_index(), self.page_count(), graph, stream)
        };
        if postings(PageStream::Reifier).next().is_some()
            || postings(PageStream::Annotation).next().is_some()
        {
            return None;
        }
        let mut asserted = postings(PageStream::Base);
        let Some(id) = asserted.next() else {
            return Some(Scope::Empty);
        };
        if asserted.next().is_some() {
            return None;
        }
        let slot = &self.pages[usize::try_from(id.0).expect("resident page index fits usize")];
        let translation = &slot.translation;
        let PageAdmission::Admit(pattern) =
            admit_pattern(translation, None, predicate, object, graph)
        else {
            return Some(Scope::Empty);
        };
        if !translation.preserves_order() {
            return None;
        }
        let data = slot.resident.get()?;
        if !data.native_subject_order_available(pattern.p, pattern.o, pattern.g) {
            return None;
        }
        Some(Scope::Page {
            data,
            translation,
            pattern,
        })
    }
}

impl<'a> Scope<'a> {
    pub(super) fn rows(
        self,
        stream: StatisticsStream,
        first: Option<GlobalTermId>,
        last: Option<GlobalTermId>,
    ) -> Option<impl SubjectOrderedScan<GlobalTermId> + 'a> {
        let mut inner = None;
        // The graph has no statement-layer postings, including in the page
        // whose asserted rows match. All three streams are supported.
        if let (
            StatisticsStream::Asserted,
            Self::Page {
                data,
                translation,
                pattern,
            },
        ) = (stream, self)
        {
            let ids = translation.global_ids();
            let start = first.map_or(0, |lower| ids.partition_point(|id| *id < lower));
            let end = last.map_or(ids.len(), |upper| ids.partition_point(|id| *id <= upper));
            if start < end {
                inner = Some((
                    data.native_ordered_subject_quads(
                        stream,
                        first.map(|_| translation.ordered_local_at(start)),
                        last.map(|_| translation.ordered_local_at(end - 1)),
                        pattern.p,
                        pattern.o,
                        pattern.g,
                    )?,
                    translation,
                ));
            }
        }
        Some(Rows { inner })
    }

    pub(super) fn upper_bound(self) -> Option<GlobalTermId> {
        let Self::Page {
            data,
            translation,
            pattern,
        } = self
        else {
            return None;
        };
        data.native_subject_upper_bound(pattern.p, pattern.o, pattern.g)
            .map(|id| translation.to_global(id))
    }
}

struct Rows<'a, S> {
    inner: Option<(S, &'a PageTranslation)>,
}

impl<S: SubjectOrderedScan<TermId>> Iterator for Rows<'_, S> {
    type Item = QuadIds<GlobalTermId>;

    fn next(&mut self) -> Option<Self::Item> {
        let (scan, translation) = self.inner.as_mut()?;
        scan.next()
            .map(|quad| quad.map_ids(|id| translation.to_global(id)))
    }
}

impl<S: SubjectOrderedScan<TermId>> SubjectOrderedScan<GlobalTermId> for Rows<'_, S> {
    fn advance_to(&mut self, minimum: GlobalTermId) {
        if let Some((scan, translation)) = &mut self.inner {
            let ids = translation.global_ids();
            let index = ids.partition_point(|id| *id < minimum);
            if index == ids.len() {
                self.inner = None;
            } else {
                scan.advance_to(translation.ordered_local_at(index));
            }
        }
    }
}
