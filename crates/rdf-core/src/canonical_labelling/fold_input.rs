// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::TryReserveError;
use std::io::{self, Write};

use purrdf_lex::allocation::StorageError;
use purrdf_lex::walk::WorkList;

use super::lines::{
    Admitter, Facts, TAG_ABSENT, TAG_PRESENT, TAG_TRIPLE, Tok, iri_encoding, literal_encoding,
};
use super::nested::{
    NestedTerms, Part, iri_source, literal_source, nested_term_digest, text_blank_source,
};
use super::view::finish_nested;
use super::{
    Direction, GraphTerm, Halt, InputReason, Kind, Labelled, LabellingCapacity, Limit, LineSet,
    Quantity, Refusal, ViewError, optimised,
};
use crate::{FastMap, FastSet, RdfTextDirection};

/// A borrowed term in a folded input. Handles identify interned values and
/// precede any triple term that names them.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum FoldTerm<'a> {
    /// IRI text.
    Iri(&'a str),
    /// A literal with its original lexical form and language.
    Literal {
        /// Lexical form.
        lexical: &'a str,
        /// Datatype IRI, including the generated datatype of a language literal.
        datatype: &'a str,
        /// Language as supplied by the source.
        language: Option<&'a str>,
        /// Base direction.
        direction: Option<Direction>,
    },
    /// A blank node with a scoped byte label.
    Blank {
        /// Scope ordinal used in its CBOR source identifier.
        ordinal: u64,
        /// Original label bytes.
        label: &'a [u8],
    },
    /// Subject, predicate and object handles.
    Triple([usize; 3]),
}

impl FoldTerm<'_> {
    const fn variant(self) -> GraphTerm {
        match self {
            Self::Iri(_) => GraphTerm::Iri,
            Self::Literal { .. } => GraphTerm::Literal,
            Self::Blank { .. } => GraphTerm::Blank,
            Self::Triple(_) => GraphTerm::Triple,
        }
    }

    fn leaf_weight(self) -> Option<u128> {
        Some(match self {
            Self::Iri(iri) => 9 + iri.len() as u128,
            Self::Blank { .. } => 9,
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                19 + lexical.len() as u128
                    + datatype.len() as u128
                    + language.map_or(0, |language| 8 + language.len() as u128)
                    + u128::from(direction.is_some())
            }
            Self::Triple(_) => return None,
        })
    }
}

/// An RDF line of an already-unioned fold, expressed in interned handles.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum FoldLine {
    /// Subject, predicate, object, and optional graph.
    Quad([usize; 3], Option<usize>),
    /// Reifier, triple term, and optional graph.
    ReifierBinding([usize; 2], Option<usize>),
    /// Reifier, predicate, object, count, and optional graph.
    Annotation([usize; 3], u64, Option<usize>),
    /// Empty named graph.
    EmptyGraph(usize),
}

impl FoldLine {
    const fn parts(self) -> (Kind, [Option<usize>; 3], Option<u64>, Option<usize>) {
        match self {
            Self::Quad([s, p, o], g) => (Kind::Quad, [Some(s), Some(p), Some(o)], None, g),
            Self::ReifierBinding([r, t], g) => {
                (Kind::ReifierBinding, [Some(r), Some(t), None], None, g)
            }
            Self::Annotation([r, p, o], count, g) => (
                Kind::Annotation,
                [Some(r), Some(p), Some(o)],
                Some(count),
                g,
            ),
            Self::EmptyGraph(g) => (Kind::EmptyGraph, [Some(g), None, None], None, None),
        }
    }
}

/// A record of an already-unioned fold.
pub trait FoldRecord {
    /// Failure while reading or encoding a record.
    type Error;

    /// Visit payload bytes and term handles in their respective order.
    /// This method is called once to measure and again to emit. The output
    /// must be identical in both passes.
    /// # Errors
    /// Returns the source, sink, or term visitor's typed failure.
    fn encode(
        &self,
        payload: &mut dyn Write,
        term: impl FnMut(usize) -> Result<(), ViewError<Self::Error>>,
    ) -> Result<(), ViewError<Self::Error>>;
}

/// Capacity inputs for labelling one folded set.
#[derive(Clone, Copy, Debug)]
pub struct FoldLabelling {
    /// The size of the folded frames before set union.
    pub size: u128,
    /// The fold overflowed an annotation count during union.
    pub annotation_width: bool,
    /// Engine work limit.
    pub limit: Limit,
}

