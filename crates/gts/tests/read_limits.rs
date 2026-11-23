// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-read limits refuse the first excess event without publishing a partial drive.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_gts::model::Term;
use purrdf_gts::reader::{
    GtsReadLimits, GtsReadResource, ReadOptions, StreamingSink, read_file_segments_with_limits,
    read_to_sink_from_reader_with_limits, read_to_sink_with_limits, read_to_sink_with_options,
    read_with_limits, segment_append_state_with_limits,
};
use purrdf_gts::wire::SELF_DESCRIBE_TAG;
use purrdf_gts::writer::{FrameOptions, Writer};
use purrdf_lex::cbor::{ReplayStorage, Value, encode};

struct Null;
impl StreamingSink for Null {}

#[derive(Default)]
struct MemoryReplay(std::io::Cursor<Vec<u8>>);

impl std::io::Read for MemoryReplay {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.0, bytes)
    }
}

impl std::io::Write for MemoryReplay {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for MemoryReplay {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, position)
    }
}

impl ReplayStorage for MemoryReplay {
    fn finish(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct ReplaySink {
    terms: usize,
}

struct FinishReplay {
    inner: std::io::Cursor<Vec<u8>>,
    finishes: Arc<AtomicUsize>,
}

impl std::io::Read for FinishReplay {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.inner, bytes)
    }
}

impl std::io::Write for FinishReplay {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.inner, bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for FinishReplay {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.inner, position)
    }
}

