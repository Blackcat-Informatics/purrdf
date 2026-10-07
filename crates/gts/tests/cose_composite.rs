// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact pinned IETF construction and strict public Sign1 boundaries.

use purrdf_ed25519::SigningKey as EdSigningKey;
use purrdf_gts::cose::{Algorithm, SigStatus, SigningKeyRef, VerifyingKeyRef, composite};
use purrdf_gts::{cose, model, wire};
use purrdf_hash::hex;
use purrdf_lex::cbor::{self, Value};
use purrdf_testkit::vectors::VectorFile;

mod composite_support;
use composite_support::composite_key;

fn hex_bytes(field: &str) -> Vec<u8> {
    hex::decode(&purrdf_testkit::vectors::decode_str(field).unwrap()).unwrap()
}

fn equal_bytes(actual: &[u8], expected: &[u8], name: &str) {
    assert_eq!(actual.len(), expected.len(), "{name}: length");
    assert_eq!(
        actual.iter().zip(expected).position(|(a, b)| a != b),
        None,
        "{name}: first difference"
    );
}

fn ietf_published_cose_example_complete_bytes() {
    let file = VectorFile::load(include_str!("composite/ietf-cose-example.txt"));
    assert_eq!(file.records().len(), 1);
    let fields = &file.records()[0].fields;
    assert_eq!(fields.len(), 8);
    let fields: Vec<_> = fields.iter().map(|field| hex_bytes(field)).collect();
    let [
        ml_seed,
        ed_seed,
        kid,
        public,
        seeds,
        payload,
        expected_representative,
        signature,
    ] = fields.as_slice()
    else {
        panic!("exact external fixture fields");
    };
    assert_eq!(ml_seed, &[0; 32]);
    assert_eq!(ed_seed, &[0; 32]);
    assert_eq!(seeds, &[0; 64]);
    assert_eq!(payload, b"hello post quantum signatures");
    assert_eq!(kid, &hex::decode("93fe1616a4131f5a").unwrap());
    assert_eq!(public.len(), composite::PUBLIC_KEY_LENGTH);
    assert_eq!(signature.len(), composite::SIGNATURE_LENGTH);
    let protected = wire::encode(&Value::Map(vec![
        (1.into(), (-58).into()),
        (4.into(), Value::Bytes(kid.clone())),
    ]));
    let message = cose::sig_structure(&protected, payload);
    equal_bytes(
        &composite::representative(&message),
        expected_representative,
        "IETF representative",
    );
    let signing = composite::SigningKey::from_bytes(seeds).unwrap();
    equal_bytes(
        &signing.verifying_key().to_bytes(),
        public,
        "IETF public key",
    );
    let verifying = composite::VerifyingKey::from_bytes(public).unwrap();
    let signature = composite::Signature::from_bytes(signature).unwrap();
    equal_bytes(&signature.to_bytes(), &fields[7], "IETF signature encoding");
    verifying.verify(&message, &signature).unwrap();
    let generated = signing.sign_deterministic(&message).unwrap();
    equal_bytes(
        &generated.to_bytes(),
        &fields[7],
        "IETF deterministic signature",
    );
    let mut export = [0; composite::SECRET_KEY_LENGTH];
    signing.export_seeds(&mut export);
    equal_bytes(&export, seeds, "IETF private seed encoding");
    purrdf_ed25519::wipe_secret(&mut export);
    // The published example is attached. Its signature authenticates the same
    // preimage when the externally supplied payload is detached: a valid
    // opaque-binary-kid neighbor, without accepting attached GTS payloads.
    let attached = wire::encode(&Value::Tag(
        18,
        Box::new(Value::Array(vec![
            Value::Bytes(protected),
            Value::Map(vec![]),
            Value::Bytes(payload.clone()),
            Value::Bytes(signature.to_bytes().to_vec()),
        ])),
    ));
    assert!(cose::parse_sign1(&attached).is_err());
    let mut detached = parts(&attached);
    detached[2] = Value::Null;
    let detached = wrap(detached);
    let parsed = cose::parse_sign1(&detached).unwrap();
    assert_eq!(parsed.kid(), Some(kid.as_slice()));
    assert!(parsed.kid_text().is_none());
    assert_eq!(
        parsed.verify(payload, VerifyingKeyRef::Composite(&verifying)),
        SigStatus::Valid
    );
}

