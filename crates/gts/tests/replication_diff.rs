// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the segment-head-prefix replication diff/splice
//! surface (`purrdf_gts::replication::{diff, splice, diff_json}`).
//!
//! Fixtures are built exclusively through the public `Writer` API (mirroring
//! `missing_returns_exact_tail_range` in `replication.rs`), except for the
//! two branches that are only reachable by constructing an [`Inventory`]
//! directly through its public fields (documented at each call site).

use purrdf_gts::model::StreamableInfo;
use purrdf_gts::replication::{
    ByteRange, DiffResult, DiffStatus, FrameInventory, Inventory, SegmentFetch, SegmentInventory,
    diff, diff_json, inventory, splice,
};
use purrdf_gts::writer::Writer;

/// `local` = one `Writer` segment; `remote` = `local` with a second segment appended.
fn append_fixture() -> (Vec<u8>, Vec<u8>) {
    let mut first = Writer::new("generic");
    first.add_blob(b"a", None, None);
    let local = first.to_bytes();

    let mut second = Writer::new("generic");
    second.add_blob(b"b", None, None);
    let b = second.to_bytes();

    let mut remote = local.clone();
    remote.extend_from_slice(&b);
    (local, remote)
}

#[test]
fn diff_reconstructs_remote_via_splice() {
    let (local, remote) = append_fixture();
    let result = diff(&inventory(&local), &inventory(&remote));
    assert_eq!(result.status, DiffStatus::Fetch);
    assert!(result.continuous);

    let spliced = splice(&local, &remote, &result).expect("splice should succeed");
    assert_eq!(spliced, remote);
    assert!(!inventory(&spliced).has_problems());
}

#[test]
fn diff_appends_new_segment_is_fetch() {
    // A naive frame-`prev` check would see the new segment's first frame
    // chaining onto its own (independently rooted) header rather than onto
    // local's head, and wrongly call this `Diverged`. Segment-head-prefix
    // equality is what proves continuity here.
    let (local, remote) = append_fixture();
    let result = diff(&inventory(&local), &inventory(&remote));
    assert_eq!(result.status, DiffStatus::Fetch);
    assert!(result.continuous);
    assert_eq!(result.fetch.len(), 1);
    assert_eq!(result.fetch[0].remote_index, 1);
    assert_eq!(result.splice_offset, Some(local.len()));
}

#[test]
fn diff_identical_is_current() {
    let (local, _remote) = append_fixture();
    let result = diff(&inventory(&local), &inventory(&local));
    assert_eq!(result.status, DiffStatus::Current);
    assert!(result.fetch.is_empty());
    assert!(result.continuous);
}

#[test]
fn diff_empty_local_fetches_all() {
    let (_local, remote) = append_fixture();
    let empty = inventory(&[]);
    // `inventory(&[])` reports a fatal `EmptyFile` diagnostic (there is
    // nothing to decode), but `diff` treats a trivially empty local as a
    // valid starting point rather than a problem worth erroring out on.
    assert!(empty.has_problems());
    assert!(empty.segments.is_empty());
    let remote_inventory = inventory(&remote);

    let result = diff(&empty, &remote_inventory);
    assert_eq!(result.status, DiffStatus::Fetch);
    assert!(result.continuous);
    assert_eq!(result.fetch.len(), remote_inventory.segments.len());
    assert_eq!(result.splice_offset, Some(0));

    let spliced = splice(&[], &remote, &result).expect("splice should succeed");
    assert_eq!(spliced, remote);
}

#[test]
fn diff_remote_behind_is_current() {
    let (local, remote) = append_fixture();
    // Swap: the bigger file is now local, the smaller one is remote.
    let result = diff(&inventory(&remote), &inventory(&local));
    assert_eq!(result.status, DiffStatus::Current);
    assert!(result.fetch.is_empty());
    assert!(result.continuous);
}

