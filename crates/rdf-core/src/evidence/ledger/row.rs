// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lazy solution annotations borrowing the original answer dictionary.

use purrdf_hash::{Domain, blake3::RecordHasher, frame::frame_le_into, hex::Digest32};

use super::{LedgerError, LedgerStatement, dictionary_node, predicate, reifier};
use crate::{
    TermValue,
    evidence::{AggregateBound, EvidenceRecord, Information, ReasonId, StandpointKey},
};

const ANNOTATION_DOMAIN: Domain = Domain::new(b"purrdf-core/evidence-annotation/v1");

/// Content key of a retained ledger node, independent of row and insertion ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LedgerNodeKey {
    /// The canonical original cause/concern description.
    Reason(ReasonId),
    /// A frame's canonical disjunction or one canonical conjunctive branch.
    Frame(Digest32),
    /// The caller's original standpoint, scenario or output-column identity.
    Reference(Digest32),
    /// One pointwise information assertion, reused across equal row contexts.
    Information(StandpointKey, Information),
    /// An independent bound for one original output column.
    Aggregate(Digest32, AggregateBound),
}

impl LedgerNodeKey {
    /// Canonical identity used by the caller's deliberate node-minting policy.
    #[must_use]
    pub fn identity(self) -> Digest32 {
        let mut hash = RecordHasher::new();
        frame_le_into(&mut hash, ANNOTATION_DOMAIN.as_bytes());
        match self {
            Self::Reason(reason) => return reason.digest(),
            Self::Frame(id) | Self::Reference(id) => return id,
            Self::Information(standpoint, information) => {
                frame_le_into(&mut hash, &[0]);
                match standpoint {
                    StandpointKey::Unscoped => frame_le_into(&mut hash, &[0]),
                    StandpointKey::Declared(id) => {
                        frame_le_into(&mut hash, &[1]);
                        frame_le_into(&mut hash, id.as_bytes());
                    }
                }
                frame_le_into(&mut hash, &[information as u8]);
            }
            Self::Aggregate(column, bound) => {
                frame_le_into(&mut hash, &[1]);
                frame_le_into(&mut hash, column.as_bytes());
                frame_le_into(&mut hash, &[bound as u8]);
            }
        }
        Digest32::new(*hash.finalize().as_bytes())
    }
}

/// Immutable original node carriers retained under the answer's native owner.
///
/// Implementations borrow the answer's dictionary. They must preserve the node
/// for a key throughout this borrow and mint content nodes through the caller's
/// explicit vocabulary. Rendering never invents a namespace or adopts a copy.
pub trait LedgerNodes: core::fmt::Debug {
    /// Borrow the original node; missing required nodes are a hard invariant error.
    fn node(&self, key: LedgerNodeKey) -> Option<&TermValue>;
}

/// Caller-selected predicates and enum values for solution evidence.
///
/// Information values follow FOUR's canonical encoding: neither, supported,
/// opposed, both. Completeness values are complete, undetermined, not-evaluated;
/// aggregate values are exact, lower, upper, undetermined. All are explicit IRIs.
#[derive(Clone, Copy, Debug)]
pub struct RowVocabulary<'a> {
    /// Independent fact of input completeness, retained when reasons are withheld.
    pub completeness: &'a TermValue,
    /// Information value, both for a fallback and a scoped assertion.
    pub information: &'a TermValue,
    /// Link to a pointwise information assertion node.
    pub information_context: &'a TermValue,
    /// The assertion's selected standpoint.
    pub standpoint: &'a TermValue,
    /// Link to the original canonical frame.
    pub frame: &'a TermValue,
    /// Link to an original content-identified reason.
    pub reason: &'a TermValue,
    /// Explicit override, without removing the original reason or incompleteness.
    pub accepted_reason: &'a TermValue,
    /// Link to an independent output-column bound.
    pub aggregate: &'a TermValue,
    /// The bound's original output column.
    pub column: &'a TermValue,
    /// Its bound value.
    pub bound: &'a TermValue,
    /// Explicit unscoped context, outside the declared standpoint order.
    pub unscoped: &'a TermValue,
    /// FOUR values in canonical encoding order.
    pub information_values: [&'a TermValue; 4],
    /// Completeness values in canonical encoding order.
    pub completeness_values: [&'a TermValue; 3],
    /// Aggregate values in canonical encoding order.
    pub bound_values: [&'a TermValue; 4],
}

/// Allocation-free row rendering, preserving original solution and dictionary.
#[derive(Debug)]
pub struct RowStatements<'a, N> {
    node: &'a TermValue,
    solution: &'a TermValue,
    record: &'a EvidenceRecord,
    nodes: &'a N,
    vocabulary: RowVocabulary<'a>,
    predicates: [&'a str; 10],
    accepted: &'a [ReasonId],
    phase: u8,
    index: usize,
}

// A rendering cursor copies only references and stack state, never N or a term.
impl<N> Clone for RowStatements<'_, N> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            solution: self.solution,
            record: self.record,
            nodes: self.nodes,
            vocabulary: self.vocabulary,
            predicates: self.predicates,
            accepted: self.accepted,
            phase: self.phase,
            index: self.index,
        }
    }
}

