// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lazy canonical frame alternatives, with no scenario-to-actual coercion.

use super::{LedgerError, LedgerNodeKey, LedgerNodes, LedgerStatement, dictionary_node, predicate};
use crate::{TermValue, evidence::Frame};

/// Explicit caller predicates and values for the frame's logical structure.
#[derive(Clone, Copy, Debug)]
pub struct FrameVocabulary<'a> {
    /// Frame mode: actual, conjunction or disjunction.
    pub mode: &'a TermValue,
    /// Link to one conjunctive alternative of a disjunction.
    pub alternative: &'a TermValue,
    /// Membership predicates in FrameComponent's canonical kind order:
    /// standpoint, counterfactual, imagined, assumed, licensed falsehood.
    pub components: [&'a TermValue; 5],
    /// Explicit unrestricted actual mode.
    pub actual: &'a TermValue,
    /// Explicit conjunctive mode.
    pub conjunction: &'a TermValue,
    /// Explicit alternative-witness mode.
    pub disjunction: &'a TermValue,
}

/// Borrowed original canonical frame and immutable dictionary node carriers.
#[derive(Debug)]
pub struct FrameStatements<'a, N> {
    frame: &'a Frame,
    nodes: &'a N,
    vocabulary: FrameVocabulary<'a>,
    predicates: [&'a str; 7],
    phase: u8,
    branch: usize,
    atom: usize,
    selected: Option<usize>,
}

// Each cursor borrows its immutable original frame and dictionary.
impl<N> Clone for FrameStatements<'_, N> {
    fn clone(&self) -> Self {
        Self {
            frame: self.frame,
            nodes: self.nodes,
            vocabulary: self.vocabulary,
            predicates: self.predicates,
            phase: self.phase,
            branch: self.branch,
            atom: self.atom,
            selected: self.selected,
        }
    }
}

impl<'a, N: LedgerNodes> FrameStatements<'a, N> {
    /// Validate every required original node before any frame statement escapes.
    ///
    /// # Errors
    /// Refuses malformed vocabulary or missing original node carriers.
    pub fn new(
        frame: &'a Frame,
        nodes: &'a N,
        vocabulary: FrameVocabulary<'a>,
    ) -> Result<Self, LedgerError> {
        let mut predicates = [""; 7];
        for (slot, term) in predicates.iter_mut().zip(
            [vocabulary.mode, vocabulary.alternative]
                .into_iter()
                .chain(vocabulary.components),
        ) {
            *slot = predicate(term)?;
        }
        for value in [
            vocabulary.actual,
            vocabulary.conjunction,
            vocabulary.disjunction,
        ] {
            predicate(value)?;
        }
        let result = Self {
            frame,
            nodes,
            vocabulary,
            predicates,
            phase: 0,
            branch: 0,
            atom: 0,
            selected: None,
        };
        dictionary_node(result.nodes, LedgerNodeKey::Frame(frame.identity()))?;
        for (index, branch) in frame.alternatives().iter().enumerate() {
            dictionary_node(
                result.nodes,
                LedgerNodeKey::Frame(frame.alternative_identity(index).ok_or_else(|| {
                    LedgerError::MissingNode(LedgerNodeKey::Frame(frame.identity()))
                })?),
            )?;
            for atom in branch {
                dictionary_node(result.nodes, LedgerNodeKey::Reference(atom.preimage().1))?;
            }
        }
        Ok(result)
    }

    /// Render one original conjunction once from the interned frame dictionary.
    ///
    /// A disjunction renders its links only; its reused branch nodes are rendered
    /// through this constructor once per unique content key, never once per row.
    ///
    /// # Errors
    /// Refuses an absent alternative or the same declaration failures as `new`.
    pub fn alternative(
        frame: &'a Frame,
        index: usize,
        nodes: &'a N,
        vocabulary: FrameVocabulary<'a>,
    ) -> Result<Self, LedgerError> {
        Self::new(frame, nodes, vocabulary)?.branch(index)
    }

    pub(super) fn branch(&self, index: usize) -> Result<Self, LedgerError> {
        self.frame
            .alternative_identity(index)
            .ok_or(LedgerError::InvalidFrameAlternative)?;
        let mut result = self.clone();
        result.selected = Some(index);
        result.branch = index;
        Ok(result)
    }

