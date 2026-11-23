// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-read limits refuse the first excess event without publishing a partial drive.

use std::sync::Arc;

use purrdf_gts::model::Term;
use purrdf_gts::reader::{
    GtsReadLimits, GtsReadResource, ReadOptions, StreamingSink, read_file_segments_with_limits,
    read_to_sink_from_reader_with_limits, read_to_sink_with_limits, read_with_limits,
    segment_append_state_with_limits,
};
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
fn nested_depth_refuses_without_publishing_a_partial_result() {
    let mut writer = Writer::new("generic");
    writer.add_blob(
        &nested_child("one"),
        Some(purrdf_gts::nested::GTS_MEDIA_TYPE),
        None,
    );
    let bytes = writer.into_bytes();
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
