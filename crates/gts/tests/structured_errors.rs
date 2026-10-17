// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native transform and identifier failures through one generic consumer.

use purrdf_gts::codec::{
    Codec, EncodeOptions, decode_chain, decode_chain_bounded, encode_chain,
    encode_chain_with_options,
};
use purrdf_gts::ulid::{DeterministicUlidGenerator, Ulid, UlidError};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

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

purrdf_testkit::harness_main!(
    codec_selection_and_configuration_retain_original_values,
    codec_native_corruption_and_budget_remain_distinct,
    ulid_native_counts_and_decoder_causes_are_exact,
    caller_messages_remain_message_data,
);