#[derive(Default)]
struct RecordBytes(u128);

impl Write for RecordBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 += bytes.len() as u128;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct RecordPayload {
    bytes: Vec<u8>,
    length: usize,
    exceeded: bool,
}

impl Write for RecordPayload {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.length - self.bytes.len() {
            self.exceeded = true;
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct MeasuredRecords {
    ends: Vec<(u128, usize)>,
    handles: Vec<usize>,
}

fn storage<E>(_: TryReserveError) -> ViewError<E> {
    ViewError::Storage(StorageError::AllocationFailed)
}

fn slot_error<E>() -> ViewError<E> {
    ViewError::Refused(Refusal::Input {
        reason: InputReason::Slot,
    })
}

struct Terms<'a, F> {
    lookup: F,
    forms: FastMap<usize, FoldTerm<'a>>,
    weights: FastMap<usize, (u128, Option<usize>)>,
    nested: FastSet<usize>,
    nested_weight: u128,
}

impl<'a, F: Fn(usize) -> Option<FoldTerm<'a>>> Terms<'a, F> {
    fn get<E>(&mut self, handle: usize) -> Result<FoldTerm<'a>, ViewError<E>> {
        if let Some(&term) = self.forms.get(&handle) {
            return Ok(term);
        }
        let term = (self.lookup)(handle).ok_or(ViewError::ForeignId)?;
        self.forms.try_reserve(1).map_err(storage)?;
        self.forms.insert(handle, term);
        Ok(term)
    }

    fn triple<E>(
        &mut self,
        handle: usize,
        ids: [usize; 3],
    ) -> Result<[FoldTerm<'a>; 3], ViewError<E>> {
        let [s, p, o] = ids;
        if [s, p, o].into_iter().any(|child| child >= handle) {
            return Err(ViewError::ForeignId);
        }
        let parts = [self.get(s)?, self.get(p)?, self.get(o)?];
        if matches!(parts[0], FoldTerm::Triple(_)) || parts[1].variant() != GraphTerm::Iri {
            return Err(slot_error());
        }
        Ok(parts)
    }

    fn measure<E>(&mut self, handle: usize) -> Result<(u128, Option<usize>), ViewError<E>> {
        if let Some(&measured) = self.weights.get(&handle) {
            return Ok(measured);
        }
        let term = self.get(handle)?;
        let measured = if let FoldTerm::Triple(ids) = term {
            let parts = self.triple(handle, ids)?;
            let weight = 1 + parts
                .into_iter()
                .map(|part| part.leaf_weight().unwrap_or(9))
                .sum::<u128>();
            (
                weight,
                matches!(parts[2], FoldTerm::Triple(_)).then_some(ids[2]),
            )
        } else {
            (term.leaf_weight().ok_or(ViewError::ForeignId)?, None)
        };
        self.weights.try_reserve(1).map_err(storage)?;
        self.weights.insert(handle, measured);
        Ok(measured)
    }

    fn weight<E>(&mut self, handle: usize) -> Result<u128, ViewError<E>> {
        let (weight, mut child) = self.measure(handle)?;
        while let Some(id) = child {
            if self.nested.contains(&id) {
                break;
            }
            self.nested.try_reserve(1).map_err(storage)?;
            self.nested.insert(id);
            let (term_weight, next) = self.measure(id)?;
            // A nested record has the same three components as the term,
            // with 37 bytes of record overhead in place of the triple tag.
            self.nested_weight += 36 + term_weight;
            child = next;
        }
        Ok(weight)
    }

    fn measure_records<R: FoldRecord>(
        &mut self,
        records: impl Iterator<Item = R>,
        weight: &mut u128,
    ) -> Result<MeasuredRecords, ViewError<R::Error>> {
        let mut measured = MeasuredRecords::default();
        for record in records {
            let mut bytes = RecordBytes::default();
            record.encode(&mut bytes, |handle| {
                *weight += self.weight(handle)?;
                measured.handles.try_reserve(1).map_err(storage)?;
                measured.handles.push(handle);
                Ok(())
            })?;
            *weight += 17 + bytes.0;
            measured.ends.try_reserve(1).map_err(storage)?;
            measured.ends.push((bytes.0, measured.handles.len()));
        }
        Ok(measured)
    }
}