fn frozen_eddsa_sign1_bytes_and_conveniences() {
    for fixture in [
        include_str!("../../../vectors/cose/sign1-basic.json"),
        include_str!("../../../vectors/cose/sign1-empty-id.json"),
    ] {
        let vector = purrdf_lex::json::read(fixture).unwrap();
        let bytes = |field: &str| hex::decode(vector[field].as_str().unwrap()).unwrap();
        let kid = vector["kid"].as_str().unwrap();
        let frame_id = bytes("frame_id");
        let key = EdSigningKey::from_bytes(bytes("seed").as_slice().try_into().unwrap());
        let expected = bytes("cose");
        equal_bytes(
            &cose::sign_id(&frame_id, &key, kid),
            &expected,
            "frozen Ed25519 Sign1",
        );
        equal_bytes(
            &cose::sign_id_deterministic(&frame_id, SigningKeyRef::Ed25519(&key), kid).unwrap(),
            &expected,
            "typed Ed25519 Sign1",
        );
        let parsed = cose::parse_sign1(&expected).unwrap();
        assert_eq!(parsed.algorithm(), Algorithm::Ed25519);
        assert_eq!(parsed.kid(), Some(kid.as_bytes()));
        assert_eq!(parsed.protected(), [0xa1, 0x01, 0x27]);
        assert_eq!(parsed.signature_bytes().len(), 64);
        assert!(cose::parse(&expected).is_some());
        assert_eq!(
            cose::verify_sig(&expected, &frame_id, &key.verifying_key()),
            SigStatus::Valid
        );
        assert_eq!(cose::signature_kid(&expected).as_deref(), Some(kid));
    }
}

fn parts(envelope: &[u8]) -> Vec<Value> {
    let value = cbor::decode(envelope, cbor::Limits::DEFAULT).unwrap();
    value.as_tag().unwrap().1.as_array().unwrap().clone()
}
fn wrap(parts: Vec<Value>) -> Vec<u8> {
    wire::encode(&Value::Tag(18, Box::new(Value::Array(parts))))
}
fn with_signature(envelope: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut fields = parts(envelope);
    fields[3] = Value::Bytes(signature.to_vec());
    wrap(fields)
}
fn public_composite_sign1_deterministic_hedged_and_key_dispatch() {
    let key = composite_key(7);
    let public = key.verifying_key();
    let frame = [17; 32];
    let first = cose::sign_id_deterministic(&frame, SigningKeyRef::Composite(&key), "").unwrap();
    let second = cose::sign_id_deterministic(&frame, SigningKeyRef::Composite(&key), "").unwrap();
    assert_eq!(first, second);
    let structure = cbor::decode(&first, cbor::Limits::DEFAULT).unwrap();
    let (tag, body) = structure.as_tag().unwrap();
    assert_eq!(tag, 18);
    let fields = body.as_array().unwrap();
    assert_eq!(fields.len(), 4);
    assert!(matches!(fields[2], Value::Null));
    assert_eq!(fields[3].as_bytes().unwrap().len(), 3373);
    assert_eq!(fields[0].as_bytes().unwrap(), [0xa1, 1, 0x38, 57]);
    let parsed = cose::parse_sign1(&first).unwrap();
    assert_eq!(parsed.algorithm(), Algorithm::CompositeMlDsa65Ed25519);
    assert_eq!(parsed.kid(), Some(b"".as_slice()));
    assert!(cose::parse(&first).is_none());
    assert_eq!(cose::signature_kid(&first).as_deref(), Some(""));
    assert_eq!(
        cose::verify_sig_with_key(&first, &frame, VerifyingKeyRef::Composite(public)),
        SigStatus::Valid
    );
    assert_eq!(
        parsed.verify(&frame, VerifyingKeyRef::Composite(public)),
        SigStatus::Valid
    );
    assert_eq!(
        parsed.verify(b"changed", VerifyingKeyRef::Composite(public)),
        SigStatus::Invalid
    );
    let ed = EdSigningKey::from_bytes(&[8; 32]);
    assert_eq!(
        cose::verify_sig(&first, &frame, &ed.verifying_key()),
        SigStatus::Invalid
    );
    let classical = cose::sign_id(&frame, &ed, "ed");
    assert_eq!(
        cose::verify_sig_with_key(&classical, &frame, VerifyingKeyRef::Composite(public)),
        SigStatus::Invalid
    );
    let hedged1 =
        cose::sign_id_hedged(&frame, SigningKeyRef::Composite(&key), "c", &[1; 32]).unwrap();
    let hedged2 =
        cose::sign_id_hedged(&frame, SigningKeyRef::Composite(&key), "c", &[2; 32]).unwrap();
    assert_ne!(hedged1, hedged2);
    for envelope in [hedged1, hedged2] {
        assert_eq!(
            cose::verify_sig_with_key(&envelope, &frame, VerifyingKeyRef::Composite(public)),
            SigStatus::Valid
        );
    }
    // No outer tag is allowed as RFC 9052 permits; still only detached Sign1.
    let untagged = wire::encode(body);
    assert_eq!(
        cose::verify_sig_with_key(&untagged, &frame, VerifyingKeyRef::Composite(public)),
        SigStatus::Valid
    );
}

