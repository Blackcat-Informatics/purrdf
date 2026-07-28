// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compaction under a PINNED dictionary (GTS-SPEC §5 header `"dct"`, §8.5
//! `zstd`/`zstd-rsyncable` `dct`, §10.1).
//!
//! [`DictStrategy::Trained`]/[`DictStrategy::RawContent`] derive a dictionary
//! from the pack's own content-blob corpus, so the bytes behind a dictionary
//! NAME are a function of whatever that pack happened to contain. A consumer
//! that ships named dictionaries and pins them in a bundle header needs the
//! opposite guarantee: one id must resolve to ONE byte sequence, everywhere,
//! forever. [`DictStrategy::Pinned`] is that guarantee, and these tests hold it
//! to the wire:
//!
//! - (a) the header `"dct"` map carries the caller's bytes BYTE-FOR-BYTE;
//! - (b) every primed frame names that catalog entry and every zstd frame
//!   header declares the pinned dictionary's finalized `Dictionary_ID`;
//! - (c) a blob-less log (the agent-memory shape) compacts under a wholly
//!   pinned plan, and still refuses under a derived one;
//! - (d) the same input + plan is byte-reproducible;
//! - (e) the pack folds back to the source's content;
//! - (f) MIXED plans (some pinned, some derived) are supported, each entry
//!   obtained on its own terms.

use std::collections::{BTreeMap, BTreeSet};

use ciborium::value::Value;
use ed25519_dalek::SigningKey;
use purrdf_gts::codec::zstd_block_layout;
use purrdf_gts::compact::{
    CompactionParams, DictPlan, DictSelection, DictStrategy, compact_streamable,
};
use purrdf_gts::dict::{dictionary_id, raw_content_dict};
use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_gts::reader::{read, segment_append_state};
use purrdf_gts::wire::{iter_items, map_get};
use purrdf_gts::writer::Writer;

/// The name the caller pins its shipped dictionary under.
const PINNED_NAME: &str = "shipped-bundle-v1";
/// The zstd level the pinned plans declare (§8.5 `level?`).
const LEVEL: i32 = 12;
/// The fixed rewrite timestamp — never ambient time (§14.1).
const TIMESTAMP: &str = "2026-01-01T00:00:00Z";

/// A fixed, deterministic Ed25519 packaging key.
fn packaging_key() -> SigningKey {
    SigningKey::from_bytes(&[11u8; 32])
}

/// The caller's SHIPPED dictionary: derived from a corpus that has nothing to do
/// with any pack compacted below, so "the pack pinned my bytes" cannot pass by
/// accidentally re-deriving the same thing.
fn shipped_dictionary() -> Vec<u8> {
    let corpus: Vec<Vec<u8>> = (0..400u32)
        .map(|i| {
            format!(
                "<https://example.org/slice/logic#c{}> <https://example.org/p/grounds> \
                 \"a shipped-vocabulary sentence unrelated to any packed content, {}\" .\n",
                i % 23,
                i
            )
            .into_bytes()
        })
        .collect();
    let refs: Vec<&[u8]> = corpus.iter().map(Vec::as_slice).collect();
    raw_content_dict(&refs, 8192).expect("the shipped dictionary builds")
}

/// A source log with content blobs of repeated structure.
fn source_with_blobs() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    for i in 0..64u32 {
        let blob = format!(
            "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
            i % 37,
            i
        )
        .into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.add_index();
    w.into_bytes()
}

/// A BLOB-LESS source log: terms and quads only, closed with an `index` footer —
/// the agent-memory shape, and the shape that has no dictionary corpus at all.
fn blobless_source() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    let mut terms: Vec<Term> = Vec::new();
    for i in 0..24u32 {
        terms.push(iri(&format!("https://example.org/memory/claim{i}")));
    }
    terms.push(iri("https://example.org/memory/recalls"));
    terms.push(iri("https://example.org/memory/session"));
    w.add_terms(&terms);
    let predicate = 24;
    let object = 25;
    let quads: Vec<(usize, usize, usize, Option<usize>)> =
        (0..24usize).map(|s| (s, predicate, object, None)).collect();
    w.add_quads(&quads);
    w.add_index();
    w.into_bytes()
}

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

