// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The writer's MULTI-dictionary surface (GTS-SPEC §5 header `"dct"`, §8.5
//! `zstd`/`zstd-rsyncable` `dct`/`level`).
//!
//! §5 has always defined `"dct"` as a map of MANY named dictionaries and the
//! reader has always resolved them; these tests pin the writer's side of that:
//! several dictionaries in one pack, per-frame selection by name, a hard error
//! on an undeclared name, a deterministic catalog-id assignment, and the
//! declared `level` as a readable wire fact.

use std::collections::BTreeSet;

use ciborium::value::Value;
use purrdf_gts::codec::{Codec, zstd_block_layout};
use purrdf_gts::dict::{dictionary_id, raw_content_dict};
use purrdf_gts::reader::{read, segment_append_state};
use purrdf_gts::wire::{SELF_DESCRIBE_TAG, append_canonical, canonical, content_id, header_id};
use purrdf_gts::writer::{FrameOptions, Writer, WriterOptions};

/// A corpus over one vocabulary; two different topics give two genuinely
/// different dictionaries.
fn corpus(topic: &str) -> Vec<Vec<u8>> {
    (0..300u32)
        .map(|i| {
            format!(
                "<https://example.org/{topic}/s{}> <https://example.org/p> \
                 \"claim {} about {topic} and nothing else at all\" .\n",
                i % 13,
                i
            )
            .into_bytes()
        })
        .collect()
}

fn dict_for(topic: &str) -> Vec<u8> {
    let owned = corpus(topic);
    let refs: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
    raw_content_dict(&refs, 4096).expect("raw-content dictionary builds")
}

/// A payload of `topic` shape, large enough to span several rsyncable blocks.
fn payload(topic: &str) -> Vec<u8> {
    (0..3000u32)
        .flat_map(|i| {
            format!(
                "<https://example.org/{topic}/s{}> <https://example.org/p> \
                 \"claim {} about {topic} and nothing else at all\" .\n",
                i % 13,
                i + 100_000
            )
            .into_bytes()
        })
        .collect()
}

fn rsyncable() -> Vec<String> {
    vec!["zstd-rsyncable".to_string()]
}

/// A two-dictionary pack whose two blob frames each select a different one.
fn two_dict_pack(dicts: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    let mut w = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dicts,
            zstd_level: Some(12),
            ..WriterOptions::default()
        },
    )
    .expect("multi-dict writer configures");
    for topic in ["cats", "dogs"] {
        w.add_blob_transformed(
            payload(topic),
            Some("text/plain"),
            Some(topic),
            &rsyncable(),
            Some(topic),
        )
        .expect("dict-primed rsyncable blob writes");
    }
    w.into_bytes()
}

/// (c) Two dictionaries in one pack, selected per frame, BOTH decode.
#[test]
fn two_dictionaries_in_one_pack_decode_per_frame() {
    let dicts = vec![
        ("cats".to_string(), dict_for("cats")),
        ("dogs".to_string(), dict_for("dogs")),
    ];
    assert_ne!(dicts[0].1, dicts[1].1, "the dictionaries must differ");
    let bytes = two_dict_pack(dicts.clone());

    let graph = read(&bytes, true, None);
    assert!(
        graph.diagnostics.is_empty(),
        "a multi-dict pack must fold cleanly: {:?}",
        graph.diagnostics
    );
    assert_eq!(graph.blobs.len(), 2);
    let decoded: BTreeSet<Vec<u8>> = graph
        .blobs
        .iter()
        .map(|(digest, entry)| {
            entry
                .decoded_vec()
                .unwrap_or_else(|err| panic!("blob {digest} decodes: {err}"))
        })
        .collect();
    assert_eq!(
        decoded,
        BTreeSet::from([payload("cats"), payload("dogs")]),
        "each frame must decode against the dictionary it was primed with"
    );

    // (h) Every rsyncable block of a frame names its own dictionary's id, and
    // the two frames name DIFFERENT ids — proof the selection reached the wire.
    let ids_by_dict: Vec<u32> = dicts
        .iter()
        .map(|(_, bytes)| dictionary_id(bytes).expect("finalized dictionary carries an id"))
        .collect();
    assert_ne!(ids_by_dict[0], ids_by_dict[1]);
    let observed = frame_block_dictionary_ids(&bytes);
    assert_eq!(observed.len(), 2, "two transformed payload frames");
    for blocks in &observed {
        assert!(blocks.len() > 1, "each payload must span several blocks");
        let unique: BTreeSet<Option<u32>> = blocks.iter().copied().collect();
        assert_eq!(unique.len(), 1, "one dictionary per frame, every block");
    }
    let per_frame: BTreeSet<Option<u32>> = observed.iter().map(|blocks| blocks[0]).collect();
    assert_eq!(
        per_frame,
        BTreeSet::from([Some(ids_by_dict[0]), Some(ids_by_dict[1])]),
        "the two frames must name the two different Dictionary_IDs"
    );
}

