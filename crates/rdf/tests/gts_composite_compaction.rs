// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual portable composite authorship, packaging and certification boundaries.

use purrdf_gts::compact::{self, CompactionParams, CompositePackaging, DictPlan};
use purrdf_gts::cose::{self, Algorithm};
use purrdf_gts::fixture::{fixed_composite_key, fixed_key, object_literal};
use purrdf_gts::model::Term;
use purrdf_gts::reader::read;
use purrdf_gts::verify::{Keyring, verify_file_with_keyring};
use purrdf_gts::wire;
use purrdf_gts::writer::{RandomnessError, Writer};
use purrdf_iri::vocab::rdf::TYPE;
use purrdf_lex::cbor::{self, Value};
use purrdf_rdf::gts_certify::{compact_and_certify, refold_digest, verify_compaction};

const TIME: &str = "2026-01-01T00:00:00Z";
const AUTHOR: &[u8] = &[0xff, 0, 0xa5];
const UTC_TIMESTAMPS: &[&str] = &[
    TIME,
    "2026-01-01T00:00:00+00:00",
    "2026-01-01T00:00:00-00:00",
    "12026-01-01T00:00:00Z",
    "-0001-01-01T00:00:00Z",
    "2026-01-01T24:00:00Z",
];
const INVALID_REWRITE_TIMESTAMPS: &[&str] = &[
    "not-a-time",
    "2026-02-30T00:00:00Z",
    "2026-01-01T00:00:00",
    "2026-01-01T00:00:00+01:00",
    "2026-01-01T00:00:00-01:00",
];

#[derive(Default)]
struct Observations(Vec<purrdf_gts::model::Signature>);

impl purrdf_gts::reader::StreamingSink for Observations {
    fn signature(&mut self, _: u64, signature: &purrdf_gts::model::Signature) {
        self.0.push(signature.clone());
    }
}

fn signed_graph(terms: &[Term], quads: &[purrdf_gts::model::Quad]) -> Vec<u8> {
    let mut writer = Writer::with_layout("purrdf.gts", Some("streamable"));
    writer.add_terms(terms);
    writer.add_quads(quads);
    writer.sign_with(fixed_key(73), "ed-pack");
    writer.add_index();
    writer.into_bytes()
}

fn draw(out: &mut [u8; 32]) -> Result<(), RandomnessError> {
    out.fill(0x85);
    Ok(())
}

fn source() -> Vec<u8> {
    let mut writer =
        Writer::new("purrdf.gts").with_composite_signer(fixed_composite_key(70), AUTHOR, draw);
    writer
        .add_terms(&[
            Term::iri("https://example.org/subject"),
            Term::iri("https://example.org/predicate"),
            Term::literal("portable composite claim", None),
        ])
        .unwrap();
    writer.sign_ed25519_with(fixed_key(72), b"ed-author");
    writer.add_quads(&[(0, 1, 2, None)]).unwrap();
    writer.into_bytes()
}

fn keys() -> Keyring {
    let mut ring = Keyring::default();
    ring.insert(
        AUTHOR,
        Box::new(fixed_composite_key(70).verifying_key().clone()),
    );
    ring.insert(b"ed-author", fixed_key(72).verifying_key());
    ring.insert(b"ed-pack", fixed_key(73).verifying_key());
    ring.insert(
        b"composite-pack",
        Box::new(fixed_composite_key(74).verifying_key().clone()),
    );
    ring
}

fn ed_pack(input: &[u8]) -> Vec<u8> {
    compact_and_certify(
        input,
        DictPlan::undicted(),
        TIME,
        false,
        (fixed_key(73), "ed-pack".into()),
    )
    .unwrap()
    .0
}

fn composite_pack(input: &[u8]) -> Vec<u8> {
    compact_and_certify(
        input,
        DictPlan::undicted(),
        TIME,
        false,
        CompositePackaging::new(fixed_composite_key(74), "composite-pack".into(), draw),
    )
    .unwrap()
    .0
}

