// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test for per-frame provenance on the streaming read path
//! (`purrdf_gts::reader::{FrameContext, StreamingSink::frame}`).
//!
//! Cross-checks the streaming `FrameContext` events fired by
//! [`read_to_sink`] against the offline `replication::inventory` view of the
//! same bytes: both derive their per-frame `(content-id, byte range, wire
//! type, valid)` from the same frames, so they must agree field-for-field.

use purrdf_gts::model::ByteRange;
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