impl<'a, N: LedgerNodes> RowStatements<'a, N> {
    /// Validate the entire row declaration before publishing any statement.
    ///
    /// Accepted ids here belong to this row; callers intersect the accepted
    /// answer set with its original reasons. Every reason is still rendered.
    ///
    /// # Errors
    /// Refuses malformed RDF, missing retained nodes and invalid acceptance.
    pub fn new(
        node: &'a TermValue,
        solution: &'a TermValue,
        record: &'a EvidenceRecord,
        nodes: &'a N,
        vocabulary: RowVocabulary<'a>,
        accepted: &'a [ReasonId],
    ) -> Result<Self, LedgerError> {
        reifier(node)?;
        if !matches!(solution, TermValue::Triple { .. }) {
            return Err(LedgerError::InvalidSolution);
        }
        if accepted.windows(2).any(|pair| pair[0] >= pair[1])
            || accepted
                .iter()
                .any(|reason| record.reasons().binary_search(reason).is_err())
        {
            return Err(LedgerError::InvalidAcceptance);
        }
        let declarations = [
            vocabulary.completeness,
            vocabulary.information,
            vocabulary.information_context,
            vocabulary.standpoint,
            vocabulary.frame,
            vocabulary.reason,
            vocabulary.accepted_reason,
            vocabulary.aggregate,
            vocabulary.column,
            vocabulary.bound,
        ];
        let mut predicates = [""; 10];
        for (slot, declaration) in predicates.iter_mut().zip(declarations) {
            *slot = predicate(declaration)?;
        }
        for value in core::iter::once(vocabulary.unscoped)
            .chain(vocabulary.information_values)
            .chain(vocabulary.completeness_values)
            .chain(vocabulary.bound_values)
        {
            predicate(value)?;
        }
        let result = Self {
            node,
            solution,
            record,
            nodes,
            vocabulary,
            predicates,
            accepted,
            phase: 0,
            index: 0,
        };
        dictionary_node(
            result.nodes,
            LedgerNodeKey::Frame(record.frame().identity()),
        )?;
        for reason in record.reasons() {
            dictionary_node(result.nodes, LedgerNodeKey::Reason(*reason))?;
        }
        for &(standpoint, information) in record.information().entries() {
            dictionary_node(
                result.nodes,
                LedgerNodeKey::Information(standpoint, information),
            )?;
            if let StandpointKey::Declared(id) = standpoint {
                dictionary_node(result.nodes, LedgerNodeKey::Reference(id))?;
            }
        }
        for &(column, bound) in record.aggregate_bounds() {
            dictionary_node(result.nodes, LedgerNodeKey::Aggregate(column, bound))?;
            dictionary_node(result.nodes, LedgerNodeKey::Reference(column))?;
        }
        Ok(result)
    }

