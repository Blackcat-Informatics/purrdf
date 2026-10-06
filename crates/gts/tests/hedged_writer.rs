// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable public Writer -> reader -> algorithm-aware verification contracts.

use std::cell::Cell;

use purrdf_ed25519::SigningKey as EdKey;
use purrdf_gts::codec::Codec;
use purrdf_gts::cose::{self, Algorithm, SigStatus, VerifyingKeyRef};
use purrdf_gts::model::Term;
use purrdf_gts::policy::{Severity, evaluate_profile_policy};
use purrdf_gts::verify::{
    Keyring, SignatureKeyring, VerifyOptions, verify_file_with_keyring, verify_file_with_options,
};
use purrdf_gts::writer::{
    Encrypt0Options, FrameOptions, Hedged, RandomnessError, RandomnessProvider, SnapshotSigner,
    Writer, WriterError, WriterOptions,
};
use purrdf_gts::{reader, wire};
use purrdf_lex::cbor::{self, Value};

mod composite_support;
use composite_support::composite_key;

// Fixed draws are fixture inputs only. Production callers supply a cryptographic
// provider; the core cannot measure its freshness or quality.
#[derive(Default)]
struct FixtureProvider {
    unavailable: bool,
    partial_failure: bool,
    draws: u8,
}

impl RandomnessProvider for FixtureProvider {
    fn fill_randomizer(&mut self, out: &mut [u8; 32]) -> Result<(), RandomnessError> {
        if self.unavailable {
            return Err(RandomnessError::new("fixture entropy input unavailable"));
        }
        if self.partial_failure {
            out[..16].fill(0xa7);
            return Err(RandomnessError::new(
                "fixture provider failed after partial fill",
            ));
        }
        self.draws += 1;
        out.fill(self.draws);
        Ok(())
    }
}

fn frozen_ed25519_writer_remains_byte_exact() {
    let vector =
        purrdf_lex::json::read(include_str!("../../../vectors/signed/basic.json")).unwrap();
    let expected = purrdf_hash::hex::decode(vector["gts"].as_str().unwrap()).unwrap();
    let seed = purrdf_hash::hex::decode(vector["seed"].as_str().unwrap()).unwrap();
    let key = EdKey::from_bytes(seed.as_slice().try_into().unwrap());
    let mut writer = Writer::new("dist");
    writer.sign_with(key, vector["kid"].as_str().unwrap());
    let (items, torn) = wire::iter_items(&expected);
    assert_eq!(torn, None);
    for (_, frame) in &items[1..] {
        let frame = frame.as_map().unwrap();
        let get = |name| {
            frame
                .iter()
                .find(|(k, _)| k.as_text() == Some(name))
                .unwrap()
                .1
                .clone()
        };
        writer.add_frame(
            get("t").as_text().unwrap(),
            Some(get("d")),
            None,
            None,
            None,
        );
    }
    assert_eq!(writer.into_bytes(), expected);
}

fn alter_frame(data: &[u8], index: usize, alter: impl FnOnce(&mut Vec<(Value, Value)>)) -> Vec<u8> {
    let (mut items, torn) = wire::iter_items(data);
    assert_eq!(torn, None);
    let Value::Map(frame) = &mut items[index].1 else {
        panic!("expected a frame map")
    };
    alter(frame);
    items
        .into_iter()
        .flat_map(|(_, item)| wire::canonical(&item))
        .collect()
}

fn field<'a>(map: &'a mut [(Value, Value)], name: &str) -> &'a mut Value {
    &mut map
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some(name))
        .unwrap()
        .1
}

fn alter_envelope(frame: &mut [(Value, Value)], alter: impl FnOnce(&mut Vec<Value>)) {
    let signature = field(frame, "sig");
    let mut envelope = cbor::decode(signature.as_bytes().unwrap(), cbor::Limits::DEFAULT).unwrap();
    let Value::Tag(18, body) = &mut envelope else {
        panic!("expected COSE_Sign1")
    };
    let Value::Array(parts) = body.as_mut() else {
        panic!("expected Sign1 array")
    };
    alter(parts);
    *signature = Value::Bytes(wire::canonical(&envelope));
}

