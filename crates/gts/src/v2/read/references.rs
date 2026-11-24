// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;

use crate::v2::fields::Fields;
use crate::v2::references::{LocalTerms, ReferenceLocation, decode_bindings, decode_suppressions};
use crate::v2::{FrameType, ReadError, ReadFailure};

use super::{GraphReader, Segment};

/// Completeness and authentication of the file's suppression record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Visibility {
    /// Every suppression frame folded and no sealed frame remains unopened.
    Complete {
        /// Folded suppression frames that do not authenticate.
        unauthenticated_suppress: u64,
    },
    /// Some frames could not contribute to the suppression record.
    Incomplete {
        /// Suppression envelopes that did not fold.
        unfolded_suppress: u64,
        /// Sealed frames that could not be opened.
        unopened_sealed: u64,
        /// Folded suppression frames that do not authenticate.
        unauthenticated_suppress: u64,
    },
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Complete {
            unauthenticated_suppress: 0,
        }
    }
}

#[derive(Default)]
pub(super) struct VisibilityProgress {
    suppress: u64,
    folded: u64,
    unauthenticated: u64,
    sealed: u64,
}

impl VisibilityProgress {
    pub(super) fn observe(&mut self, kind: Option<u64>, registry: Option<NonZeroU64>) {
        const SEALED: u64 = 18;
        if kind == Some(FrameType::Suppress as u64) {
            self.suppress += 1;
        }
        if kind == Some(SEALED) && registry.is_some_and(|version| version.get() >= 2) {
            self.sealed += 1;
        }
    }

    pub(super) const fn settle_segment(&mut self, segment: &Segment<'_>) {
        self.folded += segment.authentication.folded_suppressions();
        self.unauthenticated += segment.authentication.unauthenticated_suppressions();
    }

    pub(super) const fn finish(&self) -> Visibility {
        let unfolded_suppress = self.suppress - self.folded;
        if unfolded_suppress == 0 && self.sealed == 0 {
            Visibility::Complete {
                unauthenticated_suppress: self.unauthenticated,
            }
        } else {
            Visibility::Incomplete {
                unfolded_suppress,
                unopened_sealed: self.sealed,
                unauthenticated_suppress: self.unauthenticated,
            }
        }
    }
}

impl GraphReader<'_, '_> {
    pub(super) fn observe_reference_frame(
        &self,
        kind: FrameType,
        mut content: Fields<'_>,
        payload: &[u8],
        offset: usize,
        segment: &mut Segment<'_>,
    ) -> Result<(), ReadFailure> {
        content.record(1)?;
        let count = content.uint()?;
        let at = ReferenceLocation {
            segment: segment.file_ordinal,
            offset,
        };
        let scope = segment.dictionary.row_scope();
        let terms = LocalTerms::new(&segment.dictionary, scope, &self.graph.rdf);
        match kind {
            FrameType::Bindings => {
                let entries = decode_bindings(payload, count, segment.registry)
                    .map_err(ReadFailure::Content)?;
                segment.labelling.rows_or_entries(entries.len());
                for entry in &entries {
                    segment.labelling.leaf(entry.term)?;
                }
                segment.references.bindings(at, entries, &terms)
            }
            FrameType::Suppress => {
                let entries = decode_suppressions(payload, count, segment.registry)
                    .map_err(ReadFailure::Content)?;
                segment.labelling.rows_or_entries(entries.len());
                for entry in &entries {
                    entry.try_map(|leaf| segment.labelling.leaf(leaf))?;
                }
                segment.references.suppressions(at, entries, &terms)
            }
            _ => return Err(ReadFailure::Frame(kind)),
        }
        .map_err(ReadFailure::Fold)
    }

    pub(super) fn settle_reference_frames(
        &mut self,
        segment: &mut Segment<'_>,
        committed: bool,
        offset: usize,
    ) -> Result<(), ReadError> {
        let first_new = self.graph.reference_findings.len();
        segment
            .references
            .settle(
                committed,
                &mut self.reference_claims,
                &mut self.graph.reference_findings,
            )
            .map_err(|failure| ReadError {
                offset,
                failure: ReadFailure::Fold(failure),
            })?;
        if let Some(finding) = self.graph.reference_findings.get(first_new) {
            self.graph.note_content_omission(finding.at.offset);
        }
        Ok(())
    }
}
