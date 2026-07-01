// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use ciborium::value::Value;

use crate::mmr;
use crate::model::{Diagnostic, Graph, StreamableInfo};
use crate::reader::{push_diagnostic, StreamingSink};
use crate::wire::map_get;

#[derive(Clone, Debug)]
pub(crate) struct IndexRecord {
    pub(crate) abs_index: usize,
    pub(crate) count: usize,
    pub(crate) head: Vec<u8>,
    pub(crate) mmr: Option<Vec<u8>>,
}

/// Compute one segment's layout state and check its claim (§3.3).
///
/// For a segment claiming `"layout": "streamable"`: (a) it must carry an
/// intact `index` footer, (b) the last index's `head` must be the id of
/// frame `count`, and (c) every covered inline blob must arrive after the
/// `stream:digest` quad describing it. Frames after the last index are the
/// legal accretive tail — boundary info, never a diagnostic. Unknown layout
/// values impose no check (§5).
pub(crate) fn layout_check(
    g: &mut Graph,
    header: &[(Value, Value)],
    index_records: &[IndexRecord],
    blob_events: &[(usize, String, bool)],
    frame_ids: &[Vec<u8>],
    index_offset: usize,
    sink: &mut Option<&mut dyn StreamingSink>,
) -> StreamableInfo {
    let claimed = matches!(map_get(header, "layout"), Some(Value::Text(t)) if t == "streamable");
    let total = frame_ids.len();
    if !claimed {
        return StreamableInfo::default();
    }
    let Some(record) = index_records.last() else {
        push_diagnostic(
            g,
            sink,
            Diagnostic {
                code: "StreamableLayoutError".to_string(),
                detail: "segment claims layout 'streamable' but carries no intact \
                     index footer (§3.3)"
                    .to_string(),
                frame_index: None,
            },
        );
        return StreamableInfo {
            claimed: true,
            covered: 0,
            tail: total,
            head: None,
        };
    };
    let (abs_pos, count, head) = (record.abs_index, record.count, &record.head);
    let rel_pos = abs_pos - index_offset; // 1-based frame position of the index
    let tail = total - rel_pos;
    // The footer must IMMEDIATELY follow the frames it covers (§3.3): a
    // permissive `count <= rel_pos - 1` would let frames sit between the
    // covered prefix and the footer, counted neither as covered nor as tail.
    if count != rel_pos - 1 || count < 1 || frame_ids[count - 1] != *head {
        push_diagnostic(
            g,
            sink,
            Diagnostic {
                code: "StreamableLayoutError".to_string(),
                detail: format!(
                    "index footer contradicts the frames it covers: count {count} \
                 must name the frame immediately before the footer and head \
                 must be that frame's id (§3.3)"
                ),
                frame_index: Some(abs_pos),
            },
        );
    }
    for (blob_abs, digest, described) in blob_events {
        let blob_rel = blob_abs - index_offset;
        if blob_rel <= count && !described {
            push_diagnostic(
                g,
                sink,
                Diagnostic {
                    code: "StreamableLayoutError".to_string(),
                    detail: format!(
                        "covered blob {digest} delivered before its stream:digest \
                     description (catalog-before-payload, §3.3)"
                    ),
                    frame_index: Some(*blob_abs),
                },
            );
        }
    }
    StreamableInfo {
        claimed: true,
        covered: count,
        tail,
        head: Some(head.clone()),
    }
}

pub(crate) fn check_index_mmr(
    g: &mut Graph,
    index_records: &[IndexRecord],
    frame_ids: &[Vec<u8>],
    index_offset: usize,
    sink: &mut Option<&mut dyn StreamingSink>,
) {
    for record in index_records {
        let Some(root) = &record.mmr else {
            continue;
        };
        let rel_pos = record.abs_index.saturating_sub(index_offset);
        let preceding = rel_pos.saturating_sub(1);
        let mut detail = None;
        if root.len() != 32 {
            detail = Some("index mmr root is not a 32-byte digest".to_string());
        } else if record.count > preceding {
            detail = Some(format!(
                "index mmr covers {} frame(s), but only {preceding} precede the index",
                record.count
            ));
        } else if record.count > frame_ids.len() {
            detail = Some(format!(
                "index mmr covers {} frame(s), but the segment has {} frame id(s)",
                record.count,
                frame_ids.len()
            ));
        } else if record.count > 0 && frame_ids[record.count - 1] != record.head {
            detail = Some("index mmr head does not match the last covered frame".to_string());
        } else {
            let computed = mmr::root(&frame_ids[..record.count]);
            if computed != *root {
                detail = Some("index mmr root does not match the covered frame ids".to_string());
            }
        }
        if let Some(detail) = detail {
            push_diagnostic(
                g,
                sink,
                Diagnostic {
                    code: "IndexMmrError".to_string(),
                    detail,
                    frame_index: Some(record.abs_index),
                },
            );
        }
    }
}