fn writer_reader_mixed_algorithm_history_and_rotation() {
    let old = EdKey::from_bytes(&[7; 32]);
    let next = EdKey::from_bytes(&[8; 32]);
    let composite = composite_key(11);
    let rotated = composite_key(13);
    let mut ring = Keyring::default();
    ring.insert(b"old", old.verifying_key());
    ring.insert(b"next", next.verifying_key());
    ring.insert(b"composite", Box::new(composite.verifying_key().clone()));
    ring.insert([0xff, 0, 0x80], Box::new(rotated.verifying_key().clone()));
    let mut initial = Writer::new("purrdf.gts");
    initial.sign_with(old, "old");
    initial.add_blob(b"old authorship", None, None);
    let before = initial.to_bytes();
    let mut writer =
        initial.with_composite_signer(composite, b"composite", FixtureProvider::default());
    assert_eq!(writer.to_bytes(), before);
    writer
        .add_blob(b"composite authorship", None, None)
        .unwrap();
    writer.sign_with(next, "next");
    writer.add_meta(Value::Map(vec![])).unwrap();
    writer.sign_composite_with(rotated, &[0xff, 0, 0x80]);
    writer.add_index_with_mmr().unwrap();
    assert_eq!(
        writer.randomness_provider_mut().draws,
        2,
        "Ed25519 does not request entropy"
    );
    let bytes = writer.into_bytes();
    let graph = reader::read(&bytes, true, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(graph.signatures.len(), 4);
    for (row, expected) in graph.signatures.iter().zip([
        Algorithm::Ed25519,
        Algorithm::CompositeMlDsa65Ed25519,
        Algorithm::Ed25519,
        Algorithm::CompositeMlDsa65Ed25519,
    ]) {
        let parsed = cose::parse_sign1(row.cose.as_ref().unwrap()).unwrap();
        assert_eq!(parsed.algorithm(), expected);
        assert_eq!(
            parsed.verify(&row.frame_id, ring.resolve(parsed.kid(), expected).unwrap()),
            SigStatus::Valid
        );
        if expected == Algorithm::CompositeMlDsa65Ed25519 {
            assert_eq!(parsed.signature_bytes().len(), 3373);
            assert_eq!(
                parsed.protected(),
                wire::canonical(&Value::Map(vec![(1.into(), (-58).into())]))
            );
        }
    }
    let result = verify_file_with_keyring(&bytes, &ring);
    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(
        (
            result.signed,
            result.valid,
            result.invalid,
            result.unverified
        ),
        (4, 4, 0, 0)
    );
}

fn hedged_output_variation_and_portable_provider() {
    purrdf_testkit::harness::without_host_clock_or_entropy(|| {
        let key = composite_key(21);
        let mut ring = Keyring::default();
        ring.insert(b"key", Box::new(key.verifying_key().clone()));
        let provider = |out: &mut [u8; 32]| {
            out.fill(1);
            Ok(())
        };
        let mut first =
            Writer::new("purrdf.gts").with_composite_signer(key.clone(), b"key", provider);
        let mut second =
            Writer::new("purrdf.gts").with_composite_signer(key, b"key", |out: &mut [u8; 32]| {
                out.fill(2);
                Ok(())
            });
        let id1 = first.add_blob(b"same payload", None, None).unwrap();
        let id2 = second.add_blob(b"same payload", None, None).unwrap();
        assert_eq!(
            id1, id2,
            "signature randomness does not change content identity"
        );
        assert_ne!(first.to_bytes(), second.to_bytes());
        for bytes in [first.into_bytes(), second.into_bytes()] {
            let result = verify_file_with_keyring(&bytes, &ring);
            assert!(result.ok, "{:?}", result.errors);
            assert_eq!(result.valid, 1);
        }
    });
}

fn refuse_atomically(
    writer: &mut Writer<Hedged<FixtureProvider>>,
    append: impl FnOnce(&mut Writer<Hedged<FixtureProvider>>) -> Result<Vec<u8>, WriterError>,
) {
    let before = writer.to_bytes();
    let head = writer.head().to_vec();
    let error = append(writer).unwrap_err();
    assert!(matches!(error, WriterError::Randomness(_)), "{error}");
    assert!(error.to_string().contains("fixture"));
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(writer.to_bytes(), before);
    assert_eq!(writer.head(), head);
}

fn failed_missing_and_partial_entropy_preserve_every_append_contract() {
    let key = composite_key(31);
    let mut ring = Keyring::default();
    ring.insert(b"key", Box::new(key.verifying_key().clone()));
    let mut writer =
        Writer::new("purrdf.gts").with_composite_signer(key, b"key", FixtureProvider::default());
    writer.add_blob(b"intact prefix", None, None).unwrap();
    writer.add_index_with_mmr().unwrap();
    writer.randomness_provider_mut().unavailable = true;
    refuse_atomically(&mut writer, |w| {
        w.add_frame("meta", Some(Value::Map(vec![])), None, None, None)
    });
    refuse_atomically(&mut writer, |w| {
        w.add_frame_with_options(
            "meta",
            FrameOptions {
                payload: Some(Value::Map(vec![])),
                ..FrameOptions::default()
            },
        )
    });
    refuse_atomically(&mut writer, |w| w.add_terms(&[]));
    refuse_atomically(&mut writer, |w| w.add_quads(&[]));
    refuse_atomically(&mut writer, |w| w.add_reifies(&[]));
    refuse_atomically(&mut writer, |w| w.add_annot(&[]));
    refuse_atomically(&mut writer, |w| w.add_blob(b"new", None, None));
    refuse_atomically(&mut writer, |w| {
        w.add_blob_owned(b"new".to_vec(), None, None)
    });
    refuse_atomically(&mut writer, |w| {
        w.add_blob_transformed(b"new".to_vec(), None, None, &["zstd".into()], None)
    });
    refuse_atomically(&mut writer, |w| w.add_meta(Value::Map(vec![])));
    refuse_atomically(&mut writer, |w| w.add_suppress(vec![], None, None));
    refuse_atomically(&mut writer, Writer::add_index);
    refuse_atomically(&mut writer, Writer::add_index_with_mmr);
    writer.randomness_provider_mut().unavailable = false;
    writer.randomness_provider_mut().partial_failure = true;
    refuse_atomically(&mut writer, |w| w.add_blob(b"partial failure", None, None));
    writer.randomness_provider_mut().partial_failure = false;
    writer.add_blob(b"recovered", None, None).unwrap();
    writer.add_index_with_mmr().unwrap();
    assert_eq!(writer.randomness_provider_mut().draws, 4);
    let bytes = writer.into_bytes();
    let graph = reader::read(&bytes, true, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    let (items, _) = wire::iter_items(&bytes);
    let final_frame = items.last().unwrap().1.as_map().unwrap();
    let payload = final_frame
        .iter()
        .find(|(k, _)| k.as_text() == Some("d"))
        .unwrap()
        .1
        .as_map()
        .unwrap();
    assert_eq!(
        payload
            .iter()
            .find(|(k, _)| k.as_text() == Some("count"))
            .unwrap()
            .1,
        Value::from(3)
    );
    let result = verify_file_with_keyring(&bytes, &ring);
    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(result.valid, 4);
}

fn both_writer_produced_components_and_frame_content_are_mandatory() {
    let key = composite_key(41);
    let mut ring = Keyring::default();
    ring.insert(b"key", Box::new(key.verifying_key().clone()));
    let mut writer =
        Writer::new("purrdf.gts").with_composite_signer(key, b"key", FixtureProvider::default());
    writer.add_blob(b"first", None, None).unwrap();
    writer.add_blob(b"second", None, None).unwrap();
    let bytes = writer.into_bytes();
    for position in [0, 3309] {
        let corrupted = alter_frame(&bytes, 2, |frame| {
            alter_envelope(frame, |parts| {
                let Value::Bytes(signature) = &mut parts[3] else {
                    panic!("signature bytes")
                };
                signature[position] ^= 1;
            });
        });
        let result = verify_file_with_keyring(&corrupted, &ring);
        assert!(!result.ok);
        assert_eq!((result.valid, result.invalid), (1, 1));
    }
    let corrupted = alter_frame(&bytes, 2, |frame| {
        *field(frame, "d") = Value::Bytes(b"tampered".to_vec());
    });
    let graph = reader::read(&corrupted, true, None);
    assert!(graph.diagnostics.iter().any(|d| d.code == "DamagedFrame"));
    let result = verify_file_with_keyring(&corrupted, &ring);
    assert!(
        !result.ok,
        "valid surviving signatures must not hide a damaged frame"
    );
    assert_eq!(result.valid, 1);
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.contains("DamagedFrame"))
    );
}