fn mutate_signature(input: &[u8], index: usize, byte: usize) -> Vec<u8> {
    let mut items = wire::iter_items(input).0;
    let frame = items[index].1.as_map_mut().unwrap();
    let signature = frame
        .iter_mut()
        .find(|(key, _)| key == &Value::from("sig"))
        .unwrap()
        .1
        .as_bytes()
        .unwrap()
        .to_vec();
    let mut sign1 = cbor::decode(&signature, cbor::Limits::DEFAULT).unwrap();
    let Value::Tag(_, value) = &mut sign1 else {
        panic!("tagged Sign1");
    };
    let Value::Array(fields) = value.as_mut() else {
        panic!("Sign1 array");
    };
    let Value::Bytes(bytes) = &mut fields[3] else {
        panic!("signature bytes");
    };
    bytes[byte] ^= 1;
    frame
        .iter_mut()
        .find(|(key, _)| key == &Value::from("sig"))
        .unwrap()
        .1 = Value::Bytes(cbor::encode(&sign1));
    items
        .into_iter()
        .flat_map(|(_, value)| cbor::encode(&value))
        .collect()
}

fn authentic_composite_and_ed_packaging_preserve_exact_mixed_authorship() {
    purrdf_testkit::harness::without_host_clock_or_entropy(|| {
        let input = source();
        let original = read(&input, true, None);
        let pairs = compact::detached_signature_pairs(&original).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(verify_file_with_keyring(&input, &keys()).valid, 2);
        for (pack, algorithm) in [
            (ed_pack(&input), Algorithm::Ed25519),
            (composite_pack(&input), Algorithm::CompositeMlDsa65Ed25519),
        ] {
            let folded = read(&pack, true, None);
            assert_eq!(folded.diagnostics, []);
            assert_eq!(compact::detached_signature_pairs(&folded).unwrap(), pairs);
            assert_eq!(
                refold_digest(&original).unwrap(),
                refold_digest(&folded).unwrap()
            );
            assert!(verify_compaction(&input, &pack, &keys()).unwrap().all_ok());
            let signature = folded.signatures.last().unwrap().cose.as_ref().unwrap();
            assert_eq!(cose::parse_sign1(signature).unwrap().algorithm(), algorithm);
            let leaves = compact::detached_signature_leaves(&folded).unwrap();
            let root = purrdf_gts::mmr::root(&leaves);
            assert_eq!(
                purrdf_gts::mmr::parse_hex_32(
                    &object_literal(&folded, purrdf_gts::stream::DETACHED_SIGNATURE_ROOT).unwrap()
                )
                .unwrap(),
                root
            );
            for (id, signature) in &pairs {
                let proof = compact::detached_signature_proof(&folded, id, signature)
                    .unwrap()
                    .unwrap();
                assert_eq!(proof.root, root);
                purrdf_gts::mmr::verify_proof(&proof).unwrap();
            }
            let repack = composite_pack(&pack);
            assert_eq!(
                compact::detached_signature_pairs(&read(&repack, true, None)).unwrap(),
                pairs
            );
            assert!(verify_compaction(&pack, &repack, &keys()).unwrap().all_ok());
        }
        let direct = compact::compact_streamable(
            &input,
            CompactionParams {
                timestamp: TIME,
                seal_original: false,
                plan: DictPlan::undicted(),
                content_digest: None,
                packaging_signer: CompositePackaging::new(
                    fixed_composite_key(74),
                    "composite-pack".into(),
                    draw,
                ),
            },
        )
        .unwrap();
        assert!(
            verify_compaction(&input, &direct, &keys())
                .unwrap()
                .all_ok()
        );
    });
}