/// A plan pinning exactly the caller's bytes under [`PINNED_NAME`], priming
/// every authored frame through a level-12 `zstd-rsyncable` chain.
fn pinned_plan(dict: Vec<u8>) -> DictPlan {
    DictPlan::rsyncable(PINNED_NAME, DictStrategy::Pinned(dict), LEVEL)
}

fn params(plan: DictPlan) -> CompactionParams<'static> {
    CompactionParams {
        timestamp: TIMESTAMP,
        seal_original: false,
        plan,
        content_digest: None,
        packaging_signer: (packaging_key(), "pack-test".to_string()),
    }
}

/// The header `"dct"` map read straight off the wire as `name -> bytes`, with no
/// digesting, no re-derivation, and no reader convenience in between.
fn header_dct(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let (items, _torn) = iter_items(bytes);
    let (_, first) = items.first().expect("a pack starts with a header item");
    let inner = match first {
        Value::Tag(_, inner) => inner.as_ref(),
        other => other,
    };
    let Value::Map(entries) = inner else {
        panic!("the GTS header is a CBOR map (§5)");
    };
    let Some(Value::Map(dct)) = map_get(entries, "dct") else {
        panic!("the header must pin a \"dct\" map (§5)");
    };
    dct.iter()
        .map(|(key, value)| match (key, value) {
            (Value::Text(name), Value::Bytes(raw)) => (name.clone(), raw.clone()),
            other => panic!("\"dct\" is tstr => bstr (§5), got {other:?}"),
        })
        .collect()
}

/// `catalog id -> dictionary name` for a pack's last segment header.
fn dict_by_catalog_id(bytes: &[u8]) -> BTreeMap<i64, Option<String>> {
    segment_append_state(bytes)
        .expect("the pack header parses")
        .catalog
        .into_iter()
        .map(|row| (row.id, row.dct))
        .collect()
}

/// What one TRANSFORMED frame is observably primed by: the dictionary names its
/// `"x"` chain resolves to, and the `Dictionary_ID` every zstd frame in its
/// payload declares. Sets, so "exactly one dictionary, every block" is a
/// cardinality assertion rather than a hand-rolled scan.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FramePriming {
    /// The `dct` names the frame's catalog references resolve to.
    names: BTreeSet<Option<String>>,
    /// The `Dictionary_ID` declared by each zstd frame header in the payload.
    dictionary_ids: BTreeSet<Option<u32>>,
}

/// One [`FramePriming`] per transformed frame, in file order.
fn primed_frames(bytes: &[u8]) -> Vec<FramePriming> {
    let by_id = dict_by_catalog_id(bytes);
    let (items, _torn) = iter_items(bytes);
    let mut out = Vec::new();
    for (_, item) in items.iter().skip(1) {
        let Value::Map(frame) = item else { continue };
        let Some(Value::Array(ids)) = map_get(frame, "x") else {
            continue;
        };
        let names: BTreeSet<Option<String>> = ids
            .iter()
            .map(|id| {
                let Value::Integer(raw) = id else {
                    panic!("a catalog reference is an integer")
                };
                let id = i64::try_from(i128::from(*raw)).expect("a catalog id fits i64");
                by_id
                    .get(&id)
                    .unwrap_or_else(|| panic!("frame names catalog id {id}, which is not declared"))
                    .clone()
            })
            .collect();
        let Some(Value::Bytes(data)) = map_get(frame, "d") else {
            panic!("a transformed frame carries its payload in \"d\"");
        };
        let dictionary_ids: BTreeSet<Option<u32>> = zstd_block_layout(data)
            .expect("a zstd-family payload is an exact sequence of zstd frames")
            .into_iter()
            .map(|block| block.dictionary_id)
            .collect();
        out.push(FramePriming {
            names,
            dictionary_ids,
        });
    }
    out
}

