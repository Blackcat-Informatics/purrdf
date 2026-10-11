// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original answer rows and canonical dictionaries rendered without a graph copy.

use super::{
    AnnotationStatements, FrameStatements, FrameVocabulary, LedgerError, LedgerNodeKey,
    LedgerNodes, LedgerSource, LedgerStatement, ReasonStatements, ReasonVocabulary, RowStatements,
    RowVocabulary, dictionary_node,
};
use crate::{
    TermValue,
    evidence::{EvidenceRecord, ReasonId, ReasonTable, ReasonTableError},
};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// One original row's retained reifier, solution and evidence, including explicit
/// accepted reasons. Rendering preserves the caller's answer/merge order.
#[derive(Clone, Copy, Debug)]
pub struct LedgerRow<'a> {
    /// Original caller-minted evidence reifier.
    pub node: &'a TermValue,
    /// Original genuine solution triple term.
    pub solution: &'a TermValue,
    /// Original immutable evidence dictionary entry.
    pub record: &'a EvidenceRecord,
    /// Sorted unique original reasons explicitly accepted for this row.
    pub accepted: &'a [ReasonId],
}

/// Every rendering term is deliberate caller-selected vocabulary.
#[derive(Clone, Copy, Debug)]
pub struct LedgerVocabulary<'a> {
    /// Row and pointwise information/bound annotations.
    pub row: RowVocabulary<'a>,
    /// Canonical concern and separate cause/constraint/vault/watermark.
    pub reason: ReasonVocabulary<'a>,
    /// Canonical actual/scenario conjunctions and alternative witnesses.
    pub frame: FrameVocabulary<'a>,
}

/// A hard construction refusal; no partial ledger source is published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerBuildError {
    /// Invalid original RDF declaration or missing dictionary node.
    Declaration(LedgerError),
    /// The original retained reason dictionary is invalid or lacks a reason.
    Reasons(ReasonTableError),
    /// Two original row owners supplied the same evidence reifier.
    DuplicateRow,
    /// Distinct row/content owners alias one node, corrupting their identities.
    NodeCollision,
    /// Original native physical allocation/admission refusal.
    Storage(StorageError),
}

purrdf_lex::variant_from!(LedgerBuildError {
    Declaration(LedgerError),
    Storage(StorageError),
    Reasons(ReasonTableError),
});
impl core::fmt::Display for LedgerBuildError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Declaration(error) => error.fmt(out),
            Self::Reasons(error) => error.fmt(out),
            Self::DuplicateRow => out.write_str("ledger row reifiers must be unique"),
            Self::NodeCollision => out.write_str("distinct ledger content owners alias one node"),
            Self::Storage(error) => error.fmt(out),
        }
    }
}
impl std::error::Error for LedgerBuildError {}

/// Keeps only borrowed rendering cursors and their native array grants. Original
/// row/reason/frame owners remain authoritative; each dictionary description is
/// emitted once in canonical key order. No resident RDF graph is constructed.
#[derive(Debug)]
pub struct AnswerLedger<'a, N: LedgerNodes, S: Admission> {
    rows: Vec<RowStatements<'a, N>>,
    dictionary: Vec<(LedgerNodeKey, DictionaryStatements<'a, N>)>,
    live: usize,
    _admission: S,
}