/// The `Dictionary_ID` of every zstd block of every transformed payload frame.
fn frame_block_dictionary_ids(bytes: &[u8]) -> Vec<Vec<Option<u32>>> {
    let (items, _torn) = purrdf_gts::wire::iter_items(bytes);
    let mut out = Vec::new();
    for (_, item) in items.iter().skip(1) {
        let Value::Map(frame) = item else { continue };
        if purrdf_gts::wire::map_get(frame, "x").is_none() {
            continue;
        }
        let Some(Value::Bytes(data)) = purrdf_gts::wire::map_get(frame, "d") else {
            continue;
        };
        out.push(
            zstd_block_layout(data)
                .expect("a payload frame is an exact sequence of zstd frames")
                .into_iter()
                .map(|block| block.dictionary_id)
                .collect(),
        );
    }
    out
}

/// (d) Naming a dictionary the pack does not pin is a HARD ERROR on encode —
/// never a silent no-dictionary frame.
#[test]
fn a_frame_naming_an_undeclared_dictionary_is_a_hard_error() {
    let mut w = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dicts: vec![("cats".to_string(), dict_for("cats"))],
            ..WriterOptions::default()
        },
    )
    .expect("writer configures");
    let err = w
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(payload("dogs")),
                transform: rsyncable(),
                dict: Some("dogs".to_string()),
                ..FrameOptions::default()
            },
        )
        .expect_err("an undeclared dictionary name must hard-fail");
    let text = err.to_string();
    assert!(
        text.contains("dogs") && text.contains("does not pin"),
        "the error must name the missing dictionary: {text}"
    );
}

/// A duplicate dictionary name is refused: `"dct"` is a map, so one of the two
/// sets of bytes would be silently dropped while frames still named it.
#[test]
fn duplicate_dictionary_names_are_refused() {
    let err = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dicts: vec![
                ("cats".to_string(), dict_for("cats")),
                ("cats".to_string(), dict_for("dogs")),
            ],
            ..WriterOptions::default()
        },
    )
    .expect_err("a duplicate dictionary name must hard-fail");
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn duplicate_codec_names_and_ids_are_refused() {
    let duplicate_name = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            catalog: Some(vec![
                (0, Codec::new("zstd", "compress")),
                (1, Codec::new("zstd", "compress")),
            ]),
            ..WriterOptions::default()
        },
    )
    .expect_err("a duplicate codec name must hard-fail");
    assert!(
        duplicate_name.to_string().contains("duplicate codec name"),
        "{duplicate_name}"
    );

    let duplicate_id = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            catalog: Some(vec![
                (0, Codec::new("identity", "encode")),
                (0, Codec::new("zstd", "compress")),
            ]),
            ..WriterOptions::default()
        },
    )
    .expect_err("a duplicate codec id must hard-fail");
    assert!(
        duplicate_id.to_string().contains("duplicate codec id"),
        "{duplicate_id}"
    );
}

