// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Transferable lazy reads through the original dataset probe implementations.
//!
//! [`DatasetView`] permits a read iterator that remains on its opening thread.
//! A row cursor held between worker requests needs the stronger `Send` contract
//! here. Each iterator borrows the external view, rather than a movable query
//! context. No iterator is reopened or replayed between reads.

use crate::{AnnotationRangeSource, DatasetView, GraphMatch, QuadIds, WorkspaceReservation};

/// A dataset whose lazy row and graph probes can move between worker threads.
///
/// Every method has exactly its [`DatasetView`] counterpart's rows, order,
/// admission, pins and sticky-error behavior. The additional guarantee is that
/// the retained iterator is `Send`. Operational consumers must still check
/// [`DatasetView::read_error`] around reads and their final status on settlement.
pub trait CursorDatasetView: DatasetView + Sync {
    /// Retain the original execution permit across worker requests.
    ///
    /// # Errors
    /// Returns the original source admission refusal before any query read.
    fn cursor_reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + Send + '_, Self::ReadError>;

    /// Open the original quad probe, copying its loop-invariant plan.
    fn cursor_quads_for_pattern_with_plan(
        &self,
        plan: Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_;

    /// Open the original subject-keyed reifier run.
    fn cursor_reifier_quads_of(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_;

    /// Open the original target-keyed reifier run.
    fn cursor_reifier_quads_for_target(
        &self,
        target: Self::Id,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_;

    /// Open the original reifier component probe.
    fn cursor_reifier_quads_for_components(
        &self,
        components: [Option<Self::Id>; 3],
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_;

    /// Open the original graph-scoped reifier stream.
    fn cursor_reifier_quads_in_graph(
        &self,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_;

    /// Open the original subject-keyed annotation run.
    fn cursor_annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + Send + '_;

    /// Open the original predicate/object annotation probe.
    fn cursor_annotation_quads_for_pattern(
        &self,
        predicate: Option<Self::Id>,
        object: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_;

    /// Open the original annotation probe with its admitted value-range pruning.
    fn cursor_annotation_quads_for_range<'a, R: AnnotationRangeSource + 'a>(
        &'a self,
        predicate: Option<Self::Id>,
        object: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
        range: R,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + 'a;

    /// Open the original named-graph enumeration, including empty declarations.
    fn cursor_named_graphs(&self) -> impl Iterator<Item = Self::Id> + Send + '_;
}

// Concrete backends reveal their iterators' auto traits without imposing a new
// bound on existing DatasetView implementations. These adapters call the one
// existing scan body; they introduce no second matching or visibility law.
macro_rules! cursor_dataset {
    (@probe planned $source:ident, $plan:ident, $s:ident, $p:ident, $o:ident, $graph:ident) => {
        $source.quads_for_pattern_with_plan(&$plan, $s, $p, $o, $graph)
    };
    (@probe unit $source:ident, $plan:ident, $s:ident, $p:ident, $o:ident, $graph:ident) => {{
        let () = $plan;
        DatasetView::quads_for_pattern($source, $s, $p, $o, $graph)
    }};
    (@probe triple $source:ident, $plan:ident, $s:ident, $p:ident, $o:ident, $graph:ident) => {
        $source.planned_rows($plan, [$s, $p, $o], $graph)
    };
    (@range plain $source:ident, $predicate:ident, $object:ident, $graph:ident, $range:ident) => {{
        let _ = $range;
        DatasetView::annotation_quads_for_pattern($source, $predicate, $object, $graph)
    }};
    (@range retained $source:ident, $predicate:ident, $object:ident, $graph:ident, $range:ident) => {
        $source.annotation_quads_for_range_owned($predicate, $object, $graph, $range)
    };
    ($view:ty, $probe:ident, $range:ident) => {
        impl CursorDatasetView for $view {
            fn cursor_reserve_workspace(
                &self,
                bytes: u64,
            ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + Send + '_, Self::ReadError> {
                DatasetView::reserve_workspace(self, bytes)
            }

            fn cursor_quads_for_pattern_with_plan(
                &self,
                plan: Self::ProbePlan,
                s: Option<Self::Id>,
                p: Option<Self::Id>,
                o: Option<Self::Id>,
                graph: GraphMatch<Self::Id>,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
                cursor_dataset!(@probe $probe self, plan, s, p, o, graph)
            }

            fn cursor_reifier_quads_of(
                &self,
                reifier: Self::Id,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
                DatasetView::reifier_quads_of(self, reifier)
            }

            fn cursor_reifier_quads_for_target(
                &self,
                target: Self::Id,
                graph: GraphMatch<Self::Id>,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
                DatasetView::reifier_quads_for_target(self, target, graph)
            }

            fn cursor_reifier_quads_for_components(
                &self,
                components: [Option<Self::Id>; 3],
                graph: GraphMatch<Self::Id>,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
                DatasetView::reifier_quads_for_components(self, components, graph)
            }

            fn cursor_reifier_quads_in_graph(
                &self,
                graph: GraphMatch<Self::Id>,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
                DatasetView::reifier_quads_in_graph(self, graph)
            }

            fn cursor_annotations_of_with_graph(
                &self,
                reifier: Self::Id,
            ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + Send + '_ {
                DatasetView::annotations_of_with_graph(self, reifier)
            }

            fn cursor_annotation_quads_for_pattern(
                &self,
                predicate: Option<Self::Id>,
                object: Option<Self::Id>,
                graph: GraphMatch<Self::Id>,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
                DatasetView::annotation_quads_for_pattern(self, predicate, object, graph)
            }

            fn cursor_annotation_quads_for_range<'a, R: AnnotationRangeSource + 'a>(
                &'a self,
                predicate: Option<Self::Id>,
                object: Option<Self::Id>,
                graph: GraphMatch<Self::Id>,
                range: R,
            ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + 'a {
                cursor_dataset!(@range $range self, predicate, object, graph, range)
            }

            fn cursor_named_graphs(&self) -> impl Iterator<Item = Self::Id> + Send + '_ {
                DatasetView::named_graphs(self)
            }
        }
    };
}

cursor_dataset!(crate::RdfDataset, planned, plain);
cursor_dataset!(crate::CompositeDatasetView, planned, plain);
cursor_dataset!(crate::DeltaDatasetView, planned, plain);
cursor_dataset!(crate::PackView<'_>, unit, plain);
cursor_dataset!(crate::PagedDataset, unit, retained);
cursor_dataset!(crate::PagedQueryView<'_>, unit, retained);
cursor_dataset!(crate::PagedStackQueryView<'_>, unit, retained);
cursor_dataset!(crate::SegmentedSession, unit, retained);
cursor_dataset!(crate::SegmentedStackQuery<'_>, unit, retained);
cursor_dataset!(crate::TripleRead<'_>, triple, plain);

impl<T: CursorDatasetView + Send> CursorDatasetView for std::sync::Arc<T> {
    fn cursor_reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + Send + '_, Self::ReadError>
    {
        (**self).cursor_reserve_workspace(bytes)
    }

    fn cursor_quads_for_pattern_with_plan(
        &self,
        plan: Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
        (**self).cursor_quads_for_pattern_with_plan(plan, s, p, o, graph)
    }

    fn cursor_reifier_quads_of(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
        (**self).cursor_reifier_quads_of(reifier)
    }

    fn cursor_reifier_quads_for_target(
        &self,
        target: Self::Id,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
        (**self).cursor_reifier_quads_for_target(target, graph)
    }

    fn cursor_reifier_quads_for_components(
        &self,
        components: [Option<Self::Id>; 3],
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
        (**self).cursor_reifier_quads_for_components(components, graph)
    }

    fn cursor_reifier_quads_in_graph(
        &self,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
        (**self).cursor_reifier_quads_in_graph(graph)
    }

    fn cursor_annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + Send + '_ {
        (**self).cursor_annotations_of_with_graph(reifier)
    }

    fn cursor_annotation_quads_for_pattern(
        &self,
        predicate: Option<Self::Id>,
        object: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + '_ {
        (**self).cursor_annotation_quads_for_pattern(predicate, object, graph)
    }

    fn cursor_annotation_quads_for_range<'a, R: AnnotationRangeSource + 'a>(
        &'a self,
        predicate: Option<Self::Id>,
        object: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
        range: R,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + Send + 'a {
        (**self).cursor_annotation_quads_for_range(predicate, object, graph, range)
    }

    fn cursor_named_graphs(&self) -> impl Iterator<Item = Self::Id> + Send + '_ {
        (**self).cursor_named_graphs()
    }
}
