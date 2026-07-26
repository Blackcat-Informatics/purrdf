// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drift-guard tests for the frozen in-band-dictionary corpus vectors:
//! `vectors/30-dict-rawcontent.gts` (raw-content dictionary, plain `zstd`),
//! `vectors/31-dict-trained.gts` (FastCOVER-trained dictionary, plain `zstd`),
//! `vectors/32-dict-rsyncable.gts` (raw-content dictionary priming a
//! `zstd-rsyncable` chain at level 12 — GMEOW's mandated frame profile), and
//! `vectors/33-multi-dict.gts` (TWO named in-band dictionaries in ONE pack with
//! per-frame selection).
//!
//! Every fixed source and authoring recipe lives in
//! `purrdf_rdf::gts_dict_vectors`, shared with the freezing binary
//! `crates/rdf/src/bin/gen_dict_vectors.rs`, so a fresh regeneration here always
//! starts from the SAME bytes the frozen vectors were authored from.
//!
//! 30/32/33 are byte oracles; 31 is fold-equality evidence only, because
//! FastCOVER's scoring involves transcendental floating point and is therefore
//! deterministic on the authoring platform but not guaranteed byte-identical
//! cross-platform (see `crates/gts/src/dict.rs`).
//!
//! Every vector carries a `<id>.expected.json` fold oracle in the shared
//! one-space-indented, sorted-key GTS corpus format. The generator and this
//! drift guard use the same production GTS-to-dataset projection and N-Quads
//! serializer, while this test always derives the comparison from the frozen
//! `.gts` bytes themselves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ciborium::value::Value;
use purrdf_gts::compact::{DEFAULT_DICT_NAME, DictPlan, DictStrategy};
use purrdf_gts::model::Graph;
use purrdf_gts::reader::read;
use purrdf_gts::wire::{iter_items, map_get};
use purrdf_rdf::gts_certify::{compact_and_certify, refold_digest, verify_compaction};
use purrdf_rdf::gts_dict_vectors::{
    MULTI_DICT_NAMES, TIMESTAMP, VECTOR_ZSTD_LEVEL, authorship_key, expected_fold_json,
    fixed_source, multi_dict_pack, packaging_key, render_expected_json, rsyncable_plan,
    size_comparison_source,
};

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors")
}

fn read_vector(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|err| panic!("read {name}: {err}"))
}

fn read_expected(name: &str) -> String {
    std::fs::read_to_string(vectors_dir().join(name))
        .unwrap_or_else(|err| panic!("read {name}: {err}"))
}

fn keyring() -> HashMap<String, ed25519_dalek::VerifyingKey> {
    HashMap::from([
        ("authorA".to_string(), authorship_key().verifying_key()),
        ("pack".to_string(), packaging_key().verifying_key()),
    ])
}

#[test]
fn every_dictionary_vector_expected_json_matches_its_frozen_fold() {
    for stem in [
        "30-dict-rawcontent",
        "31-dict-trained",
        "32-dict-rsyncable",
        "33-multi-dict",
    ] {
        let vector = read_vector(&format!("{stem}.gts"));
        let expected = read_expected(&format!("{stem}.expected.json"));
        assert_eq!(
            render_expected_json(&expected_fold_json(&vector)),
            expected,
            "{stem}.expected.json must be the deterministic fold oracle for the frozen bytes"
        );
    }
}