#[test]
fn diff_fork_is_diverged() {
    let mut wa = Writer::new("generic");
    wa.add_blob(b"a", None, None);
    let a = wa.to_bytes();

    let mut wb = Writer::new("generic");
    wb.add_blob(b"b", None, None);
    let b = wb.to_bytes();

    let mut wc = Writer::new("generic");
    wc.add_blob(b"c", None, None);
    let c = wc.to_bytes();

    let mut local = a.clone();
    local.extend_from_slice(&b);
    let mut remote = a;
    remote.extend_from_slice(&c);

    let result = diff(&inventory(&local), &inventory(&remote));
    assert_eq!(result.status, DiffStatus::Diverged);
    assert!(!result.continuous);
    assert!(result.detail.is_some());
    assert!(splice(&local, &remote, &result).is_err());
}

#[test]
fn diff_problem_inventory_is_error() {
    let (_local, remote) = append_fixture();
    // Cut inside the second segment's frame item, well past the last clean
    // CBOR item boundary, so `iter_items` reports a torn trailing item.
    let torn = remote[..remote.len() - 5].to_vec();
    let torn_inventory = inventory(&torn);
    assert!(
        torn_inventory.has_problems(),
        "fixture must actually be torn"
    );

    let result = diff(&torn_inventory, &inventory(&remote));
    assert_eq!(result.status, DiffStatus::Error);
    assert!(!result.continuous);
    assert!(result.splice_offset.is_none());
    assert!(result.fetch.is_empty());
    assert!(result.detail.is_some());
    assert!(splice(&torn, &remote, &result).is_err());
}

#[test]
fn diff_json_schema_and_shape() {
    let (local, remote) = append_fixture();
    let result = diff(&inventory(&local), &inventory(&remote));
    let json = diff_json(&result);
    assert!(json.contains("\"schema\":\"gts-replication-diff-v1\""));
    assert!(json.contains("\"status\":\"fetch\""));
    assert!(json.contains("\"range\":{\"start\":"));
    assert!(json.contains("\"end\":"));
    assert!(json.contains("\"length\":"));
    assert!(json.ends_with('\n'));
}

#[test]
fn diff_midsegment_append_is_fetch() {
    // One `Writer` with three `add_blob` calls yields a single multi-frame
    // segment. Truncating at the first frame's end offset gives a `local`
    // whose only segment is a clean, byte-exact prefix of `remote`'s.
    let mut writer = Writer::new("generic");
    writer.add_blob(b"x", None, None);
    writer.add_blob(b"y", None, None);
    writer.add_blob(b"z", None, None);
    let remote = writer.to_bytes();

    let remote_inventory = inventory(&remote);
    assert!(!remote_inventory.has_problems());
    let cut = remote_inventory.segments[0].frames[0].end;
    let local = remote[..cut].to_vec();
    let local_inventory = inventory(&local);
    assert!(!local_inventory.has_problems());
    assert_eq!(local_inventory.segments[0].frames.len(), 1);

    let result = diff(&local_inventory, &remote_inventory);
    assert_eq!(result.status, DiffStatus::Fetch);
    assert!(result.continuous);
    assert_eq!(
        result.fetch,
        vec![SegmentFetch {
            remote_index: 0,
            range: ByteRange {
                start: local.len(),
                end: remote.len(),
            },
            head: remote_inventory.segments[0].head.clone(),
        }]
    );
    assert_eq!(result.splice_offset, Some(local.len()));

    let spliced = splice(&local, &remote, &result).expect("splice should succeed");
    assert_eq!(spliced, remote);
}

/// Build a minimal, "clean" (no diagnostics) single-frame [`FrameInventory`].
fn synth_frame(frame_index: usize, id: &[u8], prev: Option<&[u8]>, valid: bool) -> FrameInventory {
    FrameInventory {
        item_index: frame_index + 1,
        frame_index,
        start: 0,
        end: 0,
        id: id.to_vec(),
        frame_type: "blob".to_string(),
        valid,
        prev: prev.map(<[u8]>::to_vec),
    }
}