/// Sorted decoded blob content — an order- and codec-independent identity.
fn decoded_blobs(g: &Graph) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = g
        .blobs
        .iter()
        .map(|(digest, entry)| {
            entry
                .decoded_vec()
                .unwrap_or_else(|err| panic!("blob {digest} decodes: {err}"))
        })
        .collect();
    out.sort();
    out
}

/// Every quad as a resolved `(subject, predicate, object)` value triple, so two
/// folds can be compared without depending on term-id assignment.
fn quad_values(g: &Graph) -> BTreeSet<(String, String, String)> {
    let value = |tid: usize| -> String {
        g.terms
            .get(tid)
            .and_then(|term| term.value.clone())
            .unwrap_or_else(|| panic!("term {tid} has a value"))
    };
    g.quads
        .iter()
        .map(|&(s, p, o, _)| (value(s), value(p), value(o)))
        .collect()
}

// ---------------------------------------------------------------------------
// (a) The caller's bytes, verbatim, under the caller's name.
// ---------------------------------------------------------------------------

#[test]
fn a_pinned_plan_carries_the_callers_exact_dictionary_bytes_in_the_header() {
    let shipped = shipped_dictionary();
    let pack = compact_streamable(&source_with_blobs(), params(pinned_plan(shipped.clone())))
        .expect("a pinned-dictionary compaction succeeds");

    let dct = header_dct(&pack);
    assert_eq!(
        dct.keys().collect::<Vec<_>>(),
        vec![PINNED_NAME],
        "the pack pins exactly the dictionary the plan named"
    );
    let on_the_wire = &dct[PINNED_NAME];
    assert_eq!(
        on_the_wire, &shipped,
        "the header \"dct\" entry must be the caller's dictionary BYTE-FOR-BYTE: an id that \
         resolves to re-derived bytes does not identify a decodable dictionary"
    );

    // Anti-tautology: a derived plan over this same source pins something else
    // entirely, so byte-equality above is a real property of pinning.
    let derived = compact_streamable(
        &source_with_blobs(),
        params(DictPlan::rsyncable(
            PINNED_NAME,
            DictStrategy::RawContent,
            LEVEL,
        )),
    )
    .expect("a derived compaction succeeds over a blob-carrying source");
    assert_ne!(
        header_dct(&derived)[PINNED_NAME],
        shipped,
        "the derived strategy must genuinely produce different bytes under the same name"
    );
}

// ---------------------------------------------------------------------------
// (b) Every primed frame names the pinned entry, and declares its id on the wire.
// ---------------------------------------------------------------------------

#[test]
fn every_primed_frame_names_the_pinned_catalog_entry_and_declares_its_dictionary_id() {
    let shipped = shipped_dictionary();
    let expected_id = dictionary_id(&shipped).expect("a finalized dictionary carries an id");
    let pack = compact_streamable(&source_with_blobs(), params(pinned_plan(shipped)))
        .expect("a pinned-dictionary compaction succeeds");

    let frames = primed_frames(&pack);
    assert!(
        frames.len() > 1,
        "the pack must carry several transformed frames (index + content + blobs)"
    );
    for frame in &frames {
        assert_eq!(
            frame.names,
            BTreeSet::from([Some(PINNED_NAME.to_string())]),
            "every transformed frame's chain must resolve to the pinned dictionary's \
             catalog entry"
        );
        assert_eq!(
            frame.dictionary_ids,
            BTreeSet::from([Some(expected_id)]),
            "every zstd frame header must declare the PINNED dictionary's finalized \
             Dictionary_ID ({expected_id}) — the id must identify these exact bytes"
        );
    }

    // The catalog binds the pinned name, and declares the plan's level.
    let rows = segment_append_state(&pack)
        .expect("the pack header parses")
        .catalog;
    assert!(
        rows.iter()
            .any(|row| row.dct.as_deref() == Some(PINNED_NAME)),
        "the catalog must carry an entry bound to the pinned dictionary"
    );
    for row in rows
        .iter()
        .filter(|row| matches!(row.name.as_str(), "zstd" | "zstd-rsyncable"))
    {
        assert_eq!(
            row.level,
            Some(LEVEL),
            "catalog id {} must declare the plan's level (§8.5 level?)",
            row.id
        );
    }
}

