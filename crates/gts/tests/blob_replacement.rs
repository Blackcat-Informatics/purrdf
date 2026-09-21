// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Blob replacement preserves insertion order and segment-local metadata.

use ciborium::Value;
use purrdf_gts::model::Graph;
use purrdf_gts::reader::{BlobPayload, StreamingSink, read, read_to_sink};
use purrdf_gts::wire::{digest_str, map_get};
use purrdf_gts::writer::Writer;

#[derive(Debug, PartialEq)]
struct Event {
    segment: usize,
    bytes: Vec<u8>,
    media_type: Option<String>,
    declared: bool,
}

#[derive(Default)]
struct Sink(Vec<Event>);

fn media_type(meta: &Value) -> Option<&str> {
    map_get(meta.as_map()?, "mt")?.as_text()
}

impl StreamingSink for Sink {
    fn blob_payload(&mut self, payload: BlobPayload<'_>) {
        self.0.push(Event {
            segment: payload.segment_index,
            bytes: payload.bytes.unwrap().to_vec(),
            media_type: payload.metadata.and_then(media_type).map(str::to_owned),
            declared: payload.metadata_declared,
        });
    }
}

fn first_segment() -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"a", Some("application/first"), None);
    writer.add_blob(b"b", Some("application/second"), None);
    writer.add_blob(b"a", Some("application/replaced"), None);
    writer.add_frame(
        "snapshot",
        Some(Value::Map(vec![(
            "blobs".into(),
            Value::Map(vec![
                (digest_str(b"a").into(), Value::Bytes(b"a".to_vec())),
                (digest_str(b"c").into(), Value::Bytes(b"c".to_vec())),
            ]),
        )])),
        None,
        None,
        None,
    );
    writer.add_frame("blob", None, Some(b"a".to_vec()), None, None);
    writer.into_bytes()
}