impl ReplayStorage for FinishReplay {
    fn finish(&mut self) -> std::io::Result<()> {
        self.finishes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct CancellingReplaySink {
    terms: usize,
    finishes: Arc<AtomicUsize>,
}

impl StreamingSink for CancellingReplaySink {
    fn continue_reading(&self) -> bool {
        self.terms == 0
    }

    fn frame_scratch(&mut self) -> Option<std::io::Result<Box<dyn ReplayStorage>>> {
        Some(Ok(Box::new(FinishReplay {
            inner: std::io::Cursor::new(Vec::new()),
            finishes: Arc::clone(&self.finishes),
        })))
    }

    fn term(&mut self, _segment: u64, _id: usize, _term: Arc<Term>) {
        self.terms += 1;
    }
}

impl StreamingSink for ReplaySink {
    fn frame_scratch(&mut self) -> Option<std::io::Result<Box<dyn ReplayStorage>>> {
        Some(Ok(Box::new(MemoryReplay::default())))
    }

    fn term(&mut self, _segment: u64, _id: usize, _term: Arc<Term>) {
        self.terms += 1;
    }
}

fn rdf_bytes() -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer.add_terms(&[
        Term::iri("https://example.org/s"),
        Term::iri("https://example.org/p"),
        Term::literal("value", None),
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.into_bytes()
}

fn blob_bytes() -> Vec<u8> {
    let mut writer = Writer::new("generic");
    for fill in *b"ab" {
        writer
            .add_frame_with_options(
                "blob",
                FrameOptions {
                    raw: Some(vec![fill; 4096]),
                    transform: vec!["gzip".into()],
                    ..FrameOptions::default()
                },
            )
            .expect("small compressed blob frame");
    }
    writer.into_bytes()
}

fn declared_transformed_blob_bytes() -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer
        .add_blob_transformed(
            vec![b'x'; 4096],
            Some("application/octet-stream"),
            None,
            &["gzip".to_owned()],
            None,
        )
        .expect("small declared transformed blob");
    writer.into_bytes()
}

fn transformed_snapshot_bytes() -> Vec<u8> {
    let snapshot = Value::Map(vec![
        (
            "terms".into(),
            Value::Array(
                ["one", "two", "three"]
                    .into_iter()
                    .map(|name| {
                        purrdf_gts::writer::term_to_wire(&Term::iri(format!(
                            "https://example.org/{name}"
                        )))
                    })
                    .collect(),
            ),
        ),
        (
            "blobs".into(),
            Value::Map(vec![("embedded".into(), Value::Bytes(vec![b'x'; 4096]))]),
        ),
        (
            "meta".into(),
            Value::Map(vec![
                ("first".into(), 1.into()),
                ("second".into(), 2.into()),
            ]),
        ),
    ]);
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "snapshot",
        None,
        Some(encode(&snapshot)),
        Some(&["gzip".to_owned()]),
        None,
    );
    writer.into_bytes()
}

fn limits(resource: GtsReadResource, limit: usize) -> GtsReadLimits {
    let mut limits = GtsReadLimits::UNBOUNDED;
    match resource {
        GtsReadResource::EncodedInputBytes => limits.encoded_input_bytes = limit,
        GtsReadResource::Items => limits.items = limit,
        GtsReadResource::EncodedFrameBytes => limits.encoded_frame_bytes = limit,
        GtsReadResource::DecodedFrameBytes => limits.decoded_frame_bytes = limit,
        GtsReadResource::DecodedBlobBytes => limits.decoded_blob_bytes = limit,
        GtsReadResource::TotalDecodedBytes => limits.total_decoded_bytes = limit,
        GtsReadResource::Terms => limits.terms = limit,
        GtsReadResource::Rows => limits.rows = limit,
        GtsReadResource::MetadataEntries => limits.metadata_entries = limit,
        GtsReadResource::Diagnostics => limits.diagnostics = limit,
        GtsReadResource::Work => limits.work = limit,
        _ => unreachable!("the test enumerates every current read resource"),
    }
    limits
}

fn assert_boundary(bytes: &[u8], resource: GtsReadResource, exact: usize) {
    assert!(exact > 0, "the fixture must exercise {resource}");
    read_with_limits(bytes, ReadOptions::default(), limits(resource, exact))
        .expect("an exactly-at-limit artifact remains valid");
    let error = read_with_limits(bytes, ReadOptions::default(), limits(resource, exact - 1))
        .expect_err("the adjacent excess must be refused");
    assert_eq!(error.resource, resource);
    assert_eq!(error.limit, exact - 1);
    assert!(error.required > error.limit);
}

#[test]
fn every_rdf_read_dimension_accepts_exact_and_refuses_adjacent_excess() {
    let bytes = rdf_bytes();
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("ordinary RDF artifact");
    for (resource, exact) in [
        (
            GtsReadResource::EncodedInputBytes,
            usage.encoded_input_bytes,
        ),
        (GtsReadResource::Items, usage.items),
        (
            GtsReadResource::EncodedFrameBytes,
            usage.largest_encoded_frame_bytes,
        ),
        (
            GtsReadResource::DecodedFrameBytes,
            usage.largest_decoded_frame_bytes,
        ),
        (
            GtsReadResource::TotalDecodedBytes,
            usage.total_decoded_bytes,
        ),
        (GtsReadResource::Terms, usage.terms),
        (GtsReadResource::Rows, usage.rows),
        (GtsReadResource::MetadataEntries, usage.metadata_entries),
        (GtsReadResource::Work, usage.work),
    ] {
        assert_boundary(&bytes, resource, exact);
    }
}

#[test]
fn aggregate_compressed_blobs_stop_at_the_first_byte_over_the_total() {
    let bytes = blob_bytes();
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("ordinary compressed blobs");
    assert!(usage.total_decoded_bytes > usage.largest_decoded_blob_bytes);
    assert_boundary(
        &bytes,
        GtsReadResource::DecodedBlobBytes,
        usage.largest_decoded_blob_bytes,
    );
    assert_boundary(
        &bytes,
        GtsReadResource::TotalDecodedBytes,
        usage.total_decoded_bytes,
    );
}

#[test]
fn declared_digest_cannot_defer_a_transformed_blob_beyond_admission() {
    let bytes = declared_transformed_blob_bytes();
    assert_boundary(&bytes, GtsReadResource::DecodedBlobBytes, 4096);
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        limits(GtsReadResource::Work, usize::MAX - 1),
        &mut Null,
    )
    .expect("a finite work contract admits the transformed blob");
    assert_boundary(&bytes, GtsReadResource::Work, usage.work);
}

#[test]
fn streaming_stage_refusal_counts_each_transform_byte_once() {
    let bytes = declared_transformed_blob_bytes();
    let decoded = 4096;
    let mut exact = GtsReadLimits::UNBOUNDED;
    exact.decoded_blob_bytes = decoded;
    let (_, admitted) = read_to_sink_with_limits(&bytes, ReadOptions::default(), exact, &mut Null)
        .expect("the exact decoded stage boundary remains valid");

    let mut adjacent = exact;
    adjacent.decoded_blob_bytes -= 1;
    let error = read_to_sink_with_limits(&bytes, ReadOptions::default(), adjacent, &mut Null)
        .expect_err("the first excess decoded stage byte is refused");
    assert_eq!(error.resource, GtsReadResource::DecodedBlobBytes);
    assert_eq!(error.required, decoded);
    assert_eq!(error.usage.largest_decoded_blob_bytes, decoded);
    assert!(error.usage.work < admitted.work);
}

