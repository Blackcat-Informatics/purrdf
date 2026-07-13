// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end GTS `zstd` `dct` codec: a writer-pinned pack dictionary must be
//! invisible to the fold (§5 header `"dct"`, §8.5 `zstd` `dct` parameter) and
//! its header entry must be tamper-evident like every other header key.

use purrdf_gts::dict::raw_content_dict;
use purrdf_gts::reader::read;
use purrdf_gts::wire::digest_str;
use purrdf_gts::writer::{FrameOptions, Writer, WriterOptions};

/// A corpus with enough repeated structure to build a dictionary from.
fn corpus() -> Vec<Vec<u8>> {
    (0..400u32)
        .map(|i| {
            format!(
                "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                i % 37,
                i
            )
            .into_bytes()
        })
        .collect()
}

/// A payload sharing structure with the corpus but not identical to it —
/// exactly the case a pack dictionary targets.
fn payload() -> Vec<u8> {
    (0..64u32)
        .flat_map(|i| {
            format!(
                "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                i % 37,
                i + 10_000
            )
            .into_bytes()
        })
        .collect()
}

fn write_blob_frame(writer: &mut Writer, data: Vec<u8>) {
    let digest = digest_str(&data);
    writer
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(data),
                transform: vec!["zstd".to_string()],
                pub_meta: Some(ciborium::value::Value::Map(vec![(
                    "digest".into(),
                    digest.into(),
                )])),
                ..FrameOptions::default()
            },
        )
        .expect("blob frame with zstd transform must write");
}

#[test]
fn dict_pinned_gts_file_folds_identically_to_undictioned_file() {
    let owned = corpus();
    let corpus_slices: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
    let dict = raw_content_dict(&corpus_slices, 4096).expect("dict builds");
    let data = payload();

    // Written with a pinned pack dictionary.
    let mut with_dict = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dict: Some(("pack0".to_string(), dict)),
            ..WriterOptions::default()
        },
    )
    .expect("writer with dict constructs");
    write_blob_frame(&mut with_dict, data.clone());
    let with_dict_bytes = with_dict.into_bytes();

    // Written without a dictionary — same logical content.
    let mut without_dict = Writer::new("purrdf.gts");
    write_blob_frame(&mut without_dict, data.clone());
    let without_dict_bytes = without_dict.into_bytes();

    // The dict pins bytes uncompressed and in-band, so the dict-primed file
    // is not byte-identical to the undictioned one, but both must fold the
    // same blob content with no diagnostics — a compression detail must stay
    // invisible to the fold.
    assert_ne!(
        with_dict_bytes, without_dict_bytes,
        "a pinned dictionary changes the header, so the files must differ byte-for-byte"
    );

    let dict_graph = read(&with_dict_bytes, true, None);
    let plain_graph = read(&without_dict_bytes, true, None);

    assert!(
        dict_graph.diagnostics.is_empty(),
        "dict-primed file must fold cleanly: {:?}",
        dict_graph.diagnostics
    );
    assert!(
        plain_graph.diagnostics.is_empty(),
        "undictioned file must fold cleanly: {:?}",
        plain_graph.diagnostics
    );

    assert_eq!(dict_graph.blobs.len(), 1);
    assert_eq!(plain_graph.blobs.len(), 1);
    let dict_decoded = dict_graph.blobs[0]
        .1
        .decoded_vec()
        .expect("dict-primed blob decodes");
    let plain_decoded = plain_graph.blobs[0]
        .1
        .decoded_vec()
        .expect("undictioned blob decodes");
    assert_eq!(dict_decoded, data);
    assert_eq!(plain_decoded, data);
    assert_eq!(
        dict_decoded, plain_decoded,
        "a pack dictionary is a compression detail, invisible to the fold"
    );
}

#[test]
fn tampering_the_pinned_dictionary_bytes_is_detected_as_a_header_self_hash_mismatch() {
    let owned = corpus();
    let corpus_slices: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
    let dict = raw_content_dict(&corpus_slices, 4096).expect("dict builds");
    let data = payload();

    let mut writer = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dict: Some(("pack0".to_string(), dict.clone())),
            ..WriterOptions::default()
        },
    )
    .expect("writer with dict constructs");
    write_blob_frame(&mut writer, data);
    let mut bytes = writer.into_bytes();

    // The dictionary is stored uncompressed and in-band (§5), so its bytes
    // appear verbatim in the header — locate and flip one to simulate
    // tampering with the pinned "dct" entry.
    let at = bytes
        .windows(dict.len())
        .position(|window| window == dict.as_slice())
        .expect("dictionary bytes must appear verbatim in the written file");
    bytes[at] ^= 0xFF;

    let graph = read(&bytes, true, None);
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|d| d.code == "DamagedFrame" && d.detail.contains("header self-hash mismatch")),
        "tampering the pinned dictionary bytes must be caught as a header self-hash \
         mismatch (fail closed), got diagnostics: {:?}",
        graph.diagnostics
    );
}
