// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed RDF 1.2 reason statements from the original retained description.

use crate::TermValue;
use purrdf_lex::allocation::{Admission, Memory};

use super::{ReasonDescription, ReasonError, ReasonId};

/// Fuse a borrowed statement cursor at its own terminal phase after refusal.
fn next_statement<'a>(
    statement: Result<Option<LedgerStatement<'a>>, LedgerError>,
    phase: &mut u8,
    terminal: u8,
) -> Option<Result<LedgerStatement<'a>, LedgerError>> {
    if statement.is_err() {
        *phase = terminal;
    }
    statement.transpose()
}

mod row;
pub use row::{AnnotationStatements, LedgerNodeKey, LedgerNodes, RowStatements, RowVocabulary};
mod frame;
pub use frame::{FrameStatements, FrameVocabulary};
mod terms;
pub use terms::{BorrowedLedgerTerms, LedgerTermId};
mod view;
pub use view::{LedgerDataset, LedgerReadError, LedgerSource};
mod source;
pub use source::{AnswerLedger, LedgerBuildError, LedgerRow, LedgerVocabulary};

/// Caller-selected reason annotation predicates, with no namespace default.
#[derive(Clone, Copy, Debug)]
pub struct ReasonVocabulary<'a> {
    /// The reason's cause, independently of the concern it reifies.
    pub cause: &'a TermValue,
    /// The original constraint identity.
    pub constraint: &'a TermValue,
    /// The original vault identity.
    pub vault: &'a TermValue,
    /// The original watermark identity.
    pub watermark: &'a TermValue,
}

/// One borrowed statement in the answer's own graph, with no graph name.
#[derive(Clone, Copy, Debug)]
pub struct LedgerStatement<'a> {
    /// Original reifier node owned by the answer.
    pub subject: &'a TermValue,
    /// A validated caller predicate or the W3C RDF 1.2 reification predicate.
    pub predicate: &'a str,
    /// Original term, including a genuine triple term for a reified concern.
    pub object: &'a TermValue,
}

/// A malformed rendering declaration or original canonical spelling failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerError {
    /// A reifier must be an IRI or blank node.
    InvalidReifier,
    /// Every annotation predicate must be a valid caller-selected IRI.
    InvalidPredicate,
    /// The original reason is malformed or its native scratch was refused.
    Reason(ReasonError),
    /// A row evidence reifier must concern a genuine solution triple term.
    InvalidSolution,
    /// A required original dictionary node was not retained by the answer.
    MissingNode(LedgerNodeKey),
    /// Acceptance identities must be a sorted unique set of retained reasons.
    InvalidAcceptance,
    /// This key does not name a pointwise information or aggregate annotation.
    InvalidAnnotation,
    /// The selected conjunctive alternative is absent from the original frame.
    InvalidFrameAlternative,
}

impl core::fmt::Display for LedgerError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidReifier => out.write_str("ledger reifier must be an IRI or blank node"),
            Self::InvalidPredicate => {
                out.write_str("ledger annotation predicate must be a valid caller-selected IRI")
            }
            Self::Reason(error) => error.fmt(out),
            Self::InvalidSolution => {
                out.write_str("ledger solution must be an RDF 1.2 triple term")
            }
            Self::MissingNode(key) => write!(out, "original ledger dictionary lacks {key:?}"),
            Self::InvalidAcceptance => {
                out.write_str("ledger acceptance must name sorted unique original reasons")
            }
            Self::InvalidAnnotation => out.write_str("ledger key does not name an annotation"),
            Self::InvalidFrameAlternative => out.write_str("ledger frame alternative is absent"),
        }
    }
}

impl std::error::Error for LedgerError {}

pub(super) fn reifier(node: &TermValue) -> Result<(), LedgerError> {
    match node {
        TermValue::Blank { .. } => Ok(()),
        TermValue::Iri(_) => predicate(node)
            .map(|_| ())
            .map_err(|_| LedgerError::InvalidReifier),
        _ => Err(LedgerError::InvalidReifier),
    }
}

