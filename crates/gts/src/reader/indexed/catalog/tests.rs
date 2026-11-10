// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![cfg(test)]

use super::*;
use crate::dictionary::DictionaryLimits;
use crate::model::{Term, TermKind};
use crate::reader::indexed::{AccessBinding, AccessOwner};
use crate::wire::content_id;
use crate::writer::Writer;
use std::io::Cursor;
use std::sync::Arc;

type Identity = Box<dyn FnMut(&Cursor<Vec<u8>>) -> io::Result<AccessBinding>>;
type Catalog = IndexedCatalog<Cursor<Vec<u8>>, Identity>;
fn policy() -> Arc<LookupPolicy> {
    Arc::new(LookupPolicy {
        allow_segments: true,
        expected_head: None,
        limits: DictionaryLimits {
            encoded_frame_bytes: 2 * 1024 * 1024,
            decoded_frame_bytes: 2 * 1024 * 1024,
            metadata_entries: 128,
        },
    })
}
fn open(bytes: Vec<u8>) -> Catalog {
    open_policy(bytes, policy()).expect("same concrete decoder and policy owners")
}
fn open_policy(bytes: Vec<u8>, policy: Arc<LookupPolicy>) -> Result<Catalog, IndexedError> {
    open_context(
        bytes,
        policy,
        DecodeCapabilities::new(Arc::new(|_: &str| None)),
    )
}
fn open_context(
    bytes: Vec<u8>,
    policy: Arc<LookupPolicy>,
    capabilities: DecodeCapabilities,
) -> Result<Catalog, IndexedError> {
    let binding = AccessBinding {
        source: AccessOwner::from_owner(Arc::new(())),
        generation: 0,
        length: bytes.len() as u64,
        capabilities: capabilities.owner().clone(),
        capability_generation: 0,
        policy: policy.owner(),
    };
    let current: Identity = Box::new(move |_| Ok(binding.clone()));
    IndexedCatalog::new(
        IndexedInput::new(Cursor::new(bytes), current, policy.limits)
            .expect("bound immutable source"),
        capabilities,
        policy,
    )
}
fn terms(writer: &mut Writer) {
    writer.add_terms(&[Term {
        kind: TermKind::Iri,
        value: Some("urn:selected".into()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }]);
}
fn field<'a>(entries: &'a mut [(Value, Value)], name: &str) -> &'a mut Value {
    &mut entries
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some(name))
        .expect("generated field")
        .1
}
fn altered_index(bytes: &[u8], change: impl FnOnce(&mut Vec<(Value, Value)>)) -> Vec<u8> {
    let (mut items, torn) = crate::wire::iter_items(bytes);
    assert!(torn.is_none());
    let (_, index) = items.last_mut().expect("index footer");
    let Value::Map(entries) = index else {
        panic!("frame map")
    };
    let Value::Map(payload) = field(entries, "d") else {
        panic!("index payload")
    };
    change(payload);
    let id = content_id(entries);
    *field(entries, "id") = Value::Bytes(id);
    items
        .iter()
        .flat_map(|(_, value)| crate::wire::encode(value))
        .collect()
}