fn file_integrity_rejects_header_chain_and_torn_damage_but_allows_opaque_encryption() {
    let key = EdKey::from_bytes(&[44; 32]);
    let mut ring = Keyring::default();
    ring.insert(b"key", key.verifying_key());
    let mut writer = Writer::new("purrdf.gts");
    writer.sign_with(key.clone(), "key");
    writer.add_blob(b"first", None, None);
    writer.add_blob(b"second", None, None);
    let bytes = writer.into_bytes();
    assert!(verify_file_with_keyring(&bytes, &ring).ok);
    let (mut items, torn) = wire::iter_items(&bytes);
    assert_eq!(torn, None);
    let Value::Tag(_, header) = &mut items[0].1 else {
        panic!("header tag")
    };
    let Value::Map(header) = header.as_mut() else {
        panic!("header map")
    };
    *field(header, "v") = 99.into();
    let header_damage: Vec<u8> = items
        .into_iter()
        .flat_map(|(_, value)| wire::canonical(&value))
        .collect();
    let result = verify_file_with_keyring(&header_damage, &ring);
    assert_eq!(result.valid, 2);
    assert!(!result.ok);
    assert!(result.diagnostics.iter().any(|d| d.code == "DamagedFrame"));
    // The new frame id and signature are valid, but it does not extend the
    // observed head. Cryptographic validity must not hide a broken chain.
    let chain_damage = alter_frame(&bytes, 2, |frame| {
        *field(frame, "prev") = Value::Bytes(vec![0; 32]);
        let id = wire::content_id(frame);
        *field(frame, "id") = Value::Bytes(id.clone());
        *field(frame, "sig") = Value::Bytes(cose::sign_id(&id, &key, "key"));
    });
    let result = verify_file_with_keyring(&chain_damage, &ring);
    assert_eq!(result.valid, 2);
    assert!(!result.ok);
    assert!(result.diagnostics.iter().any(|d| d.code == "BrokenChain"));
    let mut trailing_damage = bytes;
    trailing_damage.push(0x5f); // incomplete CBOR byte string at an item boundary
    let result = verify_file_with_keyring(&trailing_damage, &ring);
    assert_eq!(result.valid, 2);
    assert!(!result.ok);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "TornAppendError")
    );
    let mut writer = Writer::new("purrdf.gts");
    writer.sign_with(key, "key");
    writer
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(b"encrypted but signed".to_vec()),
                encrypt: Some(Encrypt0Options {
                    kid: "decrypt".into(),
                    key: [1; 32],
                    iv: [2; 12],
                }),
                ..FrameOptions::default()
            },
        )
        .unwrap();
    let result = verify_file_with_keyring(&writer.into_bytes(), &ring);
    assert_eq!(result.valid, 1);
    assert!(result.diagnostics.iter().any(|d| d.code == "MissingKey"));
    assert!(
        result.ok,
        "opaque ciphertext is still fully authenticated: {:?}",
        result.errors
    );
}