impl<'a, N: LedgerNodes, S: Admission> AnswerLedger<'a, N, S> {
    /// Borrow original answer rows and their dictionaries. Validate the complete
    /// source before publication, including acceptance and node identity.
    ///
    /// # Errors
    /// Returns invalid declarations, identity collisions, missing reasons or
    /// native physical refusal after destroying construction scratch.
    pub fn new<A: Admission, T: core::ops::Deref<Target = TermValue>>(
        rows: impl IntoIterator<Item = LedgerRow<'a>>,
        reasons: &'a ReasonTable<A, T>,
        nodes: &'a N,
        vocabulary: LedgerVocabulary<'a>,
        mut admission: S,
    ) -> Result<Self, LedgerBuildError> {
        reasons.entries()?;
        let mut memory = Memory::new(&mut admission);
        let (rows, dictionary) = memory.try_scope(|memory| {
            let mut rendered = Vec::new();
            let mut dictionary = Vec::new();
            let mut owners = Vec::new();
            let result: Result<(), LedgerBuildError> =
                (|| {
                    for row in rows {
                        let cursor = RowStatements::new(
                            row.node,
                            row.solution,
                            row.record,
                            nodes,
                            vocabulary.row,
                            row.accepted,
                        )?;
                        register_node(row.node, None, &mut owners, memory)?;
                        memory.push(&mut rendered, cursor)?;
                        for reason in row.record.reasons() {
                            ensure(
                                LedgerNodeKey::Reason(*reason),
                                &mut dictionary,
                                &mut owners,
                                nodes,
                                memory,
                                |node| {
                                    Ok(DictionaryStatements::Reason(ReasonStatements::retained(
                                        node,
                                        reasons.get(*reason)?,
                                        vocabulary.reason,
                                    )?))
                                },
                            )?;
                        }
                        let frame = row.record.frame();
                        let new_frame = ensure(
                            LedgerNodeKey::Frame(frame.identity()),
                            &mut dictionary,
                            &mut owners,
                            nodes,
                            memory,
                            |_| {
                                Ok(DictionaryStatements::Frame(FrameStatements::new(
                                    frame,
                                    nodes,
                                    vocabulary.frame,
                                )?))
                            },
                        )?;
                        if new_frame {
                            let position = dictionary
                                .binary_search_by_key(
                                    &LedgerNodeKey::Frame(frame.identity()),
                                    |entry| entry.0,
                                )
                                .expect("the original frame cursor was just retained");
                            let DictionaryStatements::Frame(cursor) = &dictionary[position].1
                            else {
                                unreachable!("a frame key retains a frame cursor");
                            };
                            let cursor = cursor.clone();
                            for index in 0..frame.alternatives().len() {
                                let id = frame
                                    .alternative_identity(index)
                                    .ok_or(LedgerError::InvalidFrameAlternative)?;
                                ensure(
                                    LedgerNodeKey::Frame(id),
                                    &mut dictionary,
                                    &mut owners,
                                    nodes,
                                    memory,
                                    |_| Ok(DictionaryStatements::Frame(cursor.branch(index)?)),
                                )?;
                            }
                        }
                        let keys =
                            row.record
                                .information()
                                .entries()
                                .iter()
                                .map(|&(context, state)| LedgerNodeKey::Information(context, state))
                                .chain(row.record.aggregate_bounds().iter().map(
                                    |&(column, bound)| LedgerNodeKey::Aggregate(column, bound),
                                ));
                        for key in keys {
                            ensure(key, &mut dictionary, &mut owners, nodes, memory, |_| {
                                Ok(DictionaryStatements::Annotation(AnnotationStatements::new(
                                    key,
                                    nodes,
                                    vocabulary.row,
                                )?))
                            })?;
                        }
                    }
                    Ok(())
                })();
            let release = memory.release_vec(owners);
            result?;
            release?;
            Ok::<_, LedgerBuildError>((rendered, dictionary))
        })?;
        let live = memory.admitted_bytes();
        Ok(Self {
            rows,
            dictionary,
            live,
            _admission: admission,
        })
    }

    /// Original retained cursor layouts, excluding borrowed term payloads.
    #[must_use]
    pub const fn admitted_bytes(&self) -> usize {
        self.live
    }

    /// Number of unique reason/frame/information/bound descriptions.
    #[must_use]
    pub fn dictionary_len(&self) -> usize {
        self.dictionary.len()
    }
}

impl<'a, N: LedgerNodes, S: Admission + core::fmt::Debug> LedgerSource<'a>
    for AnswerLedger<'a, N, S>
{
    fn statements(&self) -> impl Iterator<Item = Result<LedgerStatement<'a>, LedgerError>> {
        self.rows.iter().flat_map(Clone::clone).chain(
            self.dictionary
                .iter()
                .flat_map(|(_, cursor)| cursor.clone()),
        )
    }
}

#[derive(Debug)]
enum DictionaryStatements<'a, N> {
    Reason(ReasonStatements<'a>),
    Frame(FrameStatements<'a, N>),
    Annotation(AnnotationStatements<'a>),
}

impl<N> Clone for DictionaryStatements<'_, N> {
    fn clone(&self) -> Self {
        match self {
            Self::Reason(cursor) => Self::Reason(cursor.clone()),
            Self::Frame(cursor) => Self::Frame(cursor.clone()),
            Self::Annotation(cursor) => Self::Annotation(cursor.clone()),
        }
    }
}

impl<'a, N: LedgerNodes> Iterator for DictionaryStatements<'a, N> {
    type Item = Result<LedgerStatement<'a>, LedgerError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Reason(cursor) => cursor.next().map(Ok),
            Self::Frame(cursor) => cursor.next(),
            Self::Annotation(cursor) => cursor.next().map(Ok),
        }
    }
}