fn each_component_is_mandatory_and_cannot_be_spliced() {
    let key = composite_key(11);
    let other_key = composite_key(13);
    let frame = b"authenticated frame";
    let first = cose::sign_id_deterministic(frame, SigningKeyRef::Composite(&key), "c").unwrap();
    let source = cose::parse_sign1(&first)
        .unwrap()
        .signature_bytes()
        .to_vec();
    let other_message =
        cose::sign_id_deterministic(b"another frame", SigningKeyRef::Composite(&key), "c").unwrap();
    let other_signer =
        cose::sign_id_deterministic(frame, SigningKeyRef::Composite(&other_key), "c").unwrap();
    let verify = |signature: &[u8]| {
        assert_eq!(
            cose::verify_sig_with_key(
                &with_signature(&first, signature),
                frame,
                VerifyingKeyRef::Composite(key.verifying_key())
            ),
            SigStatus::Invalid
        );
    };
    const SPLIT: usize = purrdf_gts::mldsa65::SIGNATURE_LENGTH;
    for truncated in [
        &source[..0],
        &source[..SPLIT],
        &source[SPLIT..],
        &source[..source.len() - 1],
    ] {
        verify(truncated);
    }
    for range in [0..SPLIT, SPLIT..source.len()] {
        let mut zeroed = source.clone();
        zeroed[range].fill(0);
        verify(&zeroed);
    }
    for position in [
        0,
        47,
        48,
        3248,
        SPLIT - 1,
        SPLIT,
        SPLIT + 31,
        SPLIT + 32,
        source.len() - 1,
    ] {
        let mut corrupted = source.clone();
        corrupted[position] ^= 1;
        verify(&corrupted);
    }
    for envelope in [&other_message, &other_signer] {
        let alternate = cose::parse_sign1(envelope)
            .unwrap()
            .signature_bytes()
            .to_vec();
        for range in [0..SPLIT, SPLIT..source.len()] {
            let mut spliced = source.clone();
            spliced[range.clone()].copy_from_slice(&alternate[range]);
            verify(&spliced);
        }
    }
    verify(&[&source[SPLIT..], &source[..SPLIT]].concat());
    verify(&[source.as_slice(), &[0]].concat());
    let public = key.verifying_key().to_bytes();
    let alternate = other_key.verifying_key().to_bytes();
    for range in [
        0..purrdf_gts::mldsa65::PUBLIC_KEY_LENGTH,
        purrdf_gts::mldsa65::PUBLIC_KEY_LENGTH..public.len(),
    ] {
        let mut mixed = public;
        mixed[range.clone()].copy_from_slice(&alternate[range]);
        let mixed = composite::VerifyingKey::from_bytes(&mixed).unwrap();
        assert_eq!(
            cose::verify_sig_with_key(&first, frame, VerifyingKeyRef::Composite(&mixed)),
            SigStatus::Invalid
        );
    }
    // A stripped traditional component remains domain-separated and bound to
    // the original protected alg; changing alg cannot create an EdDSA signature.
    let mut downgrade = parts(&first);
    downgrade[0] = Value::Bytes(vec![0xa1, 1, 0x27]);
    downgrade[3] = Value::Bytes(source[SPLIT..].to_vec());
    let ed = EdSigningKey::from_bytes(&[12; 32]);
    assert_eq!(
        cose::verify_sig(&wrap(downgrade), frame, &ed.verifying_key()),
        SigStatus::Invalid
    );
}