fn nested_child(name: &str) -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer.add_terms(&[
        Term::iri(format!("https://example.org/{name}")),
        Term::iri("https://example.org/p"),
        Term::literal("value", None),
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.into_bytes()
}

#[test]
fn nested_artifacts_share_one_aggregate_row_contract() {
    let mut writer = Writer::new("generic");
    writer.add_blob(
        &nested_child("one"),
        Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
        None,
    );
    writer.add_blob(
        &nested_child("two"),
        Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
        None,
    );
    let bytes = writer.into_bytes();
    let mut exact = GtsReadLimits::UNBOUNDED;
    exact.terms = 6;
    let admitted = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, usize::MAX, exact)
        .expect("the aggregate term contract accepts its exact boundary");
    assert_eq!(admitted.subgraphs.len(), 2);

    let mut limits = GtsReadLimits::UNBOUNDED;
    limits.terms = 3;
    let error = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, usize::MAX, limits)
        .expect_err("the second child exceeds the aggregate term contract");
    assert_eq!(error.resource, GtsReadResource::Terms);
    assert_eq!(error.required, 4);
}

#[test]
fn nested_size_budget_measures_transforms_without_retaining_the_excess() {
    let child = nested_child("compressed");
    let mut writer = Writer::new("generic");
    writer
        .add_blob_transformed(
            child.clone(),
            Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
            None,
            &["gzip".to_owned()],
            None,
        )
        .expect("compressed nested artifact");
    let bytes = writer.into_bytes();
    let admitted = purrdf_gts::nested::read_nested_with_limits(
        &bytes,
        1,
        child.len(),
        GtsReadLimits::UNBOUNDED,
    )
    .expect("the nested decoded-byte contract accepts its exact boundary");
    assert_eq!(admitted.subgraphs.len(), 1);

    let error = purrdf_gts::nested::read_nested_with_limits(
        &bytes,
        1,
        child.len() - 1,
        GtsReadLimits::UNBOUNDED,
    )
    .expect_err("the recursion size refusal publishes no nested result");
    assert_eq!(error.resource, GtsReadResource::NestedDecodedBytes);
    assert_eq!(error.required, child.len());
}

#[test]
fn nested_children_are_admitted_before_materialization() {
    let child = nested_child("aggregate-input");
    let mut writer = Writer::new("generic");
    writer.add_blob(&child, Some(purrdf_gts::nested::GTS_MEDIA_TYPE), None);
    let bytes = writer.into_bytes();
    let exact_input = bytes.len().saturating_add(child.len());
    let mut exact = GtsReadLimits::UNBOUNDED;
    exact.encoded_input_bytes = exact_input;
    purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), exact)
        .expect("the aggregate encoded-input contract accepts its exact boundary");

    let mut adjacent = exact;
    adjacent.encoded_input_bytes -= 1;
    let error = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), adjacent)
        .expect_err("the child is refused before its buffer is materialized");
    assert_eq!(error.resource, GtsReadResource::EncodedInputBytes);
    assert_eq!(error.required, exact_input);
}

#[test]
fn nested_measurement_obeys_remaining_work() {
    let child = nested_child("aggregate-work");
    let mut writer = Writer::new("generic");
    writer
        .add_blob_transformed(
            child.clone(),
            Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
            None,
            &["gzip".to_owned()],
            None,
        )
        .expect("compressed nested artifact");
    let bytes = writer.into_bytes();
    let (_, root_usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("measure root work");
    let mut exact_work = root_usage.work;
    let exact = loop {
        let mut candidate = GtsReadLimits::UNBOUNDED;
        candidate.work = exact_work;
        match purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), candidate) {
            Ok(_) => break candidate,
            Err(error) if error.resource == GtsReadResource::Work => {
                assert!(error.required > exact_work);
                exact_work = error.required;
            }
            Err(error) => panic!("unexpected nested refusal: {error}"),
        }
    };

    let mut adjacent = exact;
    adjacent.work -= 1;
    let error = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), adjacent)
        .expect_err("nested decoding stops at the first excess work unit");
    assert_eq!(error.resource, GtsReadResource::Work);
    assert!(error.required > adjacent.work);
}

