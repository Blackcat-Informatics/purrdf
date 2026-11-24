// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::cmp::Ordering;
use std::io::{self, Write};

use crate::v2::fields::push;
use crate::v2::parity::RecordingWriter;
use crate::v2::references::LeafNode;
use crate::v2::{
    FrameType, Line, LiteralValue, RdfGraph, ResolvedLeaf, RowOrder, Suppression, Term, TermId,
    TermOrder, ValueName,
};

use super::rdf::{BlankLabel, Definition, Form, line_kind, wire_row, write_value};
use super::{SegmentWriteError, SegmentWriter};

// The frame budget, row count, restart group and literal thresholds of P7 §4.2.
const TERM_BUDGET: u128 = 4_194_304;
pub(super) const ROW_COUNT: usize = 1_048_576;
const RESTART_GROUP: usize = 64;
const LEXICAL_THRESHOLD: usize = 1_048_576;
const OCTETS_THRESHOLD: usize = 524_288;

#[derive(Clone, Copy, Default)]
struct Slot<'a> {
    used: bool,
    depth: usize,
    wire_id: Option<u64>,
    value: Option<&'a LiteralValue>,
}

pub(super) struct Dictionary<'a> {
    slots: Vec<Slot<'a>>,
    terms: Vec<Definition<'a>>,
    pub(super) values: Vec<(ValueName, &'a [u8])>,
}

impl<'a> Dictionary<'a> {
    pub(super) fn new(
        graph: &'a RdfGraph<'_>,
        blank_label: impl Fn(u64, &'a [u8]) -> Result<BlankLabel<'a>, SegmentWriteError>,
    ) -> Result<Self, SegmentWriteError> {
        let mut slots = Vec::new();
        let count = graph.rdf.terms().len();
        slots.try_reserve_exact(count).map_err(io::Error::other)?;
        slots.resize(count, Slot::default());
        mark_roots(graph, &mut slots)?;

        // A fold's arena defines every dependency before the term that uses it.
        for (index, term) in graph.rdf.terms().enumerate().rev() {
            if !slots[index].used {
                continue;
            }
            match term {
                Term::Typed { datatype, .. } if slots[index].value.is_none() => {
                    mark(&mut slots, *datatype)?;
                }
                Term::Triple(terms) => {
                    for &term in terms {
                        mark(&mut slots, term)?;
                    }
                }
                _ => {}
            }
        }

        let count = slots.iter().filter(|slot| slot.used).count();
        let mut dictionary = Self {
            slots,
            terms: Vec::new(),
            values: Vec::new(),
        };
        dictionary
            .terms
            .try_reserve_exact(count)
            .map_err(io::Error::other)?;
        let mut iris = Vec::new();
        for (index, term) in graph.rdf.terms().enumerate() {
            if dictionary.slots[index].used
                && let Term::Iri(text) = term
            {
                push(&mut iris, (text.as_ref(), index)).map_err(io::Error::other)?;
            }
        }
        iris.sort_unstable_by_key(|&(text, _)| text);
        for (text, index) in iris {
            dictionary.assign(index, Definition::Iri(text));
        }

        let mut literals_and_blanks = Vec::new();
        let mut triples = Vec::new();
        for (index, term) in graph.rdf.terms().enumerate() {
            let slot = dictionary.slots[index];
            if !slot.used || slot.wire_id.is_some() {
                continue;
            }
            let definition = match term {
                Term::Iri(_) => unreachable!("used IRIs have already been assigned"),
                Term::Typed { lexical, datatype } => match slot.value {
                    Some(LiteralValue::Octets(octets)) if octets.len() >= OCTETS_THRESHOLD => {
                        Definition::ValueRef(dictionary.value(octets)?)
                    }
                    Some(value) => Definition::Value(value),
                    None => dictionary.literal(lexical, Form::Typed(dictionary.id(*datatype)?))?,
                },
                Term::Language {
                    lexical,
                    language,
                    direction,
                } => dictionary.literal(lexical, Form::language(language, *direction))?,
                Term::Blank { segment, label } => Definition::Blank(blank_label(*segment, label)?),
                Term::Triple(terms) => {
                    let depth = 1 + terms
                        .iter()
                        .map(|term| dictionary.slots[term.index()].depth)
                        .max()
                        .expect("the triple has three components");
                    dictionary.slots[index].depth = depth;
                    push(&mut triples, (depth, index, *terms)).map_err(io::Error::other)?;
                    continue;
                }
            };
            push(&mut literals_and_blanks, (index, definition)).map_err(io::Error::other)?;
        }
        literals_and_blanks.sort_unstable_by(|(_, left), (_, right)| compare(*left, *right));
        for (index, definition) in literals_and_blanks {
            dictionary.assign(index, definition);
        }

        triples.sort_unstable_by_key(|&(depth, _, _)| depth);
        let mut assigned = Vec::new();
        for depth in triples.chunk_by(|a, b| a.0 == b.0) {
            assigned
                .try_reserve_exact(depth.len())
                .map_err(io::Error::other)?;
            for &(_, index, [s, p, o]) in depth {
                let terms = [dictionary.id(s)?, dictionary.id(p)?, dictionary.id(o)?];
                assigned.push((terms, index));
            }
            assigned.sort_unstable();
            for &(terms, index) in &assigned {
                dictionary.assign(index, Definition::Triple(terms));
            }
            assigned.clear();
        }
        Ok(dictionary)
    }

    fn assign(&mut self, index: usize, definition: Definition<'a>) {
        self.slots[index].wire_id = Some(self.terms.len() as u64);
        self.terms.push(definition);
    }

    fn literal(
        &mut self,
        lexical: &'a str,
        form: Form<'a>,
    ) -> Result<Definition<'a>, SegmentWriteError> {
        Ok(if lexical.len() >= LEXICAL_THRESHOLD {
            Definition::LiteralRef(self.value(lexical.as_bytes())?, form)
        } else {
            Definition::Literal(lexical, form)
        })
    }

