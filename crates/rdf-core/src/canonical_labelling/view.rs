// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The lines of a dataset view, and the comparison of the present canonical
//! form with the labelling over them.
//!
//! Lowering is outside the definition (its section 11): this is the one rule
//! by which PurRDF supplies lines for a dataset view.
//!
//! * Each quad becomes a `quad` line.
//! * Each row of the reifier side table, `(reifier, rdf:reifies, triple term,
//!   graph)`, becomes a `reifier-binding` line of the reifier, the triple term
//!   and the graph.
//! * Each distinct row of the annotation side table becomes an `annotation`
//!   line whose count is the number of identical rows the view yields. A
//!   resident dataset and a composite view yield each annotation row once, so
//!   the count is one; a view that repeats a row carries the repeat in the
//!   count. Two distinct term identifiers that lower to the same terms are two
//!   lines equal but for the count, which the definition refuses (W3).
//! * Each named graph the view addresses that no statement line carries
//!   becomes an `empty-graph` line.
//! * Each caller [`Record`] becomes a `record` line. A record denotes nothing:
//!   it adds no graph and no `empty-graph` line.
//!
//! A blank node's source identifier is its scope ordinal, four bytes
//! big-endian, followed by its label bytes, so two blank nodes are one node
//! exactly when their `(label, scope)` are equal; a blank node of a record is
//! the view's blank node of the same `(label, scope)`. Terms are lowered
//! straight into tokens over a heap work list: no nested term is built and
//! none is walked on the machine stack.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::lines::{
    Admission, Admitter, Direction, Facts, GraphTokens, Kind, TAG_ABSENT, TAG_PRESENT, TAG_TRIPLE,
    Tok, iri_encoding, literal_encoding,
};
use super::{GraphTerm, Labelled, Limit, LineSet, Refusal, optimised};
use crate::governor::is_stopped;
use purrdf_lex::allocation::{Admission as StorageAdmission, Memory, StorageError};

use crate::{
    CanonError, CanonHash, CanonicalizationControl, ContentDigest, DatasetView, RdfTextDirection,
    StopSignal, TermLookupError, TermValue, try_canonical_sha256_view,
};

/// A caller's record beside a view: an opaque payload, compared whole and never
/// read, and an ordered list of terms whose blank nodes are the view's blank
/// nodes of the same `(label, scope)` and occur in the record like anywhere
/// else (3.2 of the definition).
///
/// Borrowed: the labelling copies what it keeps.
#[derive(Clone, Copy, Debug)]
pub struct Record<'a> {
    payload: &'a [u8],
    terms: &'a [TermValue],
}

impl<'a> Record<'a> {
    /// A record of `payload` and `terms`; either may be empty.
    #[must_use]
    pub const fn new(payload: &'a [u8], terms: &'a [TermValue]) -> Self {
        Self { payload, terms }
    }

    /// The payload.
    #[must_use]
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }

    /// The terms, in order.
    #[must_use]
    pub const fn terms(&self) -> &'a [TermValue] {
        self.terms
    }
}

/// Why the lines of a view were not labelled. None of these is a completed
/// result: a refusal is the definition's typed outcome, a read error is the
/// view's own, and a stop is the caller's.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ViewError<E> {
    /// The view could not be read: its own error value, unchanged.
    Read(E),
    /// A row names a term identifier that is not a term of this view.
    ForeignId,
    /// The lines are not well formed, or the engine's limit tripped: the
    /// typed refusal of the definition. [`Refusal::Limit`] carries
    /// `broken_by`, the first failures of the search-free run, and the
    /// engine-specific fields `source`, `nodes` and `units`, which are not
    /// properties of the dataset.
    Refused(Refusal),
    /// The caller's stop signal was set before the labelling completed. This is
    /// not a refusal of the definition: the input was neither accepted nor
    /// refused.
    Stopped,
    /// The caller's admission refused the working state of the labelling
    /// ([`compare_view_with_memory`]); nothing was labelled.
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for ViewError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => error.fmt(f),
            Self::ForeignId => f.write_str("a row names a term identifier the view does not hold"),
            Self::Refused(refusal) => {
                write!(
                    f,
                    "the labelling refused the input (code {})",
                    refusal.code()
                )
            }
            Self::Stopped => f.write_str("the labelling was stopped by the caller"),
            Self::Storage(error) => {
                write!(f, "the labelling's working state was not admitted: {error}")
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ViewError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::ForeignId | Self::Refused(_) | Self::Stopped => None,
        }
    }
}