/// Label a folded set of RDF lines and records in an interned term arena.
/// Size and weight are charged before payloads and lowered lines are formed.
/// Equal typed values must use the same handle, and lines and records must be
/// distinct. Blank sources use the deterministic CBOR pair `[ordinal, label]`.
///
/// # Errors
/// Returns source, storage or decoding errors, or the first applicable
/// lowering or labelling refusal.
pub fn label_fold<'a, R: FoldRecord>(
    lines: impl Iterator<Item = FoldLine>,
    records: impl Iterator<Item = R> + Clone,
    lookup: impl Fn(usize) -> Option<FoldTerm<'a>>,
    limits: FoldLabelling,
    capacity: &mut LabellingCapacity,
) -> Result<(LineSet, Labelled), ViewError<R::Error>> {
    capacity
        .admit_exact_size(limits.size)
        .map_err(ViewError::Lowering)?;
    let mut terms = Terms {
        lookup,
        forms: FastMap::default(),
        weights: FastMap::default(),
        nested: FastSet::default(),
        nested_weight: 0,
    };
    let mut weight = 0;
    let mut measured_lines = Vec::new();
    for line in lines {
        measured_lines.try_reserve(1).map_err(storage)?;
        measured_lines.push(line);
        let (kind, ids, count, graph) = line.parts();
        weight += 1 + 8 * u128::from(count.is_some()) + u128::from(kind != Kind::EmptyGraph);
        for handle in ids.into_iter().flatten().chain(graph) {
            weight += terms.weight(handle)?;
        }
    }
    let measured = terms.measure_records(records.clone(), &mut weight)?;
    capacity
        .admit_exact_size(
            measured_lines.len() as u128 + measured.ends.len() as u128 + terms.nested.len() as u128,
        )
        .map_err(ViewError::Lowering)?;
    capacity
        .admit_exact_weight(weight + terms.nested_weight)
        .map_err(ViewError::Lowering)?;
    let Terms { forms, weights, .. } = terms;
    let mut lower = Lower {
        forms,
        weights,
        admitter: Admitter::default(),
        nested: NestedTerms::default(),
        term_nodes: Vec::new(),
        nested_ids: FastMap::default(),
        leaves: FastMap::default(),
    };
    for line in measured_lines {
        lower.line(line)?;
    }
    lower.records(records, measured)?;
    if !lower.nested.is_empty() {
        finish_nested(
            &mut lower.admitter,
            &lower.nested,
            &lower.term_nodes,
            nested_term_digest,
            None,
            &mut None,
        )?;
    }
    let set = lower
        .admitter
        .finish()
        .into_result()
        .map_err(ViewError::Refused)?;
    if limits.annotation_width {
        return Err(ViewError::Refused(Refusal::Width {
            quantity: Quantity::Count,
        }));
    }
    let labelled = optimised::label_set(&set, limits.limit.get(), false, None)
        .map_err(|halt| ViewError::Refused(Halt::expect_refusal(halt)))?;
    Ok((set, labelled))
}

struct Lower<'a> {
    forms: FastMap<usize, FoldTerm<'a>>,
    weights: FastMap<usize, (u128, Option<usize>)>,
    admitter: Admitter,
    nested: NestedTerms,
    term_nodes: Vec<usize>,
    nested_ids: FastMap<usize, usize>,
    leaves: FastMap<usize, Part>,
}