    fn value(&mut self, bytes: &'a [u8]) -> io::Result<ValueName> {
        let name = ValueName::of(bytes);
        push(&mut self.values, (name, bytes)).map_err(io::Error::other)?;
        Ok(name)
    }

    pub(super) fn id(&self, term: TermId) -> Result<u64, SegmentWriteError> {
        self.slots
            .get(term.index())
            .and_then(|slot| slot.wire_id)
            .ok_or_else(|| foreign_term(term))
    }

    pub(super) fn write_rdf<W: Write>(
        &self,
        graph: &RdfGraph<'_>,
        writer: &mut SegmentWriter<W>,
    ) -> Result<(), SegmentWriteError> {
        let mut start = 0;
        let mut weight = 0;
        for (index, &term) in self.terms.iter().enumerate() {
            let term_weight = weight_of(term)?;
            if start != index && weight + term_weight > TERM_BUDGET {
                self.write_terms(start, index, writer)?;
                start = index;
                weight = 0;
            }
            weight += term_weight;
        }
        if start < self.terms.len() {
            self.write_terms(start, self.terms.len(), writer)?;
        }

        let mut batch = Vec::new();
        for kind in [
            FrameType::Quads,
            FrameType::ReifierBindings,
            FrameType::Annotations,
            FrameType::EmptyGraphs,
        ] {
            for line in graph.rdf.lines().filter(|&line| line_kind(line) == kind) {
                // All line handles were checked and marked before assigning IDs.
                let row = wire_row(line, |term| {
                    self.slots[term.index()]
                        .wire_id
                        .expect("a line member has an assigned ID")
                });
                push(&mut batch, row).map_err(io::Error::other)?;
            }
            batch.sort_unstable_by_key(|row| row.order_key());
            for rows in batch.chunks(ROW_COUNT) {
                writer.write_row_batch(kind, rows, RowOrder::Canonical)?;
            }
            batch.clear();
        }
        Ok(())
    }

    fn write_terms<W: Write>(
        &self,
        start: usize,
        end: usize,
        writer: &mut SegmentWriter<W>,
    ) -> Result<(), SegmentWriteError> {
        writer.write_term_batch(
            &self.terms[start..end],
            start as u64,
            TermOrder::Canonical,
            RESTART_GROUP,
        )
    }
}

fn foreign_term(term: TermId) -> SegmentWriteError {
    SegmentWriteError::ReferenceTerm(crate::v2::FoldError::TermHandle(term))
}

