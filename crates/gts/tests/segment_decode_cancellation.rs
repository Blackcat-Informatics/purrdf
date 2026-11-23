// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cancelling nested term resolution never publishes a partial term id.

use std::sync::Arc;

use purrdf_gts::dictionary::TermKey;
use purrdf_gts::model::{Term, Triple3};
use purrdf_gts::reader::StreamingSink;
use purrdf_gts::segment_decode::{
    PendingResolution, ResidentResolutionStorage, ResolutionCounts, ResolutionStorage,
    ResolvedSink, SegmentResolver,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CancelPoint {
    #[default]
    Never,
    Intern,
    Quad,
    Reifier,
    Annotation,
}

#[derive(Default)]
struct CancellingSink {
    cancelled: bool,
    cancel_at: CancelPoint,
    interns: usize,
    quads: usize,
    reifiers: usize,
    annotations: usize,
}

impl ResolvedSink for CancellingSink {
    type Id = usize;
    type Error = String;

    fn continue_reading(&self) -> bool {
        !self.cancelled
    }

    fn intern_iri(
        &mut self,
        _segment_index: u64,
        gts_id: usize,
        _iri: &str,
    ) -> Result<Self::Id, Self::Error> {
        self.interns += 1;
        self.cancelled |= self.cancel_at == CancelPoint::Intern;
        Ok(gts_id)
    }

    fn intern_blank(
        &mut self,
        _segment_index: u64,
        gts_id: usize,
        _label: &str,
    ) -> Result<Self::Id, Self::Error> {
        Ok(gts_id)
    }

    fn intern_literal(
        &mut self,
        _segment_index: u64,
        gts_id: usize,
        _lexical: String,
        _datatype: Option<Self::Id>,
        _lang: Option<String>,
        _direction: Option<String>,
    ) -> Result<Self::Id, Self::Error> {
        Ok(gts_id)
    }

    fn intern_triple(
        &mut self,
        _segment_index: u64,
        gts_id: usize,
        _s: Self::Id,
        _p: Self::Id,
        _o: Self::Id,
    ) -> Result<Self::Id, Self::Error> {
        Ok(gts_id)
    }

    fn push_quad(
        &mut self,
        _segment_index: u64,
        _s: Self::Id,
        _p: Self::Id,
        _o: Self::Id,
        _g: Option<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.quads += 1;
        self.cancelled |= self.cancel_at == CancelPoint::Quad;
        Ok(())
    }

    fn push_reifier(
        &mut self,
        _segment_index: u64,
        _reifier: Self::Id,
        _s: Self::Id,
        _p: Self::Id,
        _o: Self::Id,
        _g: Option<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.reifiers += 1;
        self.cancelled |= self.cancel_at == CancelPoint::Reifier;
        Ok(())
    }

    fn push_annotation(
        &mut self,
        _segment_index: u64,
        _reifier: Self::Id,
        _p: Self::Id,
        _o: Self::Id,
        _g: Option<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.annotations += 1;
        self.cancelled |= self.cancel_at == CancelPoint::Annotation;
        Ok(())
    }

    fn err_dangling_term(&self, segment_index: u64, gts_id: usize, role: &str) -> Self::Error {
        format!("segment {segment_index} term {gts_id}: {role}")
    }

    fn err_nesting_limit(&self, segment_index: u64, gts_id: usize) -> Self::Error {
        format!("segment {segment_index} term {gts_id}: nesting")
    }

    fn err_unbound_triple(&self, segment_index: u64, gts_id: usize) -> Self::Error {
        format!("segment {segment_index} term {gts_id}: unbound")
    }

    fn err_missing_reifier(&self, segment_index: u64, reifier: usize) -> Self::Error {
        format!("segment {segment_index} reifier {reifier}: missing")
    }
}

#[derive(Default)]
struct TrackingStorage {
    inner: ResidentResolutionStorage<usize>,
    segment_one_writes: usize,
    completed: usize,
}

impl ResolutionStorage<usize> for TrackingStorage {
    type Error = std::convert::Infallible;

    fn pending(&mut self, key: TermKey) -> Result<Option<PendingResolution>, Self::Error> {
        self.inner.pending(key)
    }

    fn set_pending(&mut self, key: TermKey, value: PendingResolution) -> Result<(), Self::Error> {
        if key.segment == 1 {
            self.segment_one_writes += 1;
        }
        self.inner.set_pending(key, value)
    }

    fn remove_pending(&mut self, key: TermKey) -> Result<(), Self::Error> {
        self.inner.remove_pending(key)
    }

    fn next_term(
        &mut self,
        segment: u64,
        after: Option<usize>,
    ) -> Result<Option<usize>, Self::Error> {
        self.inner.next_term(segment, after)
    }

    fn resolved(&mut self, key: TermKey) -> Result<Option<usize>, Self::Error> {
        self.inner.resolved(key)
    }

    fn set_resolved(&mut self, key: TermKey, id: usize) -> Result<(), Self::Error> {
        self.inner.set_resolved(key, id)
    }

    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, Self::Error> {
        self.inner.first_binding(key)
    }

    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), Self::Error> {
        self.inner.bind_first(key, triple)
    }

    fn complete_segment(&mut self, segment: u64) -> Result<(), Self::Error> {
        self.completed += 1;
        self.inner.complete_segment(segment)
    }

    fn counts(&self) -> ResolutionCounts {
        self.inner.counts()
    }
}

