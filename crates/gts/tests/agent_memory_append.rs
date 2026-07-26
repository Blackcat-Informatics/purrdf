// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A memory store is ONE append-only segment, not a pile of packs.
//!
//! Every `store`/`revise`/`record_tool_call` used to construct its own
//! `Writer::new(PROFILE)`, so an N-claim file was a concatenation of N
//! self-contained packs — N full GTS headers for N few-hundred-byte claims, and
//! with an in-band dictionary pinned, N copies of that dictionary too. These
//! tests pin the append semantics and the transform-chained default.

use std::collections::BTreeSet;

use purrdf_gts::dict::raw_content_dict;
use purrdf_gts::examples::agent_memory::{
    Memory, MemoryOptions, RecallOptions, RevisionOptions, StoreOptions, ToolCallOptions,
};
use purrdf_gts::reader::{read, read_file_segments};
use purrdf_gts::wire::{iter_items, map_get};

fn temp_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "purrdf-gts-agent-memory-{tag}-{}-{nanos}.gts",
        std::process::id()
    ))
}

/// Count the file's segment headers (§3.1: a map carrying `"gts"` and no `"t"`).
fn header_count(bytes: &[u8]) -> usize {
    let (items, _torn) = iter_items(bytes);
    items
        .iter()
        .filter(|(_, item)| {
            let inner = match item {
                ciborium::value::Value::Tag(_, inner) => inner.as_ref(),
                other => other,
            };
            matches!(inner, ciborium::value::Value::Map(entries)
                if map_get(entries, "gts").is_some() && map_get(entries, "t").is_none())
        })
        .count()
}

/// Every non-header item's `(prev, id)` pair, in file order.
fn chain(bytes: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let (items, _torn) = iter_items(bytes);
    items
        .iter()
        .filter_map(|(_, item)| {
            let ciborium::value::Value::Map(frame) = item else {
                return None;
            };
            map_get(frame, "t")?;
            let (
                Some(ciborium::value::Value::Bytes(prev)),
                Some(ciborium::value::Value::Bytes(id)),
            ) = (map_get(frame, "prev"), map_get(frame, "id"))
            else {
                return None;
            };
            Some((prev.clone(), id.clone()))
        })
        .collect()
}

/// The audit predicate `Memory::revise` writes for a `superseded_by` link.
const WAS_DERIVED_FROM: &str = "https://example.org/memory/wasDerivedFrom";

const CLAIMS: [&str; 5] = [
    "the rover records battery telemetry in UTC",
    "the rover records thermal telemetry in UTC",
    "the lander records battery telemetry in UTC",
    "the orbiter records radio telemetry in UTC",
    "the rover records radio telemetry in local time",
];

fn store_all(memory: &Memory) {
    for claim in CLAIMS {
        memory
            .store(
                claim,
                StoreOptions {
                    source: Some("bench"),
                    confidence: Some(0.75),
                    according_to: Some("urn:agent:test"),
                },
            )
            .expect("store appends a claim");
    }
}

/// (k) N claims produce ONE header and an unbroken `prev` chain — not N packs.
#[test]
fn appending_claims_produces_one_header_and_an_unbroken_chain() {
    let path = temp_path("append");
    let memory = Memory::new(&path);
    store_all(&memory);

    let bytes = std::fs::read(&path).expect("store file exists");
    assert_eq!(
        header_count(&bytes),
        1,
        "a store file must carry exactly ONE segment header, however many claims it holds"
    );

    let segments = read_file_segments(&bytes);
    assert!(segments.fatal.is_none(), "{:?}", segments.fatal);
    assert_eq!(segments.segments.len(), 1, "one segment");
    assert!(
        segments.segments[0].diagnostics.is_empty(),
        "the single segment must verify cleanly: {:?}",
        segments.segments[0].diagnostics
    );

    // Four payload frames per claim (terms/quads/reifies/annot).
    let links = chain(&bytes);
    assert_eq!(links.len(), CLAIMS.len() * 4, "one frame group per claim");
    for window in links.windows(2) {
        assert_eq!(
            window[1].0, window[0].1,
            "each frame's prev must be the previous frame's id — one continuous chain"
        );
    }

    let _ = std::fs::remove_file(&path);
}

