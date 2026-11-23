// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_lex::cbor::head::{self, write_bytes, write_text};

use super::{Buffer, FrameType, SegmentWriteError, SegmentWriter};
use crate::FastMap;
use crate::v2::{Line, LiteralDirection, LiteralValue, RdfFold, Term, TermId, TermOrder};

const FRAME_ROWS: usize = 4096;

impl<W: Write> SegmentWriter<W> {
    /// Append a resolved RDF fold in arrival order.
    ///
    /// Each call assigns new labels to its blank nodes. Blank nodes from
    /// different scopes or calls therefore remain distinct.
    /// Typed values, reifier bindings, annotation counts and empty graphs
    /// are emitted with their generation 2 storage forms.
    ///
    /// # Errors
    /// Returns an allocation, wire-range, or sink error. If any item of the
    /// fold has reached the sink, a failed call prevents further appends.
    pub fn add_rdf(&mut self, fold: &RdfFold) -> Result<(), SegmentWriteError> {
        if fold.has_held_rows() || fold.annotation_width_exceeded() {
            return Err(SegmentWriteError::IncompleteFold);
        }
        let count = fold.terms().len();
        if count == 0 {
            return Ok(());
        }
        let end = self.term_base + count as u128;
        if end > u128::from(u64::MAX) + 1 {
            return Err(SegmentWriteError::DictionaryFull);
        }
        let base = u64::try_from(self.term_base).map_err(|_| SegmentWriteError::DictionaryFull)?;
        let head = self.head;
        let result = self
            .write_terms(fold, base)
            .and_then(|()| self.write_lines(fold, base));
        if result.is_err() && self.head != head {
            self.head = None;
        }
        result?;
        self.term_base = end;
        Ok(())
    }

    fn write_terms(&mut self, fold: &RdfFold, base: u64) -> Result<(), SegmentWriteError> {
        let mut values = FastMap::default();
        values
            .try_reserve(fold.values().len())
            .map_err(io::Error::other)?;
        values.extend(fold.values().iter().map(|(id, value)| (id.index(), value)));
        let mut terms = fold.terms().enumerate().map(|(index, term)| {
            values
                .get(&index)
                .map_or(Definition::Term(term), |value| Definition::Value(value))
        });
        let mut batch = Vec::new();
        batch
            .try_reserve(FRAME_ROWS.min(terms.len()))
            .map_err(io::Error::other)?;
        let mut written = 0;
        loop {
            batch.extend(terms.by_ref().take(FRAME_ROWS));
            if batch.is_empty() {
                return Ok(());
            }
            self.write_term_batch(&batch, base, base + written)?;
            written += batch.len() as u64;
            batch.clear();
        }
    }