pub(super) fn predicate(term: &TermValue) -> Result<&str, LedgerError> {
    match term {
        TermValue::Iri(value) if purrdf_iri::is_absolute(value).is_ok_and(|absolute| absolute) => {
            Ok(value)
        }
        _ => Err(LedgerError::InvalidPredicate),
    }
}

pub(super) fn dictionary_node<N: LedgerNodes>(
    nodes: &N,
    key: LedgerNodeKey,
) -> Result<&TermValue, LedgerError> {
    let node = nodes.node(key).ok_or(LedgerError::MissingNode(key))?;
    reifier(node)?;
    Ok(node)
}

/// Lazy, allocation-free reason rendering borrowing the original answer owner.
///
/// Cause and concern are separate statements. A CONSTRUCT can keep the cause
/// while dropping the reified concern, vault or constraint. Row completeness is
/// independently annotated by the row renderer and never inferred from whether
/// these statements are exposed. No resident term copy or named graph is born.
#[derive(Clone, Debug)]
pub struct ReasonStatements<'a> {
    node: &'a TermValue,
    description: ReasonDescription<'a>,
    predicates: [&'a str; 4],
    identity: ReasonId,
    position: u8,
}

impl<'a> ReasonStatements<'a> {
    /// Validate the complete declaration before exposing the first statement.
    ///
    /// The caller's node is minted from the returned content identity through
    /// its chosen vocabulary. Its spelling never changes the reason identity.
    ///
    /// # Errors
    /// Refuses invalid reifiers, predicates or original reason construction.
    pub fn new<S: Admission + ?Sized>(
        node: &'a TermValue,
        description: ReasonDescription<'a>,
        vocabulary: ReasonVocabulary<'a>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, LedgerError> {
        let identity = description.identity(memory).map_err(LedgerError::Reason)?;
        Self::declared(node, description, vocabulary, identity)
    }

    /// Borrow a previously validated reason's original cached content identity.
    /// No repeated canonical spelling or copying is needed to render the table.
    ///
    /// # Errors
    /// Refuses invalid caller reifiers or annotation predicates.
    pub fn retained<T: core::ops::Deref<Target = TermValue>>(
        node: &'a TermValue,
        reason: &'a super::RetainedReason<T>,
        vocabulary: ReasonVocabulary<'a>,
    ) -> Result<Self, LedgerError> {
        Self::declared(node, reason.description(), vocabulary, reason.identity())
    }

    fn declared(
        node: &'a TermValue,
        description: ReasonDescription<'a>,
        vocabulary: ReasonVocabulary<'a>,
        identity: ReasonId,
    ) -> Result<Self, LedgerError> {
        reifier(node)?;
        let mut predicates = [""; 4];
        for (slot, term) in predicates.iter_mut().zip([
            vocabulary.cause,
            vocabulary.constraint,
            vocabulary.vault,
            vocabulary.watermark,
        ]) {
            *slot = predicate(term)?;
        }
        Ok(Self {
            node,
            description,
            predicates,
            identity,
            position: 0,
        })
    }

    /// Content identity of the borrowed original reason.
    #[must_use]
    pub const fn identity(&self) -> ReasonId {
        self.identity
    }
}

impl<'a> Iterator for ReasonStatements<'a> {
    type Item = LedgerStatement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.position < 5 {
            let position = self.position;
            self.position += 1;
            let (predicate, object) = match position {
                0 => (
                    purrdf_iri::vocab::rdf::REIFIES,
                    Some(self.description.concern),
                ),
                1 => (self.predicates[0], Some(self.description.cause)),
                2 => (self.predicates[1], self.description.constraint),
                3 => (self.predicates[2], self.description.vault),
                _ => (self.predicates[3], self.description.watermark),
            };
            if let Some(object) = object {
                return Some(LedgerStatement {
                    subject: self.node,
                    predicate,
                    object,
                });
            }
        }
        None
    }
}