type NodeOwner<'a> = (&'a TermValue, Option<LedgerNodeKey>);

fn register_node<'a, S: Admission + ?Sized>(
    node: &'a TermValue,
    key: Option<LedgerNodeKey>,
    owners: &mut Vec<NodeOwner<'a>>,
    memory: &mut Memory<'_, S>,
) -> Result<(), LedgerBuildError> {
    // Validated IRI/blank nodes never require nested comparison scratch.
    let position = match owners.binary_search_by(|(old, _)| old.cmp(&node)) {
        Ok(position) => {
            return Err(if owners[position].1.is_none() && key.is_none() {
                LedgerBuildError::DuplicateRow
            } else {
                LedgerBuildError::NodeCollision
            });
        }
        Err(position) => position,
    };
    memory.push(owners, (node, key))?;
    owners[position..].rotate_right(1);
    Ok(())
}

fn ensure<'a, N: LedgerNodes, S: Admission + ?Sized>(
    key: LedgerNodeKey,
    dictionary: &mut Vec<(LedgerNodeKey, DictionaryStatements<'a, N>)>,
    owners: &mut Vec<NodeOwner<'a>>,
    nodes: &'a N,
    memory: &mut Memory<'_, S>,
    make: impl FnOnce(&'a TermValue) -> Result<DictionaryStatements<'a, N>, LedgerBuildError>,
) -> Result<bool, LedgerBuildError> {
    let Err(position) = dictionary.binary_search_by_key(&key, |entry| entry.0) else {
        return Ok(false);
    };
    let node = dictionary_node(nodes, key)?;
    register_node(node, Some(key), owners, memory)?;
    let cursor = make(node)?;
    memory.push(dictionary, (key, cursor))?;
    dictionary[position..].rotate_right(1);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::row::tests::{Nodes, solution, vocabulary};
    use super::*;
    use crate::evidence::{
        Frame, FrameComponent, InformationMap, RetainedReason, SearchCompleteness,
    };
    use purrdf_hash::hex::Digest32;
    use purrdf_lex::allocation::Resident;

    fn declarations<'a>(
        row: &'a [TermValue; 22],
        extra: &'a [TermValue; 14],
    ) -> LedgerVocabulary<'a> {
        LedgerVocabulary {
            row: vocabulary(row),
            reason: ReasonVocabulary {
                cause: &extra[0],
                constraint: &extra[1],
                vault: &extra[2],
                watermark: &extra[3],
            },
            frame: FrameVocabulary {
                mode: &extra[4],
                alternative: &extra[5],
                components: [&extra[6], &extra[7], &extra[8], &extra[9], &extra[10]],
                actual: &extra[11],
                conjunction: &extra[12],
                disjunction: &extra[13],
            },
        }
    }

    #[test]
    fn shared_scenario_branches_are_described_once_without_actualizing_rows() {
        let solution = solution();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let first = Frame::from_components(
            &[FrameComponent::Counterfactual(Digest32::new([1; 32]))],
            &mut memory,
        )
        .unwrap();
        let second = Frame::from_components(
            &[FrameComponent::Imagined(Digest32::new([2; 32]))],
            &mut memory,
        )
        .unwrap();
        let frame = first.union(&second, &mut memory).unwrap();
        let record = EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &frame,
            &[],
            &[],
            &mut memory,
        )
        .unwrap();
        let mut nodes = Nodes::default();
        nodes.add(LedgerNodeKey::Frame(record.frame().identity()));
        for (index, branch) in record.frame().alternatives().iter().enumerate() {
            nodes.add(LedgerNodeKey::Frame(
                record.frame().alternative_identity(index).unwrap(),
            ));
            for atom in branch {
                nodes.add(LedgerNodeKey::Reference(atom.preimage().1));
            }
        }
        let row_terms = core::array::from_fn(|index| {
            TermValue::iri(format!("https://example.org/row-v/{index}"))
        });
        let extra = core::array::from_fn(|index| {
            TermValue::iri(format!("https://example.org/extra-v/{index}"))
        });
        let vocabulary = declarations(&row_terms, &extra);
        let row_nodes = [
            TermValue::iri("https://example.org/row/a"),
            TermValue::iri("https://example.org/row/b"),
        ];
        let reasons: ReasonTable<Resident, &TermValue> = ReasonTable::with_admission(Resident);
        let source = AnswerLedger::new(
            row_nodes.iter().map(|node| LedgerRow {
                node,
                solution: &solution,
                record: &record,
                accepted: &[],
            }),
            &reasons,
            &nodes,
            vocabulary,
            Resident,
        )
        .unwrap();
        assert_eq!(source.dictionary_len(), 3);
        let statements = source.statements().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(statements.len(), 4 * 2 + 7);
        assert_eq!(
            statements
                .iter()
                .filter(|statement| statement.predicate
                    == super::super::predicate(vocabulary.frame.alternative).unwrap())
                .count(),
            2
        );
        assert!(
            statements
                .iter()
                .all(|statement| statement.object != vocabulary.frame.actual)
        );
    }

    #[test]
    fn actual_source_renders_shared_dictionaries_once_and_preserves_acceptance_order() {
        let solution = solution();
        let cause = TermValue::iri("https://example.org/missing");
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let reason =
            RetainedReason::from_parts(&cause, &solution, None, None, None, &mut memory).unwrap();
        let mut reasons = ReasonTable::with_admission(Resident);
        let reason_id = reasons.intern(reason).unwrap();
        let record = EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::actual(),
            &[reason_id],
            &[],
            &mut memory,
        )
        .unwrap();
        let mut nodes = Nodes::default();
        nodes.add(LedgerNodeKey::Reason(reason_id));
        nodes.add(LedgerNodeKey::Frame(record.frame().identity()));
        let row_terms = core::array::from_fn(|index| {
            TermValue::iri(format!("https://example.org/row-v/{index}"))
        });
        let extra = core::array::from_fn(|index| {
            TermValue::iri(format!("https://example.org/extra-v/{index}"))
        });
        let vocab = declarations(&row_terms, &extra);
        let row_nodes: Vec<_> = (0..2279)
            .map(|index| TermValue::iri(format!("https://example.org/row/{index}")))
            .collect();
        let accepted = [reason_id];
        let source = AnswerLedger::new(
            row_nodes.iter().enumerate().map(|(index, node)| LedgerRow {
                node,
                solution: &solution,
                record: &record,
                accepted: if index == 1 { &accepted } else { &[] },
            }),
            &reasons,
            &nodes,
            vocab,
            Resident,
        )
        .unwrap();
        assert_eq!(source.dictionary_len(), 2);
        assert!(source.admitted_bytes() > 0);
        let first: Vec<_> = source.statements().collect::<Result<_, _>>().unwrap();
        let second: Vec<_> = source.statements().collect::<Result<_, _>>().unwrap();
        assert_eq!(first.len(), 5 * 2279 + 4);
        assert_eq!(
            first
                .iter()
                .filter(|statement| statement.predicate == purrdf_iri::vocab::rdf::REIFIES)
                .count(),
            2280
        );
        assert_eq!(
            first
                .iter()
                .filter(
                    |statement| statement.predicate == super::super::predicate(&extra[0]).unwrap()
                )
                .count(),
            1
        );
        assert_eq!(
            first
                .iter()
                .filter(|statement| statement.predicate
                    == super::super::predicate(&row_terms[6]).unwrap())
                .count(),
            1
        );
        for (a, b) in first.iter().zip(&second) {
            assert!(core::ptr::eq(a.subject, b.subject));
            assert!(core::ptr::eq(a.object, b.object));
            assert_eq!(a.predicate, b.predicate);
        }
        assert!(core::ptr::eq(first[0].subject, &raw const row_nodes[0]));
        assert!(core::ptr::eq(first[0].object, &raw const solution));
        let duplicate = LedgerRow {
            node: &row_nodes[0],
            solution: &solution,
            record: &record,
            accepted: &[],
        };
        assert!(matches!(
            AnswerLedger::new([duplicate, duplicate], &reasons, &nodes, vocab, Resident),
            Err(LedgerBuildError::DuplicateRow)
        ));
    }
}