#[test]
fn nested_multistage_materialization_uses_the_declared_stage_limit() {
    let mut child_header = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
    ];
    child_header.push((
        "id".into(),
        Value::Bytes(purrdf_gts::wire::header_id(&child_header)),
    ));
    let child = purrdf_gts::wire::canonical(&Value::Tag(
        SELF_DESCRIBE_TAG,
        Box::new(Value::Map(child_header)),
    ));
    let intermediate =
        purrdf_gts::codec::encode_chain(&["gzip".to_owned()], &child).expect("single gzip stage");
    assert!(intermediate.len() > child.len());
    let intermediate_len = intermediate.len();
    let mut writer = Writer::new("generic");
    writer
        .add_blob_transformed(
            child.clone(),
            Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
            None,
            &["gzip".to_owned(), "gzip".to_owned()],
            None,
        )
        .expect("two-stage nested artifact");
    let bytes = writer.into_bytes();
    let mut exact = GtsReadLimits::UNBOUNDED;
    exact.decoded_blob_bytes = intermediate_len;
    let nested = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), exact)
        .expect("the intermediate stage fits the explicit stage boundary");
    assert_eq!(nested.subgraphs.len(), 1);

    let mut adjacent = exact;
    adjacent.decoded_blob_bytes -= 1;
    let error = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), adjacent)
        .expect_err("the adjacent intermediate-stage excess is refused");
    assert_eq!(error.resource, GtsReadResource::DecodedBlobBytes);

    let (_, root_usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("measure work before nested traversal");
    let mut cumulative = exact;
    cumulative.work = root_usage
        .work
        .saturating_add(1)
        .saturating_add(intermediate_len)
        .saturating_add(child.len())
        .saturating_sub(1);
    let error = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, child.len(), cumulative)
        .expect_err("cumulative stage work is distinct from the per-stage byte ceiling");
    assert_eq!(error.resource, GtsReadResource::Work);
}

#[test]
fn nested_depth_refuses_without_publishing_a_partial_result() {
    let mut writer = Writer::new("generic");
    writer.add_blob(
        &nested_child("one"),
        Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
        None,
    );
    let bytes = writer.into_bytes();
    let admitted = purrdf_gts::nested::read_nested_with_limits(
        &bytes,
        1,
        usize::MAX,
        GtsReadLimits::UNBOUNDED,
    )
    .expect("the nested depth contract accepts its exact boundary");
    assert_eq!(admitted.subgraphs.len(), 1);

    let error = purrdf_gts::nested::read_nested_with_limits(
        &bytes,
        0,
        usize::MAX,
        GtsReadLimits::UNBOUNDED,
    )
    .expect_err("the depth refusal publishes no nested result");
    assert_eq!(error.resource, GtsReadResource::NestedDepth);
    assert_eq!(error.required, 1);
}

#[test]
fn nested_traversal_diagnostics_share_the_aggregate_contract() {
    let mut writer = Writer::new("generic");
    writer.add_blob(
        b"not a GTS file",
        Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
        None,
    );
    writer.add_blob(
        b"also not a GTS file",
        Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
        None,
    );
    let bytes = writer.into_bytes();
    let mut exact = GtsReadLimits::UNBOUNDED;
    exact.diagnostics = 4;
    let admitted = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, usize::MAX, exact)
        .expect("the traversal diagnostic contract accepts its exact boundary");
    assert_eq!(admitted.diagnostics.len(), 2);

    let mut limits = GtsReadLimits::UNBOUNDED;
    limits.diagnostics = 3;
    let error = purrdf_gts::nested::read_nested_with_limits(&bytes, 1, usize::MAX, limits)
        .expect_err("the second traversal diagnostic exceeds the aggregate contract");
    assert_eq!(error.resource, GtsReadResource::Diagnostics);
    assert_eq!(error.required, 4);
}

#[derive(Default)]
struct Terms(Vec<Arc<Term>>);

impl StreamingSink for Terms {
    fn term(&mut self, _segment: u64, _id: usize, term: Arc<Term>) {
        self.0.push(term);
    }
}

#[test]
fn a_refused_preflight_publishes_no_partial_sink_events() {
    let bytes = rdf_bytes();
    let mut sink = Terms::default();
    let error = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        limits(GtsReadResource::Terms, 1),
        &mut sink,
    )
    .expect_err("the second term exceeds the contract");
    assert_eq!(error.resource, GtsReadResource::Terms);
    assert_eq!(sink.0, Vec::new());
}

#[test]
fn diagnostic_growth_has_an_exact_boundary() {
    let mut writer = Writer::new("generic");
    writer.add_frame("unknown", None, None, None, None);
    let bytes = writer.into_bytes();
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("damaged input remains a total diagnostic result");
    assert_boundary(&bytes, GtsReadResource::Diagnostics, usage.diagnostics);
}

#[test]
fn incremental_input_enforces_the_complete_encoded_boundary() {
    let bytes = rdf_bytes();
    let (result, usage) = read_to_sink_from_reader_with_limits(
        std::io::Cursor::new(&bytes),
        ReadOptions::default(),
        limits(GtsReadResource::EncodedInputBytes, bytes.len()),
        &mut Null,
    )
    .expect("an exactly-at-limit stream remains valid");
    assert_eq!(result.diagnostics, Vec::new());
    assert_eq!(usage.encoded_input_bytes, bytes.len());

    let error = read_to_sink_from_reader_with_limits(
        std::io::Cursor::new(&bytes),
        ReadOptions::default(),
        limits(GtsReadResource::EncodedInputBytes, bytes.len() - 1),
        &mut Null,
    )
    .expect_err("the first excess source byte is refused");
    assert_eq!(error.resource, GtsReadResource::EncodedInputBytes);
}

