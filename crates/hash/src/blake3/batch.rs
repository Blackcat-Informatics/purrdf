// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admission and mode selection for independent-message BLAKE3.

use super::{Backend, CHUNK, Hash, KEYED_HASH, Mode, hash_mode, hash_on};
use crate::dispatch::Backend as _;

/// Maximum bytes per independent message in a batch. Longer messages can use
/// the one-shot and streaming APIs.
pub const MAX_BATCH_MESSAGE: usize = 4 * CHUNK;

/// Refusal to hash a batch. Both failures leave the output unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BatchError {
    /// The caller must provide one output slot per message.
    OutputCount {
        /// Number of input messages.
        messages: usize,
        /// Number of output slots.
        slots: usize,
    },
    /// One message exceeds [`MAX_BATCH_MESSAGE`].
    MessageLength {
        /// Zero-based input index.
        index: usize,
        /// Rejected message length.
        bytes: usize,
    },
}

impl core::fmt::Display for BatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputCount { messages, slots } => {
                write!(
                    f,
                    "BLAKE3 batch has {messages} messages and {slots} output slots"
                )
            }
            Self::MessageLength { index, bytes } => {
                write!(
                    f,
                    "BLAKE3 batch message {index} has {bytes} bytes, limit {MAX_BATCH_MESSAGE}"
                )
            }
        }
    }
}
impl core::error::Error for BatchError {}
impl crate::error::StructuredError for BatchError {
    crate::error_visit! {
        OutputCount { messages, slots } => "blake3-batch-output-count" {
            "messages" => Unsigned(*messages as u128), "slots" => Unsigned(*slots as u128)
        },
        MessageLength { index, bytes } => "blake3-batch-message-length" {
            "index" => Unsigned(*index as u128), "bytes" => Unsigned(*bytes as u128)
        },
    }
}

/// A reusable BLAKE3 derive-key context. Hashes the application-specific
/// context string once and stores only its derived key.
#[derive(Clone)]
pub struct DeriveKeyContext(Mode);

impl DeriveKeyContext {
    /// Prepare a unique application context, not a password salt.
    #[must_use]
    pub fn new(context: &str) -> Self {
        Self(Mode::derive(context, None))
    }

    /// Derive 32 bytes without rehashing the context.
    #[must_use]
    #[inline]
    pub fn derive(&self, material: &[u8]) -> [u8; 32] {
        hash_mode(material, &self.0).0
    }

    /// Derive 32 bytes per independent input without rehashing the context.
    /// Results are in input order in caller storage.
    ///
    /// # Errors
    /// Rejects a slot count mismatch or a message over [`MAX_BATCH_MESSAGE`].
    pub fn derive_batch(&self, inputs: &[&[u8]], out: &mut [Hash]) -> Result<(), BatchError> {
        batch_on(inputs, out, self.0, Backend::selected())
    }
}

crate::debug_non_exhaustive!(DeriveKeyContext {});

/// Hash independent messages in input order using the available vector lanes.
/// Empty batches are valid and use no output slots.
///
/// # Errors
/// Rejects a slot count mismatch or a message over [`MAX_BATCH_MESSAGE`].
pub fn hash_batch(inputs: &[&[u8]], out: &mut [Hash]) -> Result<(), BatchError> {
    batch_on(inputs, out, Mode::HASH, Backend::selected())
}

/// Authenticate independent messages with one BLAKE3 key, retaining input order.
///
/// # Errors
/// Rejects a slot count mismatch or a message over [`MAX_BATCH_MESSAGE`].
pub fn keyed_hash_batch(
    key: &[u8; 32],
    inputs: &[&[u8]],
    out: &mut [Hash],
) -> Result<(), BatchError> {
    batch_on(
        inputs,
        out,
        Mode::keyed(key, KEYED_HASH),
        Backend::selected(),
    )
}

impl Backend {
    /// Hash a batch on this path. Returns `None` if this target cannot run it.
    ///
    /// # Errors
    /// Rejects the same inputs as [`hash_batch`].
    pub fn hash_batch(self, inputs: &[&[u8]], out: &mut [Hash]) -> Option<Result<(), BatchError>> {
        self.is_available()
            .then(|| batch_on(inputs, out, Mode::HASH, self))
    }

    /// Authenticate a batch on this path, if it can execute here.
    ///
    /// # Errors
    /// Rejects the same inputs as [`keyed_hash_batch`].
    pub fn keyed_hash_batch(
        self,
        key: &[u8; 32],
        inputs: &[&[u8]],
        out: &mut [Hash],
    ) -> Option<Result<(), BatchError>> {
        self.is_available()
            .then(|| batch_on(inputs, out, Mode::keyed(key, KEYED_HASH), self))
    }

    /// Derive a batch on this path, if it can execute here.
    ///
    /// # Errors
    /// Rejects the same inputs as [`DeriveKeyContext::derive_batch`].
    pub fn derive_key_batch(
        self,
        context: &DeriveKeyContext,
        inputs: &[&[u8]],
        out: &mut [Hash],
    ) -> Option<Result<(), BatchError>> {
        self.is_available()
            .then(|| batch_on(inputs, out, context.0, self))
    }
}

fn batch_on(
    inputs: &[&[u8]],
    out: &mut [Hash],
    mode: Mode,
    backend: Backend,
) -> Result<(), BatchError> {
    if out.len() != inputs.len() {
        return Err(BatchError::OutputCount {
            messages: inputs.len(),
            slots: out.len(),
        });
    }
    if let Some(index) = inputs
        .iter()
        .position(|input| input.len() > MAX_BATCH_MESSAGE)
    {
        return Err(BatchError::MessageLength {
            index,
            bytes: inputs[index].len(),
        });
    }
    if backend == Backend::Portable {
        for (&input, slot) in inputs.iter().zip(out) {
            *slot = hash_on(input, backend, mode);
        }
    } else {
        // Availability is established before taking any of the named paths.
        crate::arch::blake3_messages_on(backend, inputs, mode, out)
            .expect("selected or explicitly checked BLAKE3 backend is available");
    }
    Ok(())
}
