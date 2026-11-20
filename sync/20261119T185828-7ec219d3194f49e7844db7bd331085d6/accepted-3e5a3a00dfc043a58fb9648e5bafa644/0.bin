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
//! * A triple term that is a component of another triple term is a nested
//!   term. Each distinct nested term is one term node, a blank node that is no
//!   blank node of the view, with one `record` line that holds the node and the
//!   components, every component that is a triple term standing as its own term
//!   node ([`super::nested`]). A triple term in a slot of a line or directly in
//!   a record's terms keeps its place with its components lowered, so every
//!   triple term of the lines has depth one.
//!
//! A blank node's source identifier is its scope ordinal, four bytes
//! big-endian, followed by its label bytes, so two blank nodes are one node
//! exactly when their `(label, scope)` are equal; a blank node of a record is
//! the view's blank node of the same `(label, scope)`. A composite node's
//! source identifier is four zero bytes, `0xFF` and its literal's encoding,
//! which no `(label, scope)` spells.
//!
//! The source form is decided per input: the statements of the view that the
//! scope selects together with the caller's records, which is what the
//! lowering reads. When that input holds a nested term, every blank node of it
//! takes the source identifier `E([ordinal, label])`, the deterministic CBOR
//! encoding the second-generation GTS format gives a blank node of a fold, and
//! a term node takes `E([d])` with the digest of its nested term. The digests
//! of term nodes read those bytes, and the source order is the format's: every
//! term node (first byte `0x81`) precedes every blank node of the input
//! (`0x82`). An input with no nested term keeps the four-byte form, so the
//! order of its twins, which the lowering does not change, does not move; the
//! format's form would order a shorter label first. A nested term only in a
//! graph the scope leaves out does not change the form, and a nested term only
//! in a record changes it even when the view holds none. Both forms name each
//! node once.
//!
//! The canonical map of a view is not the positions a GTS reader reports or
//! stores: the reader's source identifiers come from its own folds, so for twin
//! blank nodes and for copies of equal top components the two maps can differ by
//! an automorphism. The canonical line sequence, its bytes and its digest are
//! equal only when the view is given the same lines and records as the reader's
//! labelled set: the records of bindings, suppression and blob entries and
//! pairs of VALUES (one for each typed value) come from the caller, and the
//! dataset folds the case
//! of a language tag where the format keeps it as written. The order of delivery
//! a reader reports for a canonical file comes from the reader's own run.
//!
//! Terms are lowered from the identifiers of their components over heap work
//! lists: no triple term that holds another is expanded, each nested term is
//! interned once, and nothing is walked on the machine stack.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::document::{
    Composites, DocumentError, composite_payload, composite_source, is_composite_source,
};
use super::lines::Interner;
use super::lines::{
    Admission, Admitter, Direction, Facts, GraphTokens, Kind, TAG_ABSENT, TAG_PRESENT, TAG_TRIPLE,
    Tok, iri_encoding, literal_encoding, push_bytes,
};
use super::nested::{
    NESTED_TERM_PAYLOAD, NestedHash, NestedTerms, Part, is_term_node_source, leaf_source,
    nested_term_digest, parse_text_blank_source, term_node_source, text_blank_source,
};
use super::{GraphTerm, Labelled, Limit, LineSet, Refusal, optimised};
use crate::cdt_blank::cdt_blank_segments;
use crate::governor::is_stopped;
use purrdf_lex::allocation::{Admission as StorageAdmission, Memory, StorageError};

use crate::rdfc12::{CanonHash, try_canonical_sha256_view};
use crate::{
    BlankScope, CanonError, CanonicalizationControl, ContentDigest, DatasetView, FastMap, FastSet,
    RdfTextDirection, StopSignal, TermLookupError, TermRef, TermValue,
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
    /// The lowering refused to hand the input to the labelling: its size or
    /// its weight exceeds the capacity of the read, or two different nested
    /// terms have one digest.
    Lowering(LoweringRefusal),
}

/// Why the lowering of a view refused to hand it to the labelling. Each is
/// judged before the engine runs, in the order of the variants, and the first
/// that applies is the one returned.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LoweringRefusal {
    /// The size of the input exceeds the capacity of the read in lines
    /// ([`LabellingCapacity::new`]). Nothing is weighed or digested.
    Size {
        /// The size of the input ([`Labelling::size`]).
        size: u64,
        /// The capacity in lines.
        capacity: u64,
    },
    /// The total labelled weight of the read with the weight of this input
    /// exceeds the capacity of the read in bytes, or an earlier input of the
    /// read was refused by weight. The weight is not added to the total.
    Weight {
        /// The weight of the input ([`Labelling::weight`]).
        weight: u64,
        /// The total labelled weight of the read before this input.
        total: u64,
        /// The capacity of the read in bytes.
        capacity: u64,
    },
    /// Two different nested terms have one digest, so two term nodes would
    /// have one source identifier. Their digests are those of BLAKE3, so this
    /// is a collision of BLAKE3. The weight stays in the total of the read.
    TermNode {
        /// The digest the two share.
        digest: [u8; 32],
    },
}