#[test]
fn transformed_snapshot_replay_refuses_embedded_blobs_and_stops_rows() {
    let bytes = transformed_snapshot_bytes();
    let mut sink = ReplaySink::default();
    let error = read_to_sink_from_reader_with_limits(
        std::io::Cursor::new(&bytes),
        ReadOptions::default(),
        limits(GtsReadResource::DecodedBlobBytes, 4095),
        &mut sink,
    )
    .expect_err("the embedded blob exceeds the explicit replay ceiling");
    assert_eq!(error.resource, GtsReadResource::DecodedBlobBytes);
    assert_eq!(error.usage.largest_decoded_blob_bytes, 4096);
    assert_eq!(
        sink.terms, 3,
        "rows before the refused blob are provisional"
    );

    let mut sink = ReplaySink::default();
    let error = read_to_sink_from_reader_with_limits(
        std::io::Cursor::new(&bytes),
        ReadOptions::default(),
        limits(GtsReadResource::Terms, 1),
        &mut sink,
    )
    .expect_err("the second snapshot term exceeds the explicit row ceiling");
    assert_eq!(error.resource, GtsReadResource::Terms);
    assert_eq!(error.usage.terms, 2);
    assert_eq!(sink.terms, 1, "replay stops at the first refused row");
}

#[test]
fn cancelled_snapshot_replay_does_not_verify_or_finish_provisional_storage() {
    let finishes = Arc::new(AtomicUsize::new(0));
    let mut sink = CancellingReplaySink {
        terms: 0,
        finishes: Arc::clone(&finishes),
    };
    read_to_sink_with_options(
        &transformed_snapshot_bytes(),
        ReadOptions::default(),
        &mut sink,
    );
    assert_eq!(sink.terms, 1);
    assert_eq!(finishes.load(Ordering::SeqCst), 0);
}

#[test]
fn finite_defaults_do_not_contradict_their_component_ceilings() {
    let limits = GtsReadLimits::DEFAULT;
    let admitted_work = limits
        .encoded_input_bytes
        .saturating_add(limits.total_decoded_bytes)
        .saturating_add(limits.terms)
        .saturating_add(limits.rows)
        .saturating_add(limits.metadata_entries)
        .saturating_add(limits.diagnostics)
        .saturating_add(limits.items.saturating_mul(10));
    assert!(
        limits.work >= admitted_work,
        "the independent work ceiling must not shadow the component defaults"
    );
}

#[test]
fn low_level_file_views_apply_the_explicit_input_contract() {
    let bytes = rdf_bytes();
    let limits = limits(GtsReadResource::EncodedInputBytes, bytes.len() - 1);
    let append = segment_append_state_with_limits(&bytes, limits)
        .expect_err("append-state admission refuses the first excess byte");
    assert!(matches!(
        append,
        purrdf_gts::reader::AppendError::ReadLimit(ref error)
            if error.resource == GtsReadResource::EncodedInputBytes
    ));
    let segments = read_file_segments_with_limits(&bytes, limits)
        .expect_err("per-segment admission refuses the first excess byte");
    assert_eq!(segments.resource, GtsReadResource::EncodedInputBytes);

    let wire = purrdf_gts::wire::iter_items_with_limits(&bytes, limits)
        .expect_err("wire item admission refuses the first excess byte");
    assert_eq!(wire.resource, GtsReadResource::EncodedInputBytes);
    let inventory = purrdf_gts::replication::inventory_with_limits(&bytes, limits)
        .expect_err("inventory admission refuses the first excess byte");
    assert_eq!(inventory.resource, GtsReadResource::EncodedInputBytes);
    let nested = purrdf_gts::nested::read_nested_with_limits(&bytes, 2, bytes.len(), limits)
        .expect_err("nested admission refuses the first excess byte");
    assert_eq!(nested.resource, GtsReadResource::EncodedInputBytes);
    let verify = purrdf_gts::verify::verify_file_with_limits(&bytes, limits)
        .expect_err("verification admission refuses the first excess byte");
    assert_eq!(verify.resource, GtsReadResource::EncodedInputBytes);
    let append = Writer::appending_with_limits(&bytes, limits)
        .expect_err("writer append admission refuses the first excess byte");
    assert!(matches!(
        append,
        purrdf_gts::writer::WriterError::Append(
            purrdf_gts::reader::AppendError::ReadLimit(ref error)
        ) if error.resource == GtsReadResource::EncodedInputBytes
    ));
    let proof = purrdf_gts::mmr::prove_file_with_limits(&bytes, &[0; 32], limits)
        .expect_err("proof admission refuses the first excess byte");
    assert!(matches!(
        proof,
        purrdf_gts::mmr::ProofError::ReadLimit(ref error)
            if error.resource == GtsReadResource::EncodedInputBytes
    ));
}