fn canonical_composite_key_encodings_and_strict_ed25519() {
    let key = composite_key(21);
    let mut public = key.verifying_key().to_bytes();
    let signature = key.sign_deterministic(b"message").unwrap().to_bytes();
    for length in [
        0,
        composite::PUBLIC_KEY_LENGTH - 1,
        composite::PUBLIC_KEY_LENGTH + 1,
    ] {
        assert_eq!(
            composite::VerifyingKey::from_bytes(&vec![0; length]),
            Err(composite::Error::InvalidPublicKey)
        );
    }
    for length in [0, 63, 65] {
        assert_eq!(
            composite::SigningKey::from_bytes(&vec![0; length]).unwrap_err(),
            composite::Error::InvalidSecretKey
        );
    }
    for length in [0, 64, 3309, 3372, 3374] {
        assert_eq!(
            composite::Signature::from_bytes(&vec![0; length]),
            Err(composite::Error::InvalidSignature)
        );
    }
    let start = purrdf_gts::mldsa65::PUBLIC_KEY_LENGTH;
    for encoding in [[0; 32], [255; 32]] {
        public[start..].copy_from_slice(&encoding);
        assert_eq!(
            composite::VerifyingKey::from_bytes(&public),
            Err(composite::Error::InvalidPublicKey)
        );
    }
    let mut identity = [0; 32];
    identity[0] = 1;
    public[start..].copy_from_slice(&identity);
    assert_eq!(
        composite::VerifyingKey::from_bytes(&public),
        Err(composite::Error::InvalidPublicKey)
    );
    let start = purrdf_gts::mldsa65::SIGNATURE_LENGTH;
    for (range, byte) in [
        (start..start + 32, 0),
        (start..start + 32, 255),
        (start + 32..signature.len(), 255),
    ] {
        let mut changed = signature;
        changed[range].fill(byte);
        let decoded = composite::Signature::from_bytes(&changed).unwrap();
        assert_eq!(
            key.verifying_key().verify(b"message", &decoded),
            Err(composite::Error::InvalidSignature)
        );
    }
    let mut seeds = [0; 64];
    key.export_seeds(&mut seeds);
    let cloned = key.clone();
    assert_eq!(cloned.verifying_key(), key.verifying_key());
    assert!(!format!("{key:?}").contains("seeds"));
    purrdf_ed25519::wipe_secret(&mut seeds);
}

fn raw_ed_signature(protected: &[u8], headers: Vec<(Value, Value)>) -> Vec<u8> {
    let key = EdSigningKey::from_bytes(&[31; 32]);
    let signature = key.sign(&cose::sig_structure(protected, b"frame"));
    wrap(vec![
        Value::Bytes(protected.to_vec()),
        Value::Map(headers),
        Value::Null,
        Value::Bytes(signature.to_bytes().to_vec()),
    ])
}

