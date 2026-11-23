// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;

use super::dictionary::Position::{self, Any, Iri, Subject, Triple};
use super::terms::TermArena;
use super::{FoldError, LiteralValue, Term, TermId, ValueName};
use crate::{FastMap, FastSet};

/// A resolved line of the file fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Line {
    /// Subject, predicate and object, with an optional named graph.
    Quad([TermId; 3], Option<TermId>),
    /// Reifier and triple term, with an optional named graph.
    ReifierBinding([TermId; 2], Option<TermId>),
    /// Reifier, predicate and object, multiplicity and optional graph.
    Annotation([TermId; 3], NonZeroU64, Option<TermId>),
    /// An explicitly declared empty named graph.
    EmptyGraph(TermId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LineKey {
    Quad([TermId; 3], Option<TermId>),
    ReifierBinding([TermId; 2], Option<TermId>),
    Annotation([TermId; 3], Option<TermId>),
    EmptyGraph(TermId),
}

impl Line {
    const fn key(self) -> LineKey {
        match self {
            Self::Quad(terms, graph) => LineKey::Quad(terms, graph),
            Self::ReifierBinding(terms, graph) => LineKey::ReifierBinding(terms, graph),
            Self::Annotation(terms, _, graph) => LineKey::Annotation(terms, graph),
            Self::EmptyGraph(graph) => LineKey::EmptyGraph(graph),
        }
    }

    const fn carried_graph(self) -> Option<TermId> {
        match self {
            Self::Quad(_, graph)
            | Self::ReifierBinding(_, graph)
            | Self::Annotation(_, _, graph) => graph,
            Self::EmptyGraph(_) => None,
        }
    }
}

/// File-wide term identities and resolved line union.
#[derive(Debug, Default)]
pub struct RdfFold {
    pub(super) terms: TermArena,
    pub(super) missing_values: FastSet<ValueName>,
    line_indices: FastMap<LineKey, usize>,
    lines: Vec<Line>,
    carried_graphs: FastSet<TermId>,
    held: bool,
    width: bool,
}

impl RdfFold {
    pub(in crate::v2) fn terms(&self) -> impl ExactSizeIterator<Item = &Term<'static>> {
        self.terms.terms()
    }

    /// Intern a term whose dependencies already exist in this fold.
    ///
    /// # Errors
    /// Returns a position or resource failure or a missing term handle.
    pub fn intern_term(&mut self, term: Term<'_>) -> Result<TermId, FoldError> {
        match term {
            Term::Typed { datatype, .. } => self.check_position(datatype, Iri)?,
            Term::Triple([subject, predicate, object]) => {
                self.check_triple(subject, predicate, object)?;
            }
            _ => {}
        }
        self.terms.intern(term)
    }

    /// Merge one line into the fold.
    ///
    /// # Errors
    /// Returns a position or resource failure or a missing term handle.
    pub fn add_line(&mut self, line: Line) -> Result<(), FoldError> {
        match line {
            Line::Quad([subject, predicate, object], _)
            | Line::Annotation([subject, predicate, object], _, _) => {
                self.check_triple(subject, predicate, object)?;
            }
            Line::ReifierBinding([reifier, triple], _) => {
                self.check_position(reifier, Subject)?;
                self.check_position(triple, Triple)?;
            }
            Line::EmptyGraph(graph) => self.check_position(graph, Subject)?,
        }
        if let Some(graph) = line.carried_graph() {
            self.check_position(graph, Subject)?;
        }
        self.merge_lines(&[line], false)
    }

    fn check_triple(
        &self,
        subject: TermId,
        predicate: TermId,
        object: TermId,
    ) -> Result<(), FoldError> {
        self.check_position(subject, Subject)?;
        self.check_position(predicate, Iri)?;
        self.check_position(object, Any)
    }

    fn check_position(&self, id: TermId, position: Position) -> Result<(), FoldError> {
        let term = self.term(id).ok_or(FoldError::TermHandle(id))?;
        let kind = match term {
            Term::Iri(_) => 1,
            Term::Typed { .. } | Term::Language { .. } => 2,
            Term::Blank { .. } => 3,
            Term::Triple(_) => 4,
        };
        if position.admits(kind) {
            Ok(())
        } else {
            Err(FoldError::PositionConstraint(id))
        }
    }

    /// Look up a term handle from this fold.
    #[must_use]
    pub fn term(&self, id: TermId) -> Option<&Term<'static>> {
        self.terms.get(id)
    }

    /// The typed-value pairs in order of first definition.
    #[must_use]
    pub fn values(&self) -> &[(TermId, LiteralValue)] {
        self.terms.values()
    }

    /// Lines in order of first occurrence, with their current multiplicities.
    ///
    /// A graph with a carrier is omitted from the empty-graph declarations.
    pub fn lines(&self) -> impl Iterator<Item = Line> + '_ {
        self.lines.iter().copied().filter(
            |line| !matches!(line, Line::EmptyGraph(graph) if self.carried_graphs.contains(graph)),
        )
    }

    /// Whether any row was held because its term was unreadable.
    #[must_use]
    pub const fn has_held_rows(&self) -> bool {
        self.held
    }

    /// Whether an annotation count exceeded the wire width.
    #[must_use]
    pub const fn annotation_width_exceeded(&self) -> bool {
        self.width
    }

    pub(super) fn merge_lines(&mut self, lines: &[Line], held: bool) -> Result<(), FoldError> {
        self.line_indices.try_reserve(lines.len())?;
        self.lines.try_reserve(lines.len())?;
        self.carried_graphs.try_reserve(lines.len())?;
        for &line in lines {
            let key = line.key();
            if let Some(&index) = self.line_indices.get(&key) {
                if let (Line::Annotation(_, count, _), Line::Annotation(_, increment, _)) =
                    (&mut self.lines[index], line)
                {
                    let sum = count
                        .get()
                        .checked_add(increment.get())
                        .and_then(NonZeroU64::new);
                    self.width |= sum.is_none();
                    *count = sum.unwrap_or(NonZeroU64::MAX);
                }
            } else {
                self.line_indices.insert(key, self.lines.len());
                self.lines.push(line);
                if let Some(graph) = line.carried_graph() {
                    self.carried_graphs.insert(graph);
                }
            }
        }
        self.held |= held;
        Ok(())
    }
}
