// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Replication inventory helpers for the Rust CLI.
//!
//! These helpers expose byte ranges and frame boundaries from the same CBOR
//! sequence and hash primitives as the reader, without adding runtime JSON
//! dependencies.

use ciborium::value::Value;

pub use crate::model::ByteRange;
use crate::model::{Diagnostic, StreamableInfo};
use crate::reader::read_file_segments;
use crate::wire::{
    blake3_256, canonical, content_id, header_id, hex, iter_items, map_get, unwrap_header,
};

/// Byte range, identity, and chain-validation state for one frame.
#[derive(Clone, Debug)]
pub struct FrameInventory {
    /// Absolute CBOR sequence item index.
    pub item_index: usize,
    /// Zero-based frame index within the segment.
    pub frame_index: usize,
    /// Start byte offset in the original file.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
    /// Stored frame id when present, otherwise the computed content id.
    pub id: Vec<u8>,
    /// Wire frame `"t"` value.
    pub frame_type: String,
    /// True when both self-id and `prev` chain checks passed for this frame.
    pub valid: bool,
    /// Stored `prev` chain-link bytes the frame claims to chain onto.
    ///
    /// `None` for header-derived seed frames and non-map placeholder frames.
    pub prev: Option<Vec<u8>>,
}

/// Inventory for one segment in a possibly concatenated GTS file.
#[derive(Clone, Debug)]
pub struct SegmentInventory {
    /// Zero-based segment index.
    pub index: usize,
    /// Absolute CBOR item index of the segment header.
    pub item_start: usize,
    /// Absolute CBOR item index one past the segment.
    pub item_end: usize,
    /// Start byte offset of the segment.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
    /// Segment profile from the header.
    pub profile: String,
    /// Segment head id, when the segment was foldable.
    pub head: Option<Vec<u8>>,
    /// Number of frames after the header.
    pub frame_count: usize,
    /// Computed layout/streamability state.
    pub layout: StreamableInfo,
    /// Diagnostics produced while folding this segment.
    pub diagnostics: Vec<Diagnostic>,
    /// Per-frame byte ranges and chain validation state.
    pub frames: Vec<FrameInventory>,
}

/// Byte and segment inventory for a GTS file.
#[derive(Clone, Debug)]
pub struct Inventory {
    /// Segment inventories in file order.
    pub segments: Vec<SegmentInventory>,
    /// Fatal file-level diagnostic, if no segment inventory can be trusted.
    pub fatal: Option<Diagnostic>,
    /// Offset of a torn trailing CBOR item.
    pub torn: Option<usize>,
    /// End offset of the last complete CBOR item.
    pub clean_end: usize,
    /// Number of complete CBOR items parsed before any torn append.
    pub item_count: usize,
}

/// Result category for a replication missing-range query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissingStatus {
    /// The peer head is already current.
    Complete,
    /// Concrete byte ranges can resume the peer.
    Ranges,
    /// The peer head is unknown; a full scan or full transfer is needed.
    Unknown,
    /// The inventory is not trustworthy enough to compute ranges.
    Error,
}

/// Missing byte-range result relative to a peer's known head.
#[derive(Clone, Debug)]
pub struct MissingResult {
    /// High-level result category.
    pub status: MissingStatus,
    /// Peer head used for the query.
    pub from_head: Vec<u8>,
    /// Byte ranges needed by the peer.
    pub ranges: Vec<ByteRange>,
    /// Whether the peer must scan or re-request broader state.
    pub scan_required: bool,
    /// Human-readable explanation for `Unknown` or `Error`.
    pub detail: Option<String>,
}

impl Inventory {
    /// True when the inventory contains file-level, segment-level, or torn-append problems.
    pub fn has_problems(&self) -> bool {
        self.fatal.is_some()
            || self.torn.is_some()
            || self
                .segments
                .iter()
                .any(|segment| !segment.diagnostics.is_empty())
    }

    pub(crate) fn problem_detail(&self) -> Option<String> {
        if let Some(fatal) = &self.fatal {
            return Some(format!("{}: {}", fatal.code, fatal.detail));
        }
        if let Some(offset) = self.torn {
            return Some(format!("torn at offset {offset}"));
        }
        self.segments
            .iter()
            .flat_map(|segment| segment.diagnostics.iter())
            .next()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.detail))
    }
}

fn as_text(v: &Value) -> Option<&str> {
    if let Value::Text(text) = v {
        Some(text)
    } else {
        None
    }
}