impl fmt::Display for LoweringRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size { size, capacity } => write!(
                f,
                "the size {size} of the input exceeds the capacity of {capacity} lines"
            ),
            Self::Weight {
                weight,
                total,
                capacity,
            } => write!(
                f,
                "the weight {weight} of the input with the total {total} of the read exceeds \
                 the capacity of {capacity} bytes"
            ),
            Self::TermNode { .. } => {
                f.write_str("two different nested terms have one term-node digest")
            }
        }
    }
}

impl std::error::Error for LoweringRefusal {}

impl purrdf_hash::error::StructuredError for LoweringRefusal {
    purrdf_hash::error_visit! {
        Size { size, capacity } => "labelling-lowering-size" {
            "size" => Unsigned(u128::from(*size)),
            "capacity" => Unsigned(u128::from(*capacity)),
        },
        Weight { weight, total, capacity } => "labelling-lowering-weight" {
            "weight" => Unsigned(u128::from(*weight)),
            "total" => Unsigned(u128::from(*total)),
            "capacity" => Unsigned(u128::from(*capacity)),
        },
        TermNode { digest } => "labelling-lowering-term-node" {
            "digest" => Bytes(digest),
        },
    }
}

/// The capacities of a read that labels a sequence of inputs, and the total
/// labelled weight it has admitted so far. One value is kept for the whole
/// read and handed to [`label_view_with_capacity`] for each input in turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LabellingCapacity {
    lines: u64,
    bytes: u64,
    total: u64,
    /// Whether an input of the read was refused by weight, after which every
    /// input whose size is admitted is refused by weight.
    spent: bool,
}

impl LabellingCapacity {
    /// A read that admits an input of at most `lines` in size and inputs whose
    /// weights sum to at most `bytes`. A size or a total equal to its capacity
    /// is admitted.
    #[must_use]
    pub const fn new(lines: u64, bytes: u64) -> Self {
        Self {
            lines,
            bytes,
            total: 0,
            spent: false,
        }
    }

