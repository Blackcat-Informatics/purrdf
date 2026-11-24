// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_lex::cbor::head::{self, write_bytes, write_text};

use super::{Buffer, FrameType, SegmentWriteError, SegmentWriter};
use crate::FastMap;
use crate::v2::{
    Line, LiteralDirection, LiteralValue, RdfFold, Row, RowOrder, Term, TermId, TermOrder,
    ValueName,
};

const FRAME_ROWS: usize = 4096;

/// Write one committed RDF segment and flush the sink.
///
/// # Errors
/// Returns a fold, capacity, or sink failure. A sink can contain a partial
/// segment when writing fails.
pub fn write_rdf_segment<W: Write>(fold: &RdfFold, sink: W) -> Result<W, SegmentWriteError> {
    let mut writer = SegmentWriter::new(sink)?;
    writer.add_rdf(fold)?;
    writer.add_commit()?;
    let mut sink = writer.into_inner();
    sink.flush()?;
    Ok(sink)
}

/// Encode one committed RDF segment into an owned buffer.
///
/// # Errors
/// Returns a fold or capacity failure, including a refused byte reservation.
pub fn rdf_segment_bytes(fold: &RdfFold) -> Result<Vec<u8>, SegmentWriteError> {
    Ok(write_rdf_segment(fold, Buffer::default())?.0)
}

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
            if let Some(value) = values.get(&index) {
                return Definition::Value(value);
            }
            let id = |term| wire_id(base, term);
            match term {
                Term::Iri(text) => Definition::Iri(text),
                Term::Typed { lexical, datatype } => {
                    Definition::Literal(lexical, Form::Typed(id(*datatype)))
                }
                Term::Language {
                    lexical,
                    language,
                    direction,
                } => Definition::Literal(lexical, Form::language(language, *direction)),
                Term::Blank { .. } => Definition::Blank(BlankLabel::arrival(base + index as u64)),
                Term::Triple(terms) => Definition::Triple(terms.map(id)),
            }
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
            self.write_term_batch(&batch, base + written, TermOrder::Arrival, 1)?;
            written += batch.len() as u64;
            batch.clear();
        }
    }

    pub(super) fn write_term_batch(
        &mut self,
        terms: &[Definition<'_>],
        first: u64,
        order: TermOrder,
        restart: usize,
    ) -> Result<(), SegmentWriteError> {
        let blocks = || terms.chunk_by(|a, b| a.kind() == b.kind());
        let block_count = blocks().count() as u64;
        let mut payload = Buffer::default();
        array(&mut payload, 2)?;
        array(&mut payload, block_count)?;
        let mut languages = Languages::default();
        for run in blocks() {
            write_block(&mut payload, run, restart, &mut languages)?;
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
            uint(output, order as u64)?;
            uint(output, restart as u64)
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
            let mut rows = fold
                .lines()
                .filter(|row| line_kind(*row) == kind)
                .map(|line| wire_row(line, |id| wire_id(base, id)));
            let mut batch = Vec::new();
            loop {
                for row in rows.by_ref().take(FRAME_ROWS) {
                    batch.try_reserve(1).map_err(io::Error::other)?;
                    batch.push(row);
                }
                if batch.is_empty() {
                    break;
                }
                self.write_row_batch(kind, &batch, RowOrder::Unordered)?;
                batch.clear();
            }
        }
        Ok(())
    }

    pub(super) fn write_row_batch(
        &mut self,
        kind: FrameType,
        rows: &[Row],
        order: RowOrder,
    ) -> Result<(), SegmentWriteError> {
        let mut payload = Buffer::default();
        if kind != FrameType::EmptyGraphs {
            let columns = match kind {
                FrameType::Quads => 4,
                FrameType::ReifierBindings => 3,
                FrameType::Annotations => 5,
                _ => unreachable!("named row frame type"),
            };
            array(&mut payload, columns)?;
            write_runs(&mut payload, rows, graph, |output, graph| match graph {
                Some(graph) => {
                    array(output, 2)?;
                    uint(output, 2)?;
                    uint(output, graph)
                }
                None => {
                    array(output, 1)?;
                    uint(output, 1)
                }
            })?;
            if kind == FrameType::Quads {
                write_runs(&mut payload, rows, |row| row_values(row)[0], uint)?;
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
            array(&mut payload, rows.len() as u64)?;
            let mut previous = 0;
            for &row in rows {
                let value = row_values(row)[column];
                int(&mut payload, i128::from(value) - i128::from(previous))?;
                previous = value;
            }
        }
        self.append(kind, &payload.0, |output| {
            if kind == FrameType::EmptyGraphs {
                array(output, 1)?;
            } else {
                array(output, 2)?;
                uint(
                    output,
                    match order {
                        RowOrder::Canonical => 1,
                        RowOrder::Unordered => 2,
                    },
                )?;
            }
            uint(output, rows.len() as u64)
        })?;
        if let Some(index) = &mut self.index {
            let position = self.range.count().get() - 2;
            index
                .content
                .record_rows(rows.iter().copied(), position)
                .map_err(io::Error::other)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) enum Definition<'a> {
    Iri(&'a str),
    Literal(&'a str, Form<'a>),
    Blank(BlankLabel<'a>),
    Triple([u64; 3]),
    LiteralRef(ValueName, Form<'a>),
    Value(&'a LiteralValue),
    ValueRef(ValueName),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Form<'a> {
    Typed(u64),
    Language(&'a str),
    LeftToRight(&'a str),
    RightToLeft(&'a str),
}

impl<'a> Form<'a> {
    pub(super) const fn language(text: &'a str, direction: Option<LiteralDirection>) -> Self {
        match direction {
            None => Self::Language(text),
            Some(LiteralDirection::LeftToRight) => Self::LeftToRight(text),
            Some(LiteralDirection::RightToLeft) => Self::RightToLeft(text),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum BlankLabel<'a> {
    Inline { bytes: [u8; 9], len: usize },
    Stored(&'a [u8]),
}

impl BlankLabel<'_> {
    fn arrival(id: u64) -> Self {
        Self::from_bytes(&id.to_be_bytes())
    }

    pub(super) fn position(position: u64) -> Self {
        head::with_encoded_head(head::UNSIGNED, position, Self::from_bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut storage = [0; 9];
        storage[..bytes.len()].copy_from_slice(bytes);
        Self::Inline {
            bytes: storage,
            len: bytes.len(),
        }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        match self {
            Self::Inline { bytes, len } => &bytes[..*len],
            Self::Stored(bytes) => bytes,
        }
    }
}

impl<'a> Definition<'a> {
    const fn kind(&self) -> u64 {
        match self {
            Self::Iri(_) => 1,
            Self::Literal(..) => 2,
            Self::Blank(_) => 3,
            Self::Triple(_) => 4,
            Self::LiteralRef(..) => 5,
            Self::Value(_) => 6,
            Self::ValueRef(_) => 7,
        }
    }

    fn text(self) -> &'a [u8] {
        match self {
            Self::Iri(text) | Self::Literal(text, _) => text.as_bytes(),
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
    restart: usize,
    languages: &mut Languages<'a>,
) -> io::Result<()> {
    match terms[0] {
        Definition::Iri(_) => {
            array(output, 4)?;
            uint(output, 1)?;
            write_text_columns(output, terms, restart)?;
        }
        Definition::Literal(..) => {
            array(output, 7)?;
            uint(output, 2)?;
            write_text_columns(output, terms, restart)?;
            write_literal_forms(output, terms, languages)?;
        }
        Definition::Blank(_) => {
            array(output, 2)?;
            uint(output, 3)?;
            list(output, terms, |output, term| {
                let Definition::Blank(label) = term else {
                    unreachable!("one block holds one term kind");
                };
                write_bytes(output, label.bytes())
            })?;
        }
        Definition::Triple(_) => {
            array(output, 4)?;
            uint(output, 4)?;
            for column in 0..3 {
                list(output, terms, |output, term| {
                    let Definition::Triple(ids) = term else {
                        unreachable!("one block holds one term kind");
                    };
                    uint(output, ids[column])
                })?;
            }
        }
        Definition::Value(_) => {
            array(output, 2)?;
            uint(output, 6)?;
            list(output, terms, |output, term| {
                let Definition::Value(value) = term else {
                    unreachable!("one block holds one term kind");
                };
                write_value(output, value)
            })?;
        }
        Definition::ValueRef(_) => {
            array(output, 3)?;
            uint(output, 7)?;
            write_value_columns(output, terms)?;
        }
        Definition::LiteralRef(..) => {
            array(output, 6)?;
            uint(output, 5)?;
            write_value_columns(output, terms)?;
            write_literal_forms(output, terms, languages)?;
        }
    }
    Ok(())
}

fn write_text_columns(
    output: &mut Buffer,
    terms: &[Definition<'_>],
    restart: usize,
) -> io::Result<()> {
    let mut prefixes = Vec::new();
    if restart > 1 {
        prefixes
            .try_reserve_exact(terms.len())
            .map_err(io::Error::other)?;
        for group in terms.chunks(restart) {
            prefixes.push(0);
            prefixes.extend(
                group
                    .windows(2)
                    .map(|pair| purrdf_deflate::common_prefix_len(pair[0].text(), pair[1].text())),
            );
        }
    }
    let prefix = |index| prefixes.get(index).copied().unwrap_or(0);
    array(output, terms.len() as u64)?;
    for index in 0..terms.len() {
        uint(output, prefix(index) as u64)?;
    }
    array(output, terms.len() as u64)?;
    let mut length = 0_u64;
    for (index, term) in terms.iter().enumerate() {
        let suffix = (term.text().len() - prefix(index)) as u64;
        uint(output, suffix)?;
        length = length
            .checked_add(suffix)
            .ok_or_else(|| io::Error::other("GTS text column exceeds the wire length"))?;
    }
    head::write_head(output, head::BYTES, length)?;
    for (index, term) in terms.iter().enumerate() {
        output.write_all(&term.text()[prefix(index)..])?;
    }
    Ok(())
}

fn write_value_columns(output: &mut Buffer, terms: &[Definition<'_>]) -> io::Result<()> {
    let name = |term| match term {
        Definition::LiteralRef(name, _) | Definition::ValueRef(name) => name,
        _ => unreachable!("value reference columns contain value names"),
    };
    list(output, terms, |output, &term| {
        write_bytes(output, &name(term).digest)
    })?;
    list(output, terms, |output, &term| {
        uint(output, name(term).length)
    })
}

fn write_literal_forms<'a>(
    output: &mut Buffer,
    terms: &[Definition<'a>],
    languages: &mut Languages<'a>,
) -> io::Result<()> {
    let mut datatypes = Vec::new();
    let mut indexes = Vec::new();
    array(output, terms.len() as u64)?;
    for term in terms {
        let (Definition::Literal(_, form) | Definition::LiteralRef(_, form)) = *term else {
            unreachable!("only lexical literals have form columns");
        };
        let code = match form {
            Form::Typed(datatype) => {
                datatypes.try_reserve(1).map_err(io::Error::other)?;
                datatypes.push(datatype);
                1
            }
            Form::Language(language)
            | Form::LeftToRight(language)
            | Form::RightToLeft(language) => {
                indexes.try_reserve(1).map_err(io::Error::other)?;
                indexes.push(languages.intern(language)?);
                match form {
                    Form::Language(_) => 2,
                    Form::LeftToRight(_) => 3,
                    Form::RightToLeft(_) => 4,
                    Form::Typed(_) => unreachable!("the language is already selected"),
                }
            }
        };
        uint(output, code)?;
    }
    list(output, &datatypes, |output, &id| uint(output, id))?;
    list(output, &indexes, |output, &index| uint(output, index))
}

pub(super) fn write_value(output: &mut impl Write, value: &LiteralValue) -> io::Result<()> {
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

pub(super) const fn line_kind(line: Line) -> FrameType {
    match line {
        Line::Quad(..) => FrameType::Quads,
        Line::ReifierBinding(..) => FrameType::ReifierBindings,
        Line::Annotation(..) => FrameType::Annotations,
        Line::EmptyGraph(_) => FrameType::EmptyGraphs,
    }
}

const fn graph(row: Row) -> Option<u64> {
    match row {
        Row::Quad(_, graph) | Row::ReifierBinding(_, graph) | Row::Annotation(_, _, graph) => graph,
        Row::EmptyGraph(id) => Some(id),
    }
}

pub(super) fn wire_row(line: Line, id: impl Fn(TermId) -> u64) -> Row {
    match line {
        Line::Quad(terms, graph) => Row::Quad(terms.map(&id), graph.map(id)),
        Line::ReifierBinding(terms, graph) => Row::ReifierBinding(terms.map(&id), graph.map(id)),
        Line::Annotation(terms, count, graph) => {
            Row::Annotation(terms.map(&id), count, graph.map(id))
        }
        Line::EmptyGraph(graph) => Row::EmptyGraph(id(graph)),
    }
}

const fn row_values(row: Row) -> [u64; 4] {
    match row {
        Row::Quad([s, p, o], _) => [p, o, s, 0],
        Row::ReifierBinding([reifier, triple], _) => [reifier, triple, 0, 0],
        Row::Annotation([s, p, o], count, _) => [s, p, o, count.get()],
        Row::EmptyGraph(graph) => [graph, 0, 0, 0],
    }
}

fn write_runs<T: Copy + Eq>(
    output: &mut Buffer,
    rows: &[Row],
    key: impl Fn(Row) -> T,
    write: impl Fn(&mut Buffer, T) -> io::Result<()>,
) -> io::Result<()> {
    let runs = || rows.chunk_by(|&a, &b| key(a) == key(b));
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

pub(super) fn array(output: &mut impl Write, length: u64) -> io::Result<()> {
    head::write_head(output, head::ARRAY, length)
}

pub(super) fn uint(output: &mut impl Write, value: u64) -> io::Result<()> {
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
