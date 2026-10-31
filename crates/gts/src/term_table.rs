// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same validation view over materialized and streaming term ownership.

use std::ops::Index;
use std::sync::Arc;

use crate::model::Term;

/// Full terms remain available until segment validation finishes. Streaming
/// readers share their owners with the resolver instead of copying strings.
#[derive(Clone, Copy)]
pub(crate) enum TermsRef<'a> {
    Owned(&'a [Term]),
    Shared(&'a [Arc<Term>]),
}

impl<'a> TermsRef<'a> {
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Owned(terms) => terms.len(),
            Self::Shared(terms) => terms.len(),
        }
    }

    pub(crate) fn get(self, id: usize) -> Option<&'a Term> {
        match self {
            Self::Owned(terms) => terms.get(id),
            Self::Shared(terms) => terms.get(id).map(Arc::as_ref),
        }
    }
}

impl Index<usize> for TermsRef<'_> {
    type Output = Term;

    fn index(&self, id: usize) -> &Self::Output {
        match self {
            Self::Owned(terms) => &terms[id],
            Self::Shared(terms) => &terms[id],
        }
    }
}
