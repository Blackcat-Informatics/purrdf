// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The actual facade carries native omission evidence through rows and files.

use purrdf::{GtsBlobLimits, prepare_gts_columnar};

#[test]
fn both_columnar_outputs_preserve_located_losses_and_owned_causes() {
    let mut writer = purrdf_gts::writer::Writer::new("generic");
    writer.add_blob(b"admitted only with room", None, Some("payload"));
    let bytes = writer.into_bytes();
    let mut policy = GtsBlobLimits::UNBOUNDED;
    policy.max_encoded_bytes = 0;
    let source = prepare_gts_columnar(&bytes, policy).unwrap();
    let (rows, report) = source.project().unwrap();
    assert_eq!(rows.row_count(purrdf_columnar::Table::Blobs), 0);
    assert_eq!(report.omissions().len(), 1);
    let losses = report.losses().entries();
    assert_eq!(losses.len(), 1);
    assert_eq!(losses[0].code, "blob-byte-budget");
    assert_eq!(losses[0].from, "gts");
    assert_eq!(losses[0].to, "columnar-v1");
    assert!(losses[0].location.is_some());
    let (_, files_report) = prepare_gts_columnar(&bytes, policy)
        .unwrap()
        .write(purrdf_columnar::Compression::Uncompressed)
        .unwrap();
    assert_eq!(files_report.losses().entries(), losses);
    drop(bytes);
    assert!(report.omissions()[0].native_cause().is_some());
    let (_, clean) = {
        let mut writer = purrdf_gts::writer::Writer::new("generic");
        writer.add_blob(b"content", None, None);
        prepare_gts_columnar(&writer.into_bytes(), GtsBlobLimits::UNBOUNDED)
            .unwrap()
            .project()
            .unwrap()
    };
    assert_eq!(clean.losses().entries(), []);
    assert!(clean.omissions().is_empty());
}
