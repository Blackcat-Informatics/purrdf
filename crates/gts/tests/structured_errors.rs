// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native transform and identifier failures through one generic consumer.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_gts::codec::{
    Codec, EncodeOptions, decode_chain, decode_chain_bounded, encode_chain,
    encode_chain_with_options,
};
use purrdf_gts::ulid::{DeterministicUlidGenerator, Ulid, UlidError};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

struct HeaderFault {
    prefix: std::io::Cursor<[u8; 4]>,
    error: Option<std::io::Error>,
}

impl std::io::Read for HeaderFault {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.prefix.position() < 4 {
            return self.prefix.read(bytes);
        }
        Err(self
            .error
            .take()
            .expect("the first source failure must terminate header decoding"))
    }
}

impl std::io::Write for HeaderFault {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        Err(self
            .error
            .take()
            .expect("the first output failure must stop writing"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn tar_and_files_public_io_retain_unformattable_original_causes() {
    use purrdf_gts::{files, from_tar, reader, tar};
    use purrdf_testkit::error_capture::CallerRefusal;
    let entries = [files::FileEntry {
        path: "leaf".into(),
        data: Some(vec![7]),
        ..files::FileEntry::default()
    }];
    let failure = || HeaderFault {
        prefix: std::io::Cursor::new([0; 4]),
        error: Some(purrdf_hash::error::io_error(
            std::io::ErrorKind::Other,
            CallerRefusal { limit: u128::MAX },
        )),
    };
    let error = files::pack_entries_v2_to_writer(&entries, failure()).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-files-io");
    assert!(matches!(
        fields.field("operation"),
        ErrorValue::Code("write files-profile-v2 metadata")
    ));
    assert!(matches!(
        fields
            .child("cause")
            .child("cause")
            .child("cause")
            .field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<purrdf_hash::error::IoFailure>()
    );

    let graph = reader::read(&files::pack_entries_v2(&entries).unwrap(), true, None);
    let error = tar::to_tar(&graph, failure(), &tar::ToTarOptions::default()).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-tar-io");
    assert!(matches!(fields.field("path"), ErrorValue::Text("leaf")));
    assert!(matches!(
        fields
            .child("cause")
            .child("cause")
            .child("cause")
            .field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));

    let error = from_tar::from_tar(failure(), &from_tar::FromTarOptions::default()).unwrap_err();
    let events = Vec::with_capacity(3);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "gts-tar-io");
    assert!(matches!(
        fields.field("operation"),
        ErrorValue::Code("read tar input")
    ));
    assert!(matches!(
        fields
            .child("cause")
            .child("cause")
            .child("cause")
            .field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
}

fn tar_blob_sources_keep_observed_size_and_digest_refusals() {
    use purrdf_gts::files::{
        FileBlobBytes, FileBlobRange, FileEntry, pack_entries_v2_with_blob_bytes,
        pack_entries_v2_with_blob_ranges,
    };
    use std::collections::BTreeMap;
    let digest = purrdf_gts::writer::digest_string(&[7]);
    let mut entry = FileEntry {
        path: "leaf".into(),
        digest: Some(digest.clone()),
        size: Some(2),
        ..FileEntry::default()
    };
    let ranges = BTreeMap::from([(
        digest.clone(),
        FileBlobRange {
            offset: 0,
            size: 2,
            media_type: None,
            representation: None,
        },
    )]);
    let error = pack_entries_v2_with_blob_ranges(
        std::slice::from_ref(&entry),
        &mut std::io::Cursor::new([7]),
        &ranges,
        &mut Vec::new(),
    )
    .unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-files-io");
    let cause = fields.child("cause").child("cause").child("cause");
    assert_eq!(cause.code, "gts-files-blob-size");
    assert!(matches!(cause.field("expected"), ErrorValue::ByteCount(2)));
    assert!(matches!(cause.field("actual"), ErrorValue::ByteCount(1)));
    assert_eq!(
        error.to_string(),
        "hash inline blob range: blob source changed size: expected 2, read 1"
    );

    entry.size = Some(1);
    let blobs = BTreeMap::from([(
        digest.clone(),
        FileBlobBytes {
            data: &[9],
            media_type: None,
            representation: None,
        },
    )]);
    let error = pack_entries_v2_with_blob_bytes(&[entry], &blobs, &mut Vec::new()).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-files-blob-digest");
    assert!(matches!(fields.field("expected"), ErrorValue::Text(value) if value == digest));
    assert!(
        matches!(fields.field("actual"), ErrorValue::Text(value) if value == purrdf_gts::writer::digest_string(&[9]))
    );
}

fn tar_metadata_and_file_admission_keep_original_numbers_and_parser_causes() {
    use purrdf_gts::{files, reader, tar};
    let mut entry = files::FileEntry {
        path: "leaf".into(),
        data: Some(vec![7]),
        ..files::FileEntry::default()
    };
    let mut graph = reader::read(
        &files::pack_entries_v2(&[entry.clone()]).unwrap(),
        true,
        None,
    );
    graph
        .terms
        .iter_mut()
        .find(|term| term.value.as_deref() == Some("1"))
        .expect("the file size was written")
        .value = Some(u128::MAX.to_string());
    let error = tar::to_tar_vec(&graph, &tar::ToTarOptions::default()).unwrap_err();
    drop(graph);
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-integer");
    assert!(matches!(fields.field("field"), ErrorValue::Code("size")));
    assert!(
        matches!(fields.field("actual"), ErrorValue::Text(value) if value == u128::MAX.to_string())
    );
    let parsed = fields.child("cause");
    assert_eq!(parsed.code, "std-integer-parse");
    assert!(matches!(
        parsed.child("kind").field("name"),
        ErrorValue::Code("positive-overflow")
    ));
    entry.size = Some(u64::MAX);
    let error = files::pack_entries_v2(&[entry]).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-files-entry-size");
    assert!(
        matches!(fields.field("expected"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(1)));

    let error = files::FileEntryKind::parse("unknown-kind").unwrap_err();
    assert!(matches!(
        Capture::read(&error).field("actual"),
        ErrorValue::Text("unknown-kind")
    ));
    let error = files::pack_entries_v2(&[files::FileEntry {
        path: "../outside".into(),
        kind: files::FileEntryKind::Directory,
        ..files::FileEntry::default()
    }])
    .unwrap_err();
    assert_eq!(Capture::read(&error).code, "gts-files-traversal-path");
}

fn transport_retains_native_source_errors_before_foreign_formatting() {
    use purrdf_gts::transport::{TransportEncoding, transport_reader};
    use purrdf_testkit::error_capture::CallerRefusal;
    let source = HeaderFault {
        prefix: std::io::Cursor::new([0x28, 0xb5, 0x2f, 0xfd]),
        error: Some(purrdf_hash::error::io_error(
            std::io::ErrorKind::Other,
            CallerRefusal { limit: u128::MAX },
        )),
    };
    let Err(error) = transport_reader(source, Some(TransportEncoding::Zstd)) else {
        panic!("failed header read must remain a failure")
    };
    let copy = error.clone();
    assert_eq!(error, copy);
    drop(error);
    assert_eq!(copy.cause.kind(), std::io::ErrorKind::Other);
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&copy, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "gts-transport");
    let fields = fields.child("cause").child("cause").child("cause");
    assert_eq!(fields.code, "caller-refused");
    assert!(matches!(
        fields.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));

    let original = std::io::Error::from_raw_os_error(5);
    let kind = original.kind();
    let source = HeaderFault {
        prefix: std::io::Cursor::new([0x28, 0xb5, 0x2f, 0xfd]),
        error: Some(original),
    };
    let Err(error) = transport_reader(source, Some(TransportEncoding::Zstd)) else {
        panic!("native OS-code control must fail")
    };
    assert_eq!(error.cause.kind(), kind);
    assert_eq!(error.cause.raw_os_error(), Some(5));
    assert!(matches!(
        Capture::read(&error).child("cause").field("raw_os_error"),
        ErrorValue::Signed(5)
    ));
}

fn native_transport_framing_and_decoder_fields_survive() {
    use purrdf_gts::transport::{TransportEncoding, decode_transport};
    let mut gzip = encode_chain(&["gzip".into()], b"payload").unwrap();
    gzip[3] |= 0xe0;
    let error = decode_transport(&gzip, TransportEncoding::Gzip).unwrap_err();
    let fields = Capture::read(&error)
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(fields.code, "gzip-reserved-flags");
    assert!(matches!(fields.field("flags"), ErrorValue::Unsigned(value) if value & 0xe0 == 0xe0));

    let cases = [
        (Vec::new(), "gts-zstd-no-frame", None),
        (
            vec![0x50, 0x2a, 0x4d, 0x18, 0, 0],
            "gts-zstd-skippable-size",
            Some((2, 4)),
        ),
        (
            vec![0x50, 0x2a, 0x4d, 0x18, 8, 0, 0, 0, 1, 2],
            "gts-zstd-skippable-payload",
            Some((2, 8)),
        ),
    ];
    for (bytes, code, counts) in cases {
        let error = decode_transport(&bytes, TransportEncoding::Zstd).unwrap_err();
        let fields = Capture::read(&error)
            .child("cause")
            .child("cause")
            .child("cause");
        assert_eq!(fields.code, code);
        if let Some((actual, expected)) = counts {
            assert!(
                matches!(fields.field("actual"), ErrorValue::ByteCount(value) if value == actual)
            );
            assert!(
                matches!(fields.field("expected"), ErrorValue::ByteCount(value) if value == expected)
            );
        }
    }
    let mut bytes = encode_chain(&["zstd".into()], b"payload").unwrap();
    assert_eq!(
        decode_transport(&bytes, TransportEncoding::Zstd).unwrap(),
        b"payload"
    );
    bytes.extend([1, 2]);
    let error = decode_transport(&bytes, TransportEncoding::Zstd).unwrap_err();
    let fields = Capture::read(&error)
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(fields.code, "gts-zstd-trailing-prefix");
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(2)));
    assert_eq!(
        error.message(),
        "2 byte(s) after the last zstd frame do not begin another frame"
    );
    assert_eq!(
        error,
        decode_transport(&bytes, TransportEncoding::Zstd).unwrap_err()
    );
}

fn dictionary_and_key_envelope_errors_keep_native_fields() {
    use purrdf_gts::dict::{DictSeed, dictionary_id, raw_content_dict, trained_dict};
    let samples: &[&[u8]] = &[b"", b""];
    let raw = raw_content_dict(samples, 1024).unwrap_err();
    let trained = trained_dict(samples, 1024, DictSeed::FromCorpus).unwrap_err();
    assert_eq!(raw, trained);
    for (error, operation) in [(raw, "raw-content"), (trained, "train")] {
        let fields = Capture::read(&error);
        assert_eq!(fields.code, "gts-dictionary-empty-corpus");
        assert!(matches!(fields.field("operation"), ErrorValue::Code(value) if value == operation));
        assert!(matches!(fields.field("samples"), ErrorValue::Unsigned(2)));
        assert!(matches!(
            fields.field("target_len"),
            ErrorValue::ByteCount(1024)
        ));
        assert_eq!(
            error.to_string(),
            "dictionary construction failed: empty corpus"
        );
    }
    let error = dictionary_id(&[]).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-dictionary-external");
    assert!(matches!(
        fields.field("operation"),
        ErrorValue::Code("decode")
    ));
    assert!(matches!(
        fields.field("input_bytes"),
        ErrorValue::ByteCount(0)
    ));
    assert!(matches!(fields.field("target_len"), ErrorValue::Absent));
    assert!(matches!(fields.field("message"), ErrorValue::Message(value) if !value.is_empty()));
    let error = {
        let input = String::from("not an armored key");
        purrdf_gts::openpgp::parse_transport_key(&input).unwrap_err()
    };
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-openpgp-rule");
    assert_eq!(fields.child("rule").code, "gts-openpgp-missing-begin");
    assert_eq!(error.to_string(), "missing armor BEGIN line");
}

fn refused_frame(
    payload: purrdf_lex::cbor::Value,
    codecs: Vec<purrdf_lex::cbor::Value>,
) -> Vec<u8> {
    use purrdf_gts::{
        wire,
        writer::{FrameOptions, Writer},
    };
    use purrdf_lex::{assoc, cbor::Value};
    let mut writer = Writer::new("generic");
    writer
        .add_frame_with_options(
            "terms",
            FrameOptions {
                payload: Some(Value::Array(Vec::new())),
                ..FrameOptions::default()
            },
        )
        .unwrap();
    let (mut items, torn) = wire::iter_items(&writer.into_bytes());
    assert_eq!(torn, None);
    let frame = items.last_mut().unwrap().1.as_map_mut().unwrap();
    assoc::insert(frame, Value::from("d"), payload);
    assoc::insert(frame, Value::from("x"), Value::Array(codecs));
    let id = wire::content_id(frame);
    assoc::insert(frame, Value::from("id"), Value::Bytes(id));
    items
        .into_iter()
        .flat_map(|(_, item)| wire::canonical(&item))
        .collect()
}

#[derive(Default)]
struct DiagnosticSink {
    diagnostics: Vec<purrdf_gts::model::Diagnostic>,
    encoded_limit: Option<usize>,
    decoded_limit: Option<usize>,
}

impl purrdf_gts::reader::StreamingSink for DiagnosticSink {
    fn diagnostic(&mut self, diagnostic: &purrdf_gts::model::Diagnostic) {
        self.diagnostics.push(diagnostic.clone());
    }
    fn blob_encoded_limit(&self) -> Option<usize> {
        self.encoded_limit
    }
    fn blob_decode_limit(&self) -> Option<usize> {
        self.decoded_limit
    }
}

fn reader_causes_survive_input_graph_and_streaming_owners() {
    use purrdf_gts::reader::{read, read_to_sink};
    use purrdf_lex::cbor::Value;
    let mut gzip = encode_chain(&["gzip".into()], b"payload").unwrap();
    gzip[3] |= 0xe0;
    let bytes = refused_frame(Value::Bytes(gzip), vec![Value::from(1_u8)]);
    let graph = read(&bytes, true, None);
    let offline = graph
        .diagnostics
        .into_iter()
        .find(|d| d.code == "DamagedFrame")
        .unwrap();
    let mut sink = DiagnosticSink::default();
    let result = read_to_sink(&bytes, true, None, &mut sink);
    drop(bytes);
    let returned = result
        .diagnostics
        .into_iter()
        .find(|d| d.code == "DamagedFrame")
        .unwrap();
    let observed = sink
        .diagnostics
        .into_iter()
        .find(|d| d.code == "DamagedFrame")
        .unwrap();
    assert_eq!(offline, observed);
    assert_eq!(returned, observed);
    assert!(core::ptr::eq(
        returned.native_cause().unwrap(),
        observed.native_cause().unwrap()
    ));
    drop(returned);
    drop(offline);
    let fields = Capture::read(&observed);
    assert_eq!(fields.code, "gts-diagnostic");
    assert!(matches!(
        fields.field("frame_index"),
        ErrorValue::Unsigned(1)
    ));
    let fields = fields.child("cause");
    assert_eq!(fields.code, "gts-payload-damaged");
    let fields = fields.child("detail");
    assert_eq!(fields.code, "gts-payload-codec");
    let fields = fields.child("cause");
    assert_eq!(fields.code, "gts-codec-failed");
    let fields = fields.child("detail").child("cause");
    assert_eq!(fields.code, "gzip-reserved-flags");
    assert!(matches!(fields.field("flags"), ErrorValue::Unsigned(value) if value & 0xe0 == 0xe0));
}

fn reader_file_and_chain_diagnostics_retain_native_inputs() {
    use purrdf_gts::{
        reader::{read, read_to_sink},
        wire,
        writer::Writer,
    };
    use purrdf_lex::cbor::{self, Value};
    let mut writer = Writer::new("generic");
    writer.add_terms(&[purrdf_gts::model::Term::iri("http://example.org/s")]);
    let valid = writer.into_bytes();
    let (items, torn) = wire::iter_items(&valid);
    assert_eq!(torn, None);
    let header = wire::unwrap_header(&items[0].1).unwrap();
    let Value::Map(frame) = &items[1].1 else {
        panic!("writer frame")
    };
    let mut cases = vec![
        (Vec::new(), true, None, "gts-empty-file"),
        (
            cbor::encode(&Value::Bool(false)),
            true,
            None,
            "gts-first-item-not-header",
        ),
        (
            [valid.as_slice(), valid.as_slice()].concat(),
            false,
            None,
            "gts-segment-boundary",
        ),
        (valid.clone(), true, Some(vec![19; 32]), "gts-expected-head"),
        (
            [valid.as_slice(), &[0x5a]].concat(),
            true,
            None,
            "gts-torn-append",
        ),
        (
            [cbor::encode(&items[0].1), cbor::encode(&Value::Bool(false))].concat(),
            true,
            None,
            "gts-frame-not-map",
        ),
    ];
    for (target, field, replacement, native) in [
        (0, "id", Value::Bytes(vec![71; 32]), "gts-header-self-hash"),
        (0, "v", Value::from(u64::MAX), "gts-header-version"),
        (1, "id", Value::Bytes(vec![72; 32]), "gts-frame-self-hash"),
        (1, "prev", Value::Bytes(vec![73; 32]), "gts-frame-previous"),
    ] {
        let mut entries = if target == 0 {
            header.clone()
        } else {
            frame.clone()
        };
        entries
            .iter_mut()
            .find(|(key, _)| key.as_text() == Some(field))
            .unwrap()
            .1 = replacement;
        if field != "id" {
            let id = if target == 0 {
                wire::header_id(&entries)
            } else {
                wire::content_id(&entries)
            };
            entries
                .iter_mut()
                .find(|(key, _)| key.as_text() == Some("id"))
                .unwrap()
                .1 = Value::Bytes(id);
        }
        let mut bytes = Vec::new();
        let altered = Value::Map(entries);
        for (index, (_, item)) in items.iter().enumerate() {
            cbor::encode_into(if index == target { &altered } else { item }, &mut bytes);
        }
        cases.push((bytes, true, None, native));
    }
    for (bytes, allow_segments, expected, code) in cases {
        let graph = read(&bytes, allow_segments, expected.as_deref());
        let mut sink = DiagnosticSink::default();
        let streamed = read_to_sink(&bytes, allow_segments, expected.as_deref(), &mut sink);
        for diagnostics in [&graph.diagnostics, &streamed.diagnostics, &sink.diagnostics] {
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| {
                    diagnostic
                        .native_cause()
                        .is_some_and(|cause| Capture::read(cause).code == code)
                })
                .unwrap_or_else(|| panic!("missing {code}: {diagnostics:?}"));
            let native = diagnostic.native_cause().unwrap();
            let fields = Capture::read(native);
            match code {
                "gts-segment-boundary" => {
                    assert!(matches!(fields.field("item"), ErrorValue::Unsigned(2)));
                }
                "gts-expected-head" => assert!(
                    matches!(fields.field("expected"), ErrorValue::Bytes(value) if value == [19; 32])
                ),
                "gts-torn-append" => assert!(
                    matches!(fields.field("offset"), ErrorValue::Unsigned(value) if value == valid.len() as u128)
                ),
                "gts-header-self-hash" => assert!(
                    matches!(fields.field("stored"), ErrorValue::Bytes(value) if value == [71; 32])
                ),
                "gts-frame-self-hash" => assert!(
                    matches!(fields.field("stored"), ErrorValue::Bytes(value) if value == [72; 32])
                ),
                "gts-frame-previous" => assert!(
                    matches!(fields.field("actual"), ErrorValue::Bytes(value) if value == [73; 32])
                ),
                "gts-header-version" => assert!(
                    matches!(fields.child("version").field("value"), ErrorValue::Signed(value) if value == i128::from(u64::MAX))
                ),
                "gts-empty-file" | "gts-first-item-not-header" | "gts-frame-not-map" => {
                    assert_eq!(fields.events.len(), 0, "{code}");
                }
                _ => unreachable!("all file controls are checked"),
            }
            let buffer = Vec::with_capacity(12);
            let window = CurrentThreadWindow::open();
            let captured = Capture::read_with_buffer(native, buffer);
            assert_eq!(window.close().allocations, 0);
            assert_eq!(captured.code, code);
        }
    }
}

fn reader_row_diagnostics_keep_native_widths_ids_and_term_kinds() {
    use purrdf_gts::{
        model::Term,
        reader::{read, read_to_sink},
        writer::{FrameOptions, Writer},
    };
    use purrdf_lex::cbor::Value;
    use purrdf_testkit::error_capture::Event;
    let cases = [
        (
            "reifies",
            vec![Value::from(0_u8)],
            "DamagedFrame",
            "gts-row-width",
        ),
        (
            "reifies",
            vec![
                Value::from(0_u8),
                Value::from(1_u8),
                Value::from(u64::MAX),
                Value::from(2_u8),
            ],
            "DamagedFrame",
            "gts-row-ids",
        ),
        (
            "annot",
            vec![Value::from(0_u8), Value::from(2_u8), Value::from(1_u8)],
            "PositionConstraint",
            "gts-annotation-predicate-kind",
        ),
        (
            "annot",
            vec![
                Value::from(0_u8),
                Value::from(1_u8),
                Value::from(2_u8),
                Value::from(2_u8),
            ],
            "PositionConstraint",
            "gts-annotation-graph-kind",
        ),
        (
            "quads",
            vec![Value::from(2_u8), Value::from(1_u8), Value::from(0_u8)],
            "PositionConstraint",
            "gts-row-positions",
        ),
        (
            "quads",
            vec![Value::from(0_u8), Value::from(1_u8), Value::from(u32::MAX)],
            "PositionConstraint",
            "gts-row-bounds",
        ),
        (
            "quads",
            vec![Value::from(0_u8), Value::from(1_u8), Value::Bool(true)],
            "DamagedFrame",
            "gts-quad-noninteger-ids",
        ),
    ];
    let cases = cases
        .into_iter()
        .map(|(kind, values, code, native)| {
            (kind, Value::Array(vec![Value::Array(values)]), code, native)
        })
        .chain([
            (
                "caller-frame",
                Value::Null,
                "UnknownFrameType",
                "gts-unknown-frame-type",
            ),
            (
                "reifies",
                Value::Bool(true),
                "DamagedFrame",
                "gts-reifies-not-array",
            ),
            (
                "snapshot",
                Value::Map(vec![(Value::from("reifies"), Value::Map(vec![]))]),
                "DamagedFrame",
                "gts-reifies-not-array",
            ),
            (
                "terms",
                Value::Array(vec![Value::Map(vec![
                    (Value::from("k"), Value::from(1_u8)),
                    (Value::from("l"), Value::from("en_XX")),
                    (Value::from("dir"), Value::from("rtl")),
                ])]),
                "DamagedFrame",
                "gts-refused-language",
            ),
            (
                "terms",
                Value::Array(vec![Value::Map(vec![
                    (Value::from("k"), Value::from(1_u8)),
                    (Value::from("l"), Value::from("en_XX")),
                    (Value::from("dir"), Value::from("rtl")),
                ])]),
                "DamagedFrame",
                "gts-refused-language-direction",
            ),
            (
                "terms",
                Value::Array(vec![Value::Map(vec![
                    (Value::from("k"), Value::from(1_u8)),
                    (Value::from("dir"), Value::from("sideways")),
                ])]),
                "DamagedFrame",
                "gts-refused-direction",
            ),
            (
                "terms",
                Value::Array(vec![Value::Map(vec![
                    (Value::from("dt"), Value::from(u64::MAX)),
                    (Value::from("rf"), Value::from(u64::MAX)),
                    (
                        Value::from("tt"),
                        Value::Array(vec![
                            Value::from(u64::MAX),
                            Value::from(0_u8),
                            Value::from(1_u8),
                        ]),
                    ),
                ])]),
                "ForwardReference",
                "gts-forward-reference",
            ),
        ]);
    for (frame_type, payload, diagnostic_code, native_code) in cases {
        let mut writer = Writer::new("generic");
        writer.add_terms(&[
            Term::iri("http://example.org/s"),
            Term::iri("http://example.org/p"),
            Term::literal("literal", None),
        ]);
        writer
            .add_frame_with_options(
                frame_type,
                FrameOptions {
                    payload: Some(payload),
                    ..FrameOptions::default()
                },
            )
            .unwrap();
        let bytes = writer.into_bytes();
        let offline = read(&bytes, true, None)
            .diagnostics
            .into_iter()
            .find(|value| {
                value.frame_index == Some(2)
                    && Capture::read(value).child("cause").code == native_code
            })
            .unwrap();
        assert_eq!(offline.code, diagnostic_code);
        let mut sink = DiagnosticSink::default();
        let returned = read_to_sink(&bytes, true, None, &mut sink)
            .diagnostics
            .into_iter()
            .find(|value| {
                value.frame_index == Some(2)
                    && Capture::read(value).child("cause").code == native_code
            })
            .unwrap();
        let observed = sink
            .diagnostics
            .into_iter()
            .find(|value| {
                value.frame_index == Some(2)
                    && Capture::read(value).child("cause").code == native_code
            })
            .unwrap();
        assert_eq!(offline, returned);
        assert_eq!(observed, returned);
        assert!(core::ptr::eq(
            returned.native_cause().unwrap(),
            observed.native_cause().unwrap()
        ));
        drop(bytes);
        drop(returned);
        let fields = Capture::read(&observed).child("cause");
        assert_eq!(fields.code, native_code);
        assert_eq!(Capture::read(&offline).child("cause").code, native_code);
        match native_code {
            "gts-row-width" => {
                assert!(matches!(fields.field("actual"), ErrorValue::Unsigned(1)));
                assert!(matches!(fields.field("minimum"), ErrorValue::Unsigned(4)));
                assert!(matches!(fields.field("maximum"), ErrorValue::Unsigned(5)));
            }
            "gts-row-ids" => {
                assert!(matches!(fields.field("terms"), ErrorValue::Unsigned(3)));
                assert!(fields.events.iter().any(|event| matches!(event, Event::Value(None, ErrorValue::Signed(value)) if *value == i128::from(u64::MAX))));
            }
            "gts-annotation-predicate-kind" | "gts-annotation-graph-kind" => {
                assert_eq!(fields.child("actual").code, "gts-term-literal");
            }
            "gts-row-positions" => {
                assert_eq!(fields.child("subject_kind").code, "gts-term-literal");
            }
            "gts-row-bounds" => assert!(
                matches!(fields.field("object"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
            ),
            "gts-quad-noninteger-ids" => {
                assert!(matches!(fields.field("width"), ErrorValue::Unsigned(3)));
            }
            "gts-unknown-frame-type" => assert!(matches!(
                fields.field("frame_type"),
                ErrorValue::Text("caller-frame")
            )),
            "gts-reifies-not-array" => assert!(
                matches!(fields.field("snapshot"), ErrorValue::Boolean(value) if value == (frame_type == "snapshot"))
            ),
            "gts-refused-language" => {
                assert!(matches!(fields.field("term"), ErrorValue::Signed(3)));
                assert!(matches!(fields.field("tag"), ErrorValue::Text("en_XX")));
                assert!(matches!(fields.field("refusal"), ErrorValue::Code(_)));
            }
            "gts-refused-language-direction" | "gts-refused-direction" => assert!(
                matches!(fields.field("direction"), ErrorValue::Text(value) if value == if native_code == "gts-refused-direction" { "sideways" } else { "rtl" })
            ),
            "gts-forward-reference" => {
                assert!(
                    matches!(fields.field("datatype"), ErrorValue::Signed(value) if value == i128::from(u64::MAX))
                );
                assert!(
                    matches!(fields.field("reifier"), ErrorValue::Signed(value) if value == i128::from(u64::MAX))
                );
                assert!(fields.events.iter().any(|event| matches!(event, Event::Value(None, ErrorValue::Signed(value)) if *value == i128::from(u64::MAX))));
            }
            _ => unreachable!("all native controls have an explicit assertion"),
        }
        let native = observed.native_cause().unwrap();
        let events = Vec::with_capacity(20);
        let window = CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(native, events);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(fields.code, native_code);
    }
}

fn reader_missing_catalog_keeps_ordered_native_cbor_fields() {
    use purrdf_lex::cbor::Value;
    let id = Value::Map(vec![
        (
            Value::from("k"),
            Value::Array(vec![
                Value::from(u64::MAX),
                Value::Bytes(vec![0, 255]),
                Value::Float(-0.0),
                Value::from("input"),
                Value::Bool(false),
                Value::Null,
                Value::Simple(23),
                Value::Tag(u64::MAX, Box::new(Value::from(7_u8))),
            ]),
        ),
        (Value::from("k"), Value::from("second")),
    ]);
    let bytes = refused_frame(Value::Bytes(Vec::new()), vec![id]);
    let diagnostic = purrdf_gts::reader::read(&bytes, true, None)
        .diagnostics
        .into_iter()
        .find(|d| d.code == "UnknownCodec")
        .unwrap();
    drop(bytes);
    let fields = Capture::read(&diagnostic).child("cause").child("detail");
    assert_eq!(fields.code, "gts-payload-missing-catalog");
    let ErrorValue::Record(id) = fields.field("id") else {
        panic!("native CBOR id must remain a record")
    };
    let events = Vec::with_capacity(32);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(id, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "cbor-map");
    let pairs = fields.records("pairs");
    assert_eq!(pairs.len(), 2);
    assert!(matches!(
        pairs[1].child("value").field("value"),
        ErrorValue::Text("second")
    ));
    let values = pairs[0].child("value").records("values");
    assert_eq!(values.len(), 8);
    assert!(
        matches!(values[0].field("value"), ErrorValue::Signed(value) if value == i128::from(u64::MAX))
    );
    assert!(matches!(
        values[1].field("value"),
        ErrorValue::Bytes([0, 255])
    ));
    assert!(
        matches!(values[2].field("bits"), ErrorValue::Float64Bits(bits) if bits == (-0.0_f64).to_bits())
    );
    assert!(matches!(
        values[3].field("value"),
        ErrorValue::Text("input")
    ));
    assert!(matches!(
        values[4].field("value"),
        ErrorValue::Boolean(false)
    ));
    assert_eq!(values[5].code, "cbor-null");
    assert!(matches!(values[6].field("value"), ErrorValue::Unsigned(23)));
    assert!(
        matches!(values[7].field("tag"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(
        values[7].child("value").field("value"),
        ErrorValue::Signed(7)
    ));
}

fn reader_blob_budget_keeps_original_counts_and_priority() {
    use purrdf_gts::{reader::read_to_sink, writer::Writer};
    let mut writer = Writer::new("generic");
    writer
        .add_frame_with_options(
            "blob",
            purrdf_gts::writer::FrameOptions {
                raw: Some(b"abcdef".to_vec()),
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .unwrap();
    let bytes = writer.into_bytes();
    for encoded in [None, Some(2)] {
        let mut sink = DiagnosticSink {
            encoded_limit: encoded,
            decoded_limit: Some(2),
            ..DiagnosticSink::default()
        };
        let result = read_to_sink(&bytes, true, None, &mut sink);
        let diagnostic = result
            .diagnostics
            .into_iter()
            .find(|d| d.code == purrdf_gts::reader::BLOB_BUDGET_DIAGNOSTIC)
            .unwrap();
        let fields = Capture::read(&diagnostic).child("cause").child("detail");
        assert_eq!(
            fields.code,
            if encoded.is_some() {
                "gts-payload-encoded-blob-limit"
            } else {
                "gts-payload-decoded-blob-limit"
            }
        );
        assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(6)));
        assert!(matches!(fields.field("limit"), ErrorValue::ByteCount(2)));
    }
    let mut writer = Writer::new("generic");
    writer
        .add_frame_with_options(
            "snapshot",
            purrdf_gts::writer::FrameOptions {
                payload: Some(purrdf_lex::cbor::Value::Map(vec![(
                    purrdf_lex::cbor::Value::from("blobs"),
                    purrdf_lex::cbor::Value::Map(vec![(
                        purrdf_lex::cbor::Value::from("entry"),
                        purrdf_lex::cbor::Value::Bytes(b"abcdef".to_vec()),
                    )]),
                )])),
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .unwrap();
    let mut sink = DiagnosticSink {
        decoded_limit: Some(2),
        ..DiagnosticSink::default()
    };
    let result = read_to_sink(&writer.into_bytes(), true, None, &mut sink);
    let diagnostic = result
        .diagnostics
        .into_iter()
        .find(|d| d.code == purrdf_gts::reader::BLOB_BUDGET_DIAGNOSTIC)
        .unwrap();
    let fields = Capture::read(&diagnostic).child("cause").child("detail");
    assert_eq!(fields.code, "gts-payload-snapshot-blob-limit");
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(6)));
    assert!(matches!(fields.field("limit"), ErrorValue::ByteCount(2)));
}

fn diagnostic_native_owners_release_without_rendering_the_cause() {
    use purrdf_gts::model::Diagnostic;
    use purrdf_testkit::error_capture::CallerRefusal;
    let window = CurrentThreadWindow::open();
    {
        let original = Diagnostic {
            code: "owned".into(),
            detail: "caller failure".into(),
            frame_index: Some(u64::MAX),
            cause: None,
        }
        .with_native_cause(CallerRefusal { limit: u128::MAX });
        let copy = original.clone();
        assert_eq!(copy, original);
        drop(original);
        let outer = copy.into_native_cause().unwrap();
        let copy = outer.clone();
        drop(outer);
        let fields = Capture::read(&**copy);
        assert!(
            matches!(fields.field("frame_index"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            fields.child("cause").field("limit"),
            ErrorValue::Unsigned(u128::MAX)
        ));
    }
    assert_eq!(window.close().retained_bytes, 0);
}

fn codec_selection_and_configuration_retain_original_values() {
    let missing = {
        let codec = Codec::new("caller-codec", "compress");
        decode_chain(&[codec], b"bytes").unwrap_err()
    };
    let fields = Capture::read(&missing);
    assert_eq!(fields.code, "gts-codec-unavailable");
    assert!(matches!(
        fields.field("reason"),
        ErrorValue::Code("unknown-codec")
    ));
    let cause = fields.child("detail");
    assert_eq!(cause.code, "gts-codec-unknown");
    assert!(matches!(
        cause.field("codec"),
        ErrorValue::Text("caller-codec")
    ));
    assert_eq!(
        missing.to_string(),
        "unknown-codec: unknown codec 'caller-codec'"
    );

    let chain = ["identity".to_owned()];
    let error = encode_chain_with_options(
        &chain,
        b"",
        EncodeOptions {
            zstd_level: Some(i32::MIN),
            dict: None,
        },
    )
    .unwrap_err();
    let fields = Capture::read(&error).child("detail");
    assert_eq!(fields.code, "gts-codec-unused-level");
    assert!(
        matches!(fields.field("level"), ErrorValue::Signed(value) if value == i128::from(i32::MIN))
    );
    assert_eq!(
        error.to_string(),
        "zstd_level requires a zstd or zstd-rsyncable transform"
    );

    let error = encode_chain_with_options(
        &chain,
        b"",
        EncodeOptions {
            zstd_level: None,
            dict: Some(b"dictionary"),
        },
    )
    .unwrap_err();
    let fields = Capture::read(&error).child("detail");
    assert_eq!(fields.code, "gts-codec-unused-dictionary");
    assert!(matches!(fields.field("bytes"), ErrorValue::ByteCount(10)));
}

fn codec_native_corruption_and_budget_remain_distinct() {
    let raw = b"native payload";
    let encoded = encode_chain(&["gzip".into()], raw).unwrap();
    let codec = Codec::new("gzip", "compress");
    assert_eq!(
        decode_chain_bounded(std::slice::from_ref(&codec), &encoded, raw.len()).unwrap(),
        raw
    );
    let limit = decode_chain_bounded(std::slice::from_ref(&codec), &encoded, 3).unwrap_err();
    let fields = Capture::read(&limit);
    assert_eq!(fields.code, "gts-codec-limit");
    assert!(matches!(fields.field("limit"), ErrorValue::ByteCount(3)));
    assert_eq!(
        limit.to_string(),
        "decoded transform output exceeds 3 bytes"
    );

    let mut damaged = encoded;
    damaged[3] |= 0xe0;
    let error = decode_chain(&[codec], &damaged).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    assert_eq!(fields.code, "gts-codec-gzip");
    let native = fields.child("cause");
    assert_eq!(native.code, "gzip-reserved-flags");
    assert!(matches!(native.field("flags"), ErrorValue::Unsigned(value) if value & 0xe0 == 0xe0));
    let source = std::error::Error::source(&error).unwrap();
    assert!(
        std::error::Error::source(source)
            .unwrap()
            .is::<purrdf_deflate::Error>()
    );
}

fn ulid_native_counts_and_decoder_causes_are_exact() {
    let error = Ulid::from_parts(u64::MAX, [0; 10]).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-ulid-timestamp");
    assert!(
        matches!(fields.field("actual"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == u128::from(Ulid::MAX_TIMESTAMP_MS))
    );
    assert_eq!(error.detail(), error.to_string());

    let error = Ulid::from_counter(0, u128::MAX).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-ulid-counter");
    assert!(matches!(
        fields.field("actual"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert!(
        matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == (1u128 << 80) - 1)
    );

    for (text, code) in [
        ("short", "crockford-length"),
        ("0000000000000000000000000!", "crockford-digit"),
        ("80000000000000000000000000", "crockford-overflow"),
    ] {
        let error = text.parse::<Ulid>().unwrap_err();
        let fields = Capture::read(&error);
        assert_eq!(fields.code, "gts-ulid-parse");
        assert_eq!(fields.child("cause").code, code);
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .is::<purrdf_lex::crockford::CrockfordError>()
        );
    }

    let mut sequence = DeterministicUlidGenerator::new(42).unwrap();
    assert_eq!(
        sequence.next_ulid().unwrap(),
        Ulid::from_counter(42, 0).unwrap()
    );
    assert_eq!(
        sequence.next_ulid().unwrap(),
        Ulid::from_counter(42, 1).unwrap()
    );
    let original = Ulid::from_counter(Ulid::MAX_TIMESTAMP_MS, (1u128 << 80) - 1).unwrap();
    assert_eq!(original.to_string().parse::<Ulid>().unwrap(), original);
}

fn caller_messages_remain_message_data() {
    let native = Ulid::from_counter(0, u128::MAX).unwrap_err();
    assert_eq!(native, UlidError::new(native.detail()));
    let original = String::from("caller explanation\0verbatim");
    let error = UlidError::new(original.clone());
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy);
    assert_eq!(fields.code, "gts-ulid-message");
    assert!(matches!(fields.field("message"), ErrorValue::Message(value) if value == original));
    assert_eq!(copy.detail(), original);
}

fn writer_catalog_and_frame_failures_keep_original_fields() {
    use purrdf_gts::writer::{FrameOptions, Writer, WriterOptions};
    let duplicate = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            catalog: Some(vec![
                (i64::MIN, Codec::new("identity", "encode")),
                (i64::MIN, Codec::new("zstd", "compress")),
            ]),
            ..WriterOptions::default()
        },
    )
    .unwrap_err();
    let fields = Capture::read(&duplicate).child("cause");
    assert_eq!(fields.code, "gts-writer-duplicate-codec-id");
    assert!(
        matches!(fields.field("id"), ErrorValue::Signed(value) if value == i128::from(i64::MIN))
    );
    assert_eq!(
        duplicate.to_string(),
        format!(
            "duplicate codec id {} in the writer catalog (§5 \"cat\" is a map)",
            i64::MIN
        )
    );

    let boundary = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            catalog: Some(vec![(i64::MAX, Codec::new("zstd", "compress"))]),
            dicts: vec![("pinned".into(), vec![1])],
            ..WriterOptions::default()
        },
    )
    .unwrap_err();
    let fields = Capture::read(&boundary).child("cause");
    assert_eq!(fields.code, "gts-writer-dictionary-id-start");
    assert!(
        matches!(fields.field("maximum"), ErrorValue::Signed(value) if value == i128::from(i64::MAX))
    );
    assert_eq!(
        boundary.to_string(),
        "writer catalog ids leave no range for dictionary-bound entries"
    );

    let mut writer = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            zstd_level: Some(3),
            ..WriterOptions::default()
        },
    )
    .unwrap();
    let error = writer
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(vec![1]),
                transform: vec!["zstd".into()],
                zstd_level: Some(i32::MIN),
                ..FrameOptions::default()
            },
        )
        .unwrap_err();
    let events = Vec::with_capacity(2);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    let fields = fields.child("cause");
    assert_eq!(fields.code, "gts-writer-level-mismatch");
    assert!(
        matches!(fields.field("requested"), ErrorValue::Signed(value) if value == i128::from(i32::MIN))
    );
    assert!(matches!(fields.field("declared"), ErrorValue::Signed(3)));
    assert_eq!(
        error.to_string(),
        format!(
            "frame zstd_level {} contradicts the pack's declared level 3 (§8.5 catalog \"level\")",
            i32::MIN
        )
    );
    // A refused frame must leave the real authoring path and chain usable.
    writer
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(vec![1]),
                ..FrameOptions::default()
            },
        )
        .unwrap();
    assert_eq!(
        purrdf_gts::reader::read(&writer.into_bytes(), true, None).diagnostics,
        []
    );
}

