// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller configuration and native arrays preserve their distinct owners.

use super::{AdmittedVec, WorkspaceCapability};

/// Exclusive array storage without detaching a native buffer's admission.
#[derive(Debug)]
pub enum WorkspaceArray<T> {
    /// Storage supplied by a caller or a resident configuration constructor.
    Caller(Vec<T>),
    /// Storage constructed through its original operational admission.
    Native(AdmittedVec<T>),
}

impl<T> WorkspaceArray<T> {
    /// Native authority, when the array was authored in an operational run.
    #[must_use]
    pub fn workspace(&self) -> Option<&WorkspaceCapability> {
        match self {
            Self::Caller(_) => None,
            Self::Native(values) => Some(values.workspace()),
        }
    }
}
impl<T> std::ops::Deref for WorkspaceArray<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            Self::Caller(values) => values,
            Self::Native(values) => values,
        }
    }
}
impl<T> std::ops::DerefMut for WorkspaceArray<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            Self::Caller(values) => values,
            Self::Native(values) => values,
        }
    }
}
impl<'a, T> IntoIterator for &'a WorkspaceArray<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
