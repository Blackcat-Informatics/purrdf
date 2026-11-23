// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reader that exposes one malformed tail item before a later transport failure.

pub(crate) struct TailDrainFailureReader {
    bytes: Vec<u8>,
    position: usize,
    tail_start: usize,
    message: &'static str,
}

impl TailDrainFailureReader {
    pub(crate) fn new(bytes: Vec<u8>, tail_start: usize, message: &'static str) -> Self {
        Self {
            bytes,
            position: 0,
            tail_start,
            message,
        }
    }
}

impl std::io::Read for TailDrainFailureReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.position == self.bytes.len() {
            return Err(std::io::Error::other(self.message));
        }
        let end = if self.position < self.tail_start {
            self.tail_start
        } else {
            self.position.saturating_add(1)
        }
        .min(self.bytes.len());
        let count = output.len().min(end.saturating_sub(self.position));
        output[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
