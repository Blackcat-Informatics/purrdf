// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_lex::cbor::Value;

use crate::mmr;
use crate::model::{Diagnostic, Graph, StreamableInfo};
use crate::reader::{StreamingSink, push_diagnostic};
use crate::wire::map_get;

mod error;
use error::LayoutError;

#[derive(Clone, Debug)]
pub(crate) struct IndexRecord {
    pub(crate) abs_index: u64,
    pub(crate) count: u64,
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
    blob_events: &[(u64, String, bool)],
    frame_ids: &[Vec<u8>],
    index_offset: u64,
    sink: &mut Option<&mut dyn StreamingSink>,
) -> StreamableInfo {
    let claimed = matches!(map_get(header, "layout"), Some(Value::Text(t)) if t == "streamable");
    let total = frame_ids.len() as u64;
    if !claimed {
        return StreamableInfo::default();
    }
    let Some(record) = index_records.last() else {
        push_diagnostic(
            g,
            sink,
            Diagnostic {
                cause: None,
                code: "StreamableLayoutError".to_string(),
                detail: "segment claims layout 'streamable' but carries no intact \
                     index footer (§3.3)"
                    .to_string(),
                frame_index: None,
            }
            .with_native_cause(LayoutError::MissingFooter),
        );
        if sink.as_deref().is_some_and(|sink| !sink.continue_reading()) {
            return StreamableInfo::default();
        }
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
    if count != rel_pos - 1
        || count < 1
        || usize::try_from(count - 1)
            .ok()
            .and_then(|index| frame_ids.get(index))
            .is_none_or(|id| id != head)
    {
        push_diagnostic(
            g,
            sink,
            Diagnostic {
                cause: None,
                code: "StreamableLayoutError".to_string(),
                detail: format!(
                    "index footer contradicts the frames it covers: count {count} \
                 must name the frame immediately before the footer and head \
                 must be that frame's id (§3.3)"
                ),
                frame_index: Some(abs_pos),
            }
            .with_native_cause(LayoutError::Footer {
                count,
                preceding: rel_pos - 1,
                head: head.clone(),
                actual: count
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| frame_ids.get(index))
                    .cloned(),
            }),
        );
        if sink.as_deref().is_some_and(|sink| !sink.continue_reading()) {
            return StreamableInfo::default();
        }
    }
    for (blob_abs, digest, described) in blob_events {
        let blob_rel = blob_abs - index_offset;
        if blob_rel <= count && !described {
            push_diagnostic(
                g,
                sink,
                Diagnostic {
                    cause: None,
                    code: "StreamableLayoutError".to_string(),
                    detail: format!(
                        "covered blob {digest} delivered before its stream:digest \
                     description (catalog-before-payload, §3.3)"
                    ),
                    frame_index: Some(*blob_abs),
                }
                .with_native_cause(LayoutError::BlobOrder {
                    digest: digest.clone(),
                    frame: *blob_abs,
                    covered: count,
                }),
            );
            if sink.as_deref().is_some_and(|sink| !sink.continue_reading()) {
                return StreamableInfo::default();
            }
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
    index_offset: u64,
    sink: &mut Option<&mut dyn StreamingSink>,
) {
    for record in index_records {
        let Some(root) = &record.mmr else {
            continue;
        };
        let rel_pos = record.abs_index.saturating_sub(index_offset);
        let preceding = rel_pos.saturating_sub(1);
        let mut error = None;
        if root.len() != 32 {
            error = Some(LayoutError::MmrWidth { root: root.clone() });
        } else if record.count > preceding {
            error = Some(LayoutError::MmrPreceding {
                count: record.count,
                preceding,
            });
        } else if record.count > frame_ids.len() as u64 {
            error = Some(LayoutError::MmrCount {
                count: record.count,
                frames: frame_ids.len(),
            });
        } else if record.count > 0
            && usize::try_from(record.count - 1)
                .ok()
                .and_then(|index| frame_ids.get(index))
                .is_none_or(|id| *id != record.head)
        {
            error = Some(LayoutError::MmrHead {
                head: record.head.clone(),
                actual: usize::try_from(record.count - 1)
                    .ok()
                    .and_then(|index| frame_ids.get(index))
                    .cloned(),
            });
        } else {
            let computed = mmr::root(
                &frame_ids[..usize::try_from(record.count)
                    .expect("count admitted against bounded frame table")],
            );
            if computed != *root {
                error = Some(LayoutError::MmrRoot {
                    root: root.clone(),
                    computed,
                });
            }
        }
        if let Some(error) = error {
            push_diagnostic(
                g,
                sink,
                Diagnostic {
                    cause: None,
                    code: "IndexMmrError".to_string(),
                    detail: error.to_string(),
                    frame_index: Some(record.abs_index),
                }
                .with_native_cause(error),
            );
            if sink.as_deref().is_some_and(|sink| !sink.continue_reading()) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IndexRecord, check_index_mmr, layout_check};
    use crate::model::Graph;
    use crate::reader::StreamingSink;

    #[derive(Default)]
    struct CancelOnDiagnostic(usize);

    impl StreamingSink for CancelOnDiagnostic {
        fn continue_reading(&self) -> bool {
            self.0 == 0
        }

        fn diagnostic(&mut self, _diagnostic: &crate::model::Diagnostic) {
            self.0 += 1;
        }
    }

    #[test]
    fn index_and_layout_diagnostic_loops_stop_at_cancellation() {
        let records = [0_u64, 1].map(|abs_index| IndexRecord {
            abs_index,
            count: 0,
            head: Vec::new(),
            mmr: Some(Vec::new()),
        });
        let mut graph = Graph::default();
        let mut sink = CancelOnDiagnostic::default();
        let mut sink_slot: Option<&mut dyn StreamingSink> = Some(&mut sink);
        check_index_mmr(&mut graph, &records, &[], 0, &mut sink_slot);
        assert_eq!(sink.0, 1);

        let header = vec![("layout".into(), "streamable".into())];
        let frame_ids = vec![vec![1], vec![2], vec![3]];
        let index = [IndexRecord {
            abs_index: 3,
            count: 2,
            head: frame_ids[1].clone(),
            mmr: None,
        }];
        let blobs = vec![(1, "one".to_owned(), false), (2, "two".to_owned(), false)];
        let mut sink = CancelOnDiagnostic::default();
        let mut sink_slot: Option<&mut dyn StreamingSink> = Some(&mut sink);
        layout_check(
            &mut graph,
            &header,
            &index,
            &blobs,
            &frame_ids,
            0,
            &mut sink_slot,
        );
        assert_eq!(sink.0, 1);
    }
}
