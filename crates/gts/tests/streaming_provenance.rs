// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integration test for per-frame provenance on the streaming read path
//! (`purrdf_gts::reader::{FrameContext, StreamingSink::frame}`).
//!
//! Cross-checks the streaming `FrameContext` events fired by
//! [`read_to_sink`] against the offline `replication::inventory` view of the
//! same bytes: both derive their per-frame `(content-id, byte range, wire
//! type, valid)` from the same frames, so they must agree field-for-field.

use purrdf_gts::dictionary::{DictionaryError, TermKey, TermLease, TermStorage};
use purrdf_gts::model::{AnnotationRow, ByteRange, OpaqueNode, Quad, ReifierRow, Term};
use purrdf_gts::reader::{FrameContext, StreamingSink, read_to_sink};
use purrdf_gts::reader::{ReadOptions, read_to_sink_with_dictionary};
use purrdf_gts::replication::inventory;
use purrdf_gts::writer::Writer;
use purrdf_lex::allocation::{Ceiling, Memory, OwnedAdmission, SharedOwned};
use std::sync::Arc;

/// Owned copy of one streamed [`FrameContext`], captured for later assertion.
struct CapturedFrame {
    segment_index: u64,
    frame_index: u64,
    content_id: Vec<u8>,
    start: u64,
    end: u64,
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
    leases: Vec<TermLease>,
    refuse_scratch: bool,
    replay_scratch: bool,
}