#[test]
fn complete_sequence_item_admission_precedes_materialization() {
    let admitted = read_with_limits(
        &[0x00],
        ReadOptions::default(),
        limits(GtsReadResource::Items, 1),
    )
    .expect("one malformed item fits the item contract");
    assert_eq!(admitted.diagnostics[0].code, "DamagedFrame");

    let error = read_with_limits(
        &[0x00, 0x00],
        ReadOptions::default(),
        limits(GtsReadResource::Items, 1),
    )
    .expect_err("the ignored malformed tail is still admitted before retention");
    assert_eq!(error.resource, GtsReadResource::Items);
    assert_eq!(error.required, 2);

    let error = purrdf_gts::wire::iter_items_with_error_and_limits(
        &[0x00, 0x00],
        limits(GtsReadResource::Items, 1),
    )
    .expect_err("the error-preserving decoder cannot retain an unadmitted tail");
    assert_eq!(error.resource, GtsReadResource::Items);
}

#[test]
fn incremental_refusal_usage_includes_bytes_consumed_before_the_callback() {
    let bytes = rdf_bytes();
    let error = read_to_sink_from_reader_with_limits(
        std::io::Cursor::new(&bytes),
        ReadOptions::default(),
        limits(GtsReadResource::Items, 1),
        &mut Null,
    )
    .expect_err("the second item exceeds the item contract");
    assert_eq!(error.resource, GtsReadResource::Items);
    assert!(error.usage.encoded_input_bytes > 0);
}

#[derive(Default)]
struct EncodedCeiling {
    frame: usize,
    refusals: usize,
}

impl StreamingSink for EncodedCeiling {
    fn encoded_frame_limit(&self) -> Option<usize> {
        Some(self.frame)
    }

    fn encoded_frame_refused(&mut self, _required: usize, _limit: usize) {
        self.refusals += 1;
    }
}

#[test]
fn sink_selected_frame_ceiling_configures_slice_admission() {
    let bytes = rdf_bytes();
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("measure the largest frame");
    let mut exact = EncodedCeiling {
        frame: usage.largest_encoded_frame_bytes,
        ..EncodedCeiling::default()
    };
    read_to_sink_with_options(&bytes, ReadOptions::default(), &mut exact);
    assert_eq!(exact.refusals, 0);

    let mut adjacent = EncodedCeiling {
        frame: usage.largest_encoded_frame_bytes - 1,
        ..EncodedCeiling::default()
    };
    read_to_sink_with_options(&bytes, ReadOptions::default(), &mut adjacent);
    assert_eq!(adjacent.refusals, 1);
}

#[derive(Default)]
struct CancelAtFrame {
    cancelled: bool,
    decoded_payloads: usize,
}

impl StreamingSink for CancelAtFrame {
    fn continue_reading(&self) -> bool {
        !self.cancelled
    }

    fn frame(&mut self, _frame: purrdf_gts::reader::FrameContext<'_>) {
        self.cancelled = true;
    }

    fn decoded_payload(&mut self, _blob: bool, _bytes: usize) {
        self.decoded_payloads += 1;
    }
}

#[test]
fn frame_cancellation_precedes_payload_decode() {
    let bytes = transformed_snapshot_bytes();
    let mut sink = CancelAtFrame::default();
    read_to_sink_with_options(&bytes, ReadOptions::default(), &mut sink);
    assert!(sink.cancelled);
    assert_eq!(sink.decoded_payloads, 0);
}

#[derive(Default)]
struct CancelAtSegmentHead {
    cancelled: bool,
    layouts: usize,
}

impl StreamingSink for CancelAtSegmentHead {
    fn continue_reading(&self) -> bool {
        !self.cancelled
    }

    fn segment_head(&mut self, _segment_index: u64, _head: &[u8]) {
        self.cancelled = true;
    }

    fn streamable_layout(
        &mut self,
        _segment_index: u64,
        _info: &purrdf_gts::model::StreamableInfo,
    ) {
        self.layouts += 1;
    }
}

#[test]
fn segment_head_cancellation_stops_finalization_callbacks() {
    let bytes = rdf_bytes();
    let mut sink = CancelAtSegmentHead::default();
    let result = read_to_sink_with_options(&bytes, ReadOptions::default(), &mut sink);
    assert!(sink.cancelled);
    assert_eq!(sink.layouts, 0);
    assert_eq!(result.segment_heads.len(), 0);
}

#[derive(Default)]
struct CancelAtFinalDiagnostic {
    cancelled: bool,
    codes: Vec<String>,
}