/// The variant of a term value.
const fn variant_of(value: &TermValue) -> GraphTerm {
    match value {
        TermValue::Iri(_) => GraphTerm::Iri,
        TermValue::Literal { .. } => GraphTerm::Literal,
        TermValue::Blank { .. } => GraphTerm::Blank,
        TermValue::Triple { .. } => GraphTerm::Triple,
    }
}

struct Lowering<'a, 's, D: DatasetView> {
    view: &'a D,
    stop: Option<&'s dyn StopSignal>,
    admitter: Admitter,
    /// The graphs carried by a statement line, by view identifier.
    carried: BTreeSet<D::Id>,
}

impl<D: DatasetView> Lowering<'_, '_, D> {
    fn poll(&self) -> Result<(), ViewError<D::ReadError>> {
        if is_stopped(self.stop) {
            Err(ViewError::Stopped)
        } else {
            Ok(())
        }
    }

    /// The tokens of `value` in walk order, and its variant.
    fn value(&mut self, value: &TermValue, out: &mut Vec<Tok>) -> GraphTerm {
        let interner = &mut self.admitter.interner;
        let mut pending = vec![value];
        while let Some(value) = pending.pop() {
            match value {
                TermValue::Iri(iri) => {
                    let c = interner.constant(&iri_encoding(iri.as_bytes()));
                    out.push(Tok::Const(c));
                }
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => {
                    let direction = direction.map(|d| match d {
                        RdfTextDirection::Ltr => Direction::Ltr,
                        RdfTextDirection::Rtl => Direction::Rtl,
                    });
                    let encoding = literal_encoding(
                        lexical_form.as_bytes(),
                        datatype.as_bytes(),
                        language.as_deref().map(str::as_bytes),
                        direction,
                    );
                    out.push(Tok::Const(interner.constant(&encoding)));
                }
                TermValue::Blank { label, scope } => {
                    let mut source = Vec::with_capacity(4 + label.len());
                    source.extend_from_slice(&scope.ordinal().to_be_bytes());
                    source.extend_from_slice(label.as_bytes());
                    out.push(Tok::Blank(interner.node(&source)));
                }
                TermValue::Triple { s, p, o } => {
                    out.push(Tok::Tag(TAG_TRIPLE));
                    pending.extend([&**o, &**p, &**s]);
                }
            }
        }
        variant_of(value)
    }

    /// The tokens of the term `id`, and its variant.
    fn term(
        &mut self,
        id: D::Id,
        out: &mut Vec<Tok>,
    ) -> Result<GraphTerm, ViewError<D::ReadError>> {
        let value = self.view.term_value(id).map_err(|error| match error {
            TermLookupError::ForeignId => ViewError::ForeignId,
            TermLookupError::Read(error) => ViewError::Read(error),
        })?;
        Ok(self.value(&value, out))
    }

    /// The graph slot of a line: its tokens, and the graph term it carries.
    fn graph(
        &mut self,
        g: Option<D::Id>,
        out: &mut Vec<Tok>,
    ) -> Result<Option<GraphTokens>, ViewError<D::ReadError>> {
        let Some(g) = g else {
            out.push(Tok::Tag(TAG_ABSENT));
            return Ok(None);
        };
        out.push(Tok::Tag(TAG_PRESENT));
        let start = out.len();
        let variant = self.term(g, out)?;
        self.carried.insert(g);
        Ok(Some((out[start..].to_vec(), variant)))
    }

    /// One statement line: the kind, the optional count, the terms `ids`, of
    /// which the second must be of variant `expect` (W1), and the graph.
    fn row(
        &mut self,
        kind: Kind,
        count: Option<u64>,
        ids: &[D::Id],
        expect: GraphTerm,
        g: Option<D::Id>,
    ) -> Result<(), ViewError<D::ReadError>> {
        self.poll()?;
        let mut tokens = vec![Tok::Tag(kind as u8)];
        tokens.extend(count.map(Tok::Count));
        let mut slots_ok = true;
        for (i, &id) in ids.iter().enumerate() {
            let variant = self.term(id, &mut tokens)?;
            if i == 1 && variant != expect {
                slots_ok = false;
            }
        }
        let carried = self.graph(g, &mut tokens)?;
        self.admitter.push(
            tokens,
            Facts {
                slots_ok,
                zero_count: count == Some(0),
                carried,
                held: None,
            },
        );
        Ok(())
    }

