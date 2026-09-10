// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The dialect seam: where a *format* ends and the *law* begins.
//!
//! Everything in this module is Markdown. Everything outside it is not.
//! The crate is deliberately cut along that line, because the two halves
//! change for different reasons: a dialect changes when an author writes
//! something new (a fenced block, a front-matter header, a different
//! marker for a movement), and the law changes when the slicing
//! contract changes (a split, an identity, a projection) — which
//! re-mints every profile in existence. Keeping them in one file made
//! every dialect question look like a law question.
//!
//! # What a reader owes the law
//!
//! A dialect reader is a pure function from the document's text to a
//! [`Reading`]: a title, a flat list of sections in document order, a
//! flat list of units in document order, the concordance rows it could
//! read, and the rows it could not. Its whole contract is:
//!
//! * **Spans are byte offsets into the text it was handed**, and they
//!   are exact — no trimming, no normalization, nothing prepended.
//!   Recognition may look at a trimmed line; a span never may.
//! * **A section's span starts at its opening line.** Where a section
//!   *ends* is not the reader's to say: containment closes it (a
//!   section runs to the next section at its level or above), and that
//!   is the law's rule, applied in [`crate::model`].
//! * **A unit never spans a section boundary.** The split law relies on
//!   it — that is what "never across a heading" means — and it is a
//!   property of the ranges the reader hands over, not something the
//!   splitter can check.
//! * **Sections are in document order and a parent precedes its
//!   children**, so an index into [`Reading::sections`] is also the
//!   section's ordinal.
//! * **Nothing is dropped in silence.** A row of a concordance table
//!   the reader cannot read is reported as a [`DefectiveRow`], not
//!   discarded; the model carries it out to the caller.
//!
//! What the reader is *not* asked for: identity, digests, scalar
//! counts, the containment lattice, the oversize split, or anything
//! about RDF. Those are the law's, they are the same for every dialect,
//! and they live in [`crate::model`], [`crate::split`],
//! [`crate::identity`], and [`crate::claims`].
//!
//! # Plugging in the next format
//!
//! A second structured format is a second module beside
//! [`markdown`] with a `read(&str) -> Reading` of its own, and a
//! profile whose stage description names it. Nothing else moves: the
//! same units get the same split, the same identities, the same
//! lattice, and the same claims. The seam is deliberately a plain
//! function and a plain struct rather than a trait — there is one
//! reader per document and no dispatch to do, so a trait would add a
//! vocabulary without adding a choice.

pub(crate) mod markdown;

use crate::model::RowDefect;

/// What a dialect reader hands the law: the structure it recognized,
/// stated in byte spans over the document it read.
#[derive(Clone, Debug, Default)]
pub(crate) struct Reading {
    /// The document's title: the first non-movement heading, if any.
    pub(crate) title: Option<String>,
    /// Sections in document order; the index is the ordinal.
    pub(crate) sections: Vec<RawSection>,
    /// Units in document order, before the oversize split.
    pub(crate) units: Vec<RawUnit>,
    /// Concordance rows the reader could read.
    pub(crate) rows: Vec<RawRow>,
    /// Concordance rows the reader could not read, each naming why.
    pub(crate) defective_rows: Vec<DefectiveRow>,
}

/// A section as the reader saw it: an opening line and what that line
/// said. Where the section ends is the law's answer, not the reader's.
#[derive(Clone, Debug)]
pub(crate) struct RawSection {
    /// First byte of the opening line.
    pub(crate) start: usize,
    /// One past the last byte of the opening line (no newline).
    pub(crate) line_end: usize,
    /// Depth in the heading tree, from 1.
    pub(crate) level: u32,
    /// The heading text as the dialect reads it.
    pub(crate) heading: String,
    /// Whether the opening line was a movement marker rather than a
    /// heading.
    pub(crate) movement: bool,
}

/// A unit as the reader saw it, before the oversize split.
#[derive(Clone, Debug)]
pub(crate) struct RawUnit {
    /// First byte of the unit.
    pub(crate) start: usize,
    /// One past the unit's last byte.
    pub(crate) end: usize,
    /// The innermost section in force at the unit's start.
    pub(crate) section: Option<usize>,
    /// The verse number, when the unit opened with one.
    pub(crate) verse: Option<u64>,
    /// The heading stack in force at the unit's start, outermost first.
    pub(crate) lineage: Vec<String>,
}

/// One concordance row the reader could read: the verses it covers, the
/// canon sources, the anchors, and the row's own byte span.
#[derive(Clone, Debug)]
pub(crate) struct RawRow {
    /// First verse of the row's range.
    pub(crate) first: u64,
    /// Last verse of the row's range.
    pub(crate) last: u64,
    /// The backticked source paths of the row, in order.
    pub(crate) sources: Vec<String>,
    /// The backticked anchors of the row, in order.
    pub(crate) anchors: Vec<String>,
    /// First byte of the row's line.
    pub(crate) start: usize,
    /// One past the last byte of the row's line (no newline).
    pub(crate) end: usize,
}

/// One concordance row the reader could not read, and why.
#[derive(Clone, Debug)]
pub(crate) struct DefectiveRow {
    /// First byte of the row's line.
    pub(crate) start: usize,
    /// One past the last byte of the row's line (no newline).
    pub(crate) end: usize,
    /// What the reader could not make of it.
    pub(crate) defect: RowDefect,
}