    fn write_term_batch(
        &mut self,
        terms: &[Definition<'_>],
        base: u64,
        first: u64,
    ) -> Result<(), SegmentWriteError> {
        let blocks = || terms.chunk_by(|a, b| a.kind() == b.kind());
        let block_count = blocks().count() as u64;
        let mut payload = Buffer::default();
        array(&mut payload, 2)?;
        array(&mut payload, block_count)?;
        let mut languages = Languages::default();
        let mut id = u128::from(first);
        for run in blocks() {
            let start = u64::try_from(id).expect("each run starts inside the admitted ID range");
            write_block(&mut payload, run, base, start, &mut languages)?;
            id += run.len() as u128;
        }
        list(&mut payload, &languages.ordered, |output, language| {
            write_text(output, language)
        })?;
        self.append(FrameType::Terms, &payload.0, |output| {
            array(output, 4)?;
            uint(output, first)?;
            array(output, block_count)?;
            for run in blocks() {
                array(output, 2)?;
                uint(output, run[0].kind())?;
                uint(output, run.len() as u64)?;
            }
            uint(output, TermOrder::Arrival as u64)?;
            uint(output, 1)
        })?;
        Ok(())
    }

    fn write_lines(&mut self, fold: &RdfFold, base: u64) -> Result<(), SegmentWriteError> {
        for kind in [
            FrameType::Quads,
            FrameType::ReifierBindings,
            FrameType::Annotations,
            FrameType::EmptyGraphs,
        ] {
            let mut rows = fold.lines().filter(|row| line_kind(*row) == kind);
            let mut batch = Vec::new();
            loop {
                for row in rows.by_ref().take(FRAME_ROWS) {
                    batch.try_reserve(1).map_err(io::Error::other)?;
                    batch.push(row);
                }
                if batch.is_empty() {
                    break;
                }
                self.write_line_batch(kind, &batch, base)?;
                batch.clear();
            }
        }
        Ok(())
    }

    fn write_line_batch(
        &mut self,
        kind: FrameType,
        lines: &[Line],
        base: u64,
    ) -> Result<(), SegmentWriteError> {
        let id = |id| wire_id(base, id);
        let mut payload = Buffer::default();
        if kind != FrameType::EmptyGraphs {
            let columns = match kind {
                FrameType::Quads => 4,
                FrameType::ReifierBindings => 3,
                FrameType::Annotations => 5,
                _ => unreachable!("named row frame type"),
            };
            array(&mut payload, columns)?;
            write_runs(&mut payload, lines, graph, |output, graph| match graph {
                Some(graph) => {
                    array(output, 2)?;
                    uint(output, 2)?;
                    uint(output, id(graph))
                }
                None => {
                    array(output, 1)?;
                    uint(output, 1)
                }
            })?;
            if kind == FrameType::Quads {
                write_runs(&mut payload, lines, |line| row_values(line, base)[0], uint)?;
            }
        } else {
            array(&mut payload, 1)?;
        }
        let delta_columns = match kind {
            FrameType::Quads => 1..3,
            FrameType::ReifierBindings => 0..2,
            FrameType::Annotations => 0..4,
            _ => 0..1,
        };
        for column in delta_columns {
            array(&mut payload, lines.len() as u64)?;
            let mut previous = 0;
            for &line in lines {
                let value = row_values(line, base)[column];
                int(&mut payload, i128::from(value) - i128::from(previous))?;
                previous = value;
            }
        }
        self.append(kind, &payload.0, |output| {
            if kind == FrameType::EmptyGraphs {
                array(output, 1)?;
            } else {
                array(output, 2)?;
                uint(output, 2)?;
            }
            uint(output, lines.len() as u64)
        })?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Definition<'a> {
    Term(&'a Term<'static>),
    Value(&'a LiteralValue),
}

impl<'a> Definition<'a> {
    const fn kind(&self) -> u64 {
        match self {
            Self::Term(Term::Iri(_)) => 1,
            Self::Term(Term::Typed { .. } | Term::Language { .. }) => 2,
            Self::Term(Term::Blank { .. }) => 3,
            Self::Term(Term::Triple(_)) => 4,
            Self::Value(_) => 6,
        }
    }

    fn text(self) -> &'a [u8] {
        match self {
            Self::Term(
                Term::Iri(text)
                | Term::Typed { lexical: text, .. }
                | Term::Language { lexical: text, .. },
            ) => text.as_bytes(),
            _ => unreachable!("only text blocks have front-coded columns"),
        }
    }
}

#[derive(Default)]
struct Languages<'a> {
    index: FastMap<&'a str, u64>,
    ordered: Vec<&'a str>,
}

impl<'a> Languages<'a> {
    fn intern(&mut self, language: &'a str) -> io::Result<u64> {
        if let Some(&index) = self.index.get(language) {
            return Ok(index);
        }
        let index = self.ordered.len() as u64;
        self.index.try_reserve(1).map_err(io::Error::other)?;
        self.ordered.try_reserve(1).map_err(io::Error::other)?;
        self.index.insert(language, index);
        self.ordered.push(language);
        Ok(index)
    }
}

fn write_block<'a>(
    output: &mut Buffer,
    terms: &[Definition<'a>],
    base: u64,
    first: u64,
    languages: &mut Languages<'a>,
) -> io::Result<()> {
    match terms[0].kind() {
        1 => {
            array(output, 4)?;
            uint(output, 1)?;
            write_text_columns(output, terms)?;
        }
        2 => write_literal_block(output, terms, base, languages)?,
        3 => {
            array(output, 2)?;
            uint(output, 3)?;
            array(output, terms.len() as u64)?;
            for index in 0..terms.len() {
                let id = first + index as u64;
                write_bytes(output, &id.to_be_bytes())?;
            }
        }
        4 => {
            array(output, 4)?;
            uint(output, 4)?;
            for column in 0..3 {
                list(output, terms, |output, term| {
                    let Definition::Term(Term::Triple(ids)) = term else {
                        unreachable!("one block holds one term kind");
                    };
                    uint(output, wire_id(base, ids[column]))
                })?;
            }
        }
        _ => {
            array(output, 2)?;
            uint(output, 6)?;
            list(output, terms, |output, term| {
                let Definition::Value(value) = term else {
                    unreachable!("one block holds one term kind");
                };
                write_value(output, value)
            })?;
        }
    }
    Ok(())
}

fn write_text_columns(output: &mut Buffer, terms: &[Definition<'_>]) -> io::Result<()> {
    list(output, terms, |output, _| uint(output, 0))?;
    list(output, terms, |output, term| {
        uint(output, term.text().len() as u64)
    })?;
    let length = terms.iter().try_fold(0_u64, |sum, term| {
        sum.checked_add(term.text().len() as u64).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "GTS v2 text column exceeds the wire length",
            )
        })
    })?;
    head::write_head(output, head::BYTES, length)?;
    for term in terms {
        output.write_all(term.text())?;
    }
    Ok(())
}