impl<'a> Lower<'a> {
    fn get<E>(&self, handle: usize) -> Result<FoldTerm<'a>, ViewError<E>> {
        self.forms.get(&handle).copied().ok_or(ViewError::ForeignId)
    }

    fn records<R: FoldRecord>(
        &mut self,
        mut records: impl Iterator<Item = R>,
        measured: MeasuredRecords,
    ) -> Result<(), ViewError<R::Error>> {
        let mut start = 0;
        for (length, end) in measured.ends {
            let record = records.next().ok_or_else(slot_error)?;
            let length = usize::try_from(length)
                .map_err(|_| ViewError::Storage(StorageError::SizeOverflow))?;
            let mut payload = RecordPayload {
                bytes: Vec::new(),
                length,
                exceeded: false,
            };
            let mut handles = measured.handles[start..end].iter().copied();
            let mut body = Vec::new();
            let mut changed = false;
            record.encode(&mut payload, |handle| {
                if handles.next() != Some(handle) {
                    changed = true;
                    return Err(ViewError::ForeignId);
                }
                self.term(handle, &mut body)?;
                Ok(())
            })?;
            if changed
                || handles.next().is_some()
                || payload.exceeded
                || payload.bytes.len() != length
            {
                return Err(slot_error());
            }
            let mut tokens = Vec::new();
            self.admitter
                .interner
                .record_head(&payload.bytes, end - start, &mut tokens);
            tokens.append(&mut body);
            self.admitter.push(tokens, facts(None, None, true, false));
            start = end;
        }
        if records.next().is_some() {
            return Err(slot_error());
        }
        Ok(())
    }

    fn leaf<E>(&mut self, id: usize, keep_source: bool) -> Result<Part, ViewError<E>> {
        let term = self.get(id)?;
        let part = if let Some(&part) = self.leaves.get(&id) {
            part
        } else {
            let interner = &mut self.admitter.interner;
            let part = match term {
                FoldTerm::Iri(iri) => Part::Const(interner.constant(&iri_encoding(iri.as_bytes()))),
                FoldTerm::Literal {
                    lexical,
                    datatype,
                    language,
                    direction,
                } => Part::Const(interner.constant(&literal_encoding(
                    lexical.as_bytes(),
                    datatype.as_bytes(),
                    language.map(str::as_bytes),
                    direction,
                ))),
                FoldTerm::Blank { ordinal, label } => {
                    Part::Blank(interner.node(&text_blank_source(ordinal, label)))
                }
                FoldTerm::Triple(_) => return Err(slot_error()),
            };
            self.leaves.try_reserve(1).map_err(storage)?;
            self.leaves.insert(id, part);
            part
        };
        if keep_source && self.nested.needs_source(part) {
            let source = match term {
                FoldTerm::Iri(iri) => Some(iri_source(iri)),
                FoldTerm::Literal {
                    lexical,
                    datatype,
                    language,
                    direction,
                } => {
                    let direction = direction.map(|direction| match direction {
                        Direction::Ltr => RdfTextDirection::Ltr,
                        Direction::Rtl => RdfTextDirection::Rtl,
                    });
                    Some(literal_source(lexical, datatype, language, direction))
                }
                _ => None,
            };
            if let Some(source) = source {
                self.nested.keep_source(part, source);
            }
        }
        Ok(part)
    }

    fn nested<E>(&mut self, root: usize) -> Result<Part, ViewError<E>> {
        let mut pending = WorkList::<(usize, [usize; 3]), 8>::new();
        let mut id = root;
        while !self.nested_ids.contains_key(&id) {
            let FoldTerm::Triple(ids) = self.get(id)? else {
                return Err(ViewError::ForeignId);
            };
            pending.try_push((id, ids)).map_err(storage)?;
            let Some(&(_, Some(child))) = self.weights.get(&id) else {
                break;
            };
            id = child;
        }
        while let Some((id, [s, p, o])) = pending.pop() {
            let object = match self.nested_ids.get(&o) {
                Some(&child) => Part::Nested(child),
                None => self.leaf(o, true)?,
            };
            let parts = [self.leaf(s, true)?, self.leaf(p, true)?, object];
            let nested = if let Some(index) = self.nested.get(&parts) {
                index
            } else {
                let index = self.nested.push(parts);
                self.term_nodes.try_reserve(1).map_err(storage)?;
                self.term_nodes.push(self.admitter.interner.term_node());
                index
            };
            self.nested_ids.try_reserve(1).map_err(storage)?;
            self.nested_ids.insert(id, nested);
        }
        self.nested_ids
            .get(&root)
            .copied()
            .map(Part::Nested)
            .ok_or(ViewError::ForeignId)
    }

    fn term<E>(&mut self, handle: usize, tokens: &mut Vec<Tok>) -> Result<GraphTerm, ViewError<E>> {
        let term = self.get(handle)?;
        let parts = if let FoldTerm::Triple([s, p, o]) = term {
            tokens.push(Tok::Tag(TAG_TRIPLE));
            let object = if matches!(self.get(o)?, FoldTerm::Triple(_)) {
                self.nested(o)?
            } else {
                self.leaf(o, false)?
            };
            [
                Some(self.leaf(s, false)?),
                Some(self.leaf(p, false)?),
                Some(object),
            ]
        } else {
            [Some(self.leaf(handle, false)?), None, None]
        };
        tokens.extend(parts.into_iter().flatten().map(|part| match part {
            Part::Const(index) => Tok::Const(index),
            Part::Blank(index) => Tok::Blank(index),
            Part::Nested(index) => Tok::Blank(self.term_nodes[index]),
        }));
        Ok(term.variant())
    }

    fn line<E>(&mut self, line: FoldLine) -> Result<(), ViewError<E>> {
        let (kind, ids, count, graph) = line.parts();
        let mut tokens = vec![Tok::Tag(kind as u8)];
        tokens.extend(count.map(Tok::Count));
        let mut slots_ok = true;
        let mut held = None;
        for (index, handle) in ids.into_iter().flatten().enumerate() {
            let variant = self.term(handle, &mut tokens)?;
            if kind == Kind::EmptyGraph {
                held = Some((tokens[1..].to_vec(), variant));
            }
            if index == 1 {
                slots_ok = variant
                    == if kind == Kind::ReifierBinding {
                        GraphTerm::Triple
                    } else {
                        GraphTerm::Iri
                    };
            }
        }
        let carried = if let Some(handle) = graph {
            tokens.push(Tok::Tag(TAG_PRESENT));
            let start = tokens.len();
            let variant = self.term(handle, &mut tokens)?;
            Some((tokens[start..].to_vec(), variant))
        } else {
            if kind != Kind::EmptyGraph {
                tokens.push(Tok::Tag(TAG_ABSENT));
            }
            None
        };
        self.admitter
            .push(tokens, facts(held, carried, slots_ok, count == Some(0)));
        Ok(())
    }
}