fn header_with_catalog(cat: Vec<(Value, Value)>) -> Vec<u8> {
    let mut header = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
        ("prof".into(), "purrdf.gts".into()),
        ("cat".into(), Value::Map(cat)),
    ];
    header.push(("id".into(), Value::Bytes(header_id(&header))));
    canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(Value::Map(header))))
}

#[test]
fn append_rejects_out_of_range_and_conflicting_declared_levels() {
    let out_of_range = header_with_catalog(vec![(
        Value::Integer(0.into()),
        Value::Map(vec![
            ("name".into(), "zstd".into()),
            ("cls".into(), "compress".into()),
            ("level".into(), Value::Integer(i64::MAX.into())),
        ]),
    )]);
    let malformed = segment_append_state(&out_of_range)
        .expect_err("an unrepresentable declared level must fail closed");
    assert!(malformed.contains("level is out of range"), "{malformed}");

    let conflicting = header_with_catalog(vec![
        (
            Value::Integer(0.into()),
            Value::Map(vec![
                ("name".into(), "zstd".into()),
                ("cls".into(), "compress".into()),
                ("level".into(), Value::Integer(3.into())),
            ]),
        ),
        (
            Value::Integer(1.into()),
            Value::Map(vec![
                ("name".into(), "zstd-rsyncable".into()),
                ("cls".into(), "compress".into()),
                ("level".into(), Value::Integer(12.into())),
            ]),
        ),
    ]);
    let err = Writer::appending(&conflicting)
        .expect_err("conflicting on-wire zstd levels must fail closed");
    assert!(err.to_string().contains("conflicting zstd levels"), "{err}");
}

/// (e) A pack whose catalog names a `dct` absent from the header `"dct"` map
/// must FAIL CLOSED on read: the frame degrades to an opaque node rather than
/// decoding without the dictionary (or against some other one).
#[test]
fn a_catalog_dct_absent_from_the_header_map_fails_closed_on_read() {
    // Hand-built: no writer can emit this, which is exactly why it needs a
    // hand-built fixture — it is what a hostile or buggy producer emits.
    let mut header: Vec<(Value, Value)> = vec![
        ("gts".into(), "GTS1".into()),
        ("v".into(), Value::Integer(1.into())),
        ("prof".into(), "purrdf.gts".into()),
        (
            "cat".into(),
            Value::Map(vec![(
                Value::Integer(0.into()),
                Value::Map(vec![
                    ("name".into(), "zstd-rsyncable".into()),
                    ("cls".into(), "compress".into()),
                    // Names a dictionary the header does NOT carry.
                    ("dct".into(), "ghost".into()),
                ]),
            )]),
        ),
    ];
    let hid = header_id(&header);
    header.push(("id".into(), Value::Bytes(hid.clone())));
    let mut bytes = canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(Value::Map(header))));

    let data = purrdf_gts::codec::encode_chain(&rsyncable(), b"undicted payload bytes")
        .expect("plain rsyncable encodes");
    let mut frame: Vec<(Value, Value)> = vec![
        ("t".into(), "blob".into()),
        ("x".into(), Value::Array(vec![Value::Integer(0.into())])),
        ("d".into(), Value::Bytes(data)),
        ("prev".into(), Value::Bytes(hid)),
    ];
    let fid = content_id(&frame);
    frame.push(("id".into(), Value::Bytes(fid)));
    append_canonical(&Value::Map(frame), &mut bytes);

    let graph = read(&bytes, true, None);
    assert!(
        graph.blobs.is_empty(),
        "a frame whose codec names an unresolvable dictionary must NOT fold as a blob"
    );
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|d| d.code == "UnknownCodec" || d.detail.contains("not in catalog")),
        "the unresolvable dictionary reference must be diagnosed, got {:?}",
        graph.diagnostics
    );
    assert!(
        graph
            .opaque
            .iter()
            .any(|node| node.reason == "unknown-codec"),
        "the frame must degrade to an unknown-codec opaque node (fail closed)"
    );
}