#[test]
fn locators_are_checked_against_structure_even_after_valid_index_rehash() {
    let mut writer = Writer::new("rdf");
    terms(&mut writer);
    writer.add_frame("future-capability", Some(Value::Null), None, None, None);
    writer.add_index_with_mmr();
    let original = writer.into_bytes();
    let mut source = open(original.clone());
    let catalog = source
        .segment(0)
        .expect("default writer ti is a dictionary locator");
    assert_eq!(catalog.covered_frames(), 2);
    let dict_only = altered_index(&original, |payload| {
        payload.retain(|(key, _)| !matches!(key.as_text(), Some("off" | "ti")));
        payload.push(("dict".into(), Value::Array(vec![0.into()])));
    });
    open(dict_only)
        .segment(0)
        .expect("valid optional dict-only locator form");
    let no_locators = altered_index(&original, |payload| {
        payload.retain(|(key, _)| !matches!(key.as_text(), Some("off" | "ti")));
    });
    assert!(matches!(
        open(no_locators).segment(0),
        Err(IndexedError::Catalog(CatalogError::IndexUnavailable(0)))
    ));
    for rule in [
        "offset", "overlap", "type", "omission", "dict", "head", "count", "mmr",
    ] {
        let changed = altered_index(&original, |payload| match rule {
            "offset" | "overlap" => {
                let Value::Array(offsets) = field(payload, "off") else {
                    panic!("offsets")
                };
                offsets[1] = if rule == "offset" {
                    Value::from(u64::MAX)
                } else {
                    offsets[0].clone()
                };
            }
            "type" | "omission" => {
                let Value::Map(kinds) = field(payload, "ti") else {
                    panic!("types")
                };
                if rule == "type" {
                    *field(kinds, "terms") = Value::Array(vec![Value::from(1)]);
                } else {
                    kinds.retain(|(key, _)| key.as_text() != Some("terms"));
                }
            }
            "dict" => payload.push(("dict".into(), Value::Array(vec![Value::from(1)]))),
            "head" => *field(payload, "head") = Value::Bytes(vec![0; 32]),
            "count" => *field(payload, "count") = Value::from(1),
            "mmr" => *field(payload, "mmr") = Value::Bytes(vec![0; 32]),
            _ => unreachable!(),
        });
        assert!(
            matches!(
                open(changed).segment(0),
                Err(IndexedError::Catalog(CatalogError::Contradiction { .. }))
            ),
            "{rule}"
        );
    }
}

#[test]
fn segments_latest_index_and_tail_use_original_relative_offsets() {
    let mut first = Writer::new("rdf");
    terms(&mut first);
    first.add_index();
    let mut second = Writer::new("rdf");
    terms(&mut second);
    second.add_index();
    terms(&mut second);
    second.add_index();
    terms(&mut second);
    let mut bytes = first.into_bytes();
    bytes.extend(second.into_bytes());
    let prefix_source = bytes.clone();
    let mut source = open(bytes);
    let catalog = source.segment(1).expect("second segment latest index");
    assert_eq!(catalog.segment(), 1);
    assert_eq!(catalog.covered_frames(), 3);
    assert_eq!(catalog.frame_count(), 5);
    source.frame(&catalog, 4).expect("legal tail frame");
    assert!(matches!(
        source.segment(2),
        Err(IndexedError::Catalog(CatalogError::SegmentMissing(2)))
    ));
    let (mut items, torn) = crate::wire::iter_items(&prefix_source);
    assert!(torn.is_none());
    match &mut items[0].1 {
        Value::Tag(tag, _) => *tag = 55798,
        header => {
            let original = std::mem::replace(header, Value::Null);
            *header = Value::Tag(55798, Box::new(original));
        }
    }
    let bad_prefix = items
        .iter()
        .flat_map(|(_, item)| crate::wire::encode(item))
        .collect();
    let failure = open(bad_prefix)
        .segment(1)
        .expect_err("unsupported prefix cannot silently change later segment identity");
    assert!(matches!(
        std::error::Error::source(&failure)
            .and_then(|cause| cause.downcast_ref::<crate::wire::HeaderError>()),
        Some(crate::wire::HeaderError::Tag { actual: 55798 })
    ));

    let mut unindexed = Writer::new("rdf");
    terms(&mut unindexed);
    assert!(matches!(
        open(unindexed.into_bytes()).segment(0),
        Err(IndexedError::Catalog(CatalogError::IndexUnavailable(0)))
    ));
}