fn write_literal_block<'a>(
    output: &mut Buffer,
    terms: &[Definition<'a>],
    base: u64,
    languages: &mut Languages<'a>,
) -> io::Result<()> {
    array(output, 7)?;
    uint(output, 2)?;
    write_text_columns(output, terms)?;
    let mut datatypes = Vec::new();
    let mut indexes = Vec::new();
    array(output, terms.len() as u64)?;
    for term in terms {
        let code = match term {
            Definition::Term(Term::Typed { datatype, .. }) => {
                datatypes.try_reserve(1).map_err(io::Error::other)?;
                datatypes.push(wire_id(base, *datatype));
                1
            }
            Definition::Term(Term::Language {
                language,
                direction,
                ..
            }) => {
                indexes.try_reserve(1).map_err(io::Error::other)?;
                indexes.push(languages.intern(language)?);
                match direction {
                    None => 2,
                    Some(LiteralDirection::LeftToRight) => 3,
                    Some(LiteralDirection::RightToLeft) => 4,
                }
            }
            _ => unreachable!("only inline literals have form columns"),
        };
        uint(output, code)?;
    }
    list(output, &datatypes, |output, &id| uint(output, id))?;
    list(output, &indexes, |output, &index| uint(output, index))
}

fn write_value(output: &mut Buffer, value: &LiteralValue) -> io::Result<()> {
    array(
        output,
        if matches!(value, LiteralValue::Time(..)) {
            3
        } else {
            2
        },
    )?;
    match value {
        LiteralValue::Boolean(value) => {
            uint(output, 1)?;
            output.write_all(&[0xf4 + u8::from(*value)])
        }
        LiteralValue::Unsigned(value) => {
            uint(output, 2)?;
            uint(output, *value)
        }
        LiteralValue::Integer(value) => {
            uint(output, 3)?;
            int(output, *value)
        }
        LiteralValue::Time(seconds, nanoseconds) => {
            uint(output, 4)?;
            int(output, i128::from(*seconds))?;
            uint(output, u64::from(*nanoseconds))
        }
        LiteralValue::Octets(bytes) => {
            uint(output, 5)?;
            write_bytes(output, bytes)
        }
    }
}

const fn line_kind(line: Line) -> FrameType {
    match line {
        Line::Quad(..) => FrameType::Quads,
        Line::ReifierBinding(..) => FrameType::ReifierBindings,
        Line::Annotation(..) => FrameType::Annotations,
        Line::EmptyGraph(_) => FrameType::EmptyGraphs,
    }
}

const fn graph(line: Line) -> Option<TermId> {
    match line {
        Line::Quad(_, graph) | Line::ReifierBinding(_, graph) | Line::Annotation(_, _, graph) => {
            graph
        }
        Line::EmptyGraph(id) => Some(id),
    }
}

fn row_values(line: Line, base: u64) -> [u64; 4] {
    let id = |id| wire_id(base, id);
    match line {
        Line::Quad([s, p, o], _) => [id(p), id(o), id(s), 0],
        Line::ReifierBinding([reifier, triple], _) => [id(reifier), id(triple), 0, 0],
        Line::Annotation([s, p, o], count, _) => [id(s), id(p), id(o), count.get()],
        Line::EmptyGraph(graph) => [id(graph), 0, 0, 0],
    }
}

fn write_runs<T: Copy + Eq>(
    output: &mut Buffer,
    lines: &[Line],
    key: impl Fn(Line) -> T,
    write: impl Fn(&mut Buffer, T) -> io::Result<()>,
) -> io::Result<()> {
    let runs = || lines.chunk_by(|&a, &b| key(a) == key(b));
    array(output, runs().count() as u64)?;
    for run in runs() {
        array(output, 2)?;
        write(output, key(run[0]))?;
        uint(output, run.len() as u64)?;
    }
    Ok(())
}

fn wire_id(base: u64, id: TermId) -> u64 {
    base + id.index() as u64
}

fn list<T>(
    output: &mut Buffer,
    values: &[T],
    write: impl Fn(&mut Buffer, &T) -> io::Result<()>,
) -> io::Result<()> {
    array(output, values.len() as u64)?;
    for value in values {
        write(output, value)?;
    }
    Ok(())
}

fn array(output: &mut impl Write, length: u64) -> io::Result<()> {
    head::write_head(output, head::ARRAY, length)
}

fn uint(output: &mut impl Write, value: u64) -> io::Result<()> {
    head::write_head(output, head::UNSIGNED, value)
}

fn int(output: &mut impl Write, value: i128) -> io::Result<()> {
    let (format, magnitude) = if value >= 0 {
        (head::UNSIGNED, value)
    } else {
        (head::NEGATIVE, !value)
    };
    let argument = u64::try_from(magnitude).map_err(io::Error::other)?;
    head::write_head(output, format, argument)
}