fn both_components_and_resolved_typed_keys_are_required() {
    let input = source();
    let pack = composite_pack(&input);
    for byte in [0, 3309] {
        let bad_source = mutate_signature(&input, 1, byte);
        let carried = composite_pack(&bad_source);
        let report = verify_compaction(&bad_source, &carried, &keys()).unwrap();
        assert!(report.packaging_sig_ok);
        assert!(!report.signatures_verify && !report.all_ok());
        let index = wire::iter_items(&pack).0.len() - 1;
        let bad_package = mutate_signature(&pack, index, byte);
        let report = verify_compaction(&input, &bad_package, &keys()).unwrap();
        assert!(report.signatures_verify);
        assert!(!report.packaging_sig_ok && !report.all_ok());
    }
    let mut wrong = keys();
    wrong.insert(AUTHOR, fixed_key(70).verifying_key());
    assert!(
        !verify_compaction(&input, &pack, &wrong)
            .unwrap()
            .signatures_verify
    );
    wrong.insert(
        AUTHOR,
        Box::new(fixed_composite_key(76).verifying_key().clone()),
    );
    assert!(
        !verify_compaction(&input, &pack, &wrong)
            .unwrap()
            .signatures_verify
    );
    let mut unresolved = Keyring::default();
    unresolved.insert(
        b"composite-pack",
        Box::new(fixed_composite_key(74).verifying_key().clone()),
    );
    assert!(
        !verify_compaction(&input, &pack, &unresolved)
            .unwrap()
            .signatures_verify
    );
    let mut wrong_pack = keys();
    wrong_pack.insert(b"composite-pack", fixed_key(74).verifying_key());
    assert!(
        !verify_compaction(&input, &pack, &wrong_pack)
            .unwrap()
            .packaging_sig_ok
    );
    wrong_pack.insert(
        b"composite-pack",
        Box::new(fixed_composite_key(78).verifying_key().clone()),
    );
    assert!(
        !verify_compaction(&input, &pack, &wrong_pack)
            .unwrap()
            .packaging_sig_ok
    );
    assert!(
        !verify_compaction(&input, &pack, &Keyring::default())
            .unwrap()
            .packaging_sig_ok
    );
}

fn a_pack_followed_by_new_authored_history_keeps_both_kinds_of_authorship() {
    let input = source();
    let mut combined = ed_pack(&input);
    let mut tail = Writer::with_options(
        "purrdf.gts",
        purrdf_gts::writer::WriterOptions {
            layout: Some("streamable".into()),
            ..Default::default()
        },
    )
    .unwrap()
    .with_composite_signer(fixed_composite_key(70), AUTHOR, draw);
    tail.add_terms(&[
        Term::iri("https://example.org/later-subject"),
        Term::iri("https://example.org/predicate"),
        Term::literal("later authored history", None),
    ])
    .unwrap();
    tail.add_quads(&[(0, 1, 2, None)]).unwrap();
    tail.add_index().unwrap();
    let tail_bytes = tail.into_bytes();
    let tail_signatures = read(&tail_bytes, true, None).signatures;
    combined.extend_from_slice(&tail_bytes);
    let folded = read(&combined, true, None);
    assert_eq!(folded.diagnostics, []);
    let pairs = compact::detached_signature_pairs(&folded).unwrap();
    assert_eq!(pairs.len(), 5);
    let mut observations = Observations::default();
    let streamed = purrdf_gts::reader::read_to_sink(&combined, true, None, &mut observations);
    assert_eq!(streamed.diagnostics, []);
    assert_eq!(observations.0, folded.signatures);
    for signature in tail_signatures {
        assert!(pairs.contains(&(signature.frame_id, signature.cose.unwrap())));
    }
    let packed = composite_pack(&combined);
    let final_graph = read(&packed, true, None);
    let selected_roots =
        compact::compaction_signature_roots(&final_graph, &folded.segment_heads).unwrap();
    assert_eq!(selected_roots.len(), 1);
    assert_ne!(
        selected_roots,
        compact::compaction_signature_roots(&final_graph, &read(&input, true, None).segment_heads)
            .unwrap()
    );
    assert_eq!(
        compact::detached_signature_pairs(&read(&packed, true, None)).unwrap(),
        pairs
    );
    assert!(
        verify_compaction(&combined, &packed, &keys())
            .unwrap()
            .all_ok()
    );
}