/// Build a minimal, "clean" (no diagnostics) [`SegmentInventory`].
fn synth_segment(
    index: usize,
    start: usize,
    end: usize,
    head: Option<Vec<u8>>,
    frames: Vec<FrameInventory>,
) -> SegmentInventory {
    SegmentInventory {
        index,
        item_start: 0,
        item_end: frames.len() + 1,
        start,
        end,
        profile: "generic".to_string(),
        head,
        frame_count: frames.len(),
        layout: StreamableInfo::default(),
        diagnostics: Vec::new(),
        frames,
    }
}

#[test]
fn diff_midsegment_forged_prev_is_diverged() {
    // Given `local`/`remote` both pass the `has_problems()` gate and
    // `local`'s frame ids are a genuine content prefix of `remote`'s, the
    // reader's own chain-fold guarantees the first new frame's `prev`
    // equals local's last frame id — so a real, clean GTS byte stream can
    // never exercise this failure mode (tampering the wire bytes trips the
    // fold's own `BrokenChain` diagnostic first, landing on `Error`, not
    // `Diverged`). The check still exists as defense-in-depth for callers
    // who build an `Inventory` directly, so it is exercised the same way
    // here: through the public `Inventory`/`SegmentInventory`/
    // `FrameInventory` fields, with an internally-consistent "clean" shape
    // (no diagnostics, no torn/fatal markers) but a `prev` that does not
    // chain onto local's head.
    let id_a = vec![0xAA; 32];
    let id_b = vec![0xBB; 32];
    let unrelated_prev = vec![0xEE; 32];

    let local = Inventory {
        segments: vec![synth_segment(
            0,
            0,
            100,
            Some(vec![0x11; 32]),
            vec![synth_frame(0, &id_a, Some(&[0x00; 32]), true)],
        )],
        fatal: None,
        torn: None,
        clean_end: 100,
        item_count: 2,
    };
    let remote = Inventory {
        segments: vec![synth_segment(
            0,
            0,
            200,
            Some(vec![0x22; 32]),
            vec![
                synth_frame(0, &id_a, Some(&[0x00; 32]), true),
                synth_frame(1, &id_b, Some(&unrelated_prev), true),
            ],
        )],
        fatal: None,
        torn: None,
        clean_end: 200,
        item_count: 3,
    };
    assert!(!local.has_problems());
    assert!(!remote.has_problems());

    let result = diff(&local, &remote);
    assert_eq!(result.status, DiffStatus::Diverged);
    assert!(!result.continuous);
    assert!(result.splice_offset.is_none());
    assert!(result.detail.is_some());
}

#[test]
fn diff_reencoded_common_segment_is_diverged() {
    // Same segment head, different byte length/alignment: a re-encoded
    // mirror rather than a linear extension. `Inventory`/`SegmentInventory`
    // fields are all public, so this branch is driven deterministically
    // through a synthetic pair rather than hoping a real codec round-trip
    // happens to re-encode identically.
    let head = vec![0x77; 32];
    let local = Inventory {
        segments: vec![synth_segment(0, 0, 100, Some(head.clone()), Vec::new())],
        fatal: None,
        torn: None,
        clean_end: 100,
        item_count: 1,
    };
    let remote = Inventory {
        segments: vec![synth_segment(0, 0, 120, Some(head), Vec::new())],
        fatal: None,
        torn: None,
        clean_end: 120,
        item_count: 1,
    };
    assert!(!local.has_problems());
    assert!(!remote.has_problems());

    let result = diff(&local, &remote);
    assert_eq!(result.status, DiffStatus::Diverged);
    assert!(!result.continuous);
    assert!(result.splice_offset.is_none());
    assert!(result.detail.is_some());
    assert!(splice(&[], &[], &result).is_err());
}

#[test]
fn diff_result_debug_and_clone_are_available() {
    // `DiffResult`/`SegmentFetch` are meant to be threaded through calling
    // code (e.g. logged or retried); confirm the derives are present on the
    // public surface without over-specifying formatting.
    let (local, remote) = append_fixture();
    let result: DiffResult = diff(&inventory(&local), &inventory(&remote));
    let cloned = result.clone();
    assert_eq!(format!("{cloned:?}"), format!("{result:?}"));
}