#[test]
fn discovery_skips_unvisited_text_bodies_without_claiming_payload_verification() {
    let mut writer = Writer::new("rdf");
    terms(&mut writer);
    let marker = "unvisited-body:";
    writer.add_frame(
        "future-capability",
        Some(Value::Text(format!("{marker}{}", "x".repeat(1024 * 1024)))),
        None,
        None,
        None,
    );
    writer.add_index_with_mmr();
    let mut bytes = writer.into_bytes();
    let offset = bytes
        .windows(marker.len())
        .position(|window| window == marker.as_bytes())
        .expect("large unvisited text body");
    bytes[offset] = 0xff;
    let mut source = open(bytes);
    let catalog = source
        .segment(0)
        .expect("structural discovery does not admit unrelated UTF-8 or hash");
    assert!(
        source.work().read_bytes < 128 * 1024,
        "actual transport reads must skip the large body: {:?}",
        source.work()
    );
    source
        .frame(&catalog, 0)
        .expect("selected dictionary frame remains valid");
    assert!(matches!(
        source.frame(&catalog, 1),
        Err(IndexedError::Structure(_))
    ));
}

#[test]
fn decoder_owner_cannot_be_swapped_under_the_same_bound_input() {
    let source = open(Vec::new());
    let foreign = DecodeCapabilities::new(Arc::new(|_: &str| Some([7; 32])));
    assert!(matches!(
        IndexedCatalog::new(source.input, foreign, source.policy),
        Err(IndexedError::CapabilityOwner)
    ));
}

#[test]
fn actual_policy_owner_and_global_reader_requests_are_enforced() {
    let source = open(Vec::new());
    assert!(matches!(
        IndexedCatalog::new(source.input, source.capabilities, policy()),
        Err(IndexedError::PolicyMismatch("owner"))
    ));
    let mut global = policy();
    Arc::get_mut(&mut global)
        .expect("unshared policy")
        .expected_head = Some(vec![3; 32]);
    assert!(matches!(
        open_policy(Vec::new(), global),
        Err(IndexedError::FullVerificationRequired("expected-head"))
    ));
    let mut writer = Writer::new("rdf");
    terms(&mut writer);
    writer.add_index();
    let segment = writer.into_bytes();
    let mut bytes = segment.clone();
    bytes.extend(segment);
    let mut single = policy();
    Arc::get_mut(&mut single)
        .expect("unshared policy")
        .allow_segments = false;
    let mut source = open_policy(bytes, single).expect("concrete single-segment policy");
    let error = source
        .segment(0)
        .expect_err("cannot quietly ignore a second segment");
    assert!(matches!(
        std::error::Error::source(&error).and_then(|cause| cause.downcast_ref::<FoldError>()),
        Some(FoldError::SegmentBoundary { item: 3 })
    ));
}

#[test]
fn term_intervals_count_only_map_rows_and_include_snapshot_and_tail_contributors() {
    let mut writer = Writer::new("rdf");
    writer.add_frame(
        "terms",
        Some(Value::Array(vec![
            Value::Null,
            Value::Map(vec![
                ("k".into(), Value::from(0)),
                ("v".into(), "urn:first".into()),
            ]),
        ])),
        None,
        None,
        None,
    );
    writer.add_frame(
        "snapshot",
        Some(Value::Map(vec![(
            "terms".into(),
            Value::Array(vec![Value::Map(vec![
                ("k".into(), Value::from(0)),
                ("v".into(), "urn:snapshot".into()),
            ])]),
        )])),
        None,
        None,
        None,
    );
    writer.add_index();
    terms(&mut writer);
    let mut source = open(writer.into_bytes());
    let catalog = source
        .segment(0)
        .expect("indexed segment with snapshot and tail");
    let snapshot = source.locate_term(&catalog, 1).expect("snapshot interval");
    assert_eq!(
        (snapshot.first(), snapshot.end(), snapshot.is_snapshot()),
        (1, 2, true)
    );
    let tail = source.locate_term(&catalog, 2).expect("tail interval");
    assert_eq!(
        (tail.first(), tail.end(), tail.is_snapshot()),
        (2, 3, false)
    );
    source
        .block_payload(&catalog, &tail)
        .expect("selected tail content id");
    assert!(matches!(
        source.locate_term(&catalog, 3),
        Err(IndexedError::Dictionary(DictionaryError::Missing(_)))
    ));
}