fn unrelated_valid_signature_cannot_replace_the_ordering_commitment() {
    let input = source();
    let pack = ed_pack(&input);
    let mut items = wire::iter_items(&pack).0;
    let first = items[1].1.as_map_mut().unwrap();
    let id = wire::content_id(first);
    first.push((
        Value::from("sig"),
        Value::Bytes(cose::sign_id(&id, &fixed_key(73), "ed-pack")),
    ));
    let index = items.last_mut().unwrap().1.as_map_mut().unwrap();
    index.retain(|(key, _)| key != &Value::from("sig"));
    let unsigned_index: Vec<u8> = items
        .iter()
        .flat_map(|(_, value)| cbor::encode(value))
        .collect();
    assert_eq!(verify_file_with_keyring(&unsigned_index, &keys()).valid, 1);
    assert!(
        !verify_compaction(&input, &unsigned_index, &keys())
            .unwrap()
            .packaging_sig_ok
    );
    let index = items.last_mut().unwrap().1.as_map_mut().unwrap();
    let id = wire::content_id(index);
    index.push((
        Value::from("sig"),
        Value::Bytes(cose::sign_id(&id, &fixed_key(73), "ed-pack")),
    ));
    let signed: Vec<u8> = items
        .into_iter()
        .flat_map(|(_, value)| cbor::encode(&value))
        .collect();
    assert!(
        verify_compaction(&input, &signed, &keys())
            .unwrap()
            .packaging_sig_ok
    );
    let mut wrong_order = wire::iter_items(&signed).0;
    let index = wrong_order.last_mut().unwrap().1.as_map_mut().unwrap();
    let payload = index
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some("d"))
        .unwrap()
        .1
        .as_map_mut()
        .unwrap();
    payload
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some("count"))
        .unwrap()
        .1 = Value::from(0_u8);
    let id = wire::content_id(index);
    index
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some("id"))
        .unwrap()
        .1 = Value::Bytes(id.clone());
    index
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some("sig"))
        .unwrap()
        .1 = Value::Bytes(cose::sign_id(&id, &fixed_key(73), "ed-pack"));
    let wrong_order: Vec<u8> = wrong_order
        .into_iter()
        .flat_map(|(_, value)| cbor::encode(&value))
        .collect();
    assert_eq!(verify_file_with_keyring(&wrong_order, &keys()).valid, 2);
    assert!(
        read(&wrong_order, true, None)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "StreamableLayoutError")
    );
    assert!(
        !verify_compaction(&input, &wrong_order, &keys())
            .unwrap()
            .packaging_sig_ok
    );
    let invalid = mutate_signature(&signed, 1, 0);
    assert_eq!(verify_file_with_keyring(&invalid, &keys()).valid, 1);
    assert!(
        !verify_compaction(&input, &invalid, &keys())
            .unwrap()
            .packaging_sig_ok
    );
    let mut torn = signed;
    torn.push(0x5f);
    assert!(
        !verify_compaction(&input, &torn, &keys())
            .unwrap()
            .packaging_sig_ok
    );
}

fn entropy_failure_returns_no_pack_and_explicit_fresh_retry_succeeds() {
    let input = source();
    for partial in [false, true] {
        let provider = move |out: &mut [u8; 32]| {
            if partial {
                out[..16].fill(0x33);
            }
            Err(RandomnessError::new("caller entropy unavailable"))
        };
        let result = compact_and_certify(
            &input,
            DictPlan::undicted(),
            TIME,
            false,
            CompositePackaging::new(fixed_composite_key(74), "composite-pack".into(), provider),
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("caller entropy unavailable")
        );
    }
    assert!(
        verify_compaction(&input, &composite_pack(&input), &keys())
            .unwrap()
            .all_ok()
    );
}