/// (k) The appended store folds to the SAME claim set as the equivalent
/// pack-per-claim file, so the layout change is invisible to the reader.
#[test]
fn an_appended_store_folds_to_the_same_claims_as_a_pack_per_claim_file() {
    let appended_path = temp_path("appended");
    let appended = Memory::new(&appended_path);
    store_all(&appended);

    // The pack-per-claim shape, reproduced by writing each claim into its own
    // store file and concatenating them — exactly the old behaviour.
    let per_claim_path = temp_path("perclaim");
    let mut concatenated: Vec<u8> = Vec::new();
    for claim in CLAIMS {
        let one_path = temp_path("one");
        let one = Memory::new(&one_path);
        one.store(
            claim,
            StoreOptions {
                source: Some("bench"),
                confidence: Some(0.75),
                according_to: Some("urn:agent:test"),
            },
        )
        .expect("store");
        concatenated.extend(std::fs::read(&one_path).expect("read one-claim pack"));
        let _ = std::fs::remove_file(&one_path);
    }
    std::fs::write(&per_claim_path, &concatenated).expect("write pack-per-claim file");

    let appended_bytes = std::fs::read(&appended_path).expect("read appended");
    assert_eq!(header_count(&appended_bytes), 1);
    assert_eq!(
        header_count(&concatenated),
        CLAIMS.len(),
        "the control really is one header per claim"
    );
    assert!(
        appended_bytes.len() < concatenated.len(),
        "the single-header layout must be smaller ({} vs {} bytes)",
        appended_bytes.len(),
        concatenated.len()
    );

    let appended_claims: BTreeSet<String> = appended
        .recall(RecallOptions {
            limit: 100,
            ..RecallOptions::default()
        })
        .expect("recall")
        .into_iter()
        .map(|claim| claim.text)
        .collect();
    let control_claims: BTreeSet<String> = Memory::new(&per_claim_path)
        .recall(RecallOptions {
            limit: 100,
            ..RecallOptions::default()
        })
        .expect("recall")
        .into_iter()
        .map(|claim| claim.text)
        .collect();

    assert_eq!(
        appended_claims,
        CLAIMS.iter().map(|c| (*c).to_string()).collect(),
        "every stored claim must be recallable from the appended store"
    );
    assert_eq!(
        appended_claims, control_claims,
        "the appended store must fold to the same claim set as the pack-per-claim file"
    );

    let _ = std::fs::remove_file(&appended_path);
    let _ = std::fs::remove_file(&per_claim_path);
}

/// The default is transform-chained: payload frames carry an `"x"` chain, so
/// claims are actually compressed rather than stored verbatim.
#[test]
fn the_default_memory_profile_is_transform_chained() {
    let path = temp_path("profile");
    let memory = Memory::new(&path);
    assert_eq!(
        memory.options().transform,
        vec!["zstd-rsyncable".to_string()]
    );
    assert_eq!(memory.options().zstd_level, Some(12));
    store_all(&memory);

    let bytes = std::fs::read(&path).expect("store file exists");
    let (items, _torn) = iter_items(&bytes);
    let transformed = items
        .iter()
        .filter(|(_, item)| {
            matches!(item, ciborium::value::Value::Map(frame)
                if map_get(frame, "t").is_some() && map_get(frame, "x").is_some())
        })
        .count();
    assert_eq!(
        transformed,
        CLAIMS.len() * 4,
        "every payload frame must ride the configured transform chain"
    );

    let _ = std::fs::remove_file(&path);
}

/// The store is dictionary-capable: pinning one costs ONE copy of its bytes
/// (in the single header), not one per claim.
#[test]
fn a_dictionary_capable_store_pins_the_dictionary_exactly_once() {
    let owned: Vec<Vec<u8>> = CLAIMS
        .iter()
        .cycle()
        .take(200)
        .enumerate()
        .map(|(i, claim)| format!("{claim} #{i}\n").into_bytes())
        .collect();
    let refs: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
    let dict = raw_content_dict(&refs, 4096).expect("dictionary builds");

    let path = temp_path("dicted");
    let memory = Memory::with_options(
        &path,
        MemoryOptions {
            dicts: vec![("claims".to_string(), dict.clone())],
            dict: Some("claims".to_string()),
            ..MemoryOptions::default()
        },
    );
    store_all(&memory);
    memory
        .record_tool_call(
            "urn:purrdf:tool:probe",
            ToolCallOptions {
                arguments: Some("{}"),
                result: Some("ok"),
                invocation: Some("urn:purrdf:invocation:probe"),
                generated: &[],
            },
        )
        .expect("tool call appends");

    let bytes = std::fs::read(&path).expect("store file exists");
    let copies = bytes
        .windows(dict.len())
        .filter(|window| *window == dict.as_slice())
        .count();
    assert_eq!(
        copies, 1,
        "the pinned dictionary must appear exactly once, in the single header"
    );

    let graph = read(&bytes, true, None);
    assert!(
        graph.diagnostics.is_empty(),
        "a dict-primed store must fold cleanly: {:?}",
        graph.diagnostics
    );
    let claims: BTreeSet<String> = memory
        .claims()
        .expect("claims")
        .into_iter()
        .map(|claim| claim.text)
        .collect();
    assert_eq!(claims, CLAIMS.iter().map(|c| (*c).to_string()).collect());
    assert_eq!(memory.tool_calls().expect("tool calls").len(), 1);

    let _ = std::fs::remove_file(&path);
}

