// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test for per-frame provenance on the streaming read path
//! (`purrdf_gts::reader::{FrameContext, StreamingSink::frame}`).
//!
//! Cross-checks the streaming `FrameContext` events fired by
//! [`read_to_sink`] against the offline `replication::inventory` view of the
//! same bytes: both derive their per-frame `(content-id, byte range, wire
//! type, valid)` from the same frames, so they must agree field-for-field.

use purrdf_gts::model::{AnnotationRow, ByteRange, OpaqueNode, Quad, ReifierRow, Term};
use purrdf_gts::reader::{FrameContext, StreamingSink, read_to_sink};
use purrdf_gts::replication::inventory;
use purrdf_gts::writer::Writer;

/// Owned copy of one streamed [`FrameContext`], captured for later assertion.
struct CapturedFrame {
    segment_index: usize,
    frame_index: usize,
    content_id: Vec<u8>,
    start: usize,
    end: usize,
    frame_type: String,
    valid: bool,
}

/// A [`StreamingSink`] that records every [`FrameContext`] it receives, in
/// arrival order, and nothing else.
#[derive(Default)]
struct CapturingSink {
    frames: Vec<CapturedFrame>,
}

impl StreamingSink for CapturingSink {
    fn frame(&mut self, ctx: FrameContext<'_>) {
        self.frames.push(CapturedFrame {
            segment_index: ctx.segment_index,
            frame_index: ctx.frame_index,
            content_id: ctx.content_id.to_vec(),
            start: ctx.range.start,
            end: ctx.range.end,
            frame_type: ctx.frame_type.to_string(),
            valid: ctx.valid,
        });
    }
}

/// A 2-segment fixture: two `Writer::new("generic")` blob-writer outputs
/// concatenated, mirroring the fixture in `replication.rs`'s own tests.
fn two_segment_fixture() -> Vec<u8> {
    let mut first = Writer::new("generic");
    first.add_blob(b"a", None, None);
    first.add_blob(b"b", None, None);
    let mut data = first.to_bytes();

    let mut second = Writer::new("generic");
    second.add_blob(b"c", None, None);
    data.extend_from_slice(&second.to_bytes());
    data
}

#[test]
fn streaming_frame_provenance_matches_offline_inventory() {
    let data = two_segment_fixture();

    let mut sink = CapturingSink::default();
    let result = read_to_sink(&data, true, None, &mut sink);
    assert!(result.diagnostics.is_empty(), "fixture must fold cleanly");

    let inv = inventory(&data);
    assert!(!inv.has_problems(), "fixture must inventory cleanly");

    let expected_total: usize = inv.segments.iter().map(|s| s.frames.len()).sum();
    assert_eq!(sink.frames.len(), expected_total);

    // Captured frames arrive in file order, one-to-one with the inventory's
    // segment/frame ordering.
    let mut captured = sink.frames.iter();
    for segment in &inv.segments {
        for frame in &segment.frames {
            let ctx = captured
                .next()
                .expect("streaming sink under-reported frames");
            assert_eq!(ctx.segment_index, segment.index);
            assert_eq!(ctx.frame_index, frame.frame_index);
            assert_eq!(ctx.content_id, frame.id);
            assert_eq!(ctx.start, frame.start);
            assert_eq!(ctx.end, frame.end);
            assert_eq!(ctx.frame_type, frame.frame_type);
            assert_eq!(ctx.valid, frame.valid);
        }
    }
    assert!(
        captured.next().is_none(),
        "streaming sink over-reported frames"
    );

    // Every valid frame's provenance is independently re-verifiable against
    // the source bytes.
    for ctx in &sink.frames {
        if ctx.valid {
            let recomputed = FrameContext {
                segment_index: ctx.segment_index,
                frame_index: ctx.frame_index,
                content_id: &ctx.content_id,
                range: ByteRange {
                    start: ctx.start,
                    end: ctx.end,
                },
                frame_type: &ctx.frame_type,
                valid: ctx.valid,
            };
            assert!(
                recomputed.verify(&data),
                "valid frame at segment {} frame {} should verify",
                ctx.segment_index,
                ctx.frame_index
            );
        }
    }
}