fn malformed_and_ambiguous_carried_nodes_are_never_silently_dropped() {
    let original = source();
    let source_signature = read(&original, true, None).signatures[0].clone();
    for variant in 0..9 {
        let mut writer = Writer::new("purrdf.gts");
        let mut terms = vec![
            Term::iri(TYPE),
            Term::iri(purrdf_gts::stream::DETACHED_SIGNATURE),
            Term::blank("evidence"),
        ];
        // Duplicate vocabulary identities at different term ids are legal.
        terms.extend([
            Term::iri(TYPE),
            Term::iri(purrdf_gts::stream::DETACHED_SIGNATURE),
        ]);
        let mut quads = vec![(2, 3, 4, None)];
        if variant != 0 {
            terms.extend([
                Term::iri(purrdf_gts::stream::SOURCE_FRAME),
                Term::literal(wire::digest_label(&source_signature.frame_id), None),
                Term::iri(purrdf_gts::stream::COSE),
                Term::literal(
                    compact::base64url_unpadded(source_signature.cose.as_ref().unwrap()),
                    None,
                ),
            ]);
            quads.extend([(2, 5, 6, None), (2, 7, 8, None)]);
            match variant {
                1 => {
                    terms[8] = Term::literal("%%%", None);
                }
                2 => {
                    terms[6] = Term::literal("not-a-frame", None);
                }
                3 => {
                    terms[8] = Term::iri("https://example.org/not-a-literal");
                }
                4 => {
                    terms.push(Term::literal("conflict", None));
                    quads.push((2, 7, 9, None));
                }
                5 => {
                    terms[8] = Term::literal(compact::base64url_unpadded(b"bad COSE"), None);
                }
                6 => {
                    terms[5] = Term::literal(purrdf_gts::stream::SOURCE_FRAME, None);
                }
                7 => {
                    terms.push(Term::iri(purrdf_gts::stream::COSE));
                    quads.push((2, 9, 8, None));
                }
                8 => {}
                _ => unreachable!(),
            }
        }
        writer.add_terms(&terms);
        writer.add_quads(&quads);
        let input = writer.into_bytes();
        let folded = read(&input, true, None);
        if variant == 8 {
            assert_eq!(compact::detached_signature_pairs(&folded).unwrap().len(), 1);
            assert!(
                verify_compaction(&input, &ed_pack(&input), &keys())
                    .unwrap()
                    .all_ok()
            );
            continue;
        }
        if variant == 6 {
            assert!(
                folded
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "PositionConstraint")
            );
        } else {
            assert!(
                folded.diagnostics.is_empty(),
                "variant {variant}: {:?}",
                folded.diagnostics
            );
        }
        assert!(
            compact::detached_signature_pairs(&folded).is_err(),
            "variant {variant}"
        );
        let error = compact::compact_streamable(
            &input,
            CompactionParams {
                timestamp: TIME,
                seal_original: false,
                plan: DictPlan::undicted(),
                content_digest: None,
                packaging_signer: (fixed_key(73), "ed-pack".into()),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains(if variant == 6 {
                "PositionConstraint"
            } else {
                "detached signature"
            }),
            "variant {variant}: {error}"
        );
    }
}

fn literal_class_names_remain_content_and_do_not_fabricate_provenance() {
    let mut writer = Writer::new("purrdf.gts");
    writer.add_terms(&[
        Term::iri(TYPE),
        Term::literal(purrdf_gts::stream::DETACHED_SIGNATURE, None),
        Term::blank("ordinary-content"),
    ]);
    writer.add_quads(&[(2, 0, 1, None)]);
    let input = writer.into_bytes();
    let folded = read(&input, true, None);
    assert_eq!(folded.diagnostics, []);
    assert_eq!(
        purrdf_rdf::gts_certify::content_projection(&folded).quads,
        folded.quads
    );
    assert_eq!(compact::detached_signature_pairs(&folded).unwrap(), []);
    assert!(
        verify_compaction(&input, &ed_pack(&input), &keys())
            .unwrap()
            .all_ok()
    );
}

