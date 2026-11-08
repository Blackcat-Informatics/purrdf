// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed canonical node encoding shared by owned terms and native summaries.

use purrdf_hash::frame::try_frame_le_with;
use purrdf_lex::allocation::StorageError;

use super::{BlankScope, RdfTextDirection};

/// A node's own fields, with triple children traversed by its caller. Borrowed
/// literal fields can enter a digest without constructing an owned TermValue.
#[derive(Clone, Copy)]
pub(crate) enum CanonicalNode<'a> {
    Iri(&'a str),
    Blank {
        label: &'a str,
        scope: BlankScope,
    },
    Literal {
        lexical: &'a str,
        datatype: &'a str,
        language: Option<&'a str>,
        direction: Option<RdfTextDirection>,
    },
    Triple,
}

impl CanonicalNode<'_> {
    /// The published canonical byte tags. These are not the separate
    /// IRI/literal/blank/triple ranks used by total term ordering.
    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::Iri(_) => 0,
            Self::Blank { .. } => 1,
            Self::Literal { .. } => 2,
            Self::Triple => 3,
        }
    }

    /// Exact append size, checked before a destination grows. This visits only
    /// field descriptors; it neither allocates nor walks their string bytes.
    pub(crate) fn byte_len(self) -> Result<usize, StorageError> {
        let mut length = Some(0_usize);
        self.visit_bytes(&mut |bytes| {
            length = length.and_then(|length| length.checked_add(bytes.len()));
        });
        length.ok_or(StorageError::SizeOverflow)
    }

    /// Emit the canonical encoding as borrowed chunks. A digest consumes these
    /// directly; a byte destination reserves byte_len first and then appends.
    pub(crate) fn visit_bytes(self, output: &mut impl FnMut(&[u8])) {
        let result = self.try_visit_bytes(&mut |bytes| {
            output(bytes);
            Ok::<_, core::convert::Infallible>(())
        });
        match result {
            Ok(()) => {}
            Err(never) => match never {},
        }
    }

    /// Emit the original byte law, stopping before every later field on refusal.
    pub(crate) fn try_visit_bytes<E>(
        self,
        output: &mut impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        output(&[self.tag()])?;
        match self {
            Self::Iri(iri) => try_frame_le_with(iri.as_bytes(), output)?,
            Self::Blank { label, scope } => {
                try_frame_le_with(label.as_bytes(), &mut *output)?;
                output(&scope.ordinal().to_le_bytes())?;
            }
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                try_frame_le_with(lexical.as_bytes(), &mut *output)?;
                try_frame_le_with(datatype.as_bytes(), &mut *output)?;
                match language {
                    None => output(&[0])?,
                    Some(language) => {
                        output(&[1])?;
                        try_frame_le_with(language.as_bytes(), &mut *output)?;
                    }
                }
                match direction {
                    None => output(&[0])?,
                    Some(direction) => {
                        output(&[1])?;
                        output(&[match direction {
                            RdfTextDirection::Ltr => 0,
                            RdfTextDirection::Rtl => 1,
                        }])?;
                    }
                }
            }
            Self::Triple => {}
        }
        Ok(())
    }
}