fn headers_are_authenticated_unique_typed_and_critical_rules_enforced() {
    let key = EdSigningKey::from_bytes(&[31; 32]);
    let unprotected = || vec![(4.into(), Value::Bytes(b"kid".to_vec()))];
    let accepted = [
        vec![0xa1, 0x18, 1, 0x38, 7], // Legal non-shortest protected encoding.
        wire::encode(&Value::Map(vec![
            (1.into(), (-8).into()),
            (99.into(), true.into()),
        ])),
        wire::encode(&Value::Map(vec![
            (1.into(), (-8).into()),
            (2.into(), Value::Array(vec![1.into()])),
        ])),
    ];
    for protected in accepted {
        let envelope = raw_ed_signature(&protected, unprotected());
        let parsed = cose::parse_sign1(&envelope).unwrap();
        assert_eq!(parsed.protected(), protected);
        assert_eq!(
            parsed.verify(b"frame", VerifyingKeyRef::Ed25519(&key.verifying_key())),
            SigStatus::Valid
        );
        let mut canonical = parts(&envelope);
        canonical[0] = Value::Bytes(vec![0xa1, 1, 0x27]);
        if protected != [0xa1, 1, 0x27] {
            assert_eq!(
                cose::verify_sig(&wrap(canonical), b"frame", &key.verifying_key()),
                SigStatus::Invalid
            );
        }
    }
    let protected_kid = wire::encode(&Value::Map(vec![
        (1.into(), (-8).into()),
        (4.into(), Value::Bytes(vec![])),
        (2.into(), Value::Array(vec![1.into(), 4.into()])),
    ]));
    let envelope = raw_ed_signature(&protected_kid, vec![]);
    assert_eq!(
        cose::parse_sign1(&envelope).unwrap().kid(),
        Some(b"".as_slice())
    );
    assert_eq!(
        cose::verify_sig(&envelope, b"frame", &key.verifying_key()),
        SigStatus::Valid
    );
    let bad_maps = [
        vec![],
        vec![(1.into(), (-49).into())],
        vec![(1.into(), (-99).into())],
        vec![(1.into(), (-58).into())], // Correctly signed Ed bytes, wrong declared algorithm/size.
        vec![(1.into(), Value::Text("EdDSA".into()))],
        vec![(1.into(), Value::Bytes(vec![0x27]))],
        vec![(1.into(), (-8).into()), (1.into(), (-8).into())],
        vec![
            (1.into(), (-8).into()),
            (99.into(), true.into()),
            (99.into(), true.into()),
        ],
        vec![
            (1.into(), (-8).into()),
            ("ext".into(), true.into()),
            ("ext".into(), false.into()),
        ],
        vec![
            (1.into(), (-8).into()),
            (Value::Bytes(vec![9]), true.into()),
        ],
        vec![(1.into(), (-8).into()), (2.into(), Value::Array(vec![]))],
        vec![
            (1.into(), (-8).into()),
            (2.into(), Value::Array(vec![1.into(), 1.into()])),
        ],
        vec![
            (1.into(), (-8).into()),
            (2.into(), Value::Array(vec![4.into()])),
        ],
        vec![
            (1.into(), (-8).into()),
            (2.into(), Value::Array(vec![99.into()])),
            (99.into(), true.into()),
        ],
        vec![
            (1.into(), (-8).into()),
            (2.into(), Value::Array(vec!["ext".into()])),
            ("ext".into(), true.into()),
        ],
        vec![(1.into(), (-8).into()), (2.into(), true.into())],
        vec![
            (1.into(), (-8).into()),
            (2.into(), Value::Array(vec![Value::Bytes(vec![])])),
        ],
        vec![(1.into(), (-8).into()), (3.into(), Value::Array(vec![]))],
        vec![(1.into(), (-8).into()), (5.into(), true.into())],
        vec![(1.into(), (-8).into()), (7.into(), Value::Array(vec![]))],
    ];
    for headers in bad_maps {
        let envelope = raw_ed_signature(&wire::encode(&Value::Map(headers)), unprotected());
        assert!(cose::parse_sign1(&envelope).is_err());
        assert!(cose::parse(&envelope).is_none());
        assert_eq!(
            cose::verify_sig(&envelope, b"frame", &key.verifying_key()),
            SigStatus::Invalid
        );
    }
    let protected = [0xa1, 1, 0x27];
    let bad_unprotected = [
        vec![
            (4.into(), Value::Bytes(b"kid".to_vec())),
            (4.into(), Value::Bytes(b"kid".to_vec())),
        ],
        vec![(4.into(), Value::Text("kid".into()))],
        vec![(4.into(), Value::Bytes(vec![])), (1.into(), (-8).into())],
        vec![
            (4.into(), Value::Bytes(vec![])),
            (2.into(), Value::Array(vec![1.into()])),
        ],
        vec![
            (4.into(), Value::Bytes(vec![])),
            (5.into(), Value::Bytes(vec![])),
            (6.into(), Value::Bytes(vec![])),
        ],
    ];
    for headers in bad_unprotected {
        assert!(cose::parse_sign1(&raw_ed_signature(&protected, headers)).is_err());
    }
    // Reject the same extension label across buckets too, independent of alg.
    let protected = wire::encode(&Value::Map(vec![
        (1.into(), (-8).into()),
        ("ext".into(), true.into()),
    ]));
    let mut headers = unprotected();
    headers.push(("ext".into(), true.into()));
    assert!(cose::parse_sign1(&raw_ed_signature(&protected, headers)).is_err());
}