#[test]
fn transformed_index_and_keyed_predecessor_use_the_original_capability_decoder() {
    use crate::writer::{Encrypt0Options, FrameOptions};
    let make = || {
        let mut writer = Writer::new("rdf");
        writer
            .add_frame_with_options(
                "terms",
                FrameOptions {
                    payload: Some(Value::Array(vec![crate::writer::term_to_wire(&Term::iri(
                        "urn:keyed",
                    ))])),
                    transform: vec!["gzip".into()],
                    encrypt: Some(Encrypt0Options {
                        kid: "dictionary-key".into(),
                        key: [7; 32],
                        iv: [3; 12],
                    }),
                    ..FrameOptions::default()
                },
            )
            .expect("authored keyed dictionary");
        terms(&mut writer);
        writer
    };
    let mut writer = make();
    writer.add_index();
    let original = writer.into_bytes();
    let (items, torn) = crate::wire::iter_items(&original);
    assert!(torn.is_none());
    let Value::Map(index) = &items.last().expect("generated index").1 else {
        panic!("index map")
    };
    let payload = map_get(index, "d")
        .expect("generated locator payload")
        .clone();
    let mut writer = make();
    writer.add_frame("index", Some(payload), None, Some(&["gzip".into()]), None);
    let bytes = writer.into_bytes();
    let mut no_key = open(bytes.clone());
    let catalog = no_key
        .segment(0)
        .expect("compressed index does not require an unrelated content key");
    let failure = no_key
        .locate_term(&catalog, 1)
        .expect_err("encrypted predecessor counts require their actual key");
    assert!(
        matches!(failure, IndexedError::Native(ref cause) if purrdf_testkit::error_capture::Capture::read(&***cause).code == "gts-payload-unavailable")
    );
    let IndexedError::Native(cause) = failure else {
        unreachable!("checked native refusal")
    };
    assert!(matches!(
        purrdf_testkit::error_capture::Capture::read(&**cause).field("reason"),
        purrdf_hash::error::ErrorValue::Code("missing-key")
    ));
    let (mut unknown, torn) = crate::wire::iter_items(&original);
    assert!(torn.is_none());
    let Value::Map(frame) = &mut unknown[1].1 else {
        panic!("keyed dictionary frame")
    };
    let Value::Array(ids) = field(frame, "x") else {
        panic!("dictionary transform chain")
    };
    *ids.last_mut().expect("encrypt codec") = 9.into();
    let id = content_id(frame);
    *field(frame, "id") = Value::Bytes(id);
    let unknown = unknown
        .iter()
        .flat_map(|(_, value)| crate::wire::encode(value))
        .collect();
    let mut unknown = open(unknown);
    let catalog = unknown
        .segment(0)
        .expect("intact index locators do not certify required capabilities");
    let failure = unknown
        .locate_term(&catalog, 1)
        .expect_err("unknown dictionary codec cannot silently omit predecessor terms");
    let IndexedError::Native(cause) = failure else {
        panic!("original payload refusal")
    };
    assert!(matches!(
        purrdf_testkit::error_capture::Capture::read(&**cause).field("reason"),
        purrdf_hash::error::ErrorValue::Code("unknown-codec")
    ));
    let capabilities = DecodeCapabilities::new(Arc::new(|kid: &str| {
        (kid == "dictionary-key").then_some([7; 32])
    }));
    let mut source =
        open_context(bytes, policy(), capabilities).expect("actual keyed capability owner");
    let catalog = source
        .segment(0)
        .expect("shared decoder accepts transformed index");
    let tail = source
        .locate_term(&catalog, 1)
        .expect("transformed predecessor establishes exact append range");
    assert_eq!((tail.first(), tail.end()), (1, 2));
    let keyed = source
        .locate_term(&catalog, 0)
        .expect("selected encrypted dictionary interval");
    let payload = source
        .block_payload(&catalog, &keyed)
        .expect("selected encrypted frame authenticates and decodes");
    assert!(matches!(&payload, Value::Array(rows) if rows.len() == 1));
    assert!(source.work().decoded_payloads > 1);
}
