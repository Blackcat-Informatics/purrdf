// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Existing conversion loss codes become explicit caller-selected reason causes.

use super::{ReasonError, RetainedReason};
use crate::{LossEntry, TermValue};
use purrdf_lex::allocation::{Admission, Memory};

/// Invalid explicit mapping, missing conversion cause or original reason refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LossCauseError<'a> {
    /// A declared mapping has no machine code.
    EmptyCode,
    /// Two declarations assign the same machine code.
    DuplicateCode(&'a str),
    /// The conversion emitted a code without an explicit caller cause.
    MissingCode(&'a str),
    /// The original RDF declaration or native physical construction failed.
    Reason(ReasonError),
}

impl core::fmt::Display for LossCauseError<'_> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyCode => out.write_str("conversion cause mapping has an empty code"),
            Self::DuplicateCode(code) => {
                write!(out, "duplicate conversion cause mapping for {code}")
            }
            Self::MissingCode(code) => {
                write!(out, "conversion loss {code} lacks a caller-selected cause")
            }
            Self::Reason(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for LossCauseError<'_> {}

/// Borrowed explicit mapping over existing loss codes, with no namespace default.
///
/// This maps only the cause. A conversion's location and explanatory prose never
/// invent its concern: the caller supplies the original genuine triple term.
/// The existing LossLedger and its JSON contract are neither rewritten nor owned
/// by this mapping. No heap is allocated to configure or look up the mapping.
#[derive(Clone, Copy, Debug)]
pub struct LossCauses<'a> {
    mappings: &'a [(&'a str, &'a TermValue)],
}

impl<'a> LossCauses<'a> {
    /// Validate unambiguous caller declarations in their original order.
    ///
    /// An empty table is permitted; using it for a loss produces MissingCode.
    /// Causes are validated by the one retained-reason factory when used.
    ///
    /// # Errors
    /// Names an empty or duplicate code rather than silently choosing a mapping.
    pub fn new(mappings: &'a [(&'a str, &'a TermValue)]) -> Result<Self, LossCauseError<'a>> {
        for (index, &(code, _)) in mappings.iter().enumerate() {
            if code.is_empty() {
                return Err(LossCauseError::EmptyCode);
            }
            if purrdf_lex::assoc::get(&mappings[..index], code).is_some() {
                return Err(LossCauseError::DuplicateCode(code));
            }
        }
        Ok(Self { mappings })
    }

    /// Borrow a cause declaration for the caller's original admitted term owner.
    ///
    /// An owning evaluator can copy it through its native fallible term factory,
    /// then move that owner into RetainedReason rather than borrowing this table.
    /// The single retained-reason factory validates the final RDF declaration.
    ///
    /// # Errors
    /// Names an emitted code that has no explicit caller declaration.
    pub fn declared_cause<'b>(
        &self,
        loss: &'b LossEntry,
    ) -> Result<&'a TermValue, LossCauseError<'b>> {
        purrdf_lex::assoc::get(self.mappings, loss.code.as_ref())
            .copied()
            .ok_or_else(|| LossCauseError::MissingCode(loss.code.as_ref()))
    }

    /// Retain one original conversion loss as a canonical RDF reason.
    ///
    /// Only its declared cause code and supplied original terms determine the
    /// reason. Repeated locations with the same concern therefore intern to one
    /// reason; row-specific location/binding annotations remain on their rows.
    /// All terms remain borrowed under their original owners. Canonical spelling
    /// uses the original native scratch admission through RetainedReason.
    ///
    /// # Errors
    /// Names an unmapped code, or preserves original declaration/storage refusal.
    pub fn retain<'b, S: Admission + ?Sized>(
        &self,
        loss: &'b LossEntry,
        concern: &'b TermValue,
        constraint: Option<&'b TermValue>,
        vault: Option<&'b TermValue>,
        watermark: Option<&'b TermValue>,
        memory: &mut Memory<'_, S>,
    ) -> Result<RetainedReason<&'b TermValue>, LossCauseError<'b>>
    where
        'a: 'b,
    {
        let cause = self.declared_cause(loss)?;
        RetainedReason::from_parts(cause, concern, constraint, vault, watermark, memory)
            .map_err(LossCauseError::Reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::ReasonTable;
    use crate::{LossLedger, TermBox};
    use purrdf_lex::allocation::{Resident, StorageError};

    fn loss() -> LossEntry {
        LossEntry {
            code: crate::loss::LOSS_EMPTY_NAMED_GRAPH_DROPPED.into(),
            from: "trig".into(),
            to: "n-quads".into(),
            note: "target has no declaration-only named graph".into(),
            location: None,
        }
    }

    fn concern() -> TermValue {
        TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/empty-graph")),
            p: TermBox::new(TermValue::iri(purrdf_iri::vocab::rdf::TYPE)),
            o: TermBox::new(TermValue::iri(purrdf_iri::vocab::rdfs::RESOURCE)),
        }
    }

    #[test]
    fn original_loss_codes_share_reasons_without_changing_the_json_ledger() {
        let entry = loss();
        let mut ledger = LossLedger::new();
        ledger.record(entry.clone());
        let original = ledger.render_json();
        let cause = TermValue::iri("https://example.org/empty-graph-dropped");
        let concern = concern();
        let mappings = [(entry.code.as_ref(), &cause)];
        let causes = LossCauses::new(&mappings).unwrap();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut reasons = ReasonTable::with_admission(Resident);
        let first = causes
            .retain(&entry, &concern, None, None, None, &mut memory)
            .unwrap();
        assert!(core::ptr::eq(first.description().cause, &raw const cause));
        assert!(core::ptr::eq(
            first.description().concern,
            &raw const concern
        ));
        let id = reasons.intern(first).unwrap();
        let second = causes
            .retain(&entry, &concern, None, None, None, &mut memory)
            .unwrap();
        assert_eq!(reasons.intern(second).unwrap(), id);
        assert_eq!(reasons.entries().unwrap().len(), 1);
        assert_eq!(ledger.render_json(), original);
    }

    #[test]
    fn missing_ambiguous_and_invalid_causes_remain_hard_failures() {
        let entry = loss();
        let concern = concern();
        let cause = TermValue::iri("https://example.org/cause");
        let duplicate = [(entry.code.as_ref(), &cause), (entry.code.as_ref(), &cause)];
        assert!(
            matches!(LossCauses::new(&duplicate), Err(LossCauseError::DuplicateCode(code)) if code == entry.code.as_ref())
        );
        assert!(matches!(
            LossCauses::new(&[("", &cause)]),
            Err(LossCauseError::EmptyCode)
        ));
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        assert!(
            matches!(LossCauses::new(&[]).unwrap().retain(&entry, &concern, None, None, None, &mut memory), Err(LossCauseError::MissingCode(code)) if code == entry.code.as_ref())
        );
        let invalid = TermValue::Blank {
            label: "cause".into(),
            scope: crate::BlankScope::DEFAULT,
        };
        let mappings = [(entry.code.as_ref(), &invalid)];
        assert!(matches!(
            LossCauses::new(&mappings).unwrap().retain(
                &entry,
                &concern,
                None,
                None,
                None,
                &mut memory
            ),
            Err(LossCauseError::Reason(ReasonError::CauseMustBeIri))
        ));
    }

    #[derive(Debug)]
    struct NoCapacity;

    impl Admission for NoCapacity {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes == 0 {
                Ok(())
            } else {
                Err(StorageError::AdmissionFailed)
            }
        }
    }

    #[test]
    fn canonical_spelling_refusal_remains_physical() {
        let entry = loss();
        let cause = TermValue::iri("https://example.org/cause");
        let mut concern = concern();
        for _ in 0..24 {
            concern = TermValue::Triple {
                s: TermBox::new(concern),
                p: TermBox::new(TermValue::iri("https://example.org/p")),
                o: TermBox::new(TermValue::iri("https://example.org/o")),
            };
        }
        let mappings = [(entry.code.as_ref(), &cause)];
        let mut admission = NoCapacity;
        let mut memory = Memory::new(&mut admission);
        assert!(matches!(
            LossCauses::new(&mappings).unwrap().retain(
                &entry,
                &concern,
                None,
                None,
                None,
                &mut memory
            ),
            Err(LossCauseError::Reason(ReasonError::Storage(
                StorageError::AdmissionFailed
            )))
        ));
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