#[derive(Default)]
struct LocalReplay(std::io::Cursor<Vec<u8>>);
impl std::io::Read for LocalReplay {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.0, bytes)
    }
}
impl std::io::Write for LocalReplay {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Seek for LocalReplay {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, position)
    }
}
impl purrdf_lex::cbor::ReplayStorage for LocalReplay {
    fn finish(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl OrderingSink {
    fn record_term(&mut self, segment: u64, term: usize) {
        self.log.push(format!("term:{segment}:{term}"));
    }
}

impl StreamingSink for OrderingSink {
    fn frame_scratch(
        &mut self,
    ) -> Option<std::io::Result<Box<dyn purrdf_lex::cbor::ReplayStorage>>> {
        if self.replay_scratch {
            return Some(Ok(Box::new(LocalReplay::default())));
        }
        self.refuse_scratch.then(|| {
            Err(purrdf_hash::error::io_error(
                std::io::ErrorKind::Other,
                purrdf_testkit::error_capture::CallerRefusal { limit: u128::MAX },
            ))
        })
    }
    fn frame(&mut self, ctx: FrameContext<'_>) {
        self.log
            .push(format!("frame:{}:{}", ctx.segment_index, ctx.frame_index));
    }
    fn term(&mut self, segment_index: u64, term_id: usize, _term: Arc<Term>) {
        self.record_term(segment_index, term_id);
    }
    fn stored_term(
        &mut self,
        segment: u64,
        id: usize,
        term: TermLease,
    ) -> Result<(), DictionaryError> {
        self.record_term(segment, id);
        self.leases.push(term);
        Ok(())
    }
    fn quad(&mut self, segment_index: u64, _quad: Quad) {
        self.log.push(format!("quad:{segment_index}"));
    }
    fn reifier(&mut self, segment_index: u64, _reifier: ReifierRow) {
        self.log.push(format!("reifier:{segment_index}"));
    }
    fn annotation(&mut self, segment_index: u64, _annotation: AnnotationRow) {
        self.log.push(format!("annotation:{segment_index}"));
    }
    fn blob(&mut self, segment_index: u64, _digest: &str, _meta: Option<&purrdf_lex::cbor::Value>) {
        self.log.push(format!("blob:{segment_index}"));
    }
    fn opaque(&mut self, segment_index: u64, _opaque: &OpaqueNode) {
        self.log.push(format!("opaque:{segment_index}"));
    }
}

struct TermAccount {
    dropped: Arc<std::sync::atomic::AtomicUsize>,
    account: OwnedAdmission<Ceiling>,
}

impl Drop for TermAccount {
    fn drop(&mut self) {
        self.dropped
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Default)]
struct StoredTerms {
    terms: std::collections::BTreeMap<TermKey, TermLease>,
    bindings: std::collections::BTreeMap<TermKey, purrdf_gts::model::Triple3>,
    dropped: Arc<std::sync::atomic::AtomicUsize>,
    fail_read: bool,
    omit_read: bool,
}

impl TermStorage for StoredTerms {
    fn limits(&mut self) -> Result<purrdf_gts::dictionary::DictionaryLimits, DictionaryError> {
        Ok(purrdf_gts::dictionary::DictionaryLimits {
            encoded_frame_bytes: 4096,
            decoded_frame_bytes: 4096,
            metadata_entries: 4096,
        })
    }
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError> {
        let mut owner = TermAccount {
            dropped: Arc::clone(&self.dropped),
            account: OwnedAdmission::new(Ceiling(4096)),
        };
        let mut memory = Memory::new(&mut owner.account);
        let copied = Term {
            kind: term.kind,
            value: term
                .value
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            datatype: term.datatype,
            lang: term
                .lang
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            direction: term
                .direction
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            reifier: term.reifier,
            triple: term.triple,
        };
        memory.add_bytes(SharedOwned::<Term>::allocation_layout::<TermAccount>().size())?;
        self.terms
            .insert(key, TermLease::try_from_admitted(copied, owner)?);
        Ok(())
    }

    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError> {
        if self.fail_read {
            return Err(purrdf_hash::error::io_error(
                std::io::ErrorKind::Other,
                purrdf_testkit::error_capture::CallerRefusal { limit: u128::MAX },
            )
            .into());
        }
        Ok((!self.omit_read)
            .then(|| self.terms.get(&key).cloned())
            .flatten())
    }

    fn checkpoint(&mut self, _units: u64) -> Result<(), DictionaryError> {
        Ok(())
    }
    fn commit(&mut self) -> Result<(), DictionaryError> {
        Ok(())
    }

    fn first_binding(
        &mut self,
        key: TermKey,
    ) -> Result<Option<purrdf_gts::model::Triple3>, DictionaryError> {
        Ok(self.bindings.get(&key).copied())
    }

    fn bind_first(
        &mut self,
        key: TermKey,
        triple: purrdf_gts::model::Triple3,
    ) -> Result<(), DictionaryError> {
        self.bindings.entry(key).or_insert(triple);
        Ok(())
    }
}

#[test]
fn stored_reader_preserves_order_and_pins_and_returns_original_storage_failures() {
    struct NativeRefusal;
    impl std::io::Read for NativeRefusal {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(purrdf_hash::error::io_error(
                std::io::ErrorKind::Other,
                DictionaryError::Corrupt("original source refusal"),
            ))
        }
    }
    // A caller source may carry any native cause. Its type must not select the
    // external-storage protocol or break the resident reader's torn contract.
    let resident_refusal = purrdf_gts::reader::read_to_sink_from_reader(
        NativeRefusal,
        ReadOptions::default(),
        &mut OrderingSink::default(),
    );
    assert_eq!(resident_refusal.diagnostics[0].code, "EmptyFile");
    let mut transformed = Writer::new("rdf12");
    transformed.add_frame(
        "snapshot",
        None,
        Some(purrdf_lex::cbor::encode(&purrdf_lex::cbor::Value::Map(
            vec![(
                "terms".into(),
                purrdf_lex::cbor::Value::Array(vec![purrdf_gts::writer::term_to_wire(&Term::iri(
                    "https://example.org/transformed",
                ))]),
            )],
        ))),
        Some(&["gzip".to_owned()]),
        None,
    );
    let transformed = transformed.into_bytes();
    let valid = read_to_sink_with_dictionary(
        &transformed,
        ReadOptions::default(),
        &mut StoredTerms::default(),
        &mut OrderingSink::default(),
    )
    .expect("bounded transformed neighbour");
    assert_eq!(valid.diagnostics.len(), 0, "{:?}", valid.diagnostics);
    let (mut items, torn) = purrdf_gts::wire::iter_items(&transformed);
    assert!(torn.is_none());
    let purrdf_lex::cbor::Value::Map(frame) = &mut items[1].1 else {
        panic!("snapshot frame map");
    };
    purrdf_lex::assoc::insert(
        frame,
        "d".into(),
        purrdf_lex::cbor::Value::Bytes(b"invalid gzip".to_vec()),
    );
    let id = purrdf_gts::wire::content_id(frame);
    purrdf_lex::assoc::insert(frame, "id".into(), purrdf_lex::cbor::Value::Bytes(id));
    let mut corrupt = Vec::new();
    for (_, item) in items {
        corrupt.extend(purrdf_gts::wire::canonical(&item));
    }
    let resident = read_to_sink(
        &corrupt,
        false,
        None,
        &mut OrderingSink {
            replay_scratch: true,
            ..OrderingSink::default()
        },
    );
    let stored = read_to_sink_with_dictionary(
        &corrupt,
        ReadOptions::default(),
        &mut StoredTerms::default(),
        &mut OrderingSink {
            replay_scratch: true,
            ..OrderingSink::default()
        },
    )
    .expect("carrier corruption retains permissive diagnostics");
    assert_eq!(resident.diagnostics.len(), stored.diagnostics.len());
    for (expected, actual) in resident.diagnostics.iter().zip(&stored.diagnostics) {
        assert_eq!(
            (&actual.code, &actual.detail, actual.frame_index),
            (&expected.code, &expected.detail, expected.frame_index)
        );
    }
    assert_eq!(stored.diagnostics[0].code, "DamagedFrame");
    let refused = read_to_sink_with_dictionary(
        &transformed,
        ReadOptions::default(),
        &mut StoredTerms::default(),
        &mut OrderingSink {
            refuse_scratch: true,
            ..OrderingSink::default()
        },
    )
    .expect_err("supplied scratch failure cannot become an opaque successful read");
    assert_eq!(
        purrdf_testkit::error_capture::Capture::read(&refused).code,
        "gts-dictionary-io"
    );
    let data = three_frame_fixture();
    let mut resident = OrderingSink::default();
    let resident_read = read_to_sink(&data, false, None, &mut resident);
    assert_eq!(
        resident_read.diagnostics.len(),
        0,
        "{:?}",
        resident_read.diagnostics
    );
    let mut storage = StoredTerms::default();
    let dropped = Arc::clone(&storage.dropped);
    let mut stored = OrderingSink::default();
    let result =
        read_to_sink_with_dictionary(&data, ReadOptions::default(), &mut storage, &mut stored)
            .expect("stored reader accepts the valid neighbour");
    assert_eq!(result.diagnostics.len(), 0, "{:?}", result.diagnostics);
    assert_eq!(stored.log, resident.log);
    let lease = stored.leases[0].clone();
    drop(storage);
    drop(stored);
    assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(lease.value.as_deref(), Some("http://example.org/s"));
    drop(lease);
    assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 3);

    let mut storage = StoredTerms {
        fail_read: true,
        ..StoredTerms::default()
    };
    let error = read_to_sink_with_dictionary(
        &data,
        ReadOptions::default(),
        &mut storage,
        &mut OrderingSink::default(),
    )
    .expect_err("original host refusal must abort the drive");
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    let mut original = None;
    while let Some(error) = cause {
        if let Some(found) = error.downcast_ref::<purrdf_testkit::error_capture::CallerRefusal>() {
            original = Some(found.limit);
            break;
        }
        cause = error.source();
    }
    assert_eq!(original, Some(u128::MAX));

    let mut storage = StoredTerms {
        omit_read: true,
        ..StoredTerms::default()
    };
    let error = read_to_sink_with_dictionary(
        &data,
        ReadOptions::default(),
        &mut storage,
        &mut OrderingSink::default(),
    )
    .expect_err("an absent persisted entry cannot yield a short successful answer");
    assert_eq!(
        purrdf_testkit::error_capture::Capture::read(&error).code,
        "gts-dictionary-missing"
    );
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
            triple: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("http://example.org/p".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        },
        Term {
            kind: TermKind::Literal,
            value: Some("o".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
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