// ---------------------------------------------------------------------------
// (c) A blob-less log compacts under a pinned plan — and still refuses a derived one.
// ---------------------------------------------------------------------------

#[test]
fn a_blobless_log_compacts_under_a_wholly_pinned_plan() {
    let source = blobless_source();
    let folded_source = read(&source, true, None);
    assert!(
        folded_source.blobs.is_empty(),
        "the fixture must genuinely have NO content blobs"
    );

    let shipped = shipped_dictionary();
    let pack = compact_streamable(&source, params(pinned_plan(shipped.clone())))
        .expect("a wholly-pinned plan needs no content-blob corpus");

    assert_eq!(
        header_dct(&pack)[PINNED_NAME],
        shipped,
        "the blob-less pack pins the caller's exact bytes"
    );
    let folded = read(&pack, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the blob-less pack must fold cleanly (the pinned dictionary resolves): {:?}",
        folded.diagnostics
    );
    assert!(
        quad_values(&folded).is_superset(&quad_values(&folded_source)),
        "every source quad survives compaction of a blob-less log"
    );
    // The dictionary is not dead weight: the terms/quads frames are primed by it.
    let frames = primed_frames(&pack);
    assert!(!frames.is_empty(), "the pack must carry primed frames");
    for frame in &frames {
        assert_eq!(frame.names, BTreeSet::from([Some(PINNED_NAME.to_string())]));
    }
}