fn append_and_header_failures_keep_native_values_and_sources() {
    use purrdf_gts::{reader, wire, writer::Writer};
    use purrdf_lex::{assoc, cbor::Value};
    let writer = Writer::new("purrdf.gts");
    let (mut items, torn) = wire::iter_items(&writer.into_bytes());
    assert_eq!(torn, None);
    let Value::Tag(_, inner) = &mut items[0].1 else {
        panic!("writer emits a tagged header")
    };
    let header = inner.as_map_mut().unwrap();
    assoc::insert(
        header,
        "cat".into(),
        Value::Map(vec![(
            Value::from(0_u8),
            Value::Map(vec![
                ("name".into(), "zstd".into()),
                ("level".into(), Value::from(u64::MAX)),
            ]),
        )]),
    );
    let bytes = wire::canonical(&items[0].1);
    let error = Writer::appending(&bytes).unwrap_err();
    drop(bytes);
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-append-level-range");
    assert!(
        matches!(fields.field("actual"), ErrorValue::Signed(value) if value == i128::from(u64::MAX))
    );
    assert!(matches!(fields.field("id"), ErrorValue::Signed(0)));
    assert_eq!(
        error.to_string(),
        "cannot append: a catalog level is out of range"
    );
    assert!(std::error::Error::source(&error).is_some());

    let error = reader::segment_append_state(&[0x9f]).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-append-torn");
    assert!(matches!(fields.field("offset"), ErrorValue::ByteCount(0)));

    let item = Value::Tag(u64::MAX, Box::new(Value::Null));
    let error = wire::unwrap_header(&item).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-header-tag");
    assert!(
        matches!(fields.field("actual"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(fields.field("expected"), ErrorValue::Unsigned(value) if value == u128::from(wire::SELF_DESCRIBE_TAG))
    );
    let error = wire::unwrap_header(&Value::Bytes(vec![0, 255])).unwrap_err();
    let fields = Capture::read(&error).child("value");
    assert_eq!(fields.code, "cbor-bytes");
    assert!(matches!(fields.field("value"), ErrorValue::Bytes([0, 255])));
}

fn invalid_header_cause_survives_graph_and_streaming_diagnostics() {
    use purrdf_gts::{reader, wire};
    use purrdf_lex::cbor::Value;
    let bytes = wire::canonical(&Value::Tag(
        u64::MAX,
        Box::new(Value::Map(vec![("gts".into(), wire::MAGIC.into())])),
    ));
    let graph = reader::read(&bytes, true, None);
    let mut sink = DiagnosticSink::default();
    let streamed = reader::read_to_sink(&bytes, true, None, &mut sink);
    drop(bytes);
    for diagnostics in [&graph.diagnostics, &streamed.diagnostics, &sink.diagnostics] {
        let error = diagnostics
            .iter()
            .find(|error| error.detail.starts_with("invalid header:"))
            .unwrap();
        let fields = Capture::read(error).child("cause");
        assert_eq!(fields.code, "gts-header-tag");
        assert!(
            matches!(fields.field("actual"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
    }
}

fn memory_example_keeps_confidence_bits_and_original_io_cause() {
    use purrdf_gts::examples::agent_memory::{Memory, MemoryError, StoreOptions};
    use purrdf_testkit::error_capture::CallerRefusal;
    let memory = Memory::new("unused.gts");
    // Admission refuses before any clock or filesystem access.
    for bits in [
        0x7ff8_0000_0000_0042,
        f64::INFINITY.to_bits(),
        (-0.5_f64).to_bits(),
    ] {
        let error = memory
            .store(
                "a claim",
                StoreOptions {
                    confidence: Some(f64::from_bits(bits)),
                    ..StoreOptions::default()
                },
            )
            .unwrap_err();
        let fields = Capture::read(&error);
        assert_eq!(fields.code, "gts-memory-invalid-confidence");
        assert!(
            matches!(fields.field("actual"), ErrorValue::Float64Bits(actual) if actual == bits)
        );
        assert_eq!(
            error.to_string(),
            "confidence must be finite and in the range [0, 1]"
        );
    }
    let original = purrdf_hash::error::io_error(
        std::io::ErrorKind::Other,
        CallerRefusal { limit: u128::MAX },
    );
    let error = MemoryError::from(original);
    assert!(std::error::Error::source(&error).is_some());
    let fields = Capture::read(&error)
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(fields.code, "caller-refused");
    assert!(matches!(
        fields.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
}

fn compaction_keeps_timestamp_and_authoring_causes() {
    use purrdf_gts::compact::{CompactionParams, DictPlan, compact_streamable};
    use purrdf_gts::{fixture::fixed_key, writer::Writer};
    let source = Writer::new("purrdf.gts").into_bytes();
    for (timestamp, expected) in [
        ("2026-01-01T00:00:00+01:00", "gts-compact-timestamp-zone"),
        ("2026-02-30T00:00:00Z", "gts-compact-timestamp-parse"),
    ] {
        let error = compact_streamable(
            &source,
            CompactionParams {
                timestamp,
                seal_original: false,
                plan: DictPlan::undicted(),
                content_digest: None,
                packaging_signer: (fixed_key(99), "pack".into()),
            },
        )
        .unwrap_err();
        let fields = Capture::read(&error);
        assert_eq!(fields.code, "gts-compact-timestamp");
        assert!(matches!(fields.field("actual"), ErrorValue::Text(value) if value == timestamp));
        let cause = fields.child("cause");
        assert_eq!(cause.code, expected);
        if expected == "gts-compact-timestamp-zone" {
            assert!(matches!(cause.field("minutes"), ErrorValue::Signed(60)));
        } else {
            // The datatype's original structured error remains traversable.
            assert!(!cause.child("cause").events.is_empty());
        }
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(
            error.to_string(),
            "rewrite timestamp must be a valid XSD dateTime with an explicit UTC timezone"
        );
    }
    let mut plan = DictPlan::undicted();
    plan.transform = vec!["unregistered".into()];
    let error = compact_streamable(
        &source,
        CompactionParams {
            timestamp: "2026-01-01T00:00:00Z",
            seal_original: false,
            plan,
            content_digest: None,
            packaging_signer: (fixed_key(99), "pack".into()),
        },
    )
    .unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-compact-author-frame");
    assert!(matches!(
        fields.field("frame_type"),
        ErrorValue::Code("terms")
    ));
    let cause = fields.child("cause");
    assert_eq!(cause.code, "gts-writer-missing-codec");
    assert!(matches!(
        cause.field("name"),
        ErrorValue::Text("unregistered")
    ));
    assert_eq!(
        error.to_string(),
        "cannot author the terms frame: writer catalog has no entry for codec 'unregistered'"
    );
    let mut gzip = encode_chain(&["gzip".into()], b"payload").unwrap();
    gzip[3] |= 0xe0;
    let source = refused_frame(purrdf_lex::cbor::Value::Bytes(gzip), vec![1_u8.into()]);
    let error = compact_streamable(
        &source,
        CompactionParams {
            timestamp: "2026-01-01T00:00:00Z",
            seal_original: false,
            plan: DictPlan::undicted(),
            content_digest: None,
            packaging_signer: (fixed_key(99), "pack".into()),
        },
    )
    .unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-compact-segment");
    let diagnostic = fields.child("cause");
    assert_eq!(diagnostic.code, "gts-diagnostic");
    let original = diagnostic.child("cause");
    assert_eq!(original.code, "gts-payload-damaged");
    let original = original
        .child("detail")
        .child("cause")
        .child("detail")
        .child("cause");
    assert_eq!(original.code, "gzip-reserved-flags");
    assert!(matches!(original.field("flags"), ErrorValue::Unsigned(flags) if flags & 0xe0 == 0xe0));
}

fn compaction_provenance_lists_and_signature_failures_keep_native_fields() {
    use purrdf_gts::compact::{compaction_signature_roots, detached_signature_pairs};
    use purrdf_gts::model::{Graph, Signature};
    use purrdf_testkit::error_capture::Event;
    let mut graph = Graph::default();
    let heads = [vec![255; 32], vec![0; 32]];
    let error = compaction_signature_roots(&graph, &heads).unwrap_err();
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "gts-compact-missing-provenance");
    assert!(matches!(fields.events.as_slice(), [
        Event::BeginList(Some("source_heads"), 2),
        Event::Value(None, ErrorValue::Bytes(first)),
        Event::Value(None, ErrorValue::Bytes(second)),
        Event::EndList,
    ] if *first == heads[1] && *second == heads[0]));
    graph.signatures.push(Signature {
        frame_id: vec![7; 32],
        kid: None,
        status: "unverified".into(),
        cose: None,
        packaging: false,
    });
    let error = detached_signature_pairs(&graph).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-compact-frame-missing-cose");
    assert!(matches!(fields.field("frame_id"), ErrorValue::Bytes(bytes) if bytes == [7; 32]));
    graph.signatures[0].cose = Some(Vec::new());
    let error = detached_signature_pairs(&graph).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-compact-frame-cose");
    assert_eq!(fields.child("cause").code, "gts-sign1-malformed");
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<purrdf_gts::cose::Sign1Error>()
    );
}

fn compaction_identifier_decoders_retain_original_rejected_values() {
    use purrdf_gts::{compact::base64url_decode, mmr::parse_hex_32};
    let error = parse_hex_32("blake3:short").unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-hex32-length");
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(5)));
    let text = format!("{}g", "0".repeat(63));
    let error = parse_hex_32(&text).unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "hex-invalid-digit");
    assert!(matches!(fields.field("offset"), ErrorValue::ByteCount(63)));
    assert!(
        matches!(fields.field("byte"), ErrorValue::Unsigned(value) if value == u128::from(b'g'))
    );
    assert_eq!(error.to_string(), "hex value contains a non-hex character");
    let error = base64url_decode("a").unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-base64url-length");
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(1)));
    let error = base64url_decode("a=").unwrap_err();
    assert_eq!(Capture::read(&error).code, "gts-base64url-padding");
    let error = base64url_decode("a/").unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-base64url-alphabet");
    assert!(
        matches!(fields.field("byte"), ErrorValue::Unsigned(value) if value == u128::from(b'/'))
    );
    assert_eq!(base64url_decode("Y2F0").unwrap(), b"cat");
    assert_eq!(parse_hex_32(&"aB".repeat(32)).unwrap(), [0xab; 32]);
}