/// A `superseded_by` audit link must attach to the SUCCESSOR claim's own term
/// id, not to a duplicate row re-stating its IRI.
///
/// Same failure mode as the suppression above: term ids are positional within a
/// segment, so an appended duplicate of an IRI the segment already carries gets
/// a different id, and an annotation keyed on it is invisible to every reader
/// that looks the successor's reifier up by id.
#[test]
fn a_revision_links_its_successor_by_the_successors_own_term_id() {
    let path = temp_path("supersede");
    let memory = Memory::new(&path);
    store_all(&memory);
    let claims = memory.claims().expect("claims");
    let target = claims[1].clone();
    let successor = claims[4].clone();

    memory
        .revise(
            &target.id,
            RevisionOptions {
                reason: Some("restated"),
                superseded_by: Some(&successor.id),
            },
        )
        .expect("revise appends");

    let bytes = std::fs::read(&path).expect("store file exists");
    let graph = read(&bytes, true, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);

    let value = |id: usize| -> &str { graph.terms[id].value.as_deref().unwrap_or("") };
    let derived: Vec<(String, String, String)> = graph
        .annotations
        .iter()
        .filter(|&&(_, predicate, _, _)| value(predicate) == WAS_DERIVED_FROM)
        .map(|&(reifier, predicate, object, _)| {
            (
                value(reifier).to_string(),
                value(predicate).to_string(),
                value(object).to_string(),
            )
        })
        .collect();
    assert_eq!(
        derived,
        vec![(
            successor.id.clone(),
            WAS_DERIVED_FROM.to_string(),
            target.id.clone()
        )],
        "the audit link must read successor --wasDerivedFrom--> superseded claim"
    );

    // The successor's annotation row must key on the id the segment ALREADY
    // uses for that claim's reifier — the id `claims()` reads it back under.
    let reifier_ids: Vec<usize> = graph
        .reifiers
        .iter()
        .filter(|&&(rid, _, _)| value(rid) == successor.id)
        .map(|&(rid, _, _)| rid)
        .collect();
    assert_eq!(reifier_ids.len(), 1, "one reifier row for the successor");
    assert!(
        graph
            .annotations
            .iter()
            .any(|&(reifier, predicate, _, _)| reifier == reifier_ids[0]
                && value(predicate) == WAS_DERIVED_FROM),
        "the audit link must hang off the successor's OWN reifier term id"
    );

    // And the suppression still lands on exactly the revised claim.
    let suppressed: Vec<String> = memory
        .claims()
        .expect("claims")
        .into_iter()
        .filter(|claim| claim.suppressed)
        .map(|claim| claim.text)
        .collect();
    assert_eq!(suppressed, vec![target.text]);

    let _ = std::fs::remove_file(&path);
}

/// Revisions keep working across the append boundary: a suppression authored in
/// a later frame group must resolve to the claim it targets, not to whichever
/// term happened to be id 0 at the top of the segment.
#[test]
fn a_revision_appended_later_suppresses_the_right_claim() {
    let path = temp_path("revise");
    let memory = Memory::new(&path);
    store_all(&memory);
    let target = memory.claims().expect("claims")[2].clone();

    memory
        .revise(
            &target.id,
            RevisionOptions {
                reason: Some("superseded"),
                superseded_by: None,
            },
        )
        .expect("revise appends");

    let suppressed: Vec<String> = memory
        .claims()
        .expect("claims")
        .into_iter()
        .filter(|claim| claim.suppressed)
        .map(|claim| claim.text)
        .collect();
    assert_eq!(
        suppressed,
        vec![target.text],
        "exactly the revised claim must read as suppressed"
    );

    let _ = std::fs::remove_file(&path);
}
