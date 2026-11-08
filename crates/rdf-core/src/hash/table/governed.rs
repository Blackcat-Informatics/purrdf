// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed control adapters for the single native table engine.

use core::convert::Infallible;
use purrdf_lex::control::ControlledError;

pub(super) fn resident<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}

pub(super) fn native<T, N>(result: Result<T, ControlledError<N, Infallible>>) -> Result<T, N> {
    match result {
        Ok(value) => Ok(value),
        Err(ControlledError::Native(cause)) => Err(cause),
        Err(ControlledError::Control(never)) => match never {},
    }
}

use super::alloc::{Allocator, Global};
use super::raw::FullBucketsIndices;
use super::{HashTable, TryReserveError};
use purrdf_lex::control::Progress;

/// An insertion refused before publishing its incoming value.
/// The original value remains owned by the caller; existing entries stay valid.
#[derive(Debug)]
pub struct InsertRefusal<T, E> {
    /// The unchanged value that was not inserted.
    pub value: T,
    /// Original caller refusal or native physical allocation failure.
    pub cause: ControlledError<TryReserveError, E>,
}

/// A borrowed native full-bucket walk with fallible group advancement.
///
/// The table stays borrowed throughout the walk. A refused step leaves the
/// iterator valid and returns the original caller error; the caller explicitly
/// chooses whether to attempt another step with its work authority.
pub struct ControlledIter<'a, T, A: Allocator = Global> {
    pub(super) table: &'a HashTable<T, A>,
    pub(super) indices: FullBucketsIndices,
}

impl<T, A: Allocator> core::fmt::Debug for ControlledIter<'_, T, A> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_struct("ControlledIter")
            .field("remaining", &self.indices.len())
            .finish_non_exhaustive()
    }
}

impl<'a, T, A: Allocator> ControlledIter<'a, T, A> {
    /// Advance through the same native control groups and bucket order.
    ///
    /// # Errors
    /// Returns the caller's unchanged error before the next group scan or yield.
    /// No further callback is invoked after refusal in this attempt.
    pub fn next_with_control<C: Progress + ?Sized>(
        &mut self,
        progress: &mut C,
    ) -> Result<Option<&'a T>, C::Error> {
        let Some(index) = self.indices.next_with_control(progress)? else {
            return Ok(None);
        };
        // The native full-bucket iterator yields only occupied indices; the
        // shared table borrow prevents removal, reallocation or mutation.
        let value = self
            .table
            .raw
            .get_bucket(index)
            .expect("native full-bucket index remains occupied under its table borrow");
        Ok(Some(value))
    }
}