struct NoLookup;
impl SignatureKeyring for NoLookup {
    fn resolve(&self, _: Option<&[u8]>, _: Algorithm) -> Option<VerifyingKeyRef<'_>> {
        panic!("malformed envelopes must not resolve a key")
    }
}

fn malformed_unsupported_unresolved_and_wrong_type_are_distinct() {
    let key = composite_key(51);
    let mut writer = Writer::new("purrdf.gts").with_composite_signer(
        key,
        b"unknown",
        FixtureProvider::default(),
    );
    writer.add_blob(b"signed", None, None).unwrap();
    let bytes = writer.into_bytes();
    let unresolved = verify_file_with_keyring(&bytes, &Keyring::default());
    assert_eq!((unresolved.invalid, unresolved.unverified), (0, 1));
    assert!(!unresolved.ok);
    let mut wrong = Keyring::default();
    wrong.insert(b"unknown", EdKey::from_bytes(&[1; 32]).verifying_key());
    let result = verify_file_with_keyring(&bytes, &wrong);
    assert_eq!((result.invalid, result.unverified), (1, 0));
    for invalid in [
        alter_frame(&bytes, 1, |frame| {
            *field(frame, "sig") = Value::Bytes(vec![0]);
        }),
        alter_frame(&bytes, 1, |frame| {
            alter_envelope(frame, |parts| {
                parts[0] =
                    Value::Bytes(wire::canonical(&Value::Map(vec![(1.into(), (-49).into())])));
            });
        }),
    ] {
        let result = verify_file_with_keyring(&invalid, &NoLookup);
        assert_eq!((result.invalid, result.unverified), (1, 0));
        assert!(!result.ok);
    }
}