fn complete_detached_envelope_boundary_and_no_key_status() {
    let key = EdSigningKey::from_bytes(&[31; 32]);
    let original = cose::sign_id(b"frame", &key, "kid");
    let fields = parts(&original);
    let mut bad = Vec::new();
    for tag in [0, 16, 17, 19, 98, u64::MAX] {
        bad.push(wire::encode(&Value::Tag(
            tag,
            Box::new(Value::Array(fields.clone())),
        )));
    }
    bad.push(wire::encode(&Value::Tag(
        18,
        Box::new(Value::Tag(18, Box::new(Value::Array(fields.clone())))),
    )));
    for tail in [vec![0], vec![255], vec![0xa0], vec![0x5b, 255]] {
        bad.push([original.clone(), tail].concat());
    }
    for length in 0..original.len() {
        bad.push(original[..length].to_vec());
    }
    for length in [0, 1, 2, 3] {
        bad.push(wrap(fields[..length].to_vec()));
    }
    let mut too_many = fields.clone();
    too_many.push(Value::Null);
    bad.push(wrap(too_many));
    for (field, wrong) in [
        (0, Value::Map(vec![])),
        (0, Value::Text("header".into())),
        (1, Value::Bytes(vec![])),
        (2, Value::Bytes(vec![])),
        (2, Value::Bytes(b"frame".to_vec())),
        (2, false.into()),
        (3, Value::Text("signature".into())),
        (3, Value::Bytes(vec![0; 63])),
        (3, Value::Bytes(vec![0; 65])),
    ] {
        let mut changed = fields.clone();
        changed[field] = wrong;
        bad.push(wrap(changed));
    }
    for protected in [
        vec![],
        vec![0xa0],
        vec![0x81, 1],
        vec![0xa1, 1, 0x27, 0],
        vec![0xa1, 1],
        vec![0xd2, 0xa1, 1, 0x27],
    ] {
        let mut changed = fields.clone();
        changed[0] = Value::Bytes(protected);
        bad.push(wrap(changed));
    }
    // A real two-signer COSE_Sign shape is not this single composite algorithm.
    let signers = Value::Array(vec![
        Value::Array(vec![
            Value::Bytes(vec![]),
            Value::Map(vec![]),
            fields[3].clone(),
        ]),
        Value::Array(vec![
            Value::Bytes(vec![]),
            Value::Map(vec![]),
            fields[3].clone(),
        ]),
    ]);
    bad.push(wire::encode(&Value::Tag(
        98,
        Box::new(Value::Array(vec![
            fields[0].clone(),
            fields[1].clone(),
            Value::Null,
            signers,
        ])),
    )));
    let row = |cose| model::Signature {
        frame_id: b"frame".to_vec(),
        kid: None,
        status: String::new(),
        cose: Some(cose),
        packaging: false,
    };
    for malformed in bad {
        assert!(cose::parse_sign1(&malformed).is_err());
        assert_eq!(
            cose::verify_sig(&malformed, b"frame", &key.verifying_key()),
            SigStatus::Invalid
        );
        let mut rows = [row(malformed)];
        cose::verify_signatures(&mut rows, |_| {
            panic!("malformed envelope must fail before lookup")
        });
        assert_eq!(rows[0].status, "invalid");
        assert!(rows[0].kid.is_none());
    }
    let composite = composite_key(41);
    let supported =
        cose::sign_id_deterministic(b"frame", SigningKeyRef::Composite(&composite), "other")
            .unwrap();
    let mut rows = [row(original), row(supported)];
    cose::verify_signatures(&mut rows, |_| None);
    assert_eq!(rows[0].status, "unverified");
    assert_eq!(rows[1].status, "unverified");
    assert_eq!(rows[1].kid.as_deref(), Some("other"));
    cose::verify_signatures(&mut rows, |_| Some(key.verifying_key()));
    assert_eq!(rows[0].status, "valid");
    assert_eq!(rows[1].status, "invalid");
}

fn portable_composite_without_host_entropy_or_clock() {
    purrdf_testkit::harness::without_host_clock_or_entropy(|| {
        let key = composite_key(51);
        let encoded = cose::sign_id_hedged(
            b"portable frame",
            SigningKeyRef::Composite(&key),
            "portable",
            &[4; 32],
        )
        .unwrap();
        assert_eq!(
            cose::verify_sig_with_key(
                &encoded,
                b"portable frame",
                VerifyingKeyRef::Composite(key.verifying_key())
            ),
            SigStatus::Valid
        );
    });
}