fn compaction_terms(timestamp: &str, literal_class: bool) -> [Term; 14] {
    [
        Term::blank("ordinary-compaction-content"),
        Term::iri(TYPE),
        Term::iri(purrdf_gts::stream::COMPACTION),
        Term::iri("https://example.org/application-predicate"),
        Term::literal("ordinary content", None),
        Term::iri(TYPE), // Equivalent vocabulary at a different term id.
        if literal_class {
            Term::literal(purrdf_gts::stream::COMPACTION, None)
        } else {
            Term::iri(purrdf_gts::stream::COMPACTION)
        },
        Term::iri(purrdf_gts::stream::AGENT),
        Term::iri(purrdf_gts::stream::TIMESTAMP),
        Term::iri(purrdf_gts::stream::SOURCE_HEAD),
        Term::iri(purrdf_xsd::datatype::XSD_DATE_TIME),
        Term::literal("ordinary application", None),
        Term::literal(timestamp, Some(10)),
        Term::literal(wire::digest_label(&[0; 32]), None),
    ]
}

fn ordinary_compaction_type_keeps_verbatim_authored_index_in_both_readers() {
    let cases = [(false, true, 7), (true, true, 7)]
        .into_iter()
        .chain((0..7).map(|fields| (false, false, fields)));
    for (literal_class, foreign_predicate, fields) in cases {
        let terms = compaction_terms(TIME, literal_class);
        let mut quads = vec![(0, 5, 6, None)];
        for (field, predicate, object) in [(1, 7, 11), (2, 8, 12), (4, 9, 13)] {
            if fields & field != 0 {
                quads.push((0, predicate, object, None));
            }
        }
        if foreign_predicate {
            quads.insert(0, (0, 3, 4, None));
        }
        let input = signed_graph(&terms, &quads);
        let folded = read(&input, true, None);
        assert_eq!(folded.diagnostics, []);
        assert_eq!(folded.signatures.len(), 1);
        assert!(!folded.signatures[0].packaging);
        let pairs = compact::detached_signature_pairs(&folded).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].1, folded.signatures[0].cose.clone().unwrap());
        assert_eq!(
            purrdf_rdf::gts_certify::content_projection(&folded).quads,
            folded.quads
        );
        let mut observations = Observations::default();
        let streamed = purrdf_gts::reader::read_to_sink(&input, true, None, &mut observations);
        assert_eq!(streamed.diagnostics, []);
        assert_eq!(observations.0, folded.signatures);
        let packed = composite_pack(&input);
        assert_eq!(
            compact::detached_signature_pairs(&read(&packed, true, None)).unwrap(),
            pairs
        );
        assert!(
            verify_compaction(&input, &packed, &keys())
                .unwrap()
                .all_ok()
        );
        let repacked = ed_pack(&packed);
        assert_eq!(
            compact::detached_signature_pairs(&read(&repacked, true, None)).unwrap(),
            pairs
        );
        assert!(
            verify_compaction(&packed, &repacked, &keys())
                .unwrap()
                .all_ok()
        );
    }
}

