// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical exact-file source manifest for Columnar schema v1.

use purrdf_core::ContentDigest;
use purrdf_hash::frame::frame_le;

use crate::{ColumnarError, ParquetFiles};

/// Describe all five actual files in canonical schema order.
///
/// The versioned bytes identify physical files, including valid zero-row files.
/// This function hashes bytes; it does not validate Parquet or certify RDF.
/// A source certifier must also complete the strict [`crate::read`] path.
///
/// # Errors
/// Refuses a file length that cannot be represented by the manifest.
pub fn source_manifest(files: &ParquetFiles) -> Result<Vec<u8>, ColumnarError> {
    let mut output = Vec::new();
    output.extend_from_slice(b"PURRCOLM");
    output.extend_from_slice(&1_u32.to_le_bytes());
    output.extend_from_slice(&5_u32.to_le_bytes());
    for (table, bytes) in files.iter() {
        let length = u64::try_from(bytes.len()).map_err(|_| ColumnarError::ManifestLength {
            table,
            length: bytes.len(),
        })?;
        frame_le(&mut output, table.file_name().as_bytes());
        output.extend_from_slice(&length.to_le_bytes());
        output.extend_from_slice(ContentDigest::of(bytes).as_bytes());
    }
    Ok(output)
}