/// §3.1 boundary rule: a map carrying `"gts"` and lacking `"t"`.
fn is_header_item(item: &Value) -> bool {
    let inner = match item {
        Value::Tag(_, inner) => inner.as_ref(),
        other => other,
    };
    if let Value::Map(entries) = inner {
        map_get(entries, "gts").is_some() && map_get(entries, "t").is_none()
    } else {
        false
    }
}

fn item_end(items: &[(usize, Value)], torn: Option<usize>, data_len: usize, index: usize) -> usize {
    items
        .get(index + 1)
        .map_or_else(|| torn.unwrap_or(data_len), |(offset, _)| *offset)
}

fn header_profile(item: &Value) -> String {
    unwrap_header(item)
        .ok()
        .and_then(|header| map_get(header, "prof").and_then(as_text))
        .unwrap_or("generic")
        .to_string()
}

fn header_stored_id(item: &Value) -> Option<Vec<u8>> {
    unwrap_header(item).ok().and_then(|header| {
        if let Some(Value::Bytes(id)) = map_get(header, "id") {
            Some(id.clone())
        } else {
            None
        }
    })
}

fn header_computed_id(item: &Value) -> Option<Vec<u8>> {
    unwrap_header(item).ok().map(|header| header_id(header))
}

fn collect_frames(
    items: &[(usize, Value)],
    torn: Option<usize>,
    data_len: usize,
    start: usize,
    end: usize,
) -> Vec<FrameInventory> {
    let mut frames = Vec::new();
    let mut expected_prev = header_stored_id(&items[start].1)
        .or_else(|| header_computed_id(&items[start].1))
        .unwrap_or_default();
    for item_index in (start + 1)..end {
        let item_start = items[item_index].0;
        let item_end = item_end(items, torn, data_len, item_index);
        let frame_index = item_index - start - 1;
        let Value::Map(frame) = &items[item_index].1 else {
            frames.push(FrameInventory {
                item_index,
                frame_index,
                start: item_start,
                end: item_end,
                id: Vec::new(),
                frame_type: "<non-map>".to_string(),
                valid: false,
                prev: None,
            });
            continue;
        };
        let computed = content_id(frame);
        let stored_id = match map_get(frame, "id") {
            Some(Value::Bytes(id)) => Some(id.clone()),
            _ => None,
        };
        let id_ok = stored_id
            .as_deref()
            .is_some_and(|stored| stored == computed.as_slice());
        // Replication uses the same id/prev chain invariant as the reader, but
        // keeps scanning so callers can still identify byte ranges around bad
        // frames.
        let stored_prev = match map_get(frame, "prev") {
            Some(Value::Bytes(prev)) => Some(prev.clone()),
            _ => None,
        };
        let prev_ok = stored_prev.as_deref() == Some(expected_prev.as_slice());
        let id = stored_id.clone().unwrap_or_else(|| computed.clone());
        expected_prev = stored_id.unwrap_or(computed);
        frames.push(FrameInventory {
            item_index,
            frame_index,
            start: item_start,
            end: item_end,
            id,
            frame_type: map_get(frame, "t")
                .and_then(as_text)
                .unwrap_or("<unknown>")
                .to_string(),
            valid: id_ok && prev_ok,
            prev: stored_prev,
        });
    }
    frames
}

/// Build a replication inventory from raw GTS bytes.
pub fn inventory(data: &[u8]) -> Inventory {
    let (items, torn) = iter_items(data);
    let clean_end = torn.unwrap_or(data.len());
    let fs = read_file_segments(data);
    if items.is_empty() || fs.fatal.is_some() {
        return Inventory {
            segments: Vec::new(),
            fatal: fs.fatal,
            torn,
            clean_end,
            item_count: items.len(),
        };
    }

    let bounds: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, (_, item))| is_header_item(item))
        .map(|(index, _)| index)
        .collect();
    if bounds.first() != Some(&0) {
        return Inventory {
            segments: Vec::new(),
            fatal: fs.fatal,
            torn,
            clean_end,
            item_count: items.len(),
        };
    }

    let ends: Vec<usize> = bounds
        .iter()
        .skip(1)
        .copied()
        .chain([items.len()])
        .collect();
    let segments = bounds
        .iter()
        .zip(ends.iter())
        .enumerate()
        .map(|(index, (&start_item, &end_item))| {
            let graph = &fs.segments[index];
            let start = items[start_item].0;
            let end = if end_item < items.len() {
                items[end_item].0
            } else {
                clean_end
            };
            SegmentInventory {
                index,
                item_start: start_item,
                item_end: end_item,
                start,
                end,
                profile: graph
                    .segment_profiles
                    .first()
                    .cloned()
                    .unwrap_or_else(|| header_profile(&items[start_item].1)),
                head: graph.segment_heads.first().cloned(),
                frame_count: end_item.saturating_sub(start_item + 1),
                layout: graph
                    .segment_streamable
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                diagnostics: graph.diagnostics.clone(),
                frames: collect_frames(&items, torn, data.len(), start_item, end_item),
            }
        })
        .collect();

    Inventory {
        segments,
        fatal: fs.fatal,
        torn,
        clean_end,
        item_count: items.len(),
    }
}