fn rewrite_timestamp_shape_preserves_authorship_and_classifies_only_utc() {
    let cases = UTC_TIMESTAMPS
        .iter()
        .map(|&timestamp| (timestamp, true))
        .chain(
            INVALID_REWRITE_TIMESTAMPS
                .iter()
                .map(|&timestamp| (timestamp, false)),
        );
    for (timestamp, packaging) in cases {
        let input = signed_graph(
            &compaction_terms(timestamp, false),
            &[
                (0, 5, 6, None),
                (0, 7, 11, None),
                (0, 8, 12, None),
                (0, 9, 13, None),
            ],
        );
        let original = read(&input, true, None);
        assert_eq!(original.diagnostics, [], "{timestamp}");
        assert_eq!(verify_file_with_keyring(&input, &keys()).valid, 1);
        assert_eq!(original.signatures[0].packaging, packaging, "{timestamp}");
        let mut observations = Observations::default();
        let streamed = purrdf_gts::reader::read_to_sink(&input, true, None, &mut observations);
        assert_eq!(streamed.diagnostics, [], "{timestamp}");
        assert_eq!(observations.0, original.signatures, "{timestamp}");
        let pairs = compact::detached_signature_pairs(&original).unwrap();
        let projection = purrdf_rdf::gts_certify::content_projection(&original);
        if packaging {
            assert_eq!(pairs, [], "{timestamp}");
            assert_eq!(projection.quads, [], "{timestamp}");
        } else {
            assert_eq!(pairs.len(), 1, "{timestamp}");
            assert_eq!(pairs[0].1, original.signatures[0].cose.clone().unwrap());
            assert_eq!(projection.quads, original.quads, "{timestamp}");
        }
        let packed = composite_pack(&input);
        let folded = read(&packed, true, None);
        assert_eq!(folded.diagnostics, [], "{timestamp}");
        assert!(folded.signatures.last().unwrap().packaging);
        assert_eq!(compact::detached_signature_pairs(&folded).unwrap(), pairs);
        assert_eq!(
            refold_digest(&folded).unwrap(),
            refold_digest(&original).unwrap()
        );
        assert!(
            verify_compaction(&input, &packed, &keys())
                .unwrap()
                .all_ok(),
            "{timestamp}"
        );
        let repacked = ed_pack(&packed);
        let refolded = read(&repacked, true, None);
        assert!(refolded.signatures.last().unwrap().packaging);
        assert_eq!(compact::detached_signature_pairs(&refolded).unwrap(), pairs);
        assert_eq!(
            refold_digest(&refolded).unwrap(),
            refold_digest(&original).unwrap()
        );
        assert!(
            verify_compaction(&packed, &repacked, &keys())
                .unwrap()
                .all_ok(),
            "{timestamp}"
        );
    }
}

fn rewrite_timestamp_parameters_refuse_before_randomness_and_preserve_utc_bytes() {
    let input = source();
    for &timestamp in INVALID_REWRITE_TIMESTAMPS {
        let provider = |_: &mut [u8; 32]| -> Result<(), RandomnessError> {
            panic!("invalid rewrite time must refuse before signing")
        };
        let error = compact::compact_streamable(
            &input,
            CompactionParams {
                timestamp,
                seal_original: false,
                plan: DictPlan::undicted(),
                content_digest: None,
                packaging_signer: CompositePackaging::new(
                    fixed_composite_key(74),
                    "composite-pack".into(),
                    provider,
                ),
            },
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("rewrite timestamp"),
            "{timestamp}: {error}"
        );
        let error = compact_and_certify(
            &input,
            DictPlan::undicted(),
            timestamp,
            false,
            (fixed_key(73), "ed-pack".into()),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("rewrite timestamp"),
            "{timestamp}: {error}"
        );
    }
    for &timestamp in UTC_TIMESTAMPS {
        let packed = compact::compact_streamable(
            &input,
            CompactionParams {
                timestamp,
                seal_original: false,
                plan: DictPlan::undicted(),
                content_digest: None,
                packaging_signer: (fixed_key(73), "ed-pack".into()),
            },
        )
        .unwrap();
        let folded = read(&packed, true, None);
        assert_eq!(folded.diagnostics, [], "{timestamp}");
        assert_eq!(
            object_literal(&folded, purrdf_gts::stream::TIMESTAMP).as_deref(),
            Some(timestamp)
        );
        assert!(folded.signatures.last().unwrap().packaging);
        assert!(
            verify_compaction(&input, &packed, &keys())
                .unwrap()
                .all_ok(),
            "{timestamp}"
        );
    }
}