// -- frame-before-rows ordering ----------------------------------------------

/// A [`StreamingSink`] that records every event — `frame()` and every row
/// callback — into ONE interleaved log, tagging each entry so the test can
/// tell frame markers from row events.
#[derive(Default)]
struct OrderingSink {
    log: Vec<String>,
}

impl StreamingSink for OrderingSink {
    fn frame(&mut self, ctx: FrameContext<'_>) {
        self.log
            .push(format!("frame:{}:{}", ctx.segment_index, ctx.frame_index));
    }
    fn term(&mut self, segment_index: usize, term_id: usize, _term: &Term) {
        self.log.push(format!("term:{segment_index}:{term_id}"));
    }
    fn quad(&mut self, segment_index: usize, _quad: Quad) {
        self.log.push(format!("quad:{segment_index}"));
    }
    fn reifier(&mut self, segment_index: usize, _reifier: ReifierRow) {
        self.log.push(format!("reifier:{segment_index}"));
    }
    fn annotation(&mut self, segment_index: usize, _annotation: AnnotationRow) {
        self.log.push(format!("annotation:{segment_index}"));
    }
    fn blob(
        &mut self,
        segment_index: usize,
        _digest: &str,
        _meta: Option<&ciborium::value::Value>,
    ) {
        self.log.push(format!("blob:{segment_index}"));
    }
    fn opaque(&mut self, segment_index: usize, _opaque: &OpaqueNode) {
        self.log.push(format!("opaque:{segment_index}"));
    }
}

/// A single-segment fixture with three frames, each producing a known,
/// non-empty set of rows: a `terms` frame (3 term rows), a `quads` frame (1
/// quad row), and a `blob` frame (1 blob row).
fn three_frame_fixture() -> Vec<u8> {
    use purrdf_gts::model::TermKind;

    let mut w = Writer::new("generic");
    w.add_terms(&[
        Term {
            kind: TermKind::Iri,
            value: Some("http://example.org/s".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("http://example.org/p".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Literal,
            value: Some("o".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
    ]);
    w.add_quads(&[(0, 1, 2, None)]);
    w.add_blob(b"provenance-ordering-fixture", None, None);
    w.to_bytes()
}

/// Falsifiable regression test for the `StreamingSink::frame` contract:
/// "fired once per frame **before its rows are processed**"
/// (`purrdf_gts::reader::StreamingSink::frame`).
///
/// Drives [`read_to_sink`] over a fixture whose three frames each carry a
/// known, non-empty row set, and asserts the interleaved event log is
/// exactly `[frame, row*]` per frame, in file order. If `frame()` ever fires
/// after (or interleaved with a gap after) that frame's own rows, the block
/// boundary this test walks will not start with a `frame:` tag and the
/// assertion below fails.
#[test]
fn frame_event_precedes_its_own_row_events_for_every_frame() {
    let data = three_frame_fixture();

    let mut sink = OrderingSink::default();
    let result = read_to_sink(&data, false, None, &mut sink);
    assert!(result.diagnostics.is_empty(), "fixture must fold cleanly");

    // Expected `(frame tag, row count)` blocks in file order.
    let expected_blocks = [("frame:0:0", 3usize), ("frame:0:1", 1), ("frame:0:2", 1)];

    let mut cursor = 0usize;
    for (frame_tag, row_count) in expected_blocks {
        assert_eq!(
            sink.log.get(cursor).map(String::as_str),
            Some(frame_tag),
            "expected {frame_tag} at log position {cursor}; full log: {:?}",
            sink.log
        );
        cursor += 1;
        for offset in 0..row_count {
            let entry = sink.log.get(cursor + offset).map(String::as_str);
            assert!(
                entry.is_some_and(|tag| !tag.starts_with("frame:")),
                "row #{offset} of {frame_tag} was not a row event (got {entry:?}); \
                 frame() must fire before its rows — full log: {:?}",
                sink.log
            );
        }
        cursor += row_count;
    }
    assert_eq!(
        cursor,
        sink.log.len(),
        "unexpected trailing events after the last frame's rows: {:?}",
        sink.log
    );
}
