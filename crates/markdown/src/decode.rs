// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The decode law: the graph back to the document, byte for byte.
//!
//! The emission covers every byte of the source — a unit's plain text
//! over its byte span, a section's typed verbatim heading line over its
//! own span, a structure node's typed verbatim run over the bytes
//! nothing else owns — so a consumer holding only the claims rebuilds
//! the document by writing each covered span's bytes at its offset and
//! proving the result against the document node's `sourceDigest`.
//!
//! The law itself is the **kernel's**: [`reconstruct`], [`VerbatimSpan`]
//! and [`ReconstructError`] are `purrdf-core`'s cover law, re-exported
//! here — the same discipline [`verify_unit`](crate::verify_unit)
//! keeps for a unit's binding to its bytes. This crate is the cover
//! law's first emitter and not its owner: the next structured format
//! that plugs in at the dialect seam decodes under the very same
//! function, so two codecs cannot drift apart on what a lawful cover
//! is.
//!
//! # What the caller extracts, and by what rule
//!
//! One [`VerbatimSpan`] per text-carrying fact of the graph, read off
//! the claims by two rules and no class dispatch:
//!
//! * a node stating the vocabulary's `verbatim` contributes its
//!   `verbatimStart`, `verbatimEnd` and the literal's lexical form;
//! * a node stating the vocabulary's `text` — a unit — contributes its
//!   `byteStart`, `byteEnd` and that plain literal, with `continues`
//!   resolved to the named piece's own byte span when the edge is
//!   present.

pub use purrdf_core::cover::{ReconstructError, VerbatimSpan, reconstruct};

use crate::model::Document;

/// The model's own spans, as [`reconstruct`] takes them: what the
/// projection is about to emit, read off the model rather than back out
/// of the graph. It is the write-side half of the codec's verification
/// — [`render`](crate::render) holds the emission against it before
/// handing a claim out — and the tests' independent read of the same
/// law.
pub(crate) fn model_spans<'d>(document: &Document<'d>) -> Vec<VerbatimSpan<'d>> {
    let mut spans = Vec::new();
    for section in document.sections() {
        let heading = section.heading_span();
        spans.push(VerbatimSpan {
            byte_start: heading.start,
            byte_end: heading.end,
            text: document.structure_text(heading),
            continues: None,
        });
    }
    for unit in document.units() {
        let span = unit.span();
        spans.push(VerbatimSpan {
            byte_start: span.start,
            byte_end: span.end,
            text: unit.quote(),
            continues: unit.continues().map(|i| {
                let piece = document.units()[i].span();
                (piece.start, piece.end)
            }),
        });
    }
    for span in document.structures() {
        spans.push(VerbatimSpan {
            byte_start: span.start,
            byte_end: span.end,
            text: document.structure_text(*span),
            continues: None,
        });
    }
    spans
}