    /// The total labelled weight: the sum of the weights of the inputs the
    /// read has admitted.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    const fn admit_size(&self, size: u64) -> Result<(), LoweringRefusal> {
        if size > self.lines {
            return Err(LoweringRefusal::Size {
                size,
                capacity: self.lines,
            });
        }
        Ok(())
    }

    fn admit_weight(&mut self, weight: u64) -> Result<(), LoweringRefusal> {
        match self.total.checked_add(weight) {
            Some(total) if !self.spent && total <= self.bytes => {
                self.total = total;
                Ok(())
            }
            _ => {
                self.spent = true;
                Err(LoweringRefusal::Weight {
                    weight,
                    total: self.total,
                    capacity: self.bytes,
                })
            }
        }
    }
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
            Self::Lowering(refusal) => write!(f, "the lowering refused the input: {refusal}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ViewError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Lowering(refusal) => Some(refusal),
            Self::ForeignId | Self::Refused(_) | Self::Stopped => None,
        }
    }
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError for ViewError<E> {
    purrdf_hash::error_visit! {
        Read(cause) => "labelling-view-read" { "cause" => Cause(cause) },
        ForeignId => "labelling-view-foreign-id" {},
        Refused(refusal) => "labelling-view-refused" {
            "code" => Unsigned(u128::from(refusal.code())),
            "refusal" => Cause(refusal),
        },
        Stopped => "labelling-view-stopped" {},
        Storage(cause) => "labelling-view-storage" { "cause" => Cause(cause) },
        Lowering(cause) => "labelling-view-lowering" { "cause" => Cause(cause) },
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

struct Lowering<'a, 's, 'm, D: DatasetView> {
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
    /// The caller's admission of the bytes a new constant or node adds.
    meter: Option<&'m mut Meter<'m>>,
    /// The distinct nested terms reached, by their lowered components.
    nested: NestedTerms,
    /// The node index of the term node of each nested term.
    term_nodes: Vec<usize>,
    /// The nested term of each triple-term identifier lowered as one.
    nested_ids: FastMap<D::Id, usize>,
    /// The triple-term identifiers whose nested candidates are counted.
    holders: FastSet<D::Id>,
    /// The nested candidates: the components that are triple terms, of each
    /// distinct triple-term identifier reached and of each triple term of a
    /// record's values.
    candidates: u64,
}

/// Admits `bytes` of the lowered set through the caller's account.
pub(super) type Meter<'m> = dyn FnMut(usize) -> Result<(), StorageError> + 'm;

/// The bytes the lowering keeps for one nested term: its components in the
/// list and as the key of the index, its index and term-node entries, and the
/// term node's source slot in the set.
const NESTED_ENTRY_BYTES: usize =
    2 * size_of::<[Part; 3]>() + 2 * size_of::<usize>() + size_of::<Box<[u8]>>();

/// The bytes the lowering keeps for the source `K` of `len` bytes of a
/// component of a nested term: the bytes, their box and their key.
const fn kept_source_bytes(len: usize) -> usize {
    interned_bytes(len).saturating_add(size_of::<Part>())
}

/// The bytes the set holds for one interned byte string.
const fn interned_bytes(len: usize) -> usize {
    len.saturating_add(size_of::<Box<[u8]>>())
}

/// Admit `bytes` through `meter`, when the lowering is metered.
fn admit<E>(meter: &mut Option<&mut Meter<'_>>, bytes: usize) -> Result<(), ViewError<E>> {
    match meter {
        Some(meter) => meter(bytes).map_err(ViewError::Storage),
        None => Ok(()),
    }
}

/// Admit a constant of `encoding` that the set does not yet hold.
fn admit_constant<E>(
    meter: &mut Option<&mut Meter<'_>>,
    interner: &Interner,
    encoding: &[u8],
) -> Result<(), ViewError<E>> {
    if interner.has_constant(encoding) {
        return Ok(());
    }
    admit(meter, interned_bytes(encoding.len()))
}

/// Admit a blank node of `source` that the set does not yet hold.
fn admit_node<E>(
    meter: &mut Option<&mut Meter<'_>>,
    interner: &Interner,
    source: &[u8],
) -> Result<(), ViewError<E>> {
    if interner.has_node(source) {
        return Ok(());
    }
    admit(meter, interned_bytes(source.len()))
}

impl<D: DatasetView> Lowering<'_, '_, '_, D> {
    fn poll(&self) -> Result<(), ViewError<D::ReadError>> {
        if is_stopped(self.stop) {
            Err(ViewError::Stopped)
        } else {
            Ok(())
        }
    }

    /// The component of the IRI, literal or blank node `value`, or `None` for
    /// a triple term. A composite node reached for the first time queues its
    /// `record` line in `pending`.
    fn leaf(&mut self, value: &TermValue) -> Result<Option<Part>, ViewError<D::ReadError>> {
        let interner = &mut self.admitter.interner;
        let meter = &mut self.meter;
        let part = match value {
            TermValue::Iri(iri) => {
                let encoding = iri_encoding(iri.as_bytes());
                admit_constant(meter, interner, &encoding)?;
                Part::Const(interner.constant(&encoding))
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
                let cut = cdt_blank_segments(lexical_form, datatype).map_err(ViewError::Storage)?;
                if cut.blanks.is_empty() {
                    admit_constant(meter, interner, &encoding)?;
                    Part::Const(interner.constant(&encoding))
                } else {
                    let source = composite_source(&encoding);
                    admit_node(meter, interner, &source)?;
                    let node = interner.node(&source);
                    if self.composites.insert(node) {
                        let payload = composite_payload(datatype, &cut.segments);
                        let mut encoded = Vec::with_capacity(8 + payload.len());
                        push_bytes(&mut encoded, &payload);
                        admit_constant(meter, interner, &encoded)?;
                        let mut record = Vec::new();
                        interner.record_head(&payload, cut.blanks.len() + 1, &mut record);
                        record.push(Tok::Blank(node));
                        for (label, scope) in &cut.blanks {
                            let source = blank_source(label, *scope);
                            admit_node(meter, interner, &source)?;
                            record.push(Tok::Blank(interner.node(&source)));
                        }
                        self.pending.push(record);
                    }
                    Part::Blank(node)
                }
            }
            TermValue::Blank { label, scope } => {
                let source = blank_source(label, *scope);
                admit_node(meter, interner, &source)?;
                return Ok(Some(Part::Blank(interner.node(&source))));
            }
            TermValue::Triple { .. } => return Ok(None),
        };
        Ok(Some(part))
    }

    /// The leaf `value`, a component of a nested term: its part, its source
    /// `K` kept and admitted when it is a constant or composite node met for
    /// the first time in a nested term.
    fn nested_leaf(&mut self, value: &TermValue) -> Result<Part, ViewError<D::ReadError>> {
        // A view that answers a leaf for an identifier once and a triple term
        // for it later does not present one dataset.
        let part = self.leaf(value)?.ok_or(ViewError::ForeignId)?;
        if self.nested.needs_source(part)
            && let Some(source) = leaf_source(value)
        {
            admit(&mut self.meter, kept_source_bytes(source.len()))?;
            self.nested.keep_source(part, source);
        }
        Ok(part)
    }

    /// The token of a component: a term node stands as its blank leaf.
    fn token(&self, part: Part) -> Tok {
        match part {
            Part::Const(c) => Tok::Const(c),
            Part::Blank(b) => Tok::Blank(b),
            Part::Nested(n) => Tok::Blank(self.term_nodes[n]),
        }
    }

    /// The nested term of the components `parts`, interned once with its term
    /// node.
    fn intern_nested(&mut self, parts: [Part; 3]) -> Result<usize, ViewError<D::ReadError>> {
        if let Some(n) = self.nested.get(&parts) {
            return Ok(n);
        }
        admit(&mut self.meter, NESTED_ENTRY_BYTES)?;
        let n = self.nested.push(parts);
        let node = self.admitter.interner.term_node();
        self.term_nodes.push(node);
        Ok(n)
    }

    /// The nested term of the triple term `root`, a component of another
    /// triple term of a record's values: its components first, over a heap
    /// work list. Each triple term of the value counts its components that
    /// are triple terms as nested candidates.
    fn nested_value(&mut self, root: &TermValue) -> Result<Part, ViewError<D::ReadError>> {
        enum Step<'v> {
            Enter(&'v TermValue),
            Exit,
        }
        let mut steps = vec![Step::Enter(root)];
        let mut done: Vec<Part> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(TermValue::Triple { s, p, o }) => {
                    let inner = [&**s, &**p, &**o]
                        .iter()
                        .filter(|c| matches!(c, TermValue::Triple { .. }))
                        .count() as u64;
                    self.candidates = self.candidates.saturating_add(inner);
                    steps.extend([Step::Exit, Step::Enter(o), Step::Enter(p), Step::Enter(s)]);
                }
                Step::Enter(leaf) => done.push(self.nested_leaf(leaf)?),
                Step::Exit => {
                    // INVARIANT: an exit is pushed under the entries of its
                    // three components, and each entry leaves one part.
                    let (Some(o), Some(p), Some(s)) = (done.pop(), done.pop(), done.pop()) else {
                        unreachable!("a triple term's exit precedes its components")
                    };
                    done.push(Part::Nested(self.intern_nested([s, p, o])?));
                }
            }
        }
        // INVARIANT: the root's entry leaves exactly one part.
        let Some(part) = done.pop() else {
            unreachable!("the walk of a term leaves its part")
        };
        Ok(part)
    }

    /// The tokens of `value` in a slot of a line or in a record's list: a
    /// triple term of depth one whose components that are triple terms stand
    /// as their term nodes, or a leaf.
    fn value(
        &mut self,
        value: &TermValue,
        out: &mut Vec<Tok>,
    ) -> Result<GraphTerm, ViewError<D::ReadError>> {
        let TermValue::Triple { s, p, o } = value else {
            let Some(part) = self.leaf(value)? else {
                // INVARIANT: `value` is not a triple term, the only value
                // `leaf` answers `None` for.
                unreachable!("a value that is not a triple term is a leaf")
            };
            out.push(self.token(part));
            return Ok(variant_of(value));
        };
        let components = [&**s, &**p, &**o];
        let inner = components
            .iter()
            .filter(|c| matches!(c, TermValue::Triple { .. }))
            .count() as u64;
        self.candidates = self.candidates.saturating_add(inner);
        out.push(Tok::Tag(TAG_TRIPLE));
        for component in components {
            let part = match self.leaf(component)? {
                Some(part) => part,
                None => self.nested_value(component)?,
            };
            out.push(self.token(part));
        }
        Ok(GraphTerm::Triple)
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

    /// The components of the term `id` when it is a triple term.
    fn triple_of(&self, id: D::Id) -> Result<Option<[D::Id; 3]>, ViewError<D::ReadError>> {
        self.view
            .with_term(id, |term| match term {
                TermRef::Triple { s, p, o } => Some([s, p, o]),
                _ => None,
            })
            .map_err(ViewError::Read)
    }

    /// The value of the term `id`, which is not a triple term.
    fn leaf_value(&self, id: D::Id) -> Result<TermValue, ViewError<D::ReadError>> {
        self.view.term_value(id).map_err(|error| match error {
            TermLookupError::ForeignId => ViewError::ForeignId,
            TermLookupError::Read(error) => ViewError::Read(error),
        })
    }

    /// The leaf `id`, which is not a triple term, as a component, and its
    /// variant.
    fn leaf_id(&mut self, id: D::Id) -> Result<(Part, GraphTerm), ViewError<D::ReadError>> {
        let value = self.leaf_value(id)?;
        // A view that answers a leaf for an identifier once and a triple term
        // for it later does not present one dataset.
        let part = self.leaf(&value)?.ok_or(ViewError::ForeignId)?;
        Ok((part, variant_of(&value)))
    }

    /// Which of `components` are triple terms: the nested components of the
    /// triple term that holds them.
    fn nested_components(
        &self,
        components: [D::Id; 3],
    ) -> Result<[bool; 3], ViewError<D::ReadError>> {
        let mut nested = [false; 3];
        for (slot, component) in nested.iter_mut().zip(components) {
            *slot = self.triple_of(component)?.is_some();
        }
        Ok(nested)
    }

    /// Count the nested candidates `nested` of the triple term `id`, once for
    /// each identifier. A triple term with no nested component is recorded
    /// nowhere.
    fn hold(&mut self, id: D::Id, nested: [bool; 3]) -> Result<(), ViewError<D::ReadError>> {
        let count = nested.iter().filter(|&&n| n).count() as u64;
        if count == 0 || self.holders.contains(&id) {
            return Ok(());
        }
        admit(&mut self.meter, size_of::<D::Id>())?;
        self.holders.insert(id);
        self.candidates = self.candidates.saturating_add(count);
        Ok(())
    }

    /// The nested term of the triple term `root`, a component of another
    /// triple term: its components first, over a heap work list, each triple
    /// term identifier lowered once.
    fn nested_id(&mut self, root: D::Id) -> Result<Part, ViewError<D::ReadError>> {
        let mut pending = vec![(root, false)];
        while let Some((id, ready)) = pending.pop() {
            if self.nested_ids.contains_key(&id) {
                continue;
            }
            // A view that answers a triple term for an identifier once and a
            // leaf for it later does not present one dataset.
            let components = self.triple_of(id)?.ok_or(ViewError::ForeignId)?;
            if !ready {
                let nested = self.nested_components(components)?;
                self.hold(id, nested)?;
                pending.push((id, true));
                for (component, is_nested) in components.into_iter().zip(nested) {
                    if is_nested && !self.nested_ids.contains_key(&component) {
                        pending.push((component, false));
                    }
                }
                continue;
            }
            let mut parts = [Part::Const(0); 3];
            for (slot, component) in parts.iter_mut().zip(components) {
                *slot = match self.nested_ids.get(&component) {
                    Some(&n) => Part::Nested(n),
                    None => {
                        let value = self.leaf_value(component)?;
                        self.nested_leaf(&value)?
                    }
                };
            }
            let n = self.intern_nested(parts)?;
            admit(&mut self.meter, size_of::<(D::Id, usize)>())?;
            self.nested_ids.insert(id, n);
        }
        self.nested_ids
            .get(&root)
            .map(|&n| Part::Nested(n))
            .ok_or(ViewError::ForeignId)
    }

    /// The tokens of the term `id` in a slot of a line, and its variant: a
    /// triple term of depth one, read from the identifiers of its components,
    /// or a leaf. No triple term is expanded.
    fn term(
        &mut self,
        id: D::Id,
        out: &mut Vec<Tok>,
    ) -> Result<GraphTerm, ViewError<D::ReadError>> {
        let Some(components) = self.triple_of(id)? else {
            let (part, variant) = self.leaf_id(id)?;
            out.push(self.token(part));
            return Ok(variant);
        };
        let nested = self.nested_components(components)?;
        self.hold(id, nested)?;
        out.push(Tok::Tag(TAG_TRIPLE));
        for (component, is_nested) in components.into_iter().zip(nested) {
            let part = if is_nested {
                self.nested_id(component)?
            } else {
                self.leaf_id(component)?.0
            };
            out.push(self.token(part));
        }
        Ok(GraphTerm::Triple)
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
            let mut payload = Vec::with_capacity(8 + record.payload.len());
            push_bytes(&mut payload, record.payload);
            admit_constant(&mut self.meter, &self.admitter.interner, &payload)?;
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

/// What a lowering is given beside the view: the caller's stop signal, the
/// admission of its storage, the capacity of the read and the digest of the
/// term nodes.
pub(super) struct Run<'s, 'm, 'c> {
    pub(super) stop: Option<&'s dyn StopSignal>,
    pub(super) meter: Option<&'m mut Meter<'m>>,
    pub(super) capacity: Option<&'c mut LabellingCapacity>,
    pub(super) hash: NestedHash,
}

impl<'s> Run<'s, '_, '_> {
    /// A run with no meter and no capacity, its term nodes digested by the
    /// one construction.
    pub(super) const fn plain(stop: Option<&'s dyn StopSignal>) -> Self {
        Self {
            stop,
            meter: None,
            capacity: None,
            hash: nested_term_digest,
        }
    }
}

/// The lowered lines of a view before admission, with the figures the
/// lowering judged.
pub(super) struct Lowered {
    pub(super) admission: Admission,
    /// The number of term nodes.
    pub(super) term_nodes: usize,
    /// The size: the lines lowered from the view and the records, and the
    /// nested candidates.
    pub(super) size: u64,
    /// The weight: the bytes of the encodings of every line and every
    /// nested-term record, each blank leaf written `fixed`.
    pub(super) weight: u64,
}

/// The weight of one stored line: the length of its encoding with every blank
/// leaf `fixed`, which takes nine bytes.
fn line_weight(consts: &[Box<[u8]>], line: &[Tok]) -> u64 {
    line.iter()
        .map(|token| match token {
            Tok::Tag(_) => 1,
            Tok::Count(_) | Tok::Len(_) => 8,
            Tok::Const(c) => consts[*c].len() as u64,
            Tok::Blank(_) => 9,
        })
        .fold(0, u64::saturating_add)
}

/// The lowered lines of `view` and `records`, before admission.
///
/// The view is read once. Its lines are lowered with every nested term as a
/// term node and counted for the size; the size and then the weight are
/// judged against the capacity of the read before any digest is formed and
/// before any nested-term record is built; then the term nodes receive their
/// source identifiers, every blank node of the view takes the text form of
/// its source identifier, and one record per nested term is added. A refusal
/// drops what was lowered.
pub(super) fn lowered<D: DatasetView>(
    view: &D,
    scope: Scope<'_>,
    records: &[Record<'_>],
    run: Run<'_, '_, '_>,
) -> Result<Lowered, ViewError<D::ReadError>> {
    let Run {
        stop,
        meter,
        mut capacity,
        hash,
    } = run;
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
        meter,
        nested: NestedTerms::default(),
        term_nodes: Vec::new(),
        nested_ids: FastMap::default(),
        holders: FastSet::default(),
        candidates: 0,
    };
    match view.checked_read(|_| lowering.lower(records)) {
        Err(error) => return Err(ViewError::Read(error)),
        Ok(Err(error)) => return Err(error),
        Ok(Ok(())) => {}
    }
    let Lowering {
        mut admitter,
        mut meter,
        nested,
        term_nodes,
        candidates,
        ..
    } = lowering;
    let set = &admitter.interner.set;
    let size = (set.lines.len() as u64).saturating_add(candidates);
    if let Some(capacity) = capacity.as_deref_mut() {
        capacity.admit_size(size).map_err(ViewError::Lowering)?;
    }
    let weight = set
        .lines
        .iter()
        .map(|line| line_weight(&set.consts, line))
        .fold(nested.record_weight(&set.consts), u64::saturating_add);
    if let Some(capacity) = capacity {
        capacity.admit_weight(weight).map_err(ViewError::Lowering)?;
    }
    if !nested.is_empty() {
        let interner = &mut admitter.interner;
        // Every blank node of the view takes the text form of its source
        // identifier, which the term nodes' digests and source order read.
        for node in 0..interner.set.nodes.len() {
            let source = &interner.set.nodes[node];
            if source.is_empty() || is_composite_source(source) {
                continue;
            }
            let Some((ordinal, label)) = source.split_first_chunk::<4>() else {
                continue;
            };
            let text = text_blank_source(u32::from_be_bytes(*ordinal), label);
            admit(&mut meter, text.len().saturating_sub(source.len()))?;
            interner.set.nodes[node] = text.into_boxed_slice();
        }
        admit(&mut meter, nested.digest_bytes())?;
        let digests = nested
            .digests(&interner.set.nodes, hash)
            .map_err(|digest| ViewError::Lowering(LoweringRefusal::TermNode { digest }))?;
        let mut payload = Vec::with_capacity(8 + NESTED_TERM_PAYLOAD.len());
        push_bytes(&mut payload, &NESTED_TERM_PAYLOAD);
        admit_constant(&mut meter, interner, &payload)?;
        for (n, digest) in digests.iter().enumerate() {
            let source = term_node_source(digest);
            admit(&mut meter, interned_bytes(source.len()))?;
            interner.set.nodes[term_nodes[n]] = Box::new(source);
        }
        for (n, &node) in term_nodes.iter().enumerate() {
            let mut tokens = Vec::with_capacity(7);
            admitter
                .interner
                .record_head(&NESTED_TERM_PAYLOAD, 4, &mut tokens);
            tokens.push(Tok::Blank(node));
            tokens.extend(nested.parts(n).map(|part| match part {
                Part::Const(c) => Tok::Const(c),
                Part::Blank(b) => Tok::Blank(b),
                Part::Nested(m) => Tok::Blank(term_nodes[m]),
            }));
            admitter.push(
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
    Ok(Lowered {
        admission: admitter.finish(),
        term_nodes: term_nodes.len(),
        size,
        weight,
    })
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
    lowered(view, Scope::Dataset, records, Run::plain(None))?
        .admission
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
    let lines = lowered(view, Scope::Dataset, records, Run::plain(stop))?
        .admission
        .into_result()
        .map_err(ViewError::Refused)?;
    let (labelled, _) = label_lines(&lines, limit, stop)?;
    Ok(Comparison {
        present: outcome,
        lines,
        labelled,
        admitted_bytes: 0,
    })
}

/// Label admitted lines with the optimised engine, with the units the walk of
/// 8.3 charged.
fn label_lines<E>(
    lines: &LineSet,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
) -> Result<(Labelled, u64), ViewError<E>> {
    optimised::label_walked(lines, limit.get(), false, stop)
        .map(|(labelled, walk)| (labelled, walk.totals().total()))
        .map_err(|halt| match halt {
            super::Halt::Refused(refusal) => ViewError::Refused(refusal),
            super::Halt::Stopped => ViewError::Stopped,
        })
}

/// The admitted lines of `view` and `records`, each constant and blank node
/// admitted through `memory` before it is copied into the set, and the bytes
/// so admitted. A refusal returns what was admitted before it is reported.
fn lowered_in<D: DatasetView, S: StorageAdmission + ?Sized>(
    view: &D,
    scope: Scope<'_>,
    records: &[Record<'_>],
    stop: Option<&dyn StopSignal>,
    memory: &mut Memory<'_, S>,
) -> Result<(Admitted, usize), ViewError<D::ReadError>> {
    // The account is polled before any row is read or lowered.
    memory
        .admission_mut()
        .checkpoint(0)
        .map_err(ViewError::Storage)?;
    let charged = Cell::new(0usize);
    let result = {
        let mut meter = |bytes: usize| {
            memory.add_bytes(bytes)?;
            charged.set(charged.get().saturating_add(bytes));
            Ok(())
        };
        let run = Run {
            meter: Some(&mut meter),
            ..Run::plain(stop)
        };
        lowered(view, scope, records, run).and_then(Admitted::of)
    };
    match result {
        Ok(admitted) => Ok((admitted, charged.get())),
        Err(error) => {
            // A refusal of the refund cannot replace the error that ended the
            // run: the grant stays outstanding with the caller's account.
            let _ = memory.release_bytes(charged.get());
            Err(error)
        }
    }
}

/// Admitted lines with the figures of their lowering.
struct Admitted {
    lines: LineSet,
    term_nodes: usize,
    size: u64,
    weight: u64,
}

impl Admitted {
    /// The admitted lines of `lowered`, or the refusal of the definition.
    fn of<E>(lowered: Lowered) -> Result<Self, ViewError<E>> {
        let Lowered {
            admission,
            term_nodes,
            size,
            weight,
        } = lowered;
        Ok(Self {
            lines: admission.into_result().map_err(ViewError::Refused)?,
            term_nodes,
            size,
            weight,
        })
    }
}

/// Admit the working state of `lines` beyond the `charged` bytes already
/// admitted while lowering, and return the lines with the whole grant. A
/// refusal destroys `lines` before it returns what was admitted.
fn grant<E, S: StorageAdmission + ?Sized>(
    lines: LineSet,
    charged: usize,
    memory: &mut Memory<'_, S>,
) -> Result<(LineSet, usize), ViewError<E>> {
    let Some(bytes) = lines
        .footprint_bytes()
        .zip(optimised::working_bytes(&lines))
        .and_then(|(held, working)| held.checked_add(working))
    else {
        drop(lines);
        let _ = memory.release_bytes(charged);
        return Err(ViewError::Storage(StorageError::SizeOverflow));
    };
    let rest = bytes.saturating_sub(charged);
    match memory.add_bytes(rest) {
        Ok(()) => Ok((lines, charged.saturating_add(rest))),
        Err(error) => {
            drop(lines);
            let _ = memory.release_bytes(charged);
            Err(ViewError::Storage(error))
        }
    }
}

/// [`compare_view`] with the working state of the labelling admitted through
/// the caller's `memory`.
///
/// Each constant and blank node is admitted as lowering interns it, before it
/// is copied into the set. When the input holds a nested triple term, the
/// lowering also admits, as it keeps them: for each nested term, its
/// components in the list and as the key of its index, its index and
/// term-node entries and the term node's source slot; for each triple-term
/// identifier lowered as a nested term, its index entry; for each triple-term
/// identifier with a nested component, its entry in the set of counted
/// holders; for each constant or composite node first reached as a component
/// of a nested term, its source `K` with its box and key; before the digests,
/// an upper bound of the digests and blank-node digests they keep; the growth
/// of each blank node's source to the text form; and each term node's source.
/// An input with no nested triple term keeps none of these. After lowering, the grant covers the rest of the
/// lowered lines and an upper bound of the arrays the engine builds for the
/// components and their first calls, proportional to the incidences, the
/// constant leaf occurrences, the vertices and the structs each top component
/// holds (7.12 of the definition). A refusal of the admission is
/// [`ViewError::Storage`] and nothing is labelled. On success the grant stays
/// with the returned [`Comparison`], whose `admitted_bytes` states it: release that many bytes
/// when the comparison is dropped. On any other error the grant is returned
/// before the error is. Not metered: the present canonical form, the temporary
/// encoding of one literal while it is interned, the second copy of each
/// constant and node held by the interning index, the token copies the
/// admission of lines keeps to detect repeats (together about the size of the
/// admitted lines again, at the peak of lowering), the spare capacity of the
/// nested-term tables and the work lists and message buffers of the nested
/// walks and digests, and the certificates and nested instances held while a
/// search runs. The admission is the bound of the
/// up-front working state, and the engine holds nothing else of that order
/// before it starts.
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
    memory
        .admission_mut()
        .checkpoint(0)
        .map_err(ViewError::Storage)?;
    let outcome = try_canonical_sha256_view(view, CanonHash::Sha256, present);
    let stop = present.stop_signal();
    let (admitted, charged) = lowered_in(view, Scope::Dataset, records, stop, memory)?;
    let (lines, bytes) = grant(admitted.lines, charged, memory)?;
    match label_lines(&lines, limit, stop) {
        Ok((labelled, _)) => Ok(Comparison {
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
    walked: u64,
    blanks: Vec<Option<Id>>,
    /// Whether the lines hold term nodes, and so every blank node of the view
    /// has the text form of its source identifier.
    term_nodes: bool,
    size: u64,
    weight: u64,
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

    /// The limited units the walk of 8.3 charged, the figure a refusal of
    /// `limit` reports as its units: START, REFINE, FIX-SUM, SPLIT and TWIN.
    /// [`Labelled::explored`] is a separate engine figure that has no twin
    /// units.
    #[must_use]
    pub const fn walked_units(&self) -> u64 {
        self.walked
    }

    /// The view's identifier of the blank node with index `node` in
    /// [`Self::lines`] (index `i` of [`Labelled::placements`]). `None` for a
    /// blank node that only a [`Record`] mentions, for a composite node and
    /// for a term node: the view does not hold them.
    #[must_use]
    pub fn blank_id(&self, node: usize) -> Option<Id> {
        self.blanks.get(node).copied().flatten()
    }

    /// Every blank node of the view that occurs in a line, as
    /// `(view identifier, position)` in node order. Term nodes and composite
    /// nodes are not blank nodes of the view and are not listed, so the
    /// positions listed need not be consecutive.
    pub fn view_positions(&self) -> impl Iterator<Item = (Id, u64)> + '_ {
        self.blanks
            .iter()
            .zip(&self.labelled.placements)
            .filter_map(|(id, placement)| id.map(|id| (id, placement.position)))
    }

    /// The size of the lowered input: its lines before the nested-term
    /// records, and its nested candidates, the components of its triple terms
    /// that are triple terms. [`LabellingCapacity`] judges it.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// The weight of the lowered input: the bytes of the encoding of every line
    /// and every nested-term record, with each blank leaf written `fixed`,
    /// saturating at `2^64 - 1`. [`LabellingCapacity`] judges it.
    #[must_use]
    pub const fn weight(&self) -> u64 {
        self.weight
    }

    /// The literal each composite node and the triple term each term node of
    /// the canonical line sequence spells.
    pub(crate) fn composites(&self) -> Result<Composites, DocumentError> {
        let positions = |select: fn(&[u8]) -> bool| -> BTreeSet<u64> {
            (0..self.lines.blank_nodes())
                .filter(|&node| select(self.lines.source(node)))
                .map(|node| self.labelled.placements[node].position)
                .collect()
        };
        let composites = positions(is_composite_source);
        let term_nodes = if self.term_nodes {
            positions(is_term_node_source)
        } else {
            BTreeSet::new()
        };
        Composites::of(
            &self.labelled.lines,
            |position| composites.contains(&position),
            |position| term_nodes.contains(&position),
        )
    }
}

/// The blank node `(label, scope)` of the view that the source identifier
/// `source` names, in the form the lines hold it.
fn view_blank(source: &[u8], text_form: bool) -> Option<(&[u8], BlankScope)> {
    if text_form {
        parse_text_blank_source(source).map(|(ordinal, label)| (label, BlankScope(ordinal)))
    } else if is_composite_source(source) {
        None
    } else {
        source
            .split_first_chunk::<4>()
            .map(|(ordinal, label)| (label, BlankScope(u32::from_be_bytes(*ordinal))))
    }
}

/// The labelling of admitted lines, with the view's identifier of each blank
/// node.
fn labelling_of<D: DatasetView>(
    view: &D,
    admitted: Admitted,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelling<D::Id>, ViewError<D::ReadError>> {
    let Admitted {
        lines,
        term_nodes,
        size,
        weight,
    } = admitted;
    let (labelled, walked) = label_lines(&lines, limit, stop)?;
    let text_form = term_nodes > 0;
    let mut blanks = Vec::with_capacity(lines.blank_nodes());
    for node in 0..lines.blank_nodes() {
        let id = match view_blank(lines.source(node), text_form) {
            Some((label, scope)) => match std::str::from_utf8(label) {
                Ok(label) => view
                    .term_id_by_value(&TermValue::Blank {
                        label: label.to_owned(),
                        scope,
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
        walked,
        blanks,
        term_nodes: text_form,
        size,
        weight,
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
/// definition, [`Refusal::Limit`] included, or [`ViewError::Lowering`] with
/// [`LoweringRefusal::TermNode`].
pub fn label_view<D: DatasetView>(
    view: &D,
    records: &[Record<'_>],
    scope: Scope<'_>,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelling<D::Id>, ViewError<D::ReadError>> {
    let admitted = Admitted::of(lowered(view, scope, records, Run::plain(stop))?)?;
    labelling_of(view, admitted, limit, stop)
}

/// [`label_view`] as one input of a read with the capacities of `capacity`:
/// the size of the input is judged against its lines, and then its weight
/// against the bytes the read has left, before any digest of a term node is
/// formed, any nested-term record is built or the engine runs. An admitted
/// weight stays in the read's total whatever refusal follows it.
///
/// The refusals are judged in one order, and the first that applies is the
/// one returned: [`LoweringRefusal::Size`], [`LoweringRefusal::Weight`],
/// [`LoweringRefusal::TermNode`], then the refusals of the definition by
/// their code.
///
/// # Errors
/// As [`label_view`], and [`ViewError::Lowering`] with the size or weight
/// refusal.
pub fn label_view_with_capacity<D: DatasetView>(
    view: &D,
    records: &[Record<'_>],
    scope: Scope<'_>,
    limit: Limit,
    stop: Option<&dyn StopSignal>,
    capacity: &mut LabellingCapacity,
) -> Result<Labelling<D::Id>, ViewError<D::ReadError>> {
    let run = Run {
        capacity: Some(capacity),
        ..Run::plain(stop)
    };
    let admitted = Admitted::of(lowered(view, scope, records, run)?)?;
    labelling_of(view, admitted, limit, stop)
}

/// A labelling and the bytes of working state still granted for it.
type LabellingWithBytes<Id> = (Labelling<Id>, usize);

/// [`label_view`] with the working state of the labelling admitted through the
/// caller's `memory`, each constant and node as it is interned, as
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
    let (admitted, charged) = lowered_in(view, scope, records, stop, memory)?;
    let (lines, bytes) = grant(admitted.lines, charged, memory)?;
    let admitted = Admitted { lines, ..admitted };
    match labelling_of(view, admitted, limit, stop) {
        Ok(labelling) => Ok((labelling, bytes)),
        Err(error) => {
            // A refusal of the refund cannot replace the error that ended the
            // run: the grant stays outstanding with the caller's account.
            let _ = memory.release_bytes(bytes);
            Err(error)
        }
    }
}
