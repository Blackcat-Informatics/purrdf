// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Replication inventory helpers for the Rust CLI.
//!
//! These helpers expose byte ranges and frame boundaries from the same CBOR
//! sequence and hash primitives as the reader, without adding runtime JSON
//! dependencies.

use ciborium::value::Value;

use crate::model::{Diagnostic, StreamableInfo};
use crate::reader::read_file_segments;
use crate::wire::{
    blake3_256, canonical, content_id, header_id, hex, iter_items, map_get, unwrap_header,
};

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

/// Half-open byte range `[start, end)` needed to resume replication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteRange {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
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

    fn problem_detail(&self) -> Option<String> {
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
        .map(|(offset, _)| *offset)
        .unwrap_or_else(|| torn.unwrap_or(data_len))
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
        let prev_ok =
            matches!(map_get(frame, "prev"), Some(Value::Bytes(prev)) if prev == &expected_prev);
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
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
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
    value.map(json_hex).unwrap_or_else(|| "null".to_string())
}

fn diagnostic_json(diagnostic: &Diagnostic) -> String {
    format!(
        "{{\"code\":{},\"detail\":{},\"frame_index\":{}}}",
        json_string(&diagnostic.code),
        json_string(&diagnostic.detail),
        diagnostic
            .frame_index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "null".to_string())
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
    fatal
        .map(diagnostic_json)
        .unwrap_or_else(|| "null".to_string())
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
            .map(|offset| offset.to_string())
            .unwrap_or_else(|| "null".to_string()),
        fatal_json(inventory.fatal.as_ref())
    )
}

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
            .map(|offset| offset.to_string())
            .unwrap_or_else(|| "null".to_string()),
        fatal_json(inventory.fatal.as_ref())
    )
}

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
        for frame in &segment.frames {
            if frame.valid && frame.id.as_slice() == from_head {
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
        }
    }
    MissingResult {
        status: MissingStatus::Unknown,
        from_head: from_head.to_vec(),
        ranges: Vec::new(),
        scan_required: true,
        detail: Some("unknown peer head; scan required".to_string()),
    }
}

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
            .map(json_string)
            .unwrap_or_else(|| "null".to_string())
    )
}

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
