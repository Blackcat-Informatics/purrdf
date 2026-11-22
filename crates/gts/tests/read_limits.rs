// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-read limits refuse the first excess event without publishing a partial drive.

use std::sync::Arc;

use purrdf_gts::model::Term;
use purrdf_gts::reader::{
    GtsReadLimits, GtsReadResource, ReadOptions, StreamingSink,
    read_to_sink_from_reader_with_limits, read_to_sink_with_limits, read_with_limits,
};
use purrdf_gts::writer::{FrameOptions, Writer};

struct Null;
impl StreamingSink for Null {}

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
    for fill in [b'a', b'b'] {
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
    assert!(sink.0.is_empty());
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
    assert!(result.diagnostics.is_empty());
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