    fn pull(&mut self) -> Result<Option<LedgerStatement<'a>>, LedgerError> {
        let (subject, predicate, object) = loop {
            match self.phase {
                0 => {
                    self.phase = 1;
                    break (self.node, purrdf_iri::vocab::rdf::REIFIES, self.solution);
                }
                1 => {
                    self.phase = 2;
                    break (
                        self.node,
                        self.predicates[0],
                        self.vocabulary.completeness_values[self.record.completeness() as usize],
                    );
                }
                2 => {
                    self.phase = 3;
                    break (
                        self.node,
                        self.predicates[1],
                        self.vocabulary.information_values
                            [self.record.information().fallback() as usize],
                    );
                }
                3 => {
                    self.phase = 4;
                    break (
                        self.node,
                        self.predicates[4],
                        dictionary_node(
                            self.nodes,
                            LedgerNodeKey::Frame(self.record.frame().identity()),
                        )?,
                    );
                }
                4 | 5 => {
                    let reasons = if self.phase == 4 {
                        self.record.reasons()
                    } else {
                        self.accepted
                    };
                    if let Some(&reason) = reasons.get(self.index) {
                        self.index += 1;
                        break (
                            self.node,
                            self.predicates[if self.phase == 4 { 5 } else { 6 }],
                            dictionary_node(self.nodes, LedgerNodeKey::Reason(reason))?,
                        );
                    }
                    self.phase += 1;
                    self.index = 0;
                }
                6 => {
                    let Some(&(standpoint, information)) =
                        self.record.information().entries().get(self.index)
                    else {
                        self.phase = 7;
                        self.index = 0;
                        continue;
                    };
                    let node = dictionary_node(
                        self.nodes,
                        LedgerNodeKey::Information(standpoint, information),
                    )?;
                    self.index += 1;
                    break (self.node, self.predicates[2], node);
                }
                7 => {
                    let Some(&(column, bound)) = self.record.aggregate_bounds().get(self.index)
                    else {
                        self.phase = 8;
                        continue;
                    };
                    let node =
                        dictionary_node(self.nodes, LedgerNodeKey::Aggregate(column, bound))?;
                    self.index += 1;
                    break (self.node, self.predicates[7], node);
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

impl<'a, N: LedgerNodes> Iterator for RowStatements<'a, N> {
    type Item = Result<LedgerStatement<'a>, LedgerError>;

    fn next(&mut self) -> Option<Self::Item> {
        super::next_statement(self.pull(), &mut self.phase, 8)
    }
}

impl<N: LedgerNodes> core::iter::FusedIterator for RowStatements<'_, N> {}

/// One unique dictionary node's facts, separate from its repeated row links.
///
/// Render each content key once from the original interned dictionary. Repeated
/// row witnesses contribute their own links without duplicating RDF facts.
#[derive(Clone, Debug)]
pub struct AnnotationStatements<'a> {
    statements: core::array::IntoIter<LedgerStatement<'a>, 2>,
}

impl<'a> AnnotationStatements<'a> {
    /// Borrow one retained information or aggregate node and its original values.
    ///
    /// # Errors
    /// Refuses other node kinds, malformed declarations or missing original nodes.
    pub fn new<N: LedgerNodes>(
        key: LedgerNodeKey,
        nodes: &'a N,
        vocabulary: RowVocabulary<'a>,
    ) -> Result<Self, LedgerError> {
        let node = nodes.node(key).ok_or(LedgerError::MissingNode(key))?;
        reifier(node)?;
        let (first_predicate, first_object, second_predicate, second_object) = match key {
            LedgerNodeKey::Information(standpoint, information) => {
                let context = match standpoint {
                    StandpointKey::Unscoped => vocabulary.unscoped,
                    StandpointKey::Declared(id) => nodes
                        .node(LedgerNodeKey::Reference(id))
                        .ok_or(LedgerError::MissingNode(LedgerNodeKey::Reference(id)))?,
                };
                reifier(context)?;
                let value = vocabulary.information_values[information as usize];
                predicate(value)?;
                (
                    predicate(vocabulary.standpoint)?,
                    context,
                    predicate(vocabulary.information)?,
                    value,
                )
            }
            LedgerNodeKey::Aggregate(column, bound) => {
                let column_node = nodes
                    .node(LedgerNodeKey::Reference(column))
                    .ok_or(LedgerError::MissingNode(LedgerNodeKey::Reference(column)))?;
                reifier(column_node)?;
                let value = vocabulary.bound_values[bound as usize];
                predicate(value)?;
                (
                    predicate(vocabulary.column)?,
                    column_node,
                    predicate(vocabulary.bound)?,
                    value,
                )
            }
            _ => return Err(LedgerError::InvalidAnnotation),
        };
        Ok(Self {
            statements: [
                LedgerStatement {
                    subject: node,
                    predicate: first_predicate,
                    object: first_object,
                },
                LedgerStatement {
                    subject: node,
                    predicate: second_predicate,
                    object: second_object,
                },
            ]
            .into_iter(),
        })
    }
}

impl<'a> Iterator for AnnotationStatements<'a> {
    type Item = LedgerStatement<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.statements.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.statements.size_hint()
    }
}
impl ExactSizeIterator for AnnotationStatements<'_> {}
impl core::iter::FusedIterator for AnnotationStatements<'_> {}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::{
        TermBox,
        evidence::{Frame, FrameComponent, InformationMap, ReasonDescription, SearchCompleteness},
    };
    use purrdf_lex::allocation::{Memory, Resident};