fn root_commitments_require_current_subject_literal_identity_and_cardinality() {
    let input = source();
    let packed = ed_pack(&input);
    let original = read(&packed, true, None);
    let root_quad = original
        .quads
        .iter()
        .position(|&(_, p, _, _)| {
            original.terms[p].iri_value() == Some(purrdf_gts::stream::DETACHED_SIGNATURE_ROOT)
        })
        .unwrap();
    for variant in 0..8 {
        let mut graph = read(&packed, true, None);
        let (subject, predicate, object, _) = graph.quads[root_quad];
        let spelling = graph.terms[object].value.clone().unwrap();
        match variant {
            0 => {}
            1 => {
                let duplicate_predicate = graph.terms.len();
                graph
                    .terms
                    .push(Term::iri(purrdf_gts::stream::DETACHED_SIGNATURE_ROOT));
                let contradictory = graph.terms.len();
                graph.terms.push(Term::literal("00".repeat(32), None));
                graph
                    .quads
                    .push((subject, duplicate_predicate, contradictory, None));
            }
            2 => graph.terms[object] = Term::blank(&spelling),
            3 => graph.terms[object] = Term::literal("invalid-root", None),
            4 => {
                let neighbor = graph.terms.len();
                graph.terms.push(Term::blank("unrelated-root-subject"));
                graph.quads[root_quad].0 = neighbor;
            }
            5 => {
                let neighbor = graph.terms.len();
                graph.terms.push(Term::blank("unrelated-matching-root"));
                let correct = graph.terms.len();
                graph.terms.push(Term::literal(&spelling, None));
                graph.terms[object] = Term::literal("00".repeat(32), None);
                graph.quads.push((neighbor, predicate, correct, None));
            }
            6 => {
                let (_, _, head, _) = graph
                    .quads
                    .iter()
                    .find(|&&(s, p, _, _)| {
                        s == subject
                            && graph.terms[p].iri_value() == Some(purrdf_gts::stream::SOURCE_HEAD)
                    })
                    .copied()
                    .unwrap();
                graph.terms[head] = Term::literal(wire::digest_label(&[0; 32]), None);
            }
            7 => {
                let duplicate = graph.terms.len();
                graph.terms.push(Term::literal(
                    purrdf_gts::stream::DETACHED_SIGNATURE_ROOT,
                    None,
                ));
                // An unused literal lookalike must not shadow the actual IRI.
                graph.terms.swap(predicate, duplicate);
                graph.quads[root_quad].1 = duplicate;
            }
            _ => unreachable!(),
        }
        let rebuilt = signed_graph(&graph.terms, &graph.quads);
        let report = verify_compaction(&input, &rebuilt, &keys()).unwrap();
        if variant == 0 || variant == 7 {
            assert!(report.all_ok(), "variant {variant}: {report:?}");
        } else {
            assert!(!report.signatures_bound, "variant {variant}: {report:?}");
            assert!(!report.all_ok());
        }
        if (1..=3).contains(&variant) {
            let error = compact_and_certify(
                &rebuilt,
                DictPlan::undicted(),
                TIME,
                false,
                (fixed_key(73), "ed-pack".into()),
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("root") || error.contains("literal"),
                "{error}"
            );
        }
    }
}

purrdf_testkit::harness_main!(
    authentic_composite_and_ed_packaging_preserve_exact_mixed_authorship,
    both_components_and_resolved_typed_keys_are_required,
    a_pack_followed_by_new_authored_history_keeps_both_kinds_of_authorship,
    unrelated_valid_signature_cannot_replace_the_ordering_commitment,
    entropy_failure_returns_no_pack_and_explicit_fresh_retry_succeeds,
    malformed_and_ambiguous_carried_nodes_are_never_silently_dropped,
    literal_class_names_remain_content_and_do_not_fabricate_provenance,
    ordinary_compaction_type_keeps_verbatim_authored_index_in_both_readers,
    rewrite_timestamp_shape_preserves_authorship_and_classifies_only_utc,
    rewrite_timestamp_parameters_refuse_before_randomness_and_preserve_utc_bytes,
    root_commitments_require_current_subject_literal_identity_and_cardinality,
);