fn mark(slots: &mut [Slot<'_>], term: TermId) -> Result<(), SegmentWriteError> {
    slots
        .get_mut(term.index())
        .ok_or_else(|| foreign_term(term))?
        .used = true;
    Ok(())
}

fn mark_roots<'a>(
    graph: &'a RdfGraph<'_>,
    slots: &mut [Slot<'a>],
) -> Result<(), SegmentWriteError> {
    for line in graph.rdf.lines() {
        let (terms, graph): (&[TermId], _) = match &line {
            Line::Quad(terms, graph) | Line::Annotation(terms, _, graph) => (terms, *graph),
            Line::ReifierBinding(terms, graph) => (terms, *graph),
            Line::EmptyGraph(graph) => (core::slice::from_ref(graph), None),
        };
        for term in terms.iter().copied().chain(graph) {
            mark(slots, term)?;
        }
    }
    for (term, value) in graph.rdf.values() {
        mark(slots, *term)?;
        slots[term.index()].value = Some(value);
    }
    let mut leaf = |leaf: &ResolvedLeaf| -> Result<(), SegmentWriteError> {
        leaf.visit(
            |node| {
                if let LeafNode::Term(term) = node {
                    mark(slots, term)?;
                }
                Ok(())
            },
            |cause| io::Error::other(cause).into(),
        )
    };
    for binding in &graph.references.bindings {
        leaf(&binding.term)?;
    }
    for claim in &graph.references.suppressions {
        let (terms, graph): (&[ResolvedLeaf], _) = match &claim.entry {
            Suppression::Term(term) | Suppression::Reifier(term) | Suppression::Graph(term) => {
                (core::slice::from_ref(term), None)
            }
            Suppression::Quad(terms, graph) | Suppression::Annotation(terms, graph) => {
                (terms, graph.as_ref())
            }
            Suppression::ReifierBinding(terms, graph) => (terms, graph.as_ref()),
            Suppression::Binding(binding) => (core::slice::from_ref(&binding.term), None),
            Suppression::Blob(_) => continue,
        };
        for term in terms.iter().chain(graph) {
            leaf(term)?;
        }
    }
    Ok(())
}

fn weight_of(term: Definition<'_>) -> io::Result<u128> {
    let length = match term {
        Definition::Iri(text) | Definition::Literal(text, _) => text.len() as u128,
        Definition::Blank(label) => label.bytes().len() as u128,
        Definition::Value(value) => {
            let mut length = 0;
            write_value(
                &mut RecordingWriter {
                    sink: &mut io::sink(),
                    recorder: None,
                    length: Some(&mut length),
                },
                value,
            )?;
            u128::from(length)
        }
        Definition::Triple(_) | Definition::LiteralRef(..) | Definition::ValueRef(_) => 0,
    };
    Ok(length + 8)
}

fn compare(left: Definition<'_>, right: Definition<'_>) -> Ordering {
    match (left, right) {
        (Definition::Literal(a, af), Definition::Literal(b, bf)) => (a, af).cmp(&(b, bf)),
        (Definition::LiteralRef(a, af), Definition::LiteralRef(b, bf)) => {
            (a.digest, a.length, af).cmp(&(b.digest, b.length, bf))
        }
        (Definition::Value(a), Definition::Value(b)) => value_order(a).cmp(&value_order(b)),
        (Definition::ValueRef(a), Definition::ValueRef(b)) => {
            (a.digest, a.length).cmp(&(b.digest, b.length))
        }
        (Definition::Blank(a), Definition::Blank(b)) => a.bytes().cmp(b.bytes()),
        _ => kind_order(left).cmp(&kind_order(right)),
    }
}

const fn kind_order(definition: Definition<'_>) -> u8 {
    match definition {
        Definition::Iri(_) => 1,
        Definition::Literal(..) => 2,
        Definition::LiteralRef(..) => 3,
        Definition::Value(_) => 4,
        Definition::ValueRef(_) => 5,
        Definition::Blank(_) => 6,
        Definition::Triple(_) => 7,
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum ValueOrder<'a> {
    Boolean(bool),
    Unsigned(u64),
    Integer(i128),
    Time(i64, u32),
    Octets(&'a [u8]),
}

fn value_order(value: &LiteralValue) -> ValueOrder<'_> {
    match value {
        LiteralValue::Boolean(value) => ValueOrder::Boolean(*value),
        LiteralValue::Unsigned(value) => ValueOrder::Unsigned(*value),
        LiteralValue::Integer(value) => ValueOrder::Integer(*value),
        LiteralValue::Time(seconds, nanoseconds) => ValueOrder::Time(*seconds, *nanoseconds),
        LiteralValue::Octets(bytes) => ValueOrder::Octets(bytes),
    }
}