#[test]
fn a_blobless_log_still_refuses_a_derived_plan_with_the_named_refusal() {
    let source = blobless_source();
    for strategy in [DictStrategy::Trained, DictStrategy::RawContent] {
        let err = compact_streamable(
            &source,
            params(DictPlan::rsyncable(PINNED_NAME, strategy.clone(), LEVEL)),
        )
        .expect_err("a derived plan has no corpus on a blob-less log");
        assert!(
            err.to_string().contains("no content blobs"),
            "the refusal must still name the missing corpus for {strategy:?}: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// (d) Determinism — the format's foundational invariant (§14.1).
// ---------------------------------------------------------------------------

#[test]
fn the_same_input_and_pinned_plan_compact_to_byte_identical_packs() {
    let shipped = shipped_dictionary();
    let source = source_with_blobs();
    let a = compact_streamable(&source, params(pinned_plan(shipped.clone())))
        .expect("first compaction succeeds");
    let b = compact_streamable(&source, params(pinned_plan(shipped.clone())))
        .expect("second compaction succeeds");
    assert_eq!(a, b, "a pinned compaction must be byte-reproducible");

    // Blob-less too: the path that skips the corpus entirely is equally frozen.
    let memory = blobless_source();
    let c = compact_streamable(&memory, params(pinned_plan(shipped.clone())))
        .expect("first blob-less compaction succeeds");
    let d = compact_streamable(&memory, params(pinned_plan(shipped)))
        .expect("second blob-less compaction succeeds");
    assert_eq!(
        c, d,
        "a blob-less pinned compaction must be byte-reproducible"
    );
}

// ---------------------------------------------------------------------------
// (e) Round trip — the dictionary is a compression detail, invisible to the fold.
// ---------------------------------------------------------------------------

#[test]
fn a_pinned_pack_folds_to_the_same_content_as_its_source() {
    let source = source_with_blobs();
    let pack = compact_streamable(&source, params(pinned_plan(shipped_dictionary())))
        .expect("a pinned-dictionary compaction succeeds");

    let before = read(&source, true, None);
    let after = read(&pack, true, None);
    assert!(
        after.diagnostics.is_empty(),
        "the pinned pack must fold cleanly: {:?}",
        after.diagnostics
    );
    assert_eq!(
        decoded_blobs(&after),
        decoded_blobs(&before),
        "every blob must decode against the pinned dictionary to its ORIGINAL bytes"
    );
    assert!(
        quad_values(&after).is_superset(&quad_values(&before)),
        "every source quad survives the rewrite"
    );

    // The same content also folds out of an UNDICTED pack: the pinned
    // dictionary changes the bytes on disk and nothing about the graph.
    let undicted = compact_streamable(&source, params(DictPlan::undicted()))
        .expect("an undicted compaction succeeds");
    assert_ne!(pack, undicted, "pinning a dictionary changes the bytes");
    assert_eq!(
        decoded_blobs(&read(&undicted, true, None)),
        decoded_blobs(&after),
        "the pinned dictionary is invisible to the fold"
    );
}

// ---------------------------------------------------------------------------
// (f) MIXED plans: pinned and derived entries in one pack, each on its own terms.
// ---------------------------------------------------------------------------

/// The RULING: a mixed plan is SUPPORTED, not refused. Pinning and deriving are
/// per-entry questions — a pinned entry's bytes exist independently of the pack,
/// a derived entry's are a function of its corpus — and nothing about one
/// constrains the other. Refusing the combination would forbid the natural
/// migration shape (ship a stable dictionary for the content frames, derive a
/// pack-local one for the index frames) for no invariant's sake.
#[test]
fn a_mixed_plan_pins_the_callers_bytes_and_derives_the_rest() {
    let shipped = shipped_dictionary();
    let source = source_with_blobs();
    let mixed = DictPlan {
        dicts: vec![
            (
                PINNED_NAME.to_string(),
                DictStrategy::Pinned(shipped.clone()),
            ),
            ("pack-local".to_string(), DictStrategy::RawContent),
        ],
        content: DictSelection::Named(PINNED_NAME.to_string()),
        index: DictSelection::Named("pack-local".to_string()),
        transform: vec!["zstd-rsyncable".to_string()],
        zstd_level: Some(LEVEL),
    };
    let pack = compact_streamable(&source, params(mixed)).expect("a mixed plan compacts");

    let dct = header_dct(&pack);
    // The header `"dct"` map is ordered by NAME, never by plan order — that is
    // what keeps the emitted bytes a function of the dictionary SET rather than
    // of how the caller happened to sequence the plan. Sort the expectation
    // rather than hard-coding one arrangement, so renaming a fixture dictionary
    // cannot quietly turn this into an assertion about alphabetical luck.
    let mut expected_names = vec![PINNED_NAME, "pack-local"];
    expected_names.sort_unstable();
    assert_eq!(
        dct.keys().collect::<Vec<_>>(),
        expected_names,
        "both dictionaries must be pinned in the header, ordered by name"
    );
    assert_eq!(
        dct[PINNED_NAME], shipped,
        "the pinned entry is the caller's bytes, verbatim, even alongside a derived one"
    );

    // The derived entry is EXACTLY what a wholly-derived plan produces from the
    // same corpus — the pinned neighbour perturbs nothing.
    let derived_only = compact_streamable(
        &source,
        params(DictPlan::rsyncable(
            "pack-local",
            DictStrategy::RawContent,
            LEVEL,
        )),
    )
    .expect("the derived-only plan compacts");
    assert_eq!(
        dct["pack-local"],
        header_dct(&derived_only)["pack-local"],
        "a derived entry in a mixed plan must equal the same entry derived alone"
    );
    assert_ne!(
        dct["pack-local"], shipped,
        "anti-tautology: the two dictionaries genuinely differ"
    );

    // Both are live: the blob frames ride the pinned one, the index frames the
    // derived one, and each declares its own dictionary's id on the wire.
    let pinned_id = dictionary_id(&shipped).expect("pinned dictionary id");
    let derived_id = dictionary_id(&dct["pack-local"]).expect("derived dictionary id");
    assert_ne!(pinned_id, derived_id);
    let used: BTreeSet<(Option<String>, Option<u32>)> = primed_frames(&pack)
        .into_iter()
        .map(|frame| {
            assert_eq!(frame.names.len(), 1, "one dictionary per frame");
            assert_eq!(
                frame.dictionary_ids.len(),
                1,
                "one dictionary per frame, every block"
            );
            (
                frame.names.into_iter().next().expect("one name"),
                frame.dictionary_ids.into_iter().next().expect("one id"),
            )
        })
        .collect();
    assert_eq!(
        used,
        BTreeSet::from([
            (Some(PINNED_NAME.to_string()), Some(pinned_id)),
            (Some("pack-local".to_string()), Some(derived_id)),
        ]),
        "each frame group must ride the dictionary the plan selected for it"
    );

    let folded = read(&pack, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "a mixed-dictionary pack must fold cleanly: {:?}",
        folded.diagnostics
    );
    assert_eq!(
        decoded_blobs(&folded),
        decoded_blobs(&read(&source, true, None)),
        "every blob decodes against the dictionary it was primed with"
    );
}

/// The emitted bytes are a function of the dictionary SET, not of the order the
/// caller listed it in. Catalog ids are assigned over the SORTED (codec,
/// dict-name) set and the header `"dct"` map is name-ordered, so sequencing the
/// same two dictionaries the other way round must reproduce the pack byte for
/// byte. Without this, "compaction is deterministic" would hold only for callers
/// who happen to build their plan in the same order twice.
#[test]
fn the_order_dictionaries_appear_in_a_plan_does_not_change_the_bytes() {
    let shipped = shipped_dictionary();
    let source = source_with_blobs();
    let plan = |reversed: bool| {
        let pinned = (
            PINNED_NAME.to_string(),
            DictStrategy::Pinned(shipped.clone()),
        );
        let derived = ("pack-local".to_string(), DictStrategy::RawContent);
        DictPlan {
            dicts: if reversed {
                vec![derived, pinned]
            } else {
                vec![pinned, derived]
            },
            content: DictSelection::Named(PINNED_NAME.to_string()),
            index: DictSelection::Named("pack-local".to_string()),
            transform: vec!["zstd-rsyncable".to_string()],
            zstd_level: Some(LEVEL),
        }
    };

    let forward = compact_streamable(&source, params(plan(false))).expect("forward plan compacts");
    let reversed = compact_streamable(&source, params(plan(true))).expect("reversed plan compacts");
    assert_eq!(
        forward, reversed,
        "the same dictionary set listed in either order must compact to identical bytes"
    );
}

/// A mixed plan's DERIVED half still needs a corpus, so a blob-less log refuses
/// it — the refusal is a property of derivation, never of pinning.
#[test]
fn a_mixed_plan_over_a_blobless_log_refuses_for_its_derived_entry() {
    let mixed = DictPlan {
        dicts: vec![
            (
                PINNED_NAME.to_string(),
                DictStrategy::Pinned(shipped_dictionary()),
            ),
            ("pack-local".to_string(), DictStrategy::RawContent),
        ],
        content: DictSelection::Named(PINNED_NAME.to_string()),
        index: DictSelection::Named("pack-local".to_string()),
        transform: vec!["zstd-rsyncable".to_string()],
        zstd_level: Some(LEVEL),
    };
    let err = compact_streamable(&blobless_source(), params(mixed))
        .expect_err("the derived half of a mixed plan has no corpus on a blob-less log");
    assert!(
        err.to_string().contains("no content blobs"),
        "the refusal must name the missing corpus: {err}"
    );
}

// ---------------------------------------------------------------------------
// Refuse-don't-trust on the one input the compactor does not produce itself.
// ---------------------------------------------------------------------------

#[test]
fn pinned_bytes_that_are_not_a_finalized_dictionary_are_refused() {
    for (label, bytes) in [
        ("empty", Vec::new()),
        (
            "not a dictionary",
            b"just some bytes, not a zstd dict".to_vec(),
        ),
        ("raw content only", vec![b'a'; 4096]),
    ] {
        let err = compact_streamable(&source_with_blobs(), params(pinned_plan(bytes)))
            .expect_err("unusable pinned bytes must hard-fail");
        let text = err.to_string();
        assert!(
            text.contains(PINNED_NAME) && text.contains("finalized zstd dictionary"),
            "the refusal must name the offending dictionary for the {label} case: {text}"
        );
    }
}
