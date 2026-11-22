// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A blob whose transformed payload cannot be a stream of its codec folds as an
//! opaque node with a `DamagedFrame` diagnostic, and the frames around it still
//! fold. The file is built here and corrupted in place, so no vector is copied.

use purrdf_gts::reader::read;
use purrdf_gts::wire::digest_str;
use purrdf_gts::writer::Writer;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// A file of a plain blob, a zstd-transformed blob and a plain blob, with the
/// transformed blob's zstd magic overwritten by zeros of the same length.
fn file_with_damaged_middle_blob() -> (Vec<u8>, String) {
    let middle = b"middle blob payload, transformed".to_vec();
    let middle_digest = digest_str(&middle);
    let mut writer = Writer::new("generic");
    writer.add_blob(b"first", Some("text/plain"), None);
    writer
        .add_blob_transformed(
            middle,
            Some("text/plain"),
            None,
            &["zstd".to_string()],
            None,
        )
        .expect("zstd blob writes");
    writer.add_blob(b"last", Some("text/plain"), None);
    let mut bytes = writer.into_bytes();
    let at = bytes
        .windows(ZSTD_MAGIC.len())
        .position(|window| window == ZSTD_MAGIC)
        .expect("the transformed blob carries a zstd frame");
    bytes[at..at + ZSTD_MAGIC.len()].fill(0);
    (bytes, middle_digest)
}

#[test]
fn a_blob_that_is_not_a_zstd_stream_is_an_opaque_damaged_node() {
    let (bytes, damaged) = file_with_damaged_middle_blob();
    let graph = read(&bytes, true, None);

    assert!(
        graph.diagnostics.iter().any(|d| d.code == "DamagedFrame"),
        "{:?}",
        graph.diagnostics
    );
    assert!(
        graph.opaque.iter().any(|node| node.reason == "damaged"),
        "{:?}",
        graph.opaque
    );
    assert!(
        !graph.blobs.iter().any(|(digest, _)| *digest == damaged),
        "the damaged blob must not be admitted"
    );
}

#[test]
fn the_frames_around_a_damaged_blob_still_fold_with_their_bytes() {
    let (bytes, _damaged) = file_with_damaged_middle_blob();
    let mut graph = read(&bytes, true, None);

    let first = digest_str(b"first");
    let last = digest_str(b"last");
    let admitted: Vec<&str> = graph
        .blobs
        .iter()
        .map(|(digest, _)| digest.as_str())
        .collect();
    assert!(admitted.contains(&first.as_str()), "{admitted:?}");
    assert!(admitted.contains(&last.as_str()), "{admitted:?}");
    assert_eq!(
        graph.decoded_blobs().expect("valid blobs decode"),
        vec![(first, b"first".to_vec()), (last, b"last".to_vec()),]
    );
}