fn opaque_absent_and_empty_ids_remain_distinct_at_file_resolution() {
    let key = EdKey::from_bytes(&[71; 32]);
    let mut empty = Keyring::default();
    empty.insert(b"", key.verifying_key());
    let mut writer = Writer::new("purrdf.gts");
    writer.sign_ed25519_with(key, b"");
    writer.add_blob(b"signed", None, None);
    let bytes = writer.into_bytes();
    assert!(verify_file_with_keyring(&bytes, &empty).ok);
    let absent = alter_frame(&bytes, 1, |frame| {
        alter_envelope(frame, |parts| parts[1] = Value::Map(vec![]));
    });
    assert_eq!(verify_file_with_keyring(&absent, &empty).unverified, 1);
    let mut no_id = Keyring::default();
    no_id.insert_without_id(EdKey::from_bytes(&[71; 32]).verifying_key());
    assert!(verify_file_with_keyring(&absent, &no_id).ok);
    assert_eq!(verify_file_with_keyring(&bytes, &no_id).unverified, 1);
    let key = EdKey::from_bytes(&[72; 32]);
    let public = key.verifying_key();
    let mut writer = Writer::new("purrdf.gts");
    writer.sign_ed25519_with(key, &[0xff, 0, 0x80]);
    writer.add_blob(b"binary", None, None);
    let mut graph = reader::read(&writer.into_bytes(), true, None);
    let called = Cell::new(0);
    cose::verify_signatures_with_resolver(&mut graph.signatures, |id, alg| {
        called.set(called.get() + 1);
        assert_eq!(id, Some([0xff, 0, 0x80].as_slice()));
        assert_eq!(alg, Algorithm::Ed25519);
        Some(VerifyingKeyRef::Ed25519(&public))
    });
    assert_eq!(called.get(), 1);
    assert_eq!(graph.signatures[0].status, "valid");
    assert_eq!(graph.signatures[0].kid, None, "no fabricated text encoding");
}

fn snapshot_signer_debug_does_not_expose_secret_seed() {
    let signer = SnapshotSigner {
        secret: [73; 32],
        kid: "visible-id".into(),
        public_key_armor: "public armor".into(),
    };
    let rendered = format!("{signer:?}");
    assert!(rendered.contains("visible-id"));
    assert!(rendered.contains("public armor"));
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("73"));
    let cloned = signer.clone();
    assert_eq!(cloned.secret, [73; 32]);
    assert_eq!(signer.secret, cloned.secret);
}