    fn pull(&mut self) -> Result<Option<LedgerStatement<'a>>, LedgerError> {
        let disjunction = self.selected.is_none() && self.frame.alternatives().len() > 1;
        let identity = self.selected.map_or_else(
            || Ok(self.frame.identity()),
            |index| {
                self.frame
                    .alternative_identity(index)
                    .ok_or(LedgerError::InvalidFrameAlternative)
            },
        )?;
        let (subject, predicate, object) = loop {
            match self.phase {
                0 => {
                    self.phase = if self.frame.is_unrestricted_actual() {
                        4
                    } else if disjunction {
                        1
                    } else {
                        3
                    };
                    break (
                        dictionary_node(self.nodes, LedgerNodeKey::Frame(identity))?,
                        self.predicates[0],
                        if self.frame.is_unrestricted_actual() {
                            self.vocabulary.actual
                        } else if disjunction {
                            self.vocabulary.disjunction
                        } else {
                            self.vocabulary.conjunction
                        },
                    );
                }
                1 => {
                    let Some(id) = self.frame.alternative_identity(self.branch) else {
                        self.phase = 4;
                        continue;
                    };
                    let branch_node = dictionary_node(self.nodes, LedgerNodeKey::Frame(id))?;
                    self.branch += 1;
                    break (
                        dictionary_node(self.nodes, LedgerNodeKey::Frame(identity))?,
                        self.predicates[1],
                        branch_node,
                    );
                }
                3 => {
                    let Some(branch) = self.frame.alternatives().get(self.branch) else {
                        self.phase = 4;
                        continue;
                    };
                    let Some(atom) = branch.get(self.atom) else {
                        self.branch += 1;
                        self.atom = 0;
                        self.phase = if disjunction { 1 } else { 4 };
                        continue;
                    };
                    let (kind, id) = atom.preimage();
                    let branch_id =
                        self.frame
                            .alternative_identity(self.branch)
                            .ok_or_else(|| {
                                LedgerError::MissingNode(LedgerNodeKey::Frame(
                                    self.frame.identity(),
                                ))
                            })?;
                    self.atom += 1;
                    break (
                        dictionary_node(self.nodes, LedgerNodeKey::Frame(branch_id))?,
                        self.predicates[usize::from(kind) + 2],
                        dictionary_node(self.nodes, LedgerNodeKey::Reference(id))?,
                    );
                }
                _ => return Ok(None),
            }
        };
        Ok(Some(LedgerStatement {
            subject,
            predicate,
            object,
        }))
    }
}

impl<'a, N: LedgerNodes> Iterator for FrameStatements<'a, N> {
    type Item = Result<LedgerStatement<'a>, LedgerError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.pull() {
            Ok(statement) => statement.map(Ok),
            Err(error) => {
                self.phase = 4;
                Some(Err(error))
            }
        }
    }
}

impl<N: LedgerNodes> core::iter::FusedIterator for FrameStatements<'_, N> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{FrameComponent, ledger::row::tests::Nodes};
    use purrdf_hash::hex::Digest32;
    use purrdf_lex::allocation::{Memory, Resident};

    #[test]
    fn original_frame_alternatives_render_without_promoting_scenarios_to_actual() {
        let terms: [TermValue; 10] = core::array::from_fn(|index| {
            TermValue::iri(format!("https://example.org/frame-v/{index}"))
        });
        let vocabulary = FrameVocabulary {
            mode: &terms[0],
            alternative: &terms[1],
            components: [&terms[2], &terms[3], &terms[4], &terms[5], &terms[6]],
            actual: &terms[7],
            conjunction: &terms[8],
            disjunction: &terms[9],
        };
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let first = Frame::from_components(
            &[
                FrameComponent::Standpoint(Digest32::new([1; 32])),
                FrameComponent::Counterfactual(Digest32::new([2; 32])),
            ],
            &mut memory,
        )
        .unwrap();
        let second = Frame::from_components(
            &[FrameComponent::Imagined(Digest32::new([3; 32]))],
            &mut memory,
        )
        .unwrap();
        let frame = first.union(&second, &mut memory).unwrap();
        let mut nodes = Nodes::default();
        nodes.add(LedgerNodeKey::Frame(frame.identity()));
        for (index, branch) in frame.alternatives().iter().enumerate() {
            nodes.add(LedgerNodeKey::Frame(
                frame.alternative_identity(index).unwrap(),
            ));
            for atom in branch {
                nodes.add(LedgerNodeKey::Reference(atom.preimage().1));
            }
        }
        let mut statements = FrameStatements::new(&frame, &nodes, vocabulary)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            statements.len(),
            3,
            "the disjunction emits only its own facts"
        );
        for index in 0..frame.alternatives().len() {
            statements.extend(
                FrameStatements::alternative(&frame, index, &nodes, vocabulary)
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
            );
        }
        assert_eq!(statements.len(), 8);
        assert_eq!(statements[0].object, vocabulary.disjunction);
        assert_eq!(
            statements
                .iter()
                .filter(
                    |statement| statement.predicate == predicate(vocabulary.alternative).unwrap()
                )
                .count(),
            2
        );
        assert!(
            statements
                .iter()
                .all(|statement| statement.object != vocabulary.actual)
        );
        assert_eq!(first.identity(), frame.alternative_identity(0).unwrap());
        assert_eq!(second.identity(), frame.alternative_identity(1).unwrap());
        assert!(Frame::actual().alternative_identity(0).is_none());
        let mut actual_nodes = Nodes::default();
        let actual = Frame::actual();
        actual_nodes.add(LedgerNodeKey::Frame(actual.identity()));
        let actual_statements = FrameStatements::new(&actual, &actual_nodes, vocabulary)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(actual_statements.len(), 1);
        assert_eq!(actual_statements[0].object, vocabulary.actual);
    }
}