fn assert_blob_order(graph: &mut Graph, expected: &[&[u8]]) {
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(
        graph.decoded_blobs().unwrap(),
        expected
            .iter()
            .map(|bytes| (digest_str(bytes), bytes.to_vec()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn repeated_and_snapshot_blobs_replace_in_place_and_inherit_metadata() {
    let bytes = first_segment();
    let mut graph = read(&bytes, true, None);
    assert_blob_order(&mut graph, &[b"a", b"b", b"c"]);
    assert_eq!(
        graph
            .blob_meta
            .iter()
            .map(|(digest, meta)| (digest.clone(), media_type(meta)))
            .collect::<Vec<_>>(),
        [
            (digest_str(b"a"), Some("application/replaced")),
            (digest_str(b"b"), Some("application/second"))
        ]
    );
    let mut sink = Sink::default();
    let streamed = read_to_sink(&bytes, true, None, &mut sink);
    assert_eq!(streamed.diagnostics, graph.diagnostics);
    assert_eq!(streamed.segment_heads, graph.segment_heads);
    assert_eq!(sink.0.len(), 6);
    for (index, payload, mt, declared) in [
        (0, b"a", Some("application/first"), true),
        (1, b"b", Some("application/second"), true),
        (2, b"a", Some("application/replaced"), true),
        (3, b"a", Some("application/replaced"), false),
        (4, b"c", None, false),
        (5, b"a", Some("application/replaced"), false),
    ] {
        assert_eq!(
            sink.0[index],
            Event {
                segment: 0,
                bytes: payload.to_vec(),
                media_type: mt.map(str::to_owned),
                declared,
            }
        );
    }
}

#[test]
fn segment_union_replaces_in_place_while_streaming_metadata_resets() {
    let mut bytes = first_segment();
    let mut second = Writer::new("generic");
    second.add_frame("blob", None, Some(b"a".to_vec()), None, None);
    second.add_blob(b"d", Some("application/fourth"), None);
    second.add_blob(b"a", Some("application/final"), None);
    bytes.extend(second.into_bytes());
    let mut graph = read(&bytes, true, None);
    assert_blob_order(&mut graph, &[b"a", b"b", b"c", b"d"]);
    assert_eq!(
        graph
            .blob_meta
            .iter()
            .map(|(digest, meta)| (digest.clone(), media_type(meta)))
            .collect::<Vec<_>>(),
        [
            (digest_str(b"a"), Some("application/final")),
            (digest_str(b"b"), Some("application/second")),
            (digest_str(b"d"), Some("application/fourth"))
        ]
    );
    let mut sink = Sink::default();
    let streamed = read_to_sink(&bytes, true, None, &mut sink);
    assert_eq!(streamed.diagnostics, graph.diagnostics);
    assert_eq!(streamed.segment_heads, graph.segment_heads);
    assert_eq!(sink.0.len(), 9);
    assert_eq!(
        sink.0[6],
        Event {
            segment: 1,
            bytes: b"a".to_vec(),
            media_type: None,
            declared: false
        }
    );
    assert_eq!(
        sink.0[8],
        Event {
            segment: 1,
            bytes: b"a".to_vec(),
            media_type: Some("application/final".into()),
            declared: true
        }
    );
}

#[test]
fn replacement_switches_between_lazy_and_decoded_entries_without_reordering() {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"a", Some("application/first"), None);
    writer.add_blob(b"b", Some("application/second"), None);
    writer
        .add_blob_transformed(
            b"a".to_vec(),
            Some("application/compressed"),
            None,
            &["gzip".into()],
            None,
        )
        .unwrap();
    let mut graph = read(&writer.to_bytes(), true, None);
    assert!(graph.blobs[0].1.is_lazy());
    assert_blob_order(&mut graph, &[b"a", b"b"]);
    writer.add_blob(b"a", Some("application/plain"), None);
    let mut graph = read(&writer.into_bytes(), true, None);
    assert!(!graph.blobs[0].1.is_lazy());
    assert_blob_order(&mut graph, &[b"a", b"b"]);
    assert_eq!(media_type(&graph.blob_meta[0].1), Some("application/plain"));
}

#[test]
fn replacement_and_union_cross_the_small_table_boundary_without_stale_positions() {
    for count in [16u8, 17, 64] {
        let mut writer = Writer::new("generic");
        for index in 0..count {
            writer.add_blob(&[index], Some("application/original"), None);
        }
        for index in [0, count / 2, count - 1] {
            writer.add_blob(&[index], Some("application/updated"), None);
        }
        writer.add_frame(
            "snapshot",
            Some(Value::Map(vec![(
                "blobs".into(),
                Value::Map(vec![
                    (digest_str(&[0]).into(), Value::Bytes(vec![0])),
                    (digest_str(&[count]).into(), Value::Bytes(vec![count])),
                ]),
            )])),
            None,
            None,
            None,
        );
        let mut bytes = writer.into_bytes();
        let mut second = Writer::new("generic");
        second.add_blob(&[0], Some("application/final"), None);
        bytes.extend(second.into_bytes());
        let mut graph = read(&bytes, true, None);
        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
        assert_eq!(
            graph.decoded_blobs().unwrap(),
            (0..=count)
                .map(|index| (digest_str(&[index]), vec![index]))
                .collect::<Vec<_>>()
        );
        assert_eq!(graph.blob_meta.len(), usize::from(count));
        for (index, (digest, meta)) in graph.blob_meta.iter().enumerate() {
            let index = u8::try_from(index).unwrap();
            assert_eq!(*digest, digest_str(&[index]));
            let expected = if index == 0 {
                "application/final"
            } else if index == count / 2 || index == count - 1 {
                "application/updated"
            } else {
                "application/original"
            };
            assert_eq!(media_type(meta), Some(expected));
        }
        let mut sink = Sink::default();
        let result = read_to_sink(&bytes, true, None, &mut sink);
        assert_eq!(result.diagnostics, graph.diagnostics);
        assert_eq!(
            sink.0[usize::from(count) + 3].media_type.as_deref(),
            Some("application/updated")
        );
        assert!(!sink.0[usize::from(count) + 3].declared);
        assert_eq!(sink.0[usize::from(count) + 4].media_type, None);
        assert_eq!(
            sink.0.last().unwrap().media_type.as_deref(),
            Some("application/final")
        );
    }
}