    fn lower(&mut self, records: &[Record<'_>]) -> Result<(), ViewError<D::ReadError>> {
        let view = self.view;
        for q in view.quads() {
            self.row(Kind::Quad, None, &[q.s, q.p, q.o], GraphTerm::Iri, q.g)?;
        }
        // A reifier row is `(reifier, rdf:reifies, triple term, graph)`.
        for q in view.reifier_quads() {
            self.row(
                Kind::ReifierBinding,
                None,
                &[q.s, q.o],
                GraphTerm::Triple,
                q.g,
            )?;
        }
        // One line per distinct annotation row, its count the row's multiplicity.
        let mut annotations: BTreeMap<_, u64> = BTreeMap::new();
        for q in view.annotation_quads() {
            let rows = annotations.entry((q.s, q.p, q.o, q.g)).or_default();
            *rows = rows.saturating_add(1);
        }
        for ((s, p, o, g), count) in annotations {
            self.row(Kind::Annotation, Some(count), &[s, p, o], GraphTerm::Iri, g)?;
        }
        for g in view.named_graphs() {
            if self.carried.contains(&g) {
                continue;
            }
            self.poll()?;
            let mut tokens = vec![Tok::Tag(Kind::EmptyGraph as u8)];
            let variant = self.term(g, &mut tokens)?;
            let held = Some((tokens[1..].to_vec(), variant));
            self.admitter.push(
                tokens,
                Facts {
                    slots_ok: true,
                    zero_count: false,
                    carried: None,
                    held,
                },
            );
        }
        for record in records {
            self.poll()?;
            let mut tokens = Vec::new();
            self.admitter
                .interner
                .record_head(record.payload, record.terms.len(), &mut tokens);
            for term in record.terms {
                self.value(term, &mut tokens);
            }
            self.admitter.push(
                tokens,
                Facts {
                    slots_ok: true,
                    zero_count: false,
                    carried: None,
                    held: None,
                },
            );
        }
        Ok(())
    }
}

/// The lowered lines of `view` and `records`, before admission.
fn lowered<D: DatasetView>(
    view: &D,
    records: &[Record<'_>],
    stop: Option<&dyn StopSignal>,
) -> Result<Admission, ViewError<D::ReadError>> {
    let mut lowering = Lowering {
        view,
        stop,
        admitter: Admitter::default(),
        carried: BTreeSet::new(),
    };
    match view.checked_read(|_| lowering.lower(records)) {
        Err(error) => Err(ViewError::Read(error)),
        Ok(Err(error)) => Err(error),
        Ok(Ok(())) => Ok(lowering.admitter.finish()),
    }
}

/// The admitted lines of `view` and `records`: well formed, or the typed
/// refusal of the definition. A view whose rows break W3 or W5 is refused,
/// never repaired.
///
/// Positions, labels and identities computed over these lines are those of the
/// view WITH its records. The same view without its records, or with other
/// records, is a different input: its positions must never be mixed with these.
///
/// # Errors
/// [`ViewError::Read`] with the view's own error when it cannot be read;
/// [`ViewError::ForeignId`] when a row names a term the view does not hold;
/// [`ViewError::Refused`] when the lines are not well formed.
pub fn lines_of_view<D: DatasetView>(
    view: &D,
    records: &[Record<'_>],
) -> Result<LineSet, ViewError<D::ReadError>> {
    lowered(view, records, None)?
        .into_result()
        .map_err(ViewError::Refused)
}

/// The present canonical form against the labelling, over one view.
#[derive(Debug)]
pub struct Comparison<E> {
    /// What the present canonical form did under the caller's control: its
    /// SHA-256 digest, or why it has none.
    pub present: Result<ContentDigest, CanonError<E>>,
    /// The admitted lines the labelling was computed over.
    pub lines: LineSet,
    /// The labelling of `lines`.
    pub labelled: Labelled,
    /// The bytes admitted through the caller's [`Memory`] for this comparison
    /// by [`compare_view_with_memory`], to be released when the comparison is
    /// dropped; zero from [`compare_view`].
    pub admitted_bytes: usize,
}

/// Run the present canonical form under `present` and the labelling over the
/// lines of `view` and `records`.
///
/// The present form takes its policy and its stop signal from `present`, as
/// [`try_canonicalize_view`](crate::try_canonicalize_view) does, and leaves its
/// receipt there; its own refusal is a value of [`Comparison::present`]. The
/// labelling honours the same stop signal, polled while the rows are lowered
/// and at every call of the engine, and has no caller budget: `limit` is the
/// engine's own limit in the limited units of 8.1 of the definition. It trips
/// only past `max(limit, A)` for an input that is not `guaranteed`, as the
/// typed refusal [`Refusal::Limit`], whose `broken_by` names the first
/// failures of the search-free run.
///
/// Positions and labels are those of the view WITH its records, and must
/// never be mixed with those of a run without them. Figures that depend on the
/// source order, such as the twin units of [`Labelled`], are engine figures:
/// they are not invariant under relabelling and are not part of the structural
/// record. Spell a position as a document label with [`write_label`](super::write_label);
/// a dataset read back from those decimal labels as names is not promised the
/// identity map of the definition's Lemma I, because `10` precedes `2` bytewise.
///
/// The labelling does not meter its storage; use [`compare_view_with_memory`]
/// where the caller admits the working state of a computation.
///
/// # Errors
/// [`ViewError::Read`] with the view's own error; [`ViewError::ForeignId`];
/// [`ViewError::Refused`] with the typed refusal; [`ViewError::Stopped`] when
/// the stop signal of `present` is set. Only `Ok` is a completed result.
pub fn compare_view<D: DatasetView>(
    view: &D,
    records: &[Record<'_>],
    present: &mut CanonicalizationControl<'_>,
    limit: Limit,
) -> Result<Comparison<D::ReadError>, ViewError<D::ReadError>> {
    let outcome = try_canonical_sha256_view(view, CanonHash::Sha256, present);
    let stop = present.stop_signal();
    let lines = lowered(view, records, stop)?
        .into_result()
        .map_err(ViewError::Refused)?;
    let labelled = label_lines(&lines, limit, stop)?;
    Ok(Comparison {
        present: outcome,
        lines,
        labelled,
        admitted_bytes: 0,
    })
}

/// Label admitted lines with the optimised engine.
fn label_lines<E>(
    lines: &LineSet,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelled, ViewError<E>> {
    optimised::label_set(
        lines,
        limit.get(),
        false,
        optimised::Options::PRODUCTION,
        stop,
    )
    .map_err(|halt| match halt {
        super::Halt::Refused(refusal) => ViewError::Refused(refusal),
        super::Halt::Stopped => ViewError::Stopped,
    })
}

/// [`compare_view`] with the working state of the labelling admitted once,
/// before the engine starts, through the caller's `memory`.
///
/// The grant covers the lowered lines and an upper bound of the arrays the
/// engine builds for the components and their first calls, proportional to the
/// incidences, the constant leaf occurrences and the vertices (7.12 of the
/// definition). A refusal of the admission is [`ViewError::Storage`] and
/// nothing is labelled. On success the grant stays with the returned
/// [`Comparison`], whose `admitted_bytes` states it: release that many bytes
/// when the comparison is dropped. On any other error the grant is returned
/// before the error is. Lowering the rows, the present canonical form and the
/// certificates and nested instances held while a search runs are not metered:
/// the admission is the bound of the up-front working state, and the engine
/// holds nothing else of that order before it starts.
///
/// # Errors
/// As [`compare_view`], and [`ViewError::Storage`] when the admission refuses
/// or the sum of the bytes passes the word.
pub fn compare_view_with_memory<D: DatasetView, S: StorageAdmission + ?Sized>(
    view: &D,
    records: &[Record<'_>],
    present: &mut CanonicalizationControl<'_>,
    limit: Limit,
    memory: &mut Memory<'_, S>,
) -> Result<Comparison<D::ReadError>, ViewError<D::ReadError>> {
    let outcome = try_canonical_sha256_view(view, CanonHash::Sha256, present);
    let stop = present.stop_signal();
    let lines = lowered(view, records, stop)?
        .into_result()
        .map_err(ViewError::Refused)?;
    let bytes = lines
        .footprint_bytes()
        .zip(optimised::working_bytes(&lines))
        .and_then(|(held, working)| held.checked_add(working))
        .ok_or(ViewError::Storage(StorageError::SizeOverflow))?;
    memory
        .admission_mut()
        .checkpoint(0)
        .map_err(ViewError::Storage)?;
    memory.add_bytes(bytes).map_err(ViewError::Storage)?;
    match label_lines(&lines, limit, stop) {
        Ok(labelled) => Ok(Comparison {
            present: outcome,
            lines,
            labelled,
            admitted_bytes: bytes,
        }),
        Err(error) => {
            // A refusal of the refund cannot replace the error that ended the
            // run: the grant stays outstanding with the caller's account.
            let _ = memory.release_bytes(bytes);
            Err(error)
        }
    }
}