fn facts(
    held: Option<super::lines::GraphTokens>,
    carried: Option<super::lines::GraphTokens>,
    slots_ok: bool,
    zero_count: bool,
) -> Facts {
    Facts {
        slots_ok,
        zero_count,
        carried,
        held,
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;
    use std::io::{self, Write};

    use super::{
        FoldLabelling, FoldLine, FoldRecord, FoldTerm, LabellingCapacity, Limit, Quantity, Refusal,
        ViewError, label_fold,
    };

    #[derive(Clone, Copy)]
    struct Payload<'a>(&'a Cell<bool>);

    impl FoldRecord for Payload<'_> {
        type Error = io::Error;

        fn encode(
            &self,
            output: &mut dyn Write,
            _: impl FnMut(usize) -> Result<(), ViewError<io::Error>>,
        ) -> Result<(), ViewError<io::Error>> {
            let bytes = if self.0.replace(true) {
                &[0, 0][..]
            } else {
                &[0][..]
            };
            output.write_all(bytes).map_err(ViewError::Read)
        }
    }

    #[test]
    fn deferred_record_cannot_emit_more_than_its_admitted_payload_length() {
        let called = Cell::new(false);
        let mut capacity = LabellingCapacity::new(1, 18);
        let error = label_fold(
            core::iter::empty(),
            core::iter::once(Payload(&called)),
            |_| None,
            FoldLabelling {
                size: 1,
                annotation_width: false,
                limit: Limit::new(0),
            },
            &mut capacity,
        )
        .unwrap_err();
        assert!(
            matches!(error, ViewError::Read(error) if error.kind() == io::ErrorKind::InvalidData)
        );
        assert_eq!(capacity.total(), 18);
    }

    #[test]
    fn malformed_graph_is_reported_before_annotation_width() {
        let label = |with_empty: bool| {
            let lines = [
                Some(FoldLine::Quad([0, 1, 2], Some(3))),
                with_empty.then_some(FoldLine::EmptyGraph(3)),
            ];
            label_fold(
                lines.into_iter().flatten(),
                core::iter::empty::<Payload<'_>>(),
                |id| {
                    ["http://e/s", "http://e/p", "http://e/o", "http://e/g"]
                        .get(id)
                        .copied()
                        .map(FoldTerm::Iri)
                },
                FoldLabelling {
                    size: 2,
                    annotation_width: true,
                    limit: Limit::new(0),
                },
                &mut LabellingCapacity::new(2, u64::MAX),
            )
        };
        assert!(matches!(
            label(true),
            Err(ViewError::Refused(Refusal::Graph { .. }))
        ));
        assert!(matches!(
            label(false),
            Err(ViewError::Refused(Refusal::Width {
                quantity: Quantity::Count
            }))
        ));
    }
}