    #[derive(Debug, Default)]
    pub(in crate::evidence::ledger) struct Nodes(pub Vec<(LedgerNodeKey, TermValue)>);

    impl LedgerNodes for Nodes {
        fn node(&self, key: LedgerNodeKey) -> Option<&TermValue> {
            self.0
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, term)| term)
        }
    }

    impl Nodes {
        pub(in crate::evidence::ledger) fn add(&mut self, key: LedgerNodeKey) {
            if self.node(key).is_none() {
                self.0.push((
                    key,
                    TermValue::iri(format!("https://example.org/node/{}", key.identity())),
                ));
            }
        }
    }

    pub(in crate::evidence::ledger) fn vocabulary(terms: &[TermValue; 22]) -> RowVocabulary<'_> {
        RowVocabulary {
            completeness: &terms[0],
            information: &terms[1],
            information_context: &terms[2],
            standpoint: &terms[3],
            frame: &terms[4],
            reason: &terms[5],
            accepted_reason: &terms[6],
            aggregate: &terms[7],
            column: &terms[8],
            bound: &terms[9],
            unscoped: &terms[10],
            information_values: [&terms[11], &terms[12], &terms[13], &terms[14]],
            completeness_values: [&terms[15], &terms[16], &terms[17]],
            bound_values: [&terms[18], &terms[19], &terms[20], &terms[21]],
        }
    }

    pub(in crate::evidence::ledger) fn solution() -> TermValue {
        TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/record")),
            p: TermBox::new(TermValue::iri("https://example.org/value")),
            o: TermBox::new(TermValue::simple_literal("original")),
        }
    }

    #[test]
    fn lazy_row_annotations_keep_incompleteness_reasons_acceptance_and_original_terms() {
        let terms =
            core::array::from_fn(|index| TermValue::iri(format!("https://example.org/v/{index}")));
        let vocabulary = vocabulary(&terms);
        let solution = solution();
        let cause = TermValue::iri("https://example.org/missing-time");
        let other_cause = TermValue::iri("https://example.org/missing-place");
        let node = TermValue::iri("https://example.org/answer/row");
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let describe = |cause| ReasonDescription {
            cause,
            concern: &solution,
            constraint: None,
            vault: None,
            watermark: None,
        };
        let mut reasons = [
            describe(&cause).identity(&mut memory).unwrap(),
            describe(&other_cause).identity(&mut memory).unwrap(),
        ];
        reasons.sort_unstable();
        let standpoint = StandpointKey::Declared(Digest32::new([3; 32]));
        let information = InformationMap::from_entries(
            Information::Neither,
            &[(standpoint, Information::Both)],
            &mut memory,
        )
        .unwrap();
        let frame = Frame::from_components(
            &[FrameComponent::Counterfactual(Digest32::new([5; 32]))],
            &mut memory,
        )
        .unwrap();
        let column = Digest32::new([7; 32]);
        let record = EvidenceRecord::from_parts(
            &information,
            SearchCompleteness::Undetermined,
            &frame,
            &reasons,
            &[(column, AggregateBound::Lower)],
            &mut memory,
        )
        .unwrap();
        let mut nodes = Nodes::default();
        for key in [
            LedgerNodeKey::Frame(frame.identity()),
            LedgerNodeKey::Reason(reasons[0]),
            LedgerNodeKey::Reason(reasons[1]),
            LedgerNodeKey::Information(standpoint, Information::Both),
            LedgerNodeKey::Reference(Digest32::new([3; 32])),
            LedgerNodeKey::Aggregate(column, AggregateBound::Lower),
            LedgerNodeKey::Reference(column),
        ] {
            nodes.add(key);
        }
        let live = memory.admitted_bytes();
        let mut statements =
            RowStatements::new(&node, &solution, &record, &nodes, vocabulary, &reasons[..1])
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        for key in [
            LedgerNodeKey::Information(standpoint, Information::Both),
            LedgerNodeKey::Aggregate(column, AggregateBound::Lower),
        ] {
            statements.extend(AnnotationStatements::new(key, &nodes, vocabulary).unwrap());
        }
        assert_eq!(
            memory.admitted_bytes(),
            live,
            "rendering borrows the existing owner"
        );
        assert!(core::ptr::eq(statements[0].subject, &raw const node));
        assert!(core::ptr::eq(statements[0].object, &raw const solution));
        assert_eq!(statements[0].predicate, purrdf_iri::vocab::rdf::REIFIES);
        assert_eq!(
            statements
                .iter()
                .filter(|statement| statement.predicate == predicate(vocabulary.reason).unwrap())
                .count(),
            2
        );
        let accepted = statements
            .iter()
            .filter(|statement| {
                statement.predicate == predicate(vocabulary.accepted_reason).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(accepted.len(), 1);
        assert!(core::ptr::eq(
            accepted[0].object,
            nodes.node(LedgerNodeKey::Reason(reasons[0])).unwrap()
        ));
        let projection = statements
            .iter()
            .filter(|statement| statement.predicate == predicate(vocabulary.completeness).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            projection.len(),
            1,
            "withholding all reasons keeps the independent fact"
        );
        assert_eq!(projection[0].object, vocabulary.completeness_values[1]);
        assert!(statements.iter().any(|statement| statement.predicate
            == predicate(vocabulary.information).unwrap()
            && statement.object == vocabulary.information_values[3]));
        assert!(statements.iter().any(|statement| statement.predicate
            == predicate(vocabulary.bound).unwrap()
            && statement.object == vocabulary.bound_values[1]));
        assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
        assert_eq!(record.reasons(), reasons);
    }

    #[test]
    fn malformed_ledger_declarations_refuse_before_any_prefix_is_published() {
        let terms =
            core::array::from_fn(|index| TermValue::iri(format!("https://example.org/v/{index}")));
        let mut vocabulary = vocabulary(&terms);
        let solution = solution();
        let node = TermValue::iri("https://example.org/row");
        let record = EvidenceRecord::default();
        let mut nodes = Nodes::default();
        let key = LedgerNodeKey::Frame(record.frame().identity());
        assert!(
            matches!(RowStatements::new(&node, &solution, &record, &nodes, vocabulary, &[]), Err(LedgerError::MissingNode(found)) if found == key)
        );
        nodes.add(key);
        let invalid = TermValue::simple_literal("incomplete");
        assert!(matches!(
            RowStatements::new(&node, &invalid, &record, &nodes, vocabulary, &[]),
            Err(LedgerError::InvalidSolution)
        ));
        vocabulary.completeness = &invalid;
        assert!(matches!(
            RowStatements::new(&node, &solution, &record, &nodes, vocabulary, &[]),
            Err(LedgerError::InvalidPredicate)
        ));
    }
}