fn aggregate_digest(inventory: &Inventory) -> Vec<u8> {
    let heads: Vec<Value> = inventory
        .segments
        .iter()
        .filter_map(|segment| segment.head.as_ref())
        .map(|head| Value::Bytes(head.clone()))
        .collect();
    blake3_256(&canonical(&Value::Array(vec![
        "gts-segment-heads-v1".into(),
        Value::Array(heads),
    ])))
}

fn json_escape(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn json_string(text: &str) -> String {
    format!("\"{}\"", json_escape(text))
}

fn json_hex(bytes: &[u8]) -> String {
    json_string(&hex(bytes))
}

fn json_optional_hex(value: Option<&[u8]>) -> String {
    value.map_or_else(|| "null".to_string(), json_hex)
}

fn diagnostic_json(diagnostic: &Diagnostic) -> String {
    format!(
        "{{\"code\":{},\"detail\":{},\"frame_index\":{}}}",
        json_string(&diagnostic.code),
        json_string(&diagnostic.detail),
        diagnostic
            .frame_index
            .map_or_else(|| "null".to_string(), |index| index.to_string())
    )
}

fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    format!(
        "[{}]",
        diagnostics
            .iter()
            .map(diagnostic_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn fatal_json(fatal: Option<&Diagnostic>) -> String {
    fatal.map_or_else(|| "null".to_string(), diagnostic_json)
}

fn layout_json(layout: &StreamableInfo) -> String {
    format!(
        "{{\"claimed\":{},\"covered\":{},\"tail\":{},\"head\":{}}}",
        layout.claimed,
        layout.covered,
        layout.tail,
        json_optional_hex(layout.head.as_deref())
    )
}

fn range_json(range: &ByteRange) -> String {
    format!(
        "{{\"start\":{},\"end\":{},\"length\":{}}}",
        range.start,
        range.end,
        range.end.saturating_sub(range.start)
    )
}

/// Render the segment heads and aggregate digest as `gts-replication-heads-v1` JSON.
pub fn heads_json(inventory: &Inventory) -> String {
    let segment_heads: Vec<String> = inventory
        .segments
        .iter()
        .filter_map(|segment| segment.head.as_deref())
        .map(json_hex)
        .collect();
    let file_head = inventory
        .segments
        .last()
        .and_then(|segment| segment.head.as_deref());
    format!(
        "{{\"schema\":\"gts-replication-heads-v1\",\"clean\":{},\"segment_heads\":[{}],\
         \"aggregate\":{{\"schema\":\"gts-segment-heads-v1\",\"count\":{},\
         \"digest\":{},\"file_head\":{}}},\"torn_at\":{},\"fatal\":{}}}\n",
        !inventory.has_problems(),
        segment_heads.join(","),
        segment_heads.len(),
        json_hex(&aggregate_digest(inventory)),
        json_optional_hex(file_head),
        inventory
            .torn
            .map_or_else(|| "null".to_string(), |offset| offset.to_string()),
        fatal_json(inventory.fatal.as_ref())
    )
}

/// Render per-segment byte/item ranges and layout as `gts-replication-segments-v1` JSON.
pub fn segments_json(inventory: &Inventory) -> String {
    let segments = inventory
        .segments
        .iter()
        .map(|segment| {
            format!(
                "{{\"index\":{},\"byte_range\":{},\"item_range\":{{\"start\":{},\"end\":{}}},\
                 \"profile\":{},\"head\":{},\"frame_count\":{},\"layout\":{},\
                 \"diagnostics\":{}}}",
                segment.index,
                range_json(&ByteRange {
                    start: segment.start,
                    end: segment.end,
                }),
                segment.item_start,
                segment.item_end,
                json_string(&segment.profile),
                json_optional_hex(segment.head.as_deref()),
                segment.frame_count,
                layout_json(&segment.layout),
                diagnostics_json(&segment.diagnostics)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"gts-replication-segments-v1\",\"clean\":{},\"segments\":[{}],\
         \"item_count\":{},\"torn_at\":{},\"fatal\":{}}}\n",
        !inventory.has_problems(),
        segments,
        inventory.item_count,
        inventory
            .torn
            .map_or_else(|| "null".to_string(), |offset| offset.to_string()),
        fatal_json(inventory.fatal.as_ref())
    )
}

/// Locate the first valid frame carrying `id`, scanning segments and their
/// frames in file order.
///
/// Returns the `(segment_index, frame_index)` pair, suitable for indexing
/// back into [`Inventory::segments`] and [`SegmentInventory::frames`].
/// Invalid frames (failed id or `prev` chain checks) are skipped, matching
/// the trust rule used throughout this module: only a validated chain
/// position counts as a known head.
fn find_frame(inventory: &Inventory, id: &[u8]) -> Option<(usize, usize)> {
    for segment in &inventory.segments {
        for frame in &segment.frames {
            if frame.valid && frame.id.as_slice() == id {
                return Some((segment.index, frame.frame_index));
            }
        }
    }
    None
}

/// Compute the byte ranges a peer at `from_head` is missing from this inventory.
///
/// `from_head` may name a segment head or any valid frame id; an unknown head
/// yields [`MissingStatus::Unknown`] with `scan_required` set.
pub fn missing(inventory: &Inventory, from_head: &[u8]) -> MissingResult {
    if inventory.has_problems() {
        return MissingResult {
            status: MissingStatus::Error,
            from_head: from_head.to_vec(),
            ranges: Vec::new(),
            scan_required: false,
            detail: inventory.problem_detail(),
        };
    }
    for segment in &inventory.segments {
        if segment
            .head
            .as_deref()
            .is_some_and(|head| head == from_head)
        {
            let ranges = if segment.end < inventory.clean_end {
                vec![ByteRange {
                    start: segment.end,
                    end: inventory.clean_end,
                }]
            } else {
                Vec::new()
            };
            return MissingResult {
                status: if ranges.is_empty() {
                    MissingStatus::Complete
                } else {
                    MissingStatus::Ranges
                },
                from_head: from_head.to_vec(),
                ranges,
                scan_required: false,
                detail: None,
            };
        }
    }
    if let Some((segment_index, frame_index)) = find_frame(inventory, from_head) {
        let frame = &inventory.segments[segment_index].frames[frame_index];
        let ranges = if frame.end < inventory.clean_end {
            vec![ByteRange {
                start: frame.end,
                end: inventory.clean_end,
            }]
        } else {
            Vec::new()
        };
        return MissingResult {
            status: if ranges.is_empty() {
                MissingStatus::Complete
            } else {
                MissingStatus::Ranges
            },
            from_head: from_head.to_vec(),
            ranges,
            scan_required: false,
            detail: None,
        };
    }
    MissingResult {
        status: MissingStatus::Unknown,
        from_head: from_head.to_vec(),
        ranges: Vec::new(),
        scan_required: true,
        detail: Some("unknown peer head; scan required".to_string()),
    }
}

/// Render a [`missing`] result as `gts-replication-missing-v1` JSON.
pub fn missing_json(result: &MissingResult) -> String {
    let status = match result.status {
        MissingStatus::Complete => "complete",
        MissingStatus::Ranges => "ranges",
        MissingStatus::Unknown => "unknown",
        MissingStatus::Error => "error",
    };
    let ranges = result
        .ranges
        .iter()
        .map(range_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"gts-replication-missing-v1\",\"status\":{},\"from_head\":{},\
         \"ranges\":[{}],\"scan_required\":{},\"detail\":{}}}\n",
        json_string(status),
        json_hex(&result.from_head),
        ranges,
        result.scan_required,
        result
            .detail
            .as_deref()
            .map_or_else(|| "null".to_string(), json_string)
    )
}

/// Return the clean byte suffix that follows the frame identified by `frame_id`.
///
/// # Errors
///
/// Returns an error when the file is torn or otherwise not clean, or when no
/// valid frame carries `frame_id`.
pub fn resume_after<'a>(data: &'a [u8], frame_id: &[u8]) -> Result<&'a [u8], String> {
    let inventory = inventory(data);
    if inventory.has_problems() {
        return Err(inventory
            .problem_detail()
            .unwrap_or_else(|| "input is not clean".to_string()));
    }
    for segment in &inventory.segments {
        for frame in &segment.frames {
            if frame.valid && frame.id.as_slice() == frame_id {
                return Ok(&data[frame.end..inventory.clean_end]);
            }
        }
    }
    Err(format!("frame {} not found", hex(frame_id)))
}

/// Outcome category for a two-file replication [`diff`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffStatus {
    /// Local already matches remote through their common prefix; nothing to fetch.
    Current,
    /// Remote linearly extends local; `DiffResult::fetch` names the ranges that reconstruct it.
    Fetch,
    /// Local and remote disagree on content; remote is not a linear extension of local.
    Diverged,
    /// One or both inventories are not trustworthy enough to diff.
    Error,
}

/// One remote byte range to retrieve in order to extend local toward remote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentFetch {
    /// Index of the segment in the *remote* inventory that this range covers.
    pub remote_index: usize,
    /// Byte range within the remote file to copy.
    pub range: ByteRange,
    /// Remote segment head, when known.
    pub head: Option<Vec<u8>>,
}

/// Result of comparing a local inventory against a remote inventory.
#[derive(Clone, Debug)]
pub struct DiffResult {
    /// High-level outcome category.
    pub status: DiffStatus,
    /// Remote byte ranges to fetch, ordered by `remote_index` (file order).
    pub fetch: Vec<SegmentFetch>,
    /// Byte offset in the *local* file where the fetched suffix is appended.
    ///
    /// `None` when the result cannot be spliced (`Diverged`/`Error`).
    pub splice_offset: Option<usize>,
    /// True when the fetch (if any) forms an unbroken extension of local.
    pub continuous: bool,
    /// Human-readable explanation, set for `Diverged` and `Error`.
    pub detail: Option<String>,
}

/// Index of the first frame at which `local_frames` stops being a prefix of
/// `remote_frames`, comparing frame identity only.
fn frames_id_mismatch(
    local_frames: &[FrameInventory],
    remote_frames: &[FrameInventory],
) -> Option<usize> {
    local_frames
        .iter()
        .zip(remote_frames.iter())
        .position(|(local, remote)| local.id != remote.id)
}

/// Whether `inventory` is problematic enough to refuse diffing.
///
/// A wholly empty byte stream (`inventory(&[])`) reports a fatal `EmptyFile`
/// diagnostic so other callers can flag "nothing decoded here" — but for a
/// two-file diff, a genuinely empty local or remote is a valid trivial
/// starting point (the empty-local "fetch everything" case), not a defect
/// worth erroring out on. Any other fatal, torn, or per-segment diagnostic
/// still refuses the diff.
fn is_diff_problem(inventory: &Inventory) -> bool {
    let trivially_empty =
        inventory.segments.is_empty() && inventory.item_count == 0 && inventory.torn.is_none();
    !trivially_empty && inventory.has_problems()
}

/// Compare `local` against `remote` and describe how to extend local to match it.
///
/// Continuity is judged primarily by segment-head *prefix* equality: GTS
/// segments are independently rooted, so a whole-segment append's first
/// frame chains onto its own segment header, not onto local's head. Frame
/// `prev` chaining is only consulted when the trailing segment itself is
/// being extended with more frames (a mid-segment cat-append).
pub fn diff(local: &Inventory, remote: &Inventory) -> DiffResult {
    let local_problem = is_diff_problem(local);
    let remote_problem = is_diff_problem(remote);
    if local_problem || remote_problem {
        let mut details = Vec::new();
        if local_problem {
            details.push(format!(
                "local: {}",
                local
                    .problem_detail()
                    .unwrap_or_else(|| "unknown problem".to_string())
            ));
        }
        if remote_problem {
            details.push(format!(
                "remote: {}",
                remote
                    .problem_detail()
                    .unwrap_or_else(|| "unknown problem".to_string())
            ));
        }
        return DiffResult {
            status: DiffStatus::Error,
            fetch: Vec::new(),
            splice_offset: None,
            continuous: false,
            detail: Some(details.join("; ")),
        };
    }

    // Largest k such that local.segments[..k] and remote.segments[..k] share
    // byte-identical, byte-aligned segments (matching heads at matching offsets).
    let limit = local.segments.len().min(remote.segments.len());
    let mut k = 0usize;
    for i in 0..limit {
        let local_segment = &local.segments[i];
        let remote_segment = &remote.segments[i];
        match (&local_segment.head, &remote_segment.head) {
            (Some(local_head), Some(remote_head)) if local_head == remote_head => {
                let aligned = local_segment.start == remote_segment.start
                    && local_segment.end - local_segment.start
                        == remote_segment.end - remote_segment.start;
                if aligned {
                    k += 1;
                } else {
                    return DiffResult {
                        status: DiffStatus::Diverged,
                        fetch: Vec::new(),
                        splice_offset: None,
                        continuous: false,
                        detail: Some(format!(
                            "segment {i} heads match but byte ranges diverge (local \
                             {ls}..{le}, remote {rs}..{re}); likely a re-encoded mirror",
                            ls = local_segment.start,
                            le = local_segment.end,
                            rs = remote_segment.start,
                            re = remote_segment.end,
                        )),
                    };
                }
            }
            _ => break,
        }
    }

    if k == local.segments.len() && k == remote.segments.len() {
        return DiffResult {
            status: DiffStatus::Current,
            fetch: Vec::new(),
            splice_offset: Some(local.clean_end),
            continuous: true,
            detail: None,
        };
    }

    if k == local.segments.len() {
        // remote.segments.len() > k: remote appended one or more whole segments.
        let fetch: Vec<SegmentFetch> = remote.segments[k..]
            .iter()
            .map(|segment| SegmentFetch {
                remote_index: segment.index,
                range: ByteRange {
                    start: segment.start,
                    end: segment.end,
                },
                head: segment.head.clone(),
            })
            .collect();
        return DiffResult {
            status: DiffStatus::Fetch,
            fetch,
            splice_offset: Some(local.clean_end),
            continuous: true,
            detail: None,
        };
    }

    if k == remote.segments.len() {
        // remote.segments.len() < local.segments.len(): remote is behind local.
        return DiffResult {
            status: DiffStatus::Current,
            fetch: Vec::new(),
            splice_offset: Some(local.clean_end),
            continuous: true,
            detail: None,
        };
    }

    // k < local.segments.len() && k < remote.segments.len(): the segments at
    // index k disagree. The only continuous shape is a mid-segment
    // cat-append onto local's own last (and only divergent) segment.
    if local.segments.len() != k + 1 {
        return DiffResult {
            status: DiffStatus::Diverged,
            fetch: Vec::new(),
            splice_offset: None,
            continuous: false,
            detail: Some(format!(
                "local has {extra} segment(s) beyond the common prefix at segment {k}; \
                 only local's own last segment may be a partial extension",
                extra = local.segments.len() - (k + 1),
            )),
        };
    }

    let local_segment = &local.segments[k];
    let remote_segment = &remote.segments[k];
    let local_frames = &local_segment.frames;
    let remote_frames = &remote_segment.frames;

    // The overlap must match by content first: if this segment's head
    // differed at index k (that's how we got here), and the overlapping
    // frame ids are equal length and identical, the only remaining
    // possibility is that identical frame content sits under a differently
    // encoded header (a re-encoded mirror) — never a genuine linear
    // extension. Checking the id prefix before the length shortcut keeps a
    // same-length fork (equal frame counts, different content) from being
    // misread as "local already ahead".
    if let Some(mismatch) = frames_id_mismatch(local_frames, remote_frames) {
        return DiffResult {
            status: DiffStatus::Diverged,
            fetch: Vec::new(),
            splice_offset: None,
            continuous: false,
            detail: Some(format!(
                "segment {k} frame {mismatch} id diverges from remote's frame at the same index"
            )),
        };
    }

    if local_frames.len() >= remote_frames.len() {
        // Local already has at least as many frames in this segment and the
        // overlapping ids agree, so remote has nothing new here.
        return DiffResult {
            status: DiffStatus::Current,
            fetch: Vec::new(),
            splice_offset: Some(local.clean_end),
            continuous: true,
            detail: None,
        };
    }

    let first_new = &remote_frames[local_frames.len()];
    let chained = match local_frames.len().checked_sub(1) {
        Some(last_local_frame) => first_new
            .prev
            .as_deref()
            .is_some_and(|prev| find_frame(local, prev) == Some((k, last_local_frame))),
        None => first_new.prev.as_deref() == local_segment.head.as_deref(),
    };

    if first_new.valid && chained {
        DiffResult {
            status: DiffStatus::Fetch,
            fetch: vec![SegmentFetch {
                remote_index: remote_segment.index,
                range: ByteRange {
                    start: local.clean_end,
                    end: remote.clean_end,
                },
                head: remote_segment.head.clone(),
            }],
            splice_offset: Some(local.clean_end),
            continuous: true,
            detail: None,
        }
    } else {
        DiffResult {
            status: DiffStatus::Diverged,
            fetch: Vec::new(),
            splice_offset: None,
            continuous: false,
            detail: Some(format!(
                "segment {k}'s first new remote frame does not chain onto local's head; possible fork"
            )),
        }
    }
}

/// Reconstruct bytes by applying a [`diff`] result's fetch list onto `local_bytes`.
///
/// # Errors
///
/// Returns an error when `result.status` is [`DiffStatus::Diverged`] or
/// [`DiffStatus::Error`], when a fetch range falls outside `remote_bytes`,
/// or when the spliced (or unchanged) output is not itself a clean
/// inventory.
pub fn splice(
    local_bytes: &[u8],
    remote_bytes: &[u8],
    result: &DiffResult,
) -> Result<Vec<u8>, String> {
    if matches!(result.status, DiffStatus::Diverged | DiffStatus::Error) {
        return Err(result
            .detail
            .clone()
            .unwrap_or_else(|| "diff result cannot be spliced".to_string()));
    }

    if result.fetch.is_empty() {
        let rebuilt = inventory(local_bytes);
        if rebuilt.has_problems() {
            return Err(rebuilt
                .problem_detail()
                .unwrap_or_else(|| "local bytes are not a clean inventory".to_string()));
        }
        return Ok(local_bytes.to_vec());
    }

    let offset = result
        .splice_offset
        .ok_or_else(|| "diff result has no splice offset".to_string())?;
    if offset > local_bytes.len() {
        return Err(format!(
            "splice offset {offset} exceeds local length {}",
            local_bytes.len()
        ));
    }

    let mut out = local_bytes[..offset].to_vec();
    for fetch in &result.fetch {
        let start = fetch.range.start;
        let end = fetch.range.end;
        if start > end || end > remote_bytes.len() {
            return Err(format!(
                "fetch range {start}..{end} exceeds remote length {}",
                remote_bytes.len()
            ));
        }
        out.extend_from_slice(&remote_bytes[start..end]);
    }

    let rebuilt = inventory(&out);
    if rebuilt.has_problems() {
        return Err(rebuilt
            .problem_detail()
            .unwrap_or_else(|| "spliced bytes are not a clean inventory".to_string()));
    }
    Ok(out)
}

/// Render a [`diff`] result as `gts-replication-diff-v1` JSON.
pub fn diff_json(result: &DiffResult) -> String {
    let status = match result.status {
        DiffStatus::Current => "current",
        DiffStatus::Fetch => "fetch",
        DiffStatus::Diverged => "diverged",
        DiffStatus::Error => "error",
    };
    let fetch = result
        .fetch
        .iter()
        .map(|f| {
            format!(
                "{{\"remote_index\":{},\"range\":{},\"head\":{}}}",
                f.remote_index,
                range_json(&f.range),
                json_optional_hex(f.head.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"gts-replication-diff-v1\",\"status\":{},\"fetch\":[{}],\
         \"splice_offset\":{},\"continuous\":{},\"detail\":{}}}\n",
        json_string(status),
        fetch,
        result
            .splice_offset
            .map_or_else(|| "null".to_string(), |offset| offset.to_string()),
        result.continuous,
        result
            .detail
            .as_deref()
            .map_or_else(|| "null".to_string(), json_string)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::Writer;

    #[test]
    fn missing_returns_exact_tail_range() {
        let mut first = Writer::new("generic");
        let head = first.add_blob(b"a", None, None);
        let a = first.to_bytes();
        let mut second = Writer::new("generic");
        second.add_blob(b"b", None, None);
        let b = second.to_bytes();
        let mut data = a.clone();
        data.extend_from_slice(&b);
        let inv = inventory(&data);
        let result = missing(&inv, &head);
        assert_eq!(result.status, MissingStatus::Ranges);
        assert_eq!(
            result.ranges,
            vec![ByteRange {
                start: a.len(),
                end: data.len()
            }]
        );
    }
}