impl StreamingSink for CancelAtFinalDiagnostic {
    fn continue_reading(&self) -> bool {
        !self.cancelled
    }

    fn diagnostic(&mut self, diagnostic: &purrdf_gts::model::Diagnostic) {
        self.codes.push(diagnostic.code.clone());
        self.cancelled = true;
    }
}

#[test]
fn final_diagnostic_cancellation_prevents_later_diagnostics() {
    let mut bytes = rdf_bytes();
    bytes.push(0xff);
    let expected = [0_u8; 32];
    let mut sink = CancelAtFinalDiagnostic::default();
    let options = ReadOptions::new(false, Some(&expected));
    read_to_sink_with_options(&bytes, options, &mut sink);
    assert_eq!(sink.codes, ["TruncatedLog"]);
}

#[test]
fn malformed_transformed_payloads_charge_decoded_bytes() {
    let mut writer = Writer::new("generic");
    for _ in 0..2 {
        writer
            .add_frame_with_options(
                "unknown",
                FrameOptions {
                    raw: Some(vec![0xff; 128]),
                    transform: vec!["gzip".into()],
                    ..FrameOptions::default()
                },
            )
            .expect("small malformed transformed payload");
    }
    let error = read_with_limits(
        &writer.into_bytes(),
        ReadOptions::default(),
        limits(GtsReadResource::TotalDecodedBytes, 128),
    )
    .expect_err("the second malformed decoded payload exceeds the aggregate contract");
    assert_eq!(error.resource, GtsReadResource::TotalDecodedBytes);
    assert_eq!(error.required, 129);
}

#[test]
fn every_transform_stage_participates_in_the_work_boundary() {
    let raw: Vec<u8> = (0_u8..=255).cycle().take(32 * 1024).collect();
    let mut writer = Writer::new("generic");
    writer
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(raw.clone()),
                transform: vec!["gzip".into(), "gzip".into()],
                ..FrameOptions::default()
            },
        )
        .expect("small two-stage transformed blob");
    let bytes = writer.into_bytes();
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("the explicit unbounded contract measures every transform stage");
    assert_eq!(usage.largest_decoded_blob_bytes, raw.len());
    assert!(usage.work > bytes.len().saturating_add(raw.len()));
    assert_boundary(&bytes, GtsReadResource::Work, usage.work);
}

#[test]
fn repeated_catalog_dictionary_copies_are_admitted_as_work() {
    let dictionary = vec![b'd'; 1024];
    let catalog = (0..4)
        .map(|id| {
            (
                Value::Integer(id.into()),
                Value::Map(vec![
                    ("name".into(), format!("codec-{id}").into()),
                    ("cls".into(), "compress".into()),
                    ("dct".into(), "shared".into()),
                ]),
            )
        })
        .collect();
    let mut header = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
        ("prof".into(), "generic".into()),
        ("cat".into(), Value::Map(catalog)),
        (
            "dct".into(),
            Value::Map(vec![("shared".into(), Value::Bytes(dictionary))]),
        ),
    ];
    header.push((
        "id".into(),
        Value::Bytes(purrdf_gts::wire::header_id(&header)),
    ));
    let bytes =
        purrdf_gts::wire::canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(Value::Map(header))));
    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("the explicit unbounded contract admits catalog copies");
    assert!(usage.work >= bytes.len().saturating_add(4 * 1024));
    assert_boundary(&bytes, GtsReadResource::Work, usage.work);
}

#[test]
fn repeated_codec_strings_are_admitted_before_chain_cloning() {
    let codec_name = "codec".repeat(1024);
    let mut header = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
        ("prof".into(), "generic".into()),
        (
            "cat".into(),
            Value::Map(vec![(
                Value::Integer(0.into()),
                Value::Map(vec![
                    ("name".into(), codec_name.clone().into()),
                    ("cls".into(), "compress".into()),
                ]),
            )]),
        ),
    ];
    let header_id = purrdf_gts::wire::header_id(&header);
    header.push(("id".into(), Value::Bytes(header_id.clone())));
    let mut frame = vec![
        ("t".into(), "unknown".into()),
        ("prev".into(), Value::Bytes(header_id)),
        (
            "x".into(),
            Value::Array((0..4).map(|_| Value::Integer(0.into())).collect()),
        ),
        ("d".into(), Value::Bytes(Vec::new())),
    ];
    frame.push((
        "id".into(),
        Value::Bytes(purrdf_gts::wire::content_id(&frame)),
    ));
    let mut bytes =
        purrdf_gts::wire::canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(Value::Map(header))));
    bytes.extend(purrdf_gts::wire::canonical(&Value::Map(frame)));

    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("the explicit unbounded contract admits repeated codec strings");
    assert!(
        usage.work
            >= bytes
                .len()
                .saturating_add(codec_name.len().saturating_mul(5))
    );
    assert_boundary(&bytes, GtsReadResource::Work, usage.work);
}