fn existing_openpgp_resolution_uses_the_same_integrity_core() {
    let public = include_str!("../../../vectors/openpgp/test_key.pub.asc");
    let private = include_str!("../../../vectors/openpgp/test_key.sec.asc");
    let fingerprint = include_str!("../../../vectors/openpgp/test_key.fingerprint").trim();
    let mut writer = Writer::new("purrdf.gts");
    writer.sign_with_openpgp_secret_key(private, None).unwrap();
    writer.add_meta(Value::Map(vec![(
        "gts:transportKey".into(),
        Value::Map(vec![
            ("kid".into(), fingerprint.into()),
            ("gpg".into(), public.into()),
        ]),
    )]));
    writer.add_blob(b"payload", None, None);
    let bytes = writer.into_bytes();
    for options in [
        VerifyOptions::default(),
        VerifyOptions::default().with_armored_key(public),
    ] {
        let valid = verify_file_with_options(&bytes, &options);
        assert!(valid.ok, "{:?}", valid.errors);
        assert_eq!(valid.valid, 2);
        let damaged = alter_frame(&bytes, 2, |frame| {
            *field(frame, "d") = Value::Bytes(b"altered".to_vec());
        });
        let invalid = verify_file_with_options(&damaged, &options);
        assert!(!invalid.ok);
        assert_eq!(invalid.valid, 1);
        assert!(invalid.errors.iter().any(|e| e.contains("DamagedFrame")));
    }
    let mut unsigned = Writer::new("purrdf.gts");
    unsigned.add_blob(b"unsigned", None, None);
    let unsigned = unsigned.into_bytes();
    let options = VerifyOptions::default().require_signatures(false);
    assert!(verify_file_with_options(&unsigned, &options).ok);
    let damaged = alter_frame(&unsigned, 1, |frame| {
        *field(frame, "d") = Value::Bytes(b"altered".to_vec());
    });
    assert!(!verify_file_with_options(&damaged, &options).ok);
}

fn unsigned_opt_in_refuses_invalid_framing_and_preserves_profile_findings() {
    let options = VerifyOptions::default().require_signatures(false);
    for bytes in [vec![], vec![0x5f], vec![0xff], vec![0]] {
        let result = verify_file_with_options(&bytes, &options);
        assert!(!result.ok, "invalid unsigned framing: {bytes:?}");
        assert!(!result.errors.is_empty(), "{result:?}");
        assert!(result.diagnostics.iter().any(|diagnostic| {
            matches!(diagnostic.code.as_str(), "EmptyFile" | "DamagedFrame")
                && !diagnostic.detail.is_empty()
        }));
    }
    assert!(verify_file_with_options(&Writer::new("purrdf.gts").into_bytes(), &options).ok);
    for profile in ["purrdf.gts", "evidence", "opaque"] {
        let mut writer = Writer::new(profile);
        writer.add_blob(b"unsigned", None, None);
        let bytes = writer.into_bytes();
        let graph = reader::read(&bytes, true, None);
        let expected = evaluate_profile_policy(&graph, Some(&options.trust_policy), None);
        let result = verify_file_with_options(&bytes, &options);
        assert_eq!(result.profile_findings, expected);
        assert_eq!(result.ok, profile == "purrdf.gts");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            (
                result.signed,
                result.valid,
                result.invalid,
                result.unverified
            ),
            (0, 0, 0, 0)
        );
        if profile != "purrdf.gts" {
            assert!(result.profile_findings.iter().any(|finding| {
                finding.code == "ProfileSignatureRequired"
                    && finding.severity == Severity::Error
                    && !finding.detail.is_empty()
            }));
        }
        if profile == "evidence" {
            assert!(result.profile_findings.iter().any(|finding| {
                finding.code == "EvidenceHeadCommitmentRequired"
                    && finding.severity == Severity::Error
                    && !finding.detail.is_empty()
            }));
        }
    }
}