fn nested_terms(resolver: &mut SegmentResolver<CancellingSink>) {
    resolver.term(0, 0, Arc::new(Term::triple_term((1, 2, 3))));
    resolver.term(0, 1, Arc::new(Term::iri("https://example.org/s")));
    resolver.term(0, 2, Arc::new(Term::iri("https://example.org/p")));
    resolver.term(0, 3, Arc::new(Term::iri("https://example.org/o")));
}

fn impossible(error: std::convert::Infallible) -> String {
    match error {}
}

#[test]
fn cancellation_during_nested_interning_publishes_no_quad() {
    let mut resolver = SegmentResolver::new(CancellingSink {
        cancel_at: CancelPoint::Intern,
        ..CancellingSink::default()
    });
    nested_terms(&mut resolver);
    resolver.quad(0, (0, 2, 3, None));
    resolver
        .finish()
        .expect("consumer cancellation is a terminal non-error for a streamed drive");
    assert!(resolver.sink().cancelled);
    assert_eq!(resolver.sink().quads, 0);
}

#[test]
fn selected_resolution_reports_cancellation_instead_of_a_child_id() {
    let mut resolver = SegmentResolver::new(CancellingSink {
        cancel_at: CancelPoint::Intern,
        ..CancellingSink::default()
    });
    nested_terms(&mut resolver);
    let error = resolver
        .resolve_selected(TermKey {
            segment: 0,
            term: 0,
        })
        .expect_err("a cancelled parent resolution has no selected term id");
    assert!(error.contains("resolution cancelled"));
}

#[test]
fn selected_resolution_rejects_cached_and_pending_terms_after_cancellation() {
    let key = TermKey {
        segment: 0,
        term: 0,
    };
    let mut cached = SegmentResolver::new(CancellingSink::default());
    cached.term(0, 0, Arc::new(Term::iri("https://example.org/cached")));
    assert_eq!(cached.resolve_selected(key).unwrap(), 0);
    cached.sink_mut().cancelled = true;
    let error = cached
        .resolve_selected(key)
        .expect_err("a cached id is not published after cancellation");
    assert!(error.contains("resolution cancelled"));
    assert_eq!(cached.sink().interns, 1);

    let mut pending = SegmentResolver::new(CancellingSink::default());
    pending.term(0, 0, Arc::new(Term::iri("https://example.org/pending")));
    pending.sink_mut().cancelled = true;
    let error = pending
        .resolve_selected(key)
        .expect_err("a pending term is not interned after cancellation");
    assert!(error.contains("resolution cancelled"));
    assert_eq!(pending.sink().interns, 0);
}

#[test]
fn cancellation_during_a_segment_flush_rejects_the_next_segment() {
    let sink = CancellingSink {
        cancel_at: CancelPoint::Quad,
        ..CancellingSink::default()
    };
    let mut resolver = SegmentResolver::with_storage(sink, TrackingStorage::default(), impossible);
    for (id, name) in ["s", "p", "o"].into_iter().enumerate() {
        resolver.term(
            0,
            id,
            Arc::new(Term::iri(format!("https://example.org/{name}"))),
        );
    }
    resolver.quad(0, (0, 1, 2, None));
    resolver.term(1, 0, Arc::new(Term::iri("https://example.org/next")));
    let (sink, storage) = resolver.into_parts();
    assert!(sink.cancelled);
    assert_eq!(storage.segment_one_writes, 0);
    assert_eq!(storage.completed, 0);
}

#[test]
fn cancellation_in_each_row_phase_stops_later_phases_and_completion() {
    for phase in 0..3 {
        let sink = CancellingSink {
            cancel_at: [
                CancelPoint::Quad,
                CancelPoint::Reifier,
                CancelPoint::Annotation,
            ][phase],
            ..CancellingSink::default()
        };
        let mut resolver =
            SegmentResolver::with_storage(sink, TrackingStorage::default(), impossible);
        for (id, name) in ["s", "p", "o", "r"].into_iter().enumerate() {
            resolver.term(
                0,
                id,
                Arc::new(Term::iri(format!("https://example.org/{name}"))),
            );
        }
        resolver.quad(0, (0, 1, 2, None));
        resolver.reifier(0, (3, (0, 1, 2), None));
        resolver.annotation(0, (3, 1, 2, None));
        resolver
            .finish()
            .expect("consumer cancellation remains a terminal non-error");
        let (sink, storage) = resolver.into_parts();
        assert_eq!(sink.quads, 1);
        assert_eq!(sink.reifiers, usize::from(phase >= 1));
        assert_eq!(sink.annotations, usize::from(phase >= 2));
        assert_eq!(storage.completed, 0);
    }
}
