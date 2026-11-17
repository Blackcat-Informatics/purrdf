// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The lines of a dataset view, and the comparison of the present canonical
//! form with the labelling over them.
//!
//! Lowering is outside the definition (its section 11): this is the one rule
//! by which PurRDF supplies lines for a dataset view.
//!
//! * Each distinct quad becomes a `quad` line; a view that visits a row
//!   again repeats nothing, because a statement is a set member.
//! * Each distinct row of the reifier side table, `(reifier, rdf:reifies, triple term,
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
//! * A composite literal (`cdt:List` or `cdt:Map`) whose lexical form names a
//!   blank node is not a constant leaf. Its leaf is the blank leaf of a
//!   composite node, one node per literal value, and the first time a line
//!   holds it, one `record` line ties the node to its text and to the blank
//!   nodes the text names: the payload is
//!   [`composite_payload`](super::document::composite_payload) of the
//!   datatype and the text around the embedded tokens, and the terms are the
//!   composite node, then the blank node of each token in order, repeats
//!   kept. An embedded label denotes the blank node of the view with the same
//!   `(label, scope)` ([`crate::cdt_blank`]), so it takes part in the
//!   labelling like a label in any other position. A composite literal that
//!   names no blank node is a constant leaf like any other literal.
//!
//! A blank node's source identifier is its scope ordinal, four bytes
//! big-endian, followed by its label bytes, so two blank nodes are one node
//! exactly when their `(label, scope)` are equal; a blank node of a record is
//! the view's blank node of the same `(label, scope)`. A composite node's
//! source identifier is four zero bytes, `0xFF` and its literal's encoding,
//! which no `(label, scope)` spells. Terms are lowered straight into tokens
//! over a heap work list: no nested term is built and none is walked on the
//! machine stack.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::document::{
    Composites, DocumentError, composite_payload, composite_source, is_composite_source,
};
use super::lines::{
    Admission, Admitter, Direction, Facts, GraphTokens, Kind, TAG_ABSENT, TAG_PRESENT, TAG_TRIPLE,
    Tok, iri_encoding, literal_encoding,
};
use super::{GraphTerm, Labelled, Limit, LineSet, Refusal, optimised};
use crate::cdt_blank::cdt_blank_segments;
use crate::governor::is_stopped;
use purrdf_lex::allocation::{Admission as StorageAdmission, Memory, StorageError};

