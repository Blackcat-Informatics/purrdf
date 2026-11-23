// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cancelling nested term resolution never publishes a partial term id.

use std::sync::Arc;

use purrdf_gts::dictionary::TermKey;
use purrdf_gts::model::Term;
use purrdf_gts::reader::StreamingSink;
use purrdf_gts::segment_decode::{ResolvedSink, SegmentResolver};

#[derive(Default)]
struct CancellingSink {
    cancelled: bool,
    quads: usize,
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
        self.cancelled = true;
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

fn nested_terms(resolver: &mut SegmentResolver<CancellingSink>) {
    resolver.term(0, 0, Arc::new(Term::triple_term((1, 2, 3))));
    resolver.term(0, 1, Arc::new(Term::iri("https://example.org/s")));
    resolver.term(0, 2, Arc::new(Term::iri("https://example.org/p")));
    resolver.term(0, 3, Arc::new(Term::iri("https://example.org/o")));
}

#[test]
fn cancellation_during_nested_interning_publishes_no_quad() {
    let mut resolver = SegmentResolver::new(CancellingSink::default());
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
    let mut resolver = SegmentResolver::new(CancellingSink::default());
    nested_terms(&mut resolver);
    let error = resolver
        .resolve_selected(TermKey {
            segment: 0,
            term: 0,
        })
        .expect_err("a cancelled parent resolution has no selected term id");
    assert!(error.contains("resolution cancelled"));
}
