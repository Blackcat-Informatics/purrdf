// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native file reads for a caller's independently certified snapshot.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

use super::{SegmentedError, SegmentedProvider, SegmentedSnapshot};

/// A read-only provider over an opened native file and its certified identity.
/// The caller supplies the independently trusted receipt at session opening;
/// this adapter neither certifies data nor invents an identity from file bytes.
/// Its length is the attached snapshot range, including the authentication tree.
pub struct SegmentedFileProvider {
    file: Mutex<File>,
    bytes: u64,
    snapshot: SegmentedSnapshot,
}
purrdf_hash::debug_non_exhaustive!(SegmentedFileProvider { bytes, snapshot });

impl SegmentedFileProvider {
    /// Attach an already opened file to its caller-supplied snapshot identity.
    ///
    /// # Errors
    /// Returns the original host metadata error before constructing the provider.
    pub fn from_file(file: File, snapshot: SegmentedSnapshot) -> Result<Self, SegmentedError> {
        let bytes = file
            .metadata()
            .map_err(|error| SegmentedError::Provider {
                operation: "file metadata",
                host_code: error.raw_os_error(),
            })?
            .len();
        Ok(Self {
            file: Mutex::new(file),
            bytes,
            snapshot,
        })
    }
}

impl SegmentedProvider for SegmentedFileProvider {
    fn snapshot(&self) -> SegmentedSnapshot {
        self.snapshot
    }
    fn byte_len(&self) -> u64 {
        self.bytes
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        if position
            .checked_add(u64::try_from(output.len()).map_err(|_| SegmentedError::AddressExhausted)?)
            .is_none_or(|end| end > self.bytes)
        {
            return Err(SegmentedError::Corrupt(
                "file read escapes attached snapshot",
            ));
        }
        let mut file = self.file.lock().map_err(|_| SegmentedError::Provider {
            operation: "file mutex",
            host_code: None,
        })?;
        file.seek(SeekFrom::Start(position))
            .map_err(|error| SegmentedError::Provider {
                operation: "file seek",
                host_code: error.raw_os_error(),
            })?;
        file.read_exact(output)
            .map_err(|error| SegmentedError::Provider {
                operation: "file read_exact",
                host_code: error.raw_os_error(),
            })
    }
}