use crate::rdfc12::{CanonHash, try_canonical_sha256_view};
use crate::{
    BlankScope, CanonError, CanonicalizationControl, ContentDigest, DatasetView, RdfTextDirection,
    StopSignal, TermLookupError, TermValue,
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

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError for ViewError<E> {
    purrdf_hash::error_visit! {
        Read(cause) => "labelling-view-read" { "cause" => Cause(cause) },
        ForeignId => "labelling-view-foreign-id" {},
        Refused(refusal) => "labelling-view-refused" { "code" => Unsigned(u128::from(refusal.code())) },
        Stopped => "labelling-view-stopped" {},
        Storage(cause) => "labelling-view-storage" { "cause" => Cause(cause) },
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

/// The source identifier of the blank node `(label, scope)`.
fn blank_source(label: &str, scope: BlankScope) -> Vec<u8> {
    let mut source = Vec::with_capacity(4 + label.len());
    source.extend_from_slice(&scope.ordinal().to_be_bytes());
    source.extend_from_slice(label.as_bytes());
    source
}

/// Which statements of a view become lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope<'a> {
    /// Every statement and every named graph of the view.
    Dataset,
    /// The statements asserted in the named graph with this IRI, each lowered
    /// with its graph slot absent. A reifier or annotation row belongs to the
    /// graph its own graph slot names. A graph the view does not use names no
    /// rows: the result is the labelling of no line. No `empty-graph` line is
    /// produced, because the graph is not a graph of the lines.
    Graph(&'a str),
}

/// A [`Scope`] resolved against a view.
#[derive(Clone, Copy)]
enum Selection<Id> {
    Dataset,
    /// The id of the selected graph, or none when the view holds no such term.
    Graph(Option<Id>),
}

/// What a selection does with a row.
#[derive(Clone, Copy)]
enum Slot<Id> {
    /// The row is lowered with this graph slot.
    Kept(Option<Id>),
    /// The row is not of the selection.
    Left,
}

struct Lowering<'a, 's, D: DatasetView> {
    view: &'a D,
    selection: Selection<D::Id>,
    stop: Option<&'s dyn StopSignal>,
    admitter: Admitter,
    /// The graphs carried by a statement line, by view identifier.
    carried: BTreeSet<D::Id>,
    /// The composite nodes whose `record` line is already lowered.
    composites: BTreeSet<usize>,
    /// The `record` lines of composite nodes a line just reached, pushed after
    /// that line.
    pending: Vec<Vec<Tok>>,
}

impl<D: DatasetView> Lowering<'_, '_, D> {
    fn poll(&self) -> Result<(), ViewError<D::ReadError>> {
        if is_stopped(self.stop) {
            Err(ViewError::Stopped)
        } else {
            Ok(())
        }
    }

    /// The tokens of `value` in walk order, and its variant. A composite node
    /// reached for the first time queues its `record` line in `pending`.
    fn value(
        &mut self,
        value: &TermValue,
        out: &mut Vec<Tok>,
    ) -> Result<GraphTerm, ViewError<D::ReadError>> {
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
                    let cut =
                        cdt_blank_segments(lexical_form, datatype).map_err(ViewError::Storage)?;
                    if cut.blanks.is_empty() {
                        out.push(Tok::Const(interner.constant(&encoding)));
                        continue;
                    }
                    let node = interner.node(&composite_source(&encoding));
                    out.push(Tok::Blank(node));
                    if self.composites.insert(node) {
                        let mut record = Vec::new();
                        interner.record_head(
                            &composite_payload(datatype, &cut.segments),
                            cut.blanks.len() + 1,
                            &mut record,
                        );
                        record.push(Tok::Blank(node));
                        for (label, scope) in &cut.blanks {
                            record.push(Tok::Blank(interner.node(&blank_source(label, *scope))));
                        }
                        self.pending.push(record);
                    }
                }
                TermValue::Blank { label, scope } => {
                    out.push(Tok::Blank(interner.node(&blank_source(label, *scope))));
                }
                TermValue::Triple { s, p, o } => {
                    out.push(Tok::Tag(TAG_TRIPLE));
                    pending.extend([&**o, &**p, &**s]);
                }
            }
        }
        Ok(variant_of(value))
    }

    /// Push the `record` lines of the composite nodes the last line reached.
    fn flush_composites(&mut self) {
        for tokens in std::mem::take(&mut self.pending) {
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
        self.value(&value, out)
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
        self.flush_composites();
        Ok(())
    }

    /// The graph slot a row of graph `g` is lowered with, or [`Slot::Left`]
    /// when the selection leaves the row out.
    fn slot(&self, g: Option<D::Id>) -> Slot<D::Id> {
        match self.selection {
            Selection::Dataset => Slot::Kept(g),
            Selection::Graph(Some(selected)) if g == Some(selected) => Slot::Kept(None),
            Selection::Graph(_) => Slot::Left,
        }
    }

    fn lower(&mut self, records: &[Record<'_>]) -> Result<(), ViewError<D::ReadError>> {
        let view = self.view;
        // A view may visit a physical row more than once; a statement and a
        // reifier binding are set members, so each is lowered once. An
        // annotation row is not a set member: its multiplicity is its count.
        let mut statements = BTreeSet::new();
        for q in view.quads() {
            if let Slot::Kept(g) = self.slot(q.g)
                && statements.insert((q.s, q.p, q.o, g))
            {
                self.row(Kind::Quad, None, &[q.s, q.p, q.o], GraphTerm::Iri, g)?;
            }
        }
        // A reifier row is `(reifier, rdf:reifies, triple term, graph)`.
        let mut bindings = BTreeSet::new();
        for q in view.reifier_quads() {
            if let Slot::Kept(g) = self.slot(q.g)
                && bindings.insert((q.s, q.o, g))
            {
                self.row(
                    Kind::ReifierBinding,
                    None,
                    &[q.s, q.o],
                    GraphTerm::Triple,
                    g,
                )?;
            }
        }
        // One line per distinct annotation row, its count the row's multiplicity.
        let mut annotations: BTreeMap<_, u64> = BTreeMap::new();
        for q in view.annotation_quads() {
            self.poll()?;
            if let Slot::Kept(g) = self.slot(q.g) {
                let rows = annotations.entry((q.s, q.p, q.o, g)).or_default();
                *rows = rows.saturating_add(1);
            }
        }
        for ((s, p, o, g), count) in annotations {
            self.row(Kind::Annotation, Some(count), &[s, p, o], GraphTerm::Iri, g)?;
        }
        for g in view.named_graphs() {
            if matches!(self.selection, Selection::Graph(_)) || self.carried.contains(&g) {
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
            self.flush_composites();
        }
        for record in records {
            self.poll()?;
            let mut tokens = Vec::new();
            self.admitter
                .interner
                .record_head(record.payload, record.terms.len(), &mut tokens);
            for term in record.terms {
                self.value(term, &mut tokens)?;
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
            self.flush_composites();
        }
        Ok(())
    }
}

/// The lowered lines of `view` and `records`, before admission.
fn lowered<D: DatasetView>(
    view: &D,
    scope: Scope<'_>,
    records: &[Record<'_>],
    stop: Option<&dyn StopSignal>,
) -> Result<Admission, ViewError<D::ReadError>> {
    let selection = match scope {
        Scope::Dataset => Selection::Dataset,
        Scope::Graph(iri) => Selection::Graph(
            view.term_id_by_value(&TermValue::Iri(iri.to_owned()))
                .map_err(ViewError::Read)?,
        ),
    };
    let mut lowering = Lowering {
        view,
        selection,
        stop,
        admitter: Admitter::default(),
        carried: BTreeSet::new(),
        composites: BTreeSet::new(),
        pending: Vec::new(),
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
    lowered(view, Scope::Dataset, records, None)?
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
    let lines = lowered(view, Scope::Dataset, records, stop)?
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
    // The account is polled before any row is read or lowered.
    memory
        .admission_mut()
        .checkpoint(0)
        .map_err(ViewError::Storage)?;
    let outcome = try_canonical_sha256_view(view, CanonHash::Sha256, present);
    let stop = present.stop_signal();
    let lines = lowered(view, Scope::Dataset, records, stop)?
        .into_result()
        .map_err(ViewError::Refused)?;
    let bytes = lines
        .footprint_bytes()
        .zip(optimised::working_bytes(&lines))
        .and_then(|(held, working)| held.checked_add(working))
        .ok_or(ViewError::Storage(StorageError::SizeOverflow))?;
    memory.add_bytes(bytes).map_err(ViewError::Storage)?;
    match label_lines(&lines, limit, stop) {
        Ok(labelled) => Ok(Comparison {
            present: outcome,
            lines,
            labelled,
            admitted_bytes: bytes,
        }),
        Err(error) => {
            // The lines the grant covers are destroyed before it is refunded. A
            // refusal of the refund cannot replace the error that ended the
            // run: the grant stays outstanding with the caller's account.
            drop(lines);
            let _ = memory.release_bytes(bytes);
            Err(error)
        }
    }
}

/// A labelled view: the admitted lines, their labelling, and the view's own
/// identifier of each blank node.
#[derive(Clone, Debug)]
pub struct Labelling<Id> {
    lines: LineSet,
    labelled: Labelled,
    blanks: Vec<Option<Id>>,
}

impl<Id: Copy> Labelling<Id> {
    /// The admitted lines the labelling was computed over.
    #[must_use]
    pub const fn lines(&self) -> &LineSet {
        &self.lines
    }

    /// The labelling of [`Self::lines`]: positions, identities, the canonical
    /// line sequence, its digest and the structural record.
    #[must_use]
    pub const fn labelled(&self) -> &Labelled {
        &self.labelled
    }

    /// The view's identifier of the blank node with index `node` in
    /// [`Self::lines`] (index `i` of [`Labelled::placements`]). `None` for a
    /// blank node that only a [`Record`] mentions, and for a composite node:
    /// the view does not hold it.
    #[must_use]
    pub fn blank_id(&self, node: usize) -> Option<Id> {
        self.blanks.get(node).copied().flatten()
    }

    /// Every blank node of the view that occurs in a line, as
    /// `(view identifier, position)` in node order.
    pub fn view_positions(&self) -> impl Iterator<Item = (Id, u64)> + '_ {
        self.blanks
            .iter()
            .zip(&self.labelled.placements)
            .filter_map(|(id, placement)| id.map(|id| (id, placement.position)))
    }

    /// The literal each composite node of the canonical line sequence spells.
    pub(crate) fn composites(&self) -> Result<Composites, DocumentError> {
        let positions: BTreeSet<u64> = (0..self.lines.blank_nodes())
            .filter(|&node| is_composite_source(self.lines.source(node)))
            .map(|node| self.labelled.placements[node].position)
            .collect();
        Composites::of(&self.labelled.lines, |position| {
            positions.contains(&position)
        })
    }
}

/// The labelling of admitted `lines`, with the view's identifier of each
/// blank node.
fn labelling_of<D: DatasetView>(
    view: &D,
    lines: LineSet,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelling<D::Id>, ViewError<D::ReadError>> {
    let labelled = label_lines(&lines, limit, stop)?;
    let mut blanks = Vec::with_capacity(lines.blank_nodes());
    for node in 0..lines.blank_nodes() {
        let source = lines.source(node);
        let id = match source.split_first_chunk::<4>() {
            Some((ordinal, label)) => match std::str::from_utf8(label) {
                Ok(label) => view
                    .term_id_by_value(&TermValue::Blank {
                        label: label.to_owned(),
                        scope: BlankScope(u32::from_be_bytes(*ordinal)),
                    })
                    .map_err(ViewError::Read)?,
                Err(_) => None,
            },
            None => None,
        };
        blanks.push(id);
    }
    Ok(Labelling {
        lines,
        labelled,
        blanks,
    })
}

/// Label the lines of `view` and `records` within `scope`, with the engine's
/// own `limit` and the caller's `stop` signal (polled while the rows are
/// lowered and at every call of the engine).
///
/// Positions are those of the lines of this scope and records together; they
/// are never mixed with the positions of another scope or record set. The
/// labelling does not meter its storage; use [`label_view_with_memory`] where
/// the caller admits the working state of a computation.
///
/// # Errors
/// As [`lines_of_view`], and [`ViewError::Stopped`] when `stop` is set. A
/// refusal is [`ViewError::Refused`] with the typed refusal of the
/// definition, [`Refusal::Limit`] included.
pub fn label_view<D: DatasetView>(
    view: &D,
    records: &[Record<'_>],
    scope: Scope<'_>,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelling<D::Id>, ViewError<D::ReadError>> {
    let lines = lowered(view, scope, records, stop)?
        .into_result()
        .map_err(ViewError::Refused)?;
    labelling_of(view, lines, limit, stop)
}

/// A labelling and the bytes of working state still granted for it.
type LabellingWithBytes<Id> = (Labelling<Id>, usize);

/// [`label_view`] with the working state of the labelling admitted once, before
/// the engine starts, through the caller's `memory`, as
/// [`compare_view_with_memory`] does. On success the grant stays outstanding and
/// its size is returned: release that many bytes when the labelling is dropped.
/// On any error the grant is returned before the error is.
///
/// # Errors
/// As [`label_view`], and [`ViewError::Storage`] when the admission refuses or
/// the sum of the bytes passes the word.
pub fn label_view_with_memory<D: DatasetView, S: StorageAdmission + ?Sized>(
    view: &D,
    records: &[Record<'_>],
    scope: Scope<'_>,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
    memory: &mut Memory<'_, S>,
) -> Result<LabellingWithBytes<D::Id>, ViewError<D::ReadError>> {
    // The account is polled before any row is read or lowered.
    memory
        .admission_mut()
        .checkpoint(0)
        .map_err(ViewError::Storage)?;
    let lines = lowered(view, scope, records, stop)?
        .into_result()
        .map_err(ViewError::Refused)?;
    let bytes = lines
        .footprint_bytes()
        .zip(optimised::working_bytes(&lines))
        .and_then(|(held, working)| held.checked_add(working))
        .ok_or(ViewError::Storage(StorageError::SizeOverflow))?;
    memory.add_bytes(bytes).map_err(ViewError::Storage)?;
    match labelling_of(view, lines, limit, stop) {
        Ok(labelling) => Ok((labelling, bytes)),
        Err(error) => {
            // A refusal of the refund cannot replace the error that ended the
            // run: the grant stays outstanding with the caller's account.
            let _ = memory.release_bytes(bytes);
            Err(error)
        }
    }
}