fn opaque_binary_kids_authenticate_without_text_lookup_fallback() {
    let kid = [255, 0, 128];
    let key = EdSigningKey::from_bytes(&[61; 32]);
    let encoded = cose::sign_id_deterministic(b"frame", SigningKeyRef::Ed25519(&key), kid).unwrap();
    let parsed = cose::parse_sign1(&encoded).unwrap();
    assert_eq!(parsed.kid(), Some(kid.as_slice()));
    assert!(parsed.kid_text().is_none());
    assert_eq!(
        cose::verify_sig(&encoded, b"frame", &key.verifying_key()),
        SigStatus::Valid
    );
    assert!(cose::parse(&encoded).is_none());
    assert!(cose::signature_kid(&encoded).is_none());
    let mut rows = [model::Signature {
        frame_id: b"frame".to_vec(),
        kid: None,
        status: String::new(),
        cose: Some(encoded),
        packaging: false,
    }];
    cose::verify_signatures(&mut rows, |_| {
        panic!("binary identifiers cannot enter text resolver")
    });
    assert_eq!(rows[0].status, "unverified");
    assert!(rows[0].kid.is_none());
    let composite = composite_key(71);
    let encoded = cose::sign_id_hedged(
        b"frame",
        SigningKeyRef::Composite(&composite),
        kid,
        &[1; 32],
    )
    .unwrap();
    assert_eq!(
        cose::parse_sign1(&encoded).unwrap().kid(),
        Some(kid.as_slice())
    );
    assert_eq!(
        cose::verify_sig_with_key(
            &encoded,
            b"frame",
            VerifyingKeyRef::Composite(composite.verifying_key())
        ),
        SigStatus::Valid
    );
}

fn missing_kid_is_distinct_from_empty_and_needs_no_discovery() {
    let ed = EdSigningKey::from_bytes(&[81; 32]);
    let ed_signed = cose::sign_id(b"frame", &ed, "");
    let composite = composite_key(91);
    let composite_signed =
        cose::sign_id_deterministic(b"frame", SigningKeyRef::Composite(&composite), "").unwrap();
    for (encoded, public) in [
        (ed_signed, VerifyingKeyRef::Ed25519(&ed.verifying_key())),
        (
            composite_signed,
            VerifyingKeyRef::Composite(composite.verifying_key()),
        ),
    ] {
        let mut fields = parts(&encoded);
        fields[1] = Value::Map(vec![]);
        let no_kid = wrap(fields);
        let parsed = cose::parse_sign1(&no_kid).unwrap();
        assert_eq!(parsed.kid(), None);
        assert_eq!(parsed.kid_text(), None);
        assert_eq!(parsed.verify(b"frame", public), SigStatus::Valid);
        assert!(cose::parse(&no_kid).is_none());
        assert!(cose::signature_kid(&no_kid).is_none());
        let mut rows = [model::Signature {
            frame_id: b"frame".to_vec(),
            kid: None,
            status: String::new(),
            cose: Some(no_kid),
            packaging: false,
        }];
        cose::verify_signatures(&mut rows, |_| {
            panic!("absent kid must not invoke empty-ID lookup")
        });
        assert_eq!(rows[0].status, "unverified");
        assert_eq!(
            cose::parse_sign1(&encoded).unwrap().kid(),
            Some(b"".as_slice())
        );
    }
    let mut empty = [model::Signature {
        frame_id: b"frame".to_vec(),
        kid: None,
        status: String::new(),
        cose: Some(cose::sign_id(b"frame", &ed, "")),
        packaging: false,
    }];
    cose::verify_signatures(&mut empty, |kid| {
        assert_eq!(kid, "");
        Some(ed.verifying_key())
    });
    assert_eq!(empty[0].status, "valid");
    assert_eq!(empty[0].kid.as_deref(), Some(""));
}