impl core::iter::FusedIterator for ReasonStatements<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TermBox,
        evidence::{ReasonTable, RetainedReason},
    };
    use purrdf_lex::allocation::Resident;
    use std::{cell::Cell, rc::Rc};

    #[derive(Debug)]
    struct OriginalTerm {
        value: TermValue,
        live: Rc<Cell<usize>>,
    }
    impl core::ops::Deref for OriginalTerm {
        type Target = TermValue;
        fn deref(&self) -> &TermValue {
            &self.value
        }
    }
    impl Drop for OriginalTerm {
        fn drop(&mut self) {
            self.live.set(self.live.get() - 1);
        }
    }

    #[test]
    fn lazy_reason_projection_keeps_original_owners_and_separates_cause_from_concern() {
        let live = Rc::new(Cell::new(0));
        let own = |value| {
            live.set(live.get() + 1);
            OriginalTerm {
                value,
                live: Rc::clone(&live),
            }
        };
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let reason = RetainedReason::from_parts(
            own(TermValue::iri("https://example.org/missing-time")),
            own(TermValue::Triple {
                s: TermBox::new(TermValue::iri("https://example.org/private-vault")),
                p: TermBox::new(TermValue::iri("https://example.org/time")),
                o: TermBox::new(TermValue::simple_literal("required")),
            }),
            None,
            Some(own(TermValue::iri("https://example.org/private-vault"))),
            None,
            &mut memory,
        )
        .unwrap();
        assert_eq!(live.get(), 3);
        let node = TermValue::iri("https://example.org/reason-node");
        let cause = TermValue::iri("https://example.org/cause");
        let constraint = TermValue::iri("https://example.org/constraint");
        let vault = TermValue::iri("https://example.org/vault");
        let watermark = TermValue::iri("https://example.org/watermark");
        let vocabulary = ReasonVocabulary {
            cause: &cause,
            constraint: &constraint,
            vault: &vault,
            watermark: &watermark,
        };
        let mut statements =
            ReasonStatements::new(&node, reason.description(), vocabulary, &mut memory).unwrap();
        assert_eq!(statements.identity(), reason.identity());
        let reified = statements.next().unwrap();
        assert_eq!(reified.predicate, purrdf_iri::vocab::rdf::REIFIES);
        assert!(matches!(reified.object, TermValue::Triple { .. }));
        assert!(core::ptr::eq(reified.object, reason.description().concern));
        let cause_statement = statements.next().unwrap();
        assert_eq!(cause_statement.predicate, "https://example.org/cause");
        assert!(core::ptr::eq(
            cause_statement.object,
            reason.description().cause
        ));
        // A presenter can keep this cause without the original private concern.
        assert!(!matches!(cause_statement.object, TermValue::Triple { .. }));
        assert_eq!(
            statements.next().unwrap().predicate,
            "https://example.org/vault"
        );
        assert!(statements.next().is_none());
        assert!(statements.next().is_none());
        assert_eq!(live.get(), 3);
        assert_eq!(memory.admitted_bytes(), 0);
        let invalid = TermValue::iri("relative");
        assert!(matches!(
            ReasonStatements::new(
                &node,
                reason.description(),
                ReasonVocabulary {
                    cause: &invalid,
                    ..vocabulary
                },
                &mut memory
            ),
            Err(LedgerError::InvalidPredicate)
        ));
        let original_id = reason.identity();
        let mut table = ReasonTable::with_admission(Resident);
        assert_eq!(table.intern(reason).unwrap(), original_id);
        for _ in 0..2279 {
            let duplicate = RetainedReason::from_parts(
                own(TermValue::iri("https://example.org/missing-time")),
                own(TermValue::Triple {
                    s: TermBox::new(TermValue::iri("https://example.org/private-vault")),
                    p: TermBox::new(TermValue::iri("https://example.org/time")),
                    o: TermBox::new(TermValue::simple_literal("required")),
                }),
                None,
                Some(own(TermValue::iri("https://example.org/private-vault"))),
                None,
                &mut memory,
            )
            .unwrap();
            assert_eq!(live.get(), 6);
            assert_eq!(table.intern(duplicate).unwrap(), original_id);
            assert_eq!(live.get(), 3);
        }
        assert_eq!(table.entries().unwrap().len(), 1);
        assert_eq!(table.get(original_id).unwrap().identity(), original_id);
        drop(table);
        assert_eq!(live.get(), 0);
    }
}