#[test]
fn invalid_codec_after_a_valid_transform_charges_the_drained_work() {
    let raw = vec![b'x'; 32 * 1024];
    let encoded =
        purrdf_gts::codec::encode_chain(&["gzip".to_owned()], &raw).expect("single gzip stage");
    let mut header = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
        ("prof".into(), "generic".into()),
        (
            "cat".into(),
            Value::Map(vec![
                (
                    Value::Integer(0.into()),
                    Value::Map(vec![
                        ("name".into(), "unknown".into()),
                        ("cls".into(), "compress".into()),
                    ]),
                ),
                (
                    Value::Integer(1.into()),
                    Value::Map(vec![
                        ("name".into(), "gzip".into()),
                        ("cls".into(), "compress".into()),
                    ]),
                ),
            ]),
        ),
    ];
    let mut previous = purrdf_gts::wire::header_id(&header);
    header.push(("id".into(), Value::Bytes(previous.clone())));
    let mut bytes =
        purrdf_gts::wire::canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(Value::Map(header))));
    for _ in 0..2 {
        let mut frame = vec![
            ("t".into(), "blob".into()),
            ("prev".into(), Value::Bytes(previous)),
            (
                "x".into(),
                Value::Array(vec![Value::Integer(0.into()), Value::Integer(1.into())]),
            ),
            ("d".into(), Value::Bytes(encoded.clone())),
        ];
        previous = purrdf_gts::wire::content_id(&frame);
        frame.push(("id".into(), Value::Bytes(previous.clone())));
        bytes.extend(purrdf_gts::wire::canonical(&Value::Map(frame)));
    }

    let (_, usage) = read_to_sink_with_limits(
        &bytes,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("unavailable outer codecs retain work from valid inner transforms");
    assert!(usage.work >= bytes.len().saturating_add(raw.len().saturating_mul(2)));
    assert_boundary(&bytes, GtsReadResource::Work, usage.work);
}

#[test]
fn unresolved_codec_ids_do_not_charge_uncloned_valid_neighbors() {
    let large_name = "identity-neighbor".repeat(2048);
    let mut header = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
        ("prof".into(), "generic".into()),
        (
            "cat".into(),
            Value::Map(vec![
                (
                    Value::Integer(0.into()),
                    Value::Map(vec![
                        ("name".into(), large_name.into()),
                        ("cls".into(), "encode".into()),
                    ]),
                ),
                (
                    "invalid-id".into(),
                    Value::Map(vec![("name".into(), "not-retained".repeat(2048).into())]),
                ),
                (
                    Value::Integer(1.into()),
                    Value::Map(vec![
                        ("name".into(), "missing-dictionary".repeat(2048).into()),
                        ("dct".into(), "absent".into()),
                    ]),
                ),
            ]),
        ),
    ];
    let header_id = purrdf_gts::wire::header_id(&header);
    header.push(("id".into(), Value::Bytes(header_id.clone())));
    let make_file = |ids: Vec<Value>| {
        let mut frame = vec![
            ("t".into(), "unknown".into()),
            ("prev".into(), Value::Bytes(header_id.clone())),
            ("x".into(), Value::Array(ids)),
            ("d".into(), Value::Bytes(Vec::new())),
        ];
        frame.push((
            "id".into(),
            Value::Bytes(purrdf_gts::wire::content_id(&frame)),
        ));
        let mut bytes = purrdf_gts::wire::canonical(&Value::Tag(
            SELF_DESCRIBE_TAG,
            Box::new(Value::Map(header.clone())),
        ));
        bytes.extend(purrdf_gts::wire::canonical(&Value::Map(frame)));
        bytes
    };
    let baseline = make_file(vec![Value::Integer(99.into())]);
    let neighbors = make_file(vec![
        Value::Integer(99.into()),
        Value::Integer(0.into()),
        Value::Integer(0.into()),
    ]);
    let (_, baseline_usage) = read_to_sink_with_limits(
        &baseline,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("early unresolved codec baseline");
    let (_, neighbor_usage) = read_to_sink_with_limits(
        &neighbors,
        ReadOptions::default(),
        GtsReadLimits::UNBOUNDED,
        &mut Null,
    )
    .expect("later valid codecs are never cloned after the unresolved prefix");
    assert!(neighbor_usage.work.saturating_sub(baseline_usage.work) < 128);
    assert_boundary(&neighbors, GtsReadResource::Work, neighbor_usage.work);
}
