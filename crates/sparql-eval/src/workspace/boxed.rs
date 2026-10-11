// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Mutable native trait payloads retain their original concrete box admission.

use super::{LexicalFrame, WorkspaceCapability};
use crate::EvalError;

/// An exclusive native payload destroyed before its original backing grant.
#[derive(Debug)]
pub struct WorkspaceBox<T: ?Sized> {
    value: Box<T>,
    frame: LexicalFrame,
}

impl<T> WorkspaceBox<T> {
    /// Admit the concrete payload before the original fallible box allocator.
    ///
    /// # Errors
    /// Returns the original layout, admission or allocator refusal.
    pub fn new_admitted(value: T, workspace: &WorkspaceCapability) -> Result<Self, EvalError> {
        let mut frame = LexicalFrame::new(workspace);
        let value = purrdf_lex::allocation::Memory::new(&mut frame)
            .boxed(value)
            .map_err(|error| frame.storage_error(error, "native trait payload"))?;
        Ok(Self { value, frame })
    }
}

impl<T: ?Sized> WorkspaceBox<T> {
    /// Change the box's type through an allocation-free unsizing coercion.
    /// The coercion must retain the same allocation; its original frame moves
    /// with it. No admitted payload is detached into a bare public box.
    #[must_use]
    pub fn coerce<U: ?Sized>(self, coerce: impl FnOnce(Box<T>) -> Box<U>) -> WorkspaceBox<U> {
        WorkspaceBox {
            value: coerce(self.value),
            frame: self.frame,
        }
    }
}

impl<T: ?Sized> std::ops::Deref for WorkspaceBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T: ?Sized> std::ops::DerefMut for WorkspaceBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