fn proof_json_retains_rejected_values_without_formatting_or_number_rounding() {
    use purrdf_gts::mmr::Proof;
    let error =
        Proof::from_json(r#"{"schema":{"n":18446744073709551616,"n":[false,null,"verbatim"]}}"#)
            .unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-json-string");
    assert!(matches!(fields.field("field"), ErrorValue::Code("schema")));
    let actual = fields.child("actual");
    assert_eq!(actual.code, "json-object");
    let members = actual.records("members");
    assert_eq!(members.len(), 2);
    for member in &members {
        assert!(matches!(member.field("name"), ErrorValue::Text("n")));
    }
    let number = members[0].child("value");
    assert_eq!(number.code, "json-number");
    assert!(matches!(
        number.field("lexeme"),
        ErrorValue::Text("18446744073709551616")
    ));
    let values = members[1].child("value").records("values");
    assert_eq!(values[0].code, "json-boolean");
    assert!(matches!(
        values[0].field("value"),
        ErrorValue::Boolean(false)
    ));
    assert_eq!(values[1].code, "json-null");
    assert_eq!(values[2].code, "json-string");
    assert!(matches!(
        values[2].field("value"),
        ErrorValue::Text("verbatim")
    ));
    let ErrorValue::Record(value) = fields.field("actual") else {
        panic!("original value")
    };
    let events = Vec::with_capacity(10);
    let window = CurrentThreadWindow::open();
    let captured = Capture::read_with_buffer(value, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(captured.code, "json-object");

    let nested = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    let error = Proof::from_json(&nested).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-json");
    let original = fields.child("cause");
    assert_eq!(original.child("kind").code, "json-depth-limit");
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<purrdf_lex::json::Error>()
    );
    let proof = purrdf_gts::mmr::prove(&[vec![7; 32]], 0).unwrap();
    let text = proof
        .to_json()
        .replace("\"count\": 1,", "\"count\": 18446744073709551616,");
    let error = Proof::from_json(&text).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-json-unsigned");
    assert!(matches!(fields.field("field"), ErrorValue::Code("count")));
    assert!(matches!(
        fields.child("actual").field("lexeme"),
        ErrorValue::Text("18446744073709551616")
    ));
}

fn proof_verification_keeps_native_bounds_and_reconstructed_hashes() {
    use purrdf_gts::mmr::{prove, verify_proof};
    let proof = prove(&[vec![7; 32]], 0).unwrap();
    verify_proof(&proof).unwrap();
    let mut outside = proof.clone();
    outside.count = usize::MAX;
    outside.leaf_index = usize::MAX;
    let error = verify_proof(&outside).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-leaf-range");
    assert!(
        matches!(fields.field("leaf_index"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(
        matches!(fields.field("count"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    let mut changed = proof.clone();
    changed.root[0] ^= 1;
    let error = verify_proof(&changed).unwrap_err();
    let expected = changed.root.clone();
    drop(changed);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-root-mismatch");
    assert!(matches!(fields.field("actual"), ErrorValue::Bytes(value) if value == proof.root));
    assert!(matches!(fields.field("expected"), ErrorValue::Bytes(value) if value == expected));
    assert_eq!(
        error.to_string(),
        "proof peaks do not reconstruct the declared root"
    );
    let mut changed = proof.clone();
    changed.peaks[0].hash[0] ^= 1;
    let error = verify_proof(&changed).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-peak-mismatch");
    assert!(
        matches!(fields.field("actual"), ErrorValue::Bytes(value) if value == proof.peaks[0].hash)
    );
    assert!(
        matches!(fields.field("expected"), ErrorValue::Bytes(value) if value == changed.peaks[0].hash)
    );
}

fn file_proof_keeps_original_identity_faults_and_valid_proofs() {
    use purrdf_gts::{
        mmr::{prove_file, verify_proof},
        wire,
        writer::Writer,
    };
    use purrdf_lex::{assoc, cbor::Value};
    let mut writer = Writer::new("purrdf.gts");
    let target = writer.add_blob(b"proof payload", None, None);
    writer.add_index_with_mmr();
    let bytes = writer.into_bytes();
    let proof = prove_file(&bytes, &target).unwrap();
    verify_proof(&proof).unwrap();
    let (mut items, torn) = wire::iter_items(&bytes);
    assert_eq!(torn, None);
    let frame = items[1].1.as_map_mut().unwrap();
    assoc::insert(frame, "id".into(), Value::Bytes(vec![9; 32]));
    let corrupted: Vec<u8> = items
        .into_iter()
        .flat_map(|(_, item)| wire::canonical(&item))
        .collect();
    let error = prove_file(&corrupted, &target).unwrap_err();
    drop(corrupted);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-file-id-mismatch");
    assert!(matches!(fields.field("item"), ErrorValue::Unsigned(1)));
    assert!(matches!(fields.field("header"), ErrorValue::Boolean(false)));
    assert!(matches!(fields.field("actual"), ErrorValue::Bytes(value) if value == [9; 32]));
    assert!(matches!(fields.field("expected"), ErrorValue::Bytes(value) if value == target));
    assert_eq!(error.to_string(), "item 1 id mismatch");
    let error = prove_file(&[], &[1]).unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "gts-proof-digest-length");
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(1)));
}

purrdf_testkit::harness_main!(
    reader_file_and_chain_diagnostics_retain_native_inputs,
    reader_row_diagnostics_keep_native_widths_ids_and_term_kinds,
    tar_blob_sources_keep_observed_size_and_digest_refusals,
    tar_and_files_public_io_retain_unformattable_original_causes,
    tar_metadata_and_file_admission_keep_original_numbers_and_parser_causes,
    proof_json_retains_rejected_values_without_formatting_or_number_rounding,
    proof_verification_keeps_native_bounds_and_reconstructed_hashes,
    file_proof_keeps_original_identity_faults_and_valid_proofs,
    compaction_keeps_timestamp_and_authoring_causes,
    compaction_provenance_lists_and_signature_failures_keep_native_fields,
    compaction_identifier_decoders_retain_original_rejected_values,
    memory_example_keeps_confidence_bits_and_original_io_cause,
    writer_catalog_and_frame_failures_keep_original_fields,
    append_and_header_failures_keep_native_values_and_sources,
    invalid_header_cause_survives_graph_and_streaming_diagnostics,
    transport_retains_native_source_errors_before_foreign_formatting,
    native_transport_framing_and_decoder_fields_survive,
    dictionary_and_key_envelope_errors_keep_native_fields,
    reader_causes_survive_input_graph_and_streaming_owners,
    reader_missing_catalog_keeps_ordered_native_cbor_fields,
    reader_blob_budget_keeps_original_counts_and_priority,
    diagnostic_native_owners_release_without_rendering_the_cause,
    codec_selection_and_configuration_retain_original_values,
    codec_native_corruption_and_budget_remain_distinct,
    ulid_native_counts_and_decoder_causes_are_exact,
    caller_messages_remain_message_data,
);