/// (f) The same multi-dict pack emitted twice in-process is byte-identical, and
/// the catalog id assignment is a pure function of the SORTED dictionary-name
/// set — not of the caller's row order, and not of any hash iteration order.
#[test]
fn multi_dict_packs_are_byte_deterministic_and_order_independent() {
    let cats = dict_for("cats");
    let dogs = dict_for("dogs");
    let forward = vec![
        ("cats".to_string(), cats.clone()),
        ("dogs".to_string(), dogs.clone()),
    ];
    let reversed = vec![("dogs".to_string(), dogs), ("cats".to_string(), cats)];

    let a = two_dict_pack(forward.clone());
    let b = two_dict_pack(forward);
    assert_eq!(a, b, "two in-process emissions must be byte-identical");

    let c = two_dict_pack(reversed);
    assert_eq!(
        a, c,
        "the catalog id assignment must depend only on the SORTED (codec, dict) set, \
         so the caller's dictionary row order cannot change the bytes"
    );

    // The assignment itself, not merely the bytes: every catalog id is distinct
    // and every (codec, dict) pair is present exactly once. A same-name-collapsing
    // name->id table could not represent this at all.
    let rows = segment_append_state(&a).expect("header parses").catalog;
    let mut keys: Vec<(String, Option<String>)> = rows
        .iter()
        .map(|row| (row.name.clone(), row.dct.clone()))
        .collect();
    let mut ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    keys.sort();
    ids.sort_unstable();
    let mut unique_keys = keys.clone();
    unique_keys.dedup();
    let mut unique_ids = ids.clone();
    unique_ids.dedup();
    assert_eq!(keys, unique_keys, "no (codec, dict) pair may appear twice");
    assert_eq!(ids, unique_ids, "no catalog id may be reused");
    assert!(
        keys.contains(&("zstd-rsyncable".to_string(), Some("cats".to_string())))
            && keys.contains(&("zstd-rsyncable".to_string(), Some("dogs".to_string())))
            && keys.contains(&("zstd-rsyncable".to_string(), None)),
        "the catalog must carry the plain entry AND one per dictionary: {keys:?}"
    );
}

/// The declared `level` is a WIRE FACT: recoverable from the catalog, and the
/// writer refuses a frame that would contradict it.
#[test]
fn the_declared_zstd_level_is_readable_and_binding() {
    let bytes = two_dict_pack(vec![
        ("cats".to_string(), dict_for("cats")),
        ("dogs".to_string(), dict_for("dogs")),
    ]);
    let rows = segment_append_state(&bytes).expect("header parses").catalog;
    for row in rows
        .iter()
        .filter(|row| matches!(row.name.as_str(), "zstd" | "zstd-rsyncable"))
    {
        assert_eq!(
            row.level,
            Some(12),
            "catalog id {} must declare the pack's level",
            row.id
        );
    }
    assert!(
        rows.iter()
            .filter(|row| !matches!(row.name.as_str(), "zstd" | "zstd-rsyncable"))
            .all(|row| row.level.is_none()),
        "a level is meaningless on a non-zstd entry and must not be emitted"
    );

    let mut w = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dicts: vec![("cats".to_string(), dict_for("cats"))],
            zstd_level: Some(12),
            ..WriterOptions::default()
        },
    )
    .expect("writer configures");
    let err = w
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(payload("cats")),
                transform: rsyncable(),
                zstd_level: Some(3),
                dict: Some("cats".to_string()),
                ..FrameOptions::default()
            },
        )
        .expect_err("a frame level contradicting the declared level must hard-fail");
    assert!(err.to_string().contains("declared level"), "{err}");
}
