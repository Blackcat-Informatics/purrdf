// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The present canonical labelling against the queue-refinement labelling,
//! over one in-memory dataset view.
//!
//! ```ignore
//! use purrdf_core::label32::{compare::compare_view, view::Presentation, Options};
//! let report = compare_view(&*dataset, Presentation::Overlay, 4_194_304, Options::DEFAULT);
//! let old = report.old.as_ref().ok();            // the present search's digest
//! let new = report.new.as_ref().ok();            // the new labelling's outcome
//! let erased = report.erased_input;               // the label-erased digest both share
//! ```

use std::convert::Infallible;

use sha2::{Digest, Sha256};

use super::view::{Presentation, lines_from_view};
use super::{Input, Labelled, Options, Refusal, optimised};
use crate::{
    CanonError, CanonHash, CanonicalizationControl, ContentDigest, DatasetView, SearchPolicy,
    try_canonical_sha256_view,
};

/// What the present search did.
#[derive(Debug)]
pub struct OldRun {
    /// The digest of its canonical document, or why it refused.
    pub outcome: Result<ContentDigest, CanonError<Infallible>>,
    /// Its charged work events.
    pub work: u64,
}

/// What the queue-refinement labelling did.
#[derive(Debug)]
pub struct NewRun {
    /// SHA-256 of the result document (the spelled lines with a blank node,
    /// sorted, one per row).
    pub digest: [u8; 32],
    /// The labelling: positions, statistics record and work.
    pub labelled: Labelled,
}

/// The comparison of the two labellings over one view.
#[derive(Debug)]
pub struct Comparison {
    /// The lines with a blank node the view yields in the chosen presentation.
    pub lines: u64,
    /// Its blank nodes.
    pub blank_nodes: u64,
    pub old: OldRun,
    pub new: Result<NewRun, Refusal>,
    /// SHA-256 of the input's lines with every blank token erased to `_:`,
    /// sorted. Both results rename the nodes of these typed lines and nothing
    /// else, so each erases to this digest by construction; the present
    /// search exposes no typed result lines from which to recompute its own.
    pub erased_input: [u8; 32],
}

/// SHA-256 of the sorted lines of `input`, each blank token written by `token`.
fn lines_digest(input: &Input, mut token: impl FnMut(u32, &mut Vec<u8>)) -> [u8; 32] {
    let mut rows: Vec<Vec<u8>> = input
        .lines
        .iter()
        .map(|line| {
            let mut out = Vec::with_capacity(line.text.len());
            line.spell(&mut out, &mut token);
            out
        })
        .collect();
    rows.sort_unstable();
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(&row);
        hasher.update(b"\n");
    }
    hasher.finalize().into()
}

/// Compare the present labelling (`old_budget` charged events, overlay
/// presentation, SHA-256) with the queue-refinement labelling under `options`
/// over the lines of `view` in `presentation`.
pub fn compare_view<D: DatasetView<ReadError = Infallible>>(
    view: &D,
    presentation: Presentation,
    old_budget: u64,
    options: Options,
) -> Comparison {
    let mut control = CanonicalizationControl::new(SearchPolicy::Finite(old_budget), None);
    let outcome = try_canonical_sha256_view(view, CanonHash::Sha256, &mut control);
    let work = control.receipts().last().map_or(0, |r| r.work.total());
    let text = lines_from_view(view, presentation);
    let input = Input::read(&text).expect("the view's lines tokenise");
    let erased_input = lines_digest(&input, |_, o| o.extend_from_slice(b"_:"));
    let new = optimised::label_input(&input, options).map(|labelled| NewRun {
        digest: Sha256::digest(labelled.document.as_bytes()).into(),
        labelled,
    });
    Comparison {
        lines: input.lines.len() as u64,
        blank_nodes: input.nodes() as u64,
        old: OldRun { outcome, work },
        new,
        erased_input,
    }
}