/// Sorted `(digest, decoded bytes)` for every blob in `g` — an order- and
/// codec-independent content identity, unaffected by which in-band
/// dictionary compressed the frames.
fn decoded_blobs(g: &Graph) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = g
        .blobs
        .iter()
        .map(|(digest, entry)| {
            (
                digest.clone(),
                entry
                    .decoded_vec()
                    .unwrap_or_else(|err| panic!("blob {digest} decodes: {err}")),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Whether the file's header item (the first CBOR item, §3.1) carries a
/// non-empty `"dct"` map (§5) — the functional signal that a pack dictionary
/// was actually pinned in-band, not merely that some codec ran.
fn header_carries_dct_entry(bytes: &[u8]) -> bool {
    let (items, _torn) = iter_items(bytes);
    let Some((_, first)) = items.first() else {
        return false;
    };
    let inner = match first {
        Value::Tag(_, inner) => inner.as_ref(),
        other => other,
    };
    let Value::Map(entries) = inner else {
        return false;
    };
    matches!(map_get(entries, "dct"), Some(Value::Map(dct)) if !dct.is_empty())
}

/// The 4-byte zstd frame magic number (`28 B5 2F FD`, little-endian
/// `0xFD2FB528`) — the same byte pattern `xxd -p … | grep -oc 28b52ffd`
/// counts. Counting occurrences in the raw file bytes is a blunt but
/// falsifiable proxy for "at least one frame is actually zstd-compressed":
/// an inert pinned dictionary with every blob stored `identity` would count
/// zero, exactly the bug this drift guard exists to catch.
const ZSTD_FRAME_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Count non-overlapping-position occurrences of the zstd frame magic number
/// in `bytes`.
fn count_zstd_frame_magic(bytes: &[u8]) -> usize {
    bytes
        .windows(ZSTD_FRAME_MAGIC.len())
        .filter(|window| *window == ZSTD_FRAME_MAGIC)
        .count()
}

#[test]
fn dict_primed_vectors_are_actually_zstd_compressed_not_an_inert_header() {
    let rawcontent = read_vector("30-dict-rawcontent.gts");
    let trained = read_vector("31-dict-trained.gts");

    // Every one of the 40 content blobs must be zstd-transformed against the
    // pinned pack dictionary: a pinned `dct` header entry with zero zstd
    // frames means the dictionary is dead weight, which is exactly the bug
    // this test must fail on.
    let rawcontent_frames = count_zstd_frame_magic(&rawcontent);
    let trained_frames = count_zstd_frame_magic(&trained);
    assert!(
        rawcontent_frames > 0,
        "30-dict-rawcontent.gts must contain at least one zstd frame magic number \
         (28b52ffd); found {rawcontent_frames} — the pinned dictionary is unused"
    );
    assert!(
        trained_frames > 0,
        "31-dict-trained.gts must contain at least one zstd frame magic number \
         (28b52ffd); found {trained_frames} — the pinned dictionary is unused"
    );
}

#[test]
fn dict_primed_packs_are_strictly_smaller_than_the_undicted_pack() {
    let source = size_comparison_source();

    let (rawcontent, _cert) = compact_and_certify(
        &source,
        DictPlan::single(DictStrategy::RawContent),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("raw-content dict compaction succeeds over the fixed source");
    let (trained, _cert) = compact_and_certify(
        &source,
        DictPlan::single(DictStrategy::Trained),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("trained dict compaction succeeds over the fixed source");
    let (undicted, _cert) = compact_and_certify(
        &source,
        DictPlan::undicted(),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("undicted compaction succeeds over the fixed source");

    // A dict-primed pack pays the dictionary bytes up front (stored
    // uncompressed and in-band, §5) but must recoup that cost by actually
    // compressing the content-blob corpus against it — over a 40-blob
    // repeated-structure corpus the net must land strictly smaller than the
    // same source with no dictionary and no compression at all. This is the
    // falsifiable half of the "trained-dictionary-compressed" claim: an inert
    // dictionary (frames still stored `identity`) would only ever be larger.
    assert!(
        rawcontent.len() < undicted.len(),
        "raw-content dict-primed pack ({} bytes) must be strictly smaller than the \
         undicted pack ({} bytes)",
        rawcontent.len(),
        undicted.len()
    );
    assert!(
        trained.len() < undicted.len(),
        "trained dict-primed pack ({} bytes) must be strictly smaller than the \
         undicted pack ({} bytes)",
        trained.len(),
        undicted.len()
    );
}

#[test]
fn rawcontent_vector_is_byte_identical_to_a_fresh_regeneration() {
    let frozen = read_vector("30-dict-rawcontent.gts");
    let source = fixed_source();

    let (regenerated, _cert) = compact_and_certify(
        &source,
        DictPlan::single(DictStrategy::RawContent),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("raw-content dict compaction succeeds over the fixed source");

    assert_eq!(
        regenerated, frozen,
        "the raw-content dict producer has no platform-dependent floating point, so a \
         fresh regeneration from the SAME fixed source must be byte-identical to the \
         frozen vector"
    );

    let folded = read(&frozen, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the frozen raw-content vector must fold cleanly: {:?}",
        folded.diagnostics
    );
    assert_eq!(
        folded.blobs.len(),
        40,
        "every content blob survives compaction"
    );

    let ring = keyring();
    let report = verify_compaction(&source, &frozen, &ring).expect("verify_compaction succeeds");
    assert!(
        report.all_ok(),
        "the frozen raw-content vector must independently verify (incl. the carried \
         stream:detachedSignatureRoot over the signed source): {report:?}"
    );
}

#[test]
fn trained_vector_folds_cleanly_decodes_and_carries_a_dct_entry() {
    let frozen = read_vector("31-dict-trained.gts");
    let source = fixed_source();

    let folded = read(&frozen, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the frozen trained-dict vector must fold cleanly: {:?}",
        folded.diagnostics
    );
    assert_eq!(
        folded.blobs.len(),
        40,
        "every content blob survives compaction"
    );
    for (digest, entry) in &folded.blobs {
        entry.decoded_vec().unwrap_or_else(|err| {
            panic!("blob {digest} decodes against the pinned in-band trained dictionary: {err}")
        });
    }
    assert!(
        header_carries_dct_entry(&frozen),
        "the trained-dict vector's header must pin a named, non-empty \"dct\" entry (§5)"
    );

    let ring = keyring();
    let report = verify_compaction(&source, &frozen, &ring).expect("verify_compaction succeeds");
    assert!(
        report.all_ok(),
        "the frozen trained-dict vector must independently verify (incl. the carried \
         stream:detachedSignatureRoot over the signed source): {report:?}"
    );
}

#[test]
fn trained_vector_folds_identically_to_a_fresh_regeneration() {
    let frozen = read_vector("31-dict-trained.gts");
    let source = fixed_source();

    let (regenerated, _cert) = compact_and_certify(
        &source,
        DictPlan::single(DictStrategy::Trained),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("trained dict compaction succeeds over the fixed source");

    let frozen_fold = read(&frozen, true, None);
    let regenerated_fold = read(&regenerated, true, None);
    assert!(
        frozen_fold.diagnostics.is_empty(),
        "frozen vector folds cleanly"
    );
    assert!(
        regenerated_fold.diagnostics.is_empty(),
        "freshly regenerated pack folds cleanly"
    );

    // FastCOVER's dict bytes — and therefore the header/pack bytes — are
    // deliberately NOT asserted byte-equal here (cross-platform FP; see the
    // module docs). The FOLD is asserted identical instead: the same decoded
    // blob content and the same RDFC-1.0 content-refold digest, regardless of
    // which dictionary bytes compressed the frames on this platform.
    assert_eq!(
        decoded_blobs(&frozen_fold),
        decoded_blobs(&regenerated_fold),
        "a fresh trained-dict regeneration must decode to the SAME blob content as the \
         frozen vector, even if the trained dictionary bytes differ cross-platform"
    );
    assert_eq!(
        refold_digest(&frozen_fold).expect("frozen content-refold digest"),
        refold_digest(&regenerated_fold).expect("regenerated content-refold digest"),
        "fold-equality: the RDFC-1.0 content-refold digest must agree"
    );

    // Anti-tautology: the trained and raw-content strategies genuinely pin
    // different bytes over the SAME source — this is not a vacuous
    // byte-equality check against a strategy that ignores dict choice.
    let (raw_regenerated, _raw_cert) = compact_and_certify(
        &source,
        DictPlan::single(DictStrategy::RawContent),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("raw-content dict compaction succeeds");
    assert_ne!(
        regenerated, raw_regenerated,
        "sanity: trained vs raw-content dict strategies pin different bytes"
    );
}

// ---------------------------------------------------------------------------
// vectors/32-dict-rsyncable.gts — one dictionary priming a `zstd-rsyncable`
// chain at level 12 (GMEOW's mandated frame profile).
// ---------------------------------------------------------------------------

/// The catalog rows of a pack's LAST segment header, as they appear on the wire.
fn catalog_rows(bytes: &[u8]) -> Vec<purrdf_gts::reader::CatalogRow> {
    purrdf_gts::reader::segment_append_state(bytes)
        .expect("a frozen vector's header parses")
        .catalog
}

#[test]
fn rsyncable_vector_is_byte_identical_to_a_fresh_regeneration() {
    let frozen = read_vector("32-dict-rsyncable.gts");
    let source = fixed_source();

    let (regenerated, _cert) = compact_and_certify(
        &source,
        rsyncable_plan(),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("rsyncable dict compaction succeeds over the fixed source");
    assert_eq!(
        regenerated, frozen,
        "the raw-content dict producer has no platform-dependent floating point, so a \
         fresh regeneration from the SAME fixed source must be byte-identical to the \
         frozen vector"
    );

    let folded = read(&frozen, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the frozen rsyncable vector must fold cleanly: {:?}",
        folded.diagnostics
    );
    assert_eq!(folded.blobs.len(), 40, "every content blob survives");
    for (digest, entry) in &folded.blobs {
        entry.decoded_vec().unwrap_or_else(|err| {
            panic!("blob {digest} decodes against the pinned in-band dictionary: {err}")
        });
    }
    assert!(
        header_carries_dct_entry(&frozen),
        "the rsyncable vector's header must pin a named, non-empty \"dct\" entry (§5)"
    );

    let report =
        verify_compaction(&source, &frozen, &keyring()).expect("verify_compaction succeeds");
    assert!(
        report.all_ok(),
        "the frozen rsyncable vector must independently verify: {report:?}"
    );
}

/// The whole point of the profile: EVERY zstd-family catalog entry declares the
/// mandated level, and every entry that names a dictionary is `zstd-rsyncable`
/// (never plain `zstd`) — so a downstream "enforce rsyncable at level 12" gate
/// has something on the wire to enforce against.
#[test]
fn rsyncable_vector_declares_its_level_and_rsyncable_chain_on_the_wire() {
    let frozen = read_vector("32-dict-rsyncable.gts");
    let rows = catalog_rows(&frozen);

    assert!(
        rows.iter().any(|row| row.dct.is_some()),
        "a dict-primed pack must carry at least one dict-bound catalog entry"
    );
    for row in rows
        .iter()
        .filter(|row| matches!(row.name.as_str(), "zstd" | "zstd-rsyncable"))
    {
        assert_eq!(
            row.level,
            Some(VECTOR_ZSTD_LEVEL),
            "catalog id {} must declare the pack's zstd level on the wire (§8.5 level?)",
            row.id
        );
    }

    // Every payload frame rides EXACTLY ONE transform, and that transform is
    // the dictionary-primed `zstd-rsyncable` entry — GMEOW's mandated profile
    // (one transform per payload frame, rsyncable, dict-primed).
    let chains = frame_codec_chains(&frozen);
    assert!(!chains.is_empty(), "the pack must carry transformed frames");
    for chain in &chains {
        assert_eq!(
            chain.len(),
            1,
            "the mandated profile is EXACTLY one transform per payload frame, got {chain:?}"
        );
        assert_eq!(chain[0].0, "zstd-rsyncable", "chain must be rsyncable");
        assert_eq!(
            chain[0].1.as_deref(),
            Some(DEFAULT_DICT_NAME),
            "every payload frame must be primed by the pinned dictionary"
        );
    }
}

/// `compact_streamable` authors EVERY payload frame through the plan's chain.
///
/// Before, only the content blobs were transformed (and only under a hard-coded
/// plain `zstd`): the streaming-index and content-graph `terms`/`quads` frames
/// went out as a bare `"d"` payload with no `"x"` at all. That was a silent
/// split profile — the pack declared one frame profile and shipped two — and it
/// is invisible to a chain-shape check that only inspects frames which HAVE an
/// `"x"`. So this asserts from the other side: the only frames without a
/// transform are the ones deliberately excluded.
#[test]
fn the_only_untransformed_frames_in_a_compacted_pack_are_the_deliberate_exclusions() {
    let frozen = read_vector("32-dict-rsyncable.gts");
    let (items, _torn) = iter_items(&frozen);

    let mut untransformed: Vec<String> = Vec::new();
    let mut transformed = 0usize;
    for (_, item) in items.iter().skip(1) {
        let Value::Map(frame) = item else { continue };
        let Some(Value::Text(kind)) = map_get(frame, "t") else {
            continue;
        };
        if map_get(frame, "x").is_some() {
            transformed += 1;
        } else {
            untransformed.push(kind.clone());
        }
    }

    assert!(transformed > 0, "the pack must carry transformed frames");
    assert_eq!(
        untransformed,
        vec!["index".to_string()],
        "only the `index` FOOTER may ride untransformed — it is §6.2's seek table, \
         which a streaming reader consults to find every other frame. Any `terms`, \
         `quads`, `reifies`, `annot`, `suppress`, or content `blob` frame appearing \
         here means the authored payload silently skipped the plan's transform chain."
    );
}

/// Each transformed frame's resolved codec chain as `(name, dict-name)` rows.
fn frame_codec_chains(bytes: &[u8]) -> Vec<Vec<(String, Option<String>)>> {
    let by_id: HashMap<i64, (String, Option<String>)> = catalog_rows(bytes)
        .into_iter()
        .map(|row| (row.id, (row.name, row.dct)))
        .collect();
    let (items, _torn) = iter_items(bytes);
    let mut out = Vec::new();
    for (_, item) in items.iter().skip(1) {
        let Value::Map(frame) = item else { continue };
        let Some(Value::Array(ids)) = map_get(frame, "x") else {
            continue;
        };
        out.push(
            ids.iter()
                .map(|id| {
                    let Value::Integer(raw) = id else {
                        panic!("a catalog reference must be an integer")
                    };
                    let id = i64::try_from(i128::from(*raw)).expect("catalog id fits i64");
                    by_id.get(&id).expect("frame names a catalog id").clone()
                })
                .collect(),
        );
    }
    out
}

// ---------------------------------------------------------------------------
// vectors/33-multi-dict.gts — TWO named dictionaries in ONE pack.
// ---------------------------------------------------------------------------

/// The header `"dct"` map of a pack, name → bytes.
fn header_dicts(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    purrdf_gts::reader::segment_append_state(bytes)
        .expect("a frozen vector's header parses")
        .dicts
}

#[test]
fn multi_dict_vector_is_byte_identical_to_a_fresh_regeneration() {
    let frozen = read_vector("33-multi-dict.gts");
    assert_eq!(
        multi_dict_pack(),
        frozen,
        "the multi-dict vector uses only raw-content dictionaries, so a fresh \
         regeneration must be byte-identical"
    );

    let folded = read(&frozen, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the frozen multi-dict vector must fold cleanly: {:?}",
        folded.diagnostics
    );
    for (digest, entry) in &folded.blobs {
        entry.decoded_vec().unwrap_or_else(|err| {
            panic!("blob {digest} decodes against the dictionary it was primed with: {err}")
        });
    }
}

#[test]
fn multi_dict_vector_pins_two_distinct_dictionaries_and_selects_per_frame() {
    let frozen = read_vector("33-multi-dict.gts");
    let dicts = header_dicts(&frozen);
    assert_eq!(
        dicts.len(),
        MULTI_DICT_NAMES.len(),
        "the pack must pin exactly the declared dictionaries (§5 \"dct\" is a map)"
    );
    for name in MULTI_DICT_NAMES {
        assert!(
            dicts.contains_key(name),
            "dictionary {name:?} must be pinned"
        );
    }
    let bytes: Vec<&Vec<u8>> = dicts.values().collect();
    assert_ne!(
        bytes[0], bytes[1],
        "anti-tautology: the two pinned dictionaries must genuinely differ"
    );

    // The catalog must carry one dict-bound entry per (zstd-family codec,
    // dictionary) pair PLUS the plain entries — this is precisely the shape the
    // old `HashMap<String, i64>` name→id table could not represent.
    let rows = catalog_rows(&frozen);
    let mut bound: Vec<(String, String)> = rows
        .iter()
        .filter_map(|row| row.dct.clone().map(|dct| (row.name.clone(), dct)))
        .collect();
    bound.sort();
    assert_eq!(
        bound,
        vec![
            ("zstd".to_string(), "cats".to_string()),
            ("zstd".to_string(), "dogs".to_string()),
            ("zstd-rsyncable".to_string(), "cats".to_string()),
            ("zstd-rsyncable".to_string(), "dogs".to_string()),
        ],
        "the catalog must bind every zstd-family codec to every pinned dictionary"
    );
    // Catalog ids are unique — a collapsing name→id map would have lost rows.
    let mut ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    ids.sort_unstable();
    let unique = {
        let mut u = ids.clone();
        u.dedup();
        u
    };
    assert_eq!(ids, unique, "every catalog id must be distinct");

    // Both dictionaries are actually USED: the frames' `"x"` chains name
    // dict-bound ids from both groups.
    let used = used_dict_names(&frozen);
    for name in MULTI_DICT_NAMES {
        assert!(
            used.contains(name),
            "no frame is primed by dictionary {name:?} — the pin would be dead weight"
        );
    }
}

/// The set of dictionary names actually referenced by frames' `"x"` chains.
fn used_dict_names(bytes: &[u8]) -> std::collections::BTreeSet<String> {
    let rows = catalog_rows(bytes);
    let by_id: HashMap<i64, Option<String>> =
        rows.into_iter().map(|row| (row.id, row.dct)).collect();
    let (items, _torn) = iter_items(bytes);
    let mut out = std::collections::BTreeSet::new();
    for (_, item) in items.iter().skip(1) {
        let Value::Map(frame) = item else { continue };
        let Some(Value::Array(ids)) = map_get(frame, "x") else {
            continue;
        };
        for id in ids {
            let Value::Integer(raw) = id else { continue };
            let id = i64::try_from(i128::from(*raw)).expect("catalog id fits i64");
            if let Some(Some(name)) = by_id.get(&id) {
                out.insert(name.clone());
            }
        }
    }
    out
}