fn content_type_headers_obey_restricted_names_before_lookup() {
    let key = EdSigningKey::from_bytes(&[31; 32]);
    let valid = [
        "text/plain".to_string(),
        "TeXT/PlAiN".to_string(),
        "0/1".to_string(),
        "application/vnd.example+cbor".to_string(),
        "a!#$&-^_.+/B!#$&-^_.+".to_string(),
        format!("{}/b", "a".repeat(127)),
        format!("a/{}", "b".repeat(127)),
        format!("{}/{}", "a".repeat(127), "B".repeat(127)),
    ];
    let mut invalid = vec![
        String::new(),
        "invalid".to_string(),
        "/plain".to_string(),
        "text/".to_string(),
        " text/plain".to_string(),
        "text/plain ".to_string(),
        "text /plain".to_string(),
        "text/ plain".to_string(),
        "text/pl ain".to_string(),
        "text/plain/extra".to_string(),
        "text//plain".to_string(),
        "text/🙂".to_string(),
        "téxt/plain".to_string(),
        "text/\u{a0}plain".to_string(),
        "text/plain;charset=utf-8".to_string(),
        "text/plain; charset=utf-8".to_string(),
        format!("{}/plain", "a".repeat(128)),
        format!("text/{}", "b".repeat(128)),
    ];
    // Representative bytes outside the restricted-name character class, including
    // separators, controls and DEL, is refused in either position's continuation.
    for byte in [
        b'\t', b'\r', b'\n', 0, 127, b'(', b')', b'*', b'%', b'\'', b'`', b'|', b'~', b'"', b'\\',
        b':', b'=', b'?', b'@', b'[', b']',
    ] {
        invalid.push(format!("a{}/plain", char::from(byte)));
        invalid.push(format!("text/a{}", char::from(byte)));
    }
    for first in "!#$&-^_.+".chars() {
        invalid.push(format!("{first}a/plain"));
        invalid.push(format!("text/{first}a"));
    }
    for protected_position in [true, false] {
        let encode = |content_type: Value| {
            let mut protected = vec![(1.into(), (-8).into())];
            let mut unprotected = vec![(4.into(), Value::Bytes(b"kid".to_vec()))];
            if protected_position {
                protected.push((3.into(), content_type));
            } else {
                unprotected.push((3.into(), content_type));
            }
            let protected = wire::encode(&Value::Map(protected));
            let envelope = raw_ed_signature(&protected, unprotected);
            (protected, envelope)
        };
        let row = |encoded| model::Signature {
            frame_id: b"frame".to_vec(),
            kid: None,
            status: String::new(),
            cose: Some(encoded),
            packaging: false,
        };
        for value in valid
            .iter()
            .cloned()
            .map(Value::Text)
            .chain([Value::Integer(0.into()), Value::Integer(u64::MAX.into())])
        {
            let (protected, envelope) = encode(value);
            let parsed = cose::parse_sign1(&envelope).unwrap();
            assert_eq!(parsed.protected(), protected);
            assert_eq!(
                parsed.verify(b"frame", VerifyingKeyRef::Ed25519(&key.verifying_key())),
                SigStatus::Valid
            );
            let mut rows = [row(envelope)];
            cose::verify_signatures(&mut rows, |kid| {
                assert_eq!(kid, "kid");
                Some(key.verifying_key())
            });
            assert_eq!(rows[0].status, "valid");
        }
        for value in invalid.iter().cloned().map(Value::Text).chain([
            Value::Integer((-1).into()),
            Value::Bytes(vec![]),
            Value::Null,
        ]) {
            let (_, envelope) = encode(value);
            assert_eq!(
                cose::parse_sign1(&envelope),
                Err(cose::Sign1Error::Malformed)
            );
            assert_eq!(
                cose::verify_sig(&envelope, b"frame", &key.verifying_key()),
                SigStatus::Invalid
            );
            let mut rows = [row(envelope)];
            cose::verify_signatures(&mut rows, |_| {
                panic!("malformed content type must fail before lookup")
            });
            assert_eq!(rows[0].status, "invalid");
        }
    }
}

purrdf_testkit::harness_main!(
    ietf_published_cose_example_complete_bytes,
    frozen_eddsa_sign1_bytes_and_conveniences,
    public_composite_sign1_deterministic_hedged_and_key_dispatch,
    each_component_is_mandatory_and_cannot_be_spliced,
    canonical_composite_key_encodings_and_strict_ed25519,
    headers_are_authenticated_unique_typed_and_critical_rules_enforced,
    complete_detached_envelope_boundary_and_no_key_status,
    portable_composite_without_host_entropy_or_clock,
    opaque_binary_kids_authenticate_without_text_lookup_fallback,
    missing_kid_is_distinct_from_empty_and_needs_no_discovery,
    content_type_headers_obey_restricted_names_before_lookup,
);