fn unsigned_evidence_retains_the_existing_sealed_source_exception() {
    let options = VerifyOptions::default().require_signatures(false);
    for predicate in [
        purrdf_gts::stream::SEALED_SOURCE,
        "https://example.org/unsealed",
    ] {
        let mut writer = Writer::new("evidence");
        writer.add_terms(&[
            Term::iri("https://example.org/bundle"),
            Term::iri(predicate),
            Term::iri("https://example.org/source"),
        ]);
        writer.add_quads(&[(0, 1, 2, None)]);
        let bytes = writer.into_bytes();
        let graph = reader::read(&bytes, true, None);
        let expected = evaluate_profile_policy(&graph, Some(&options.trust_policy), None);
        let result = verify_file_with_options(&bytes, &options);
        assert_eq!(result.profile_findings, expected);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.signed, 0);
        if predicate == purrdf_gts::stream::SEALED_SOURCE {
            assert!(
                result.ok,
                "existing sealed-source evidence policy: {:?}",
                result.profile_findings
            );
            assert!(
                result
                    .profile_findings
                    .iter()
                    .all(|finding| finding.severity != Severity::Error)
            );
        } else {
            assert!(!result.ok);
            assert!(
                result
                    .profile_findings
                    .iter()
                    .any(|finding| finding.code == "ProfileSignatureRequired")
            );
        }
    }
}

fn signed_unknown_codec_remains_authenticated_opaque_payload() {
    let key = EdKey::from_bytes(&[45; 32]);
    let mut ring = Keyring::default();
    ring.insert(b"key", key.verifying_key());
    let mut writer = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            catalog: Some(vec![
                (0, Codec::new("identity", "encode")),
                (127, Codec::new("future-codec", "compress")),
            ]),
            ..WriterOptions::default()
        },
    )
    .unwrap();
    writer.sign_with(key.clone(), "key");
    // Without a declared blob digest, folding must attempt the codec to compute
    // the content digest. A declared digest instead keeps the payload lazy.
    writer.add_frame(
        "blob",
        None,
        Some(b"future opaque encoding".to_vec()),
        None,
        None,
    );
    // Model an authenticated external encoder whose declared codec this reader
    // does not implement. The real file verifier authenticates the carried bytes.
    let bytes = alter_frame(&writer.into_bytes(), 1, |frame| {
        frame.push(("x".into(), Value::Array(vec![127.into()])));
        let id = wire::content_id(frame);
        *field(frame, "id") = Value::Bytes(id.clone());
        *field(frame, "sig") = Value::Bytes(cose::sign_id(&id, &key, "key"));
    });
    let result = verify_file_with_keyring(&bytes, &ring);
    assert!(result.ok, "{:?}", result.errors);
    assert_eq!((result.valid, result.invalid, result.unverified), (1, 0, 0));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "UnknownCodec")
    );
}

purrdf_testkit::harness_main!(
    frozen_ed25519_writer_remains_byte_exact,
    writer_reader_mixed_algorithm_history_and_rotation,
    hedged_output_variation_and_portable_provider,
    failed_missing_and_partial_entropy_preserve_every_append_contract,
    both_writer_produced_components_and_frame_content_are_mandatory,
    file_integrity_rejects_header_chain_and_torn_damage_but_allows_opaque_encryption,
    malformed_unsupported_unresolved_and_wrong_type_are_distinct,
    opaque_absent_and_empty_ids_remain_distinct_at_file_resolution,
    snapshot_signer_debug_does_not_expose_secret_seed,
    existing_openpgp_resolution_uses_the_same_integrity_core,
    unsigned_opt_in_refuses_invalid_framing_and_preserves_profile_findings,
    unsigned_evidence_retains_the_existing_sealed_source_exception,
    signed_unknown_codec_remains_authenticated_opaque_payload,
);
