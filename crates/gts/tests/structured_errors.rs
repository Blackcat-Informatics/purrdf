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

purrdf_testkit::harness_main!(
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
