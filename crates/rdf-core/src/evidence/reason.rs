// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Content-identified causes and concerns, independent of row keys and ranks.

use core::fmt::{self, Write as _};

use purrdf_hash::{
    Domain,
    blake3::RecordHasher,
    frame::{frame_le_into, frame_length_into},
    hex::Digest32,
};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::{TermValue, sink::TextOut, term_writer::write_term_value_with_memory};

const REASON_DOMAIN: Domain = Domain::new(b"purrdf-core/evidence-reason/v1");

/// Canonical content identity of a reason, never a query-local insertion number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReasonId(Digest32);

impl purrdf_hash::error::StructuredError for ReasonId {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        let Self(id) = self;
        visitor.variant("evidence-reason-id");
        visitor.value(
            Some("digest"),
            purrdf_hash::error::ErrorValue::Bytes(id.as_bytes()),
        );
    }
}

impl ReasonId {
    /// The shared digest value, rendered by the original base16 home.
    #[must_use]
    pub const fn digest(self) -> Digest32 {
        self.0
    }
}

/// A malformed reason declaration or refusal of its native construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReasonError {
    /// A reason must reify a real RDF 1.2 triple term.
    ConcernMustBeTriple,
    /// Its separately annotated cause must be a caller-selected IRI.
    CauseMustBeIri,
    /// Native admission, layout or allocation refusal.
    Storage(StorageError),
}

impl purrdf_hash::error::StructuredError for ReasonError {
    purrdf_hash::error_visit! {
        ConcernMustBeTriple => "evidence-reason-concern-must-be-triple" {},
        CauseMustBeIri => "evidence-reason-cause-must-be-iri" {},
        Storage(cause) => "evidence-reason-storage" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(ReasonError { Storage(StorageError) });

impl fmt::Display for ReasonError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConcernMustBeTriple => {
                out.write_str("evidence reason concern must be an RDF 1.2 triple term")
            }
            Self::CauseMustBeIri => {
                out.write_str("evidence reason cause must be a caller-selected IRI")
            }
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for ReasonError {}

/// A reason's immutable canonical description borrowed from its original owner.
///
/// Cause and concern are independent statements in the eventual RDF rendering.
/// A row id, binding, rank or score does not enter this description: many rows
/// with the same missing constraint/vault/watermark therefore share one reason.
#[derive(Clone, Copy, Debug)]
pub struct ReasonDescription<'a> {
    /// Caller-selected cause IRI; conversion loss codes map here explicitly.
    pub cause: &'a TermValue,
    /// The genuine triple term which the reason reifies.
    pub concern: &'a TermValue,
    /// Optional original constraint identity.
    pub constraint: Option<&'a TermValue>,
    /// Optional original vault identity.
    pub vault: Option<&'a TermValue>,
    /// Optional original watermark identity.
    pub watermark: Option<&'a TermValue>,
}

/// A reason retained through its original term owners and their native grants.
///
/// `T` is the existing immutable term carrier, such as the evaluator's admitted
/// term owner. Construction moves those carriers; it never clones resident RDF
/// terms or detaches their physical admission. There is deliberately no `Clone`.
/// The description is borrowed only while these original owners remain alive.
#[derive(Debug)]
pub struct RetainedReason<T> {
    cause: T,
    concern: T,
    constraint: Option<T>,
    vault: Option<T>,
    watermark: Option<T>,
    identity: ReasonId,
}

impl<T: core::ops::Deref<Target = TermValue>> RetainedReason<T> {
    /// Validate and identify original admitted terms before publishing a reason.
    ///
    /// Failed construction drops every moved term owner and its original grant.
    /// Canonical spelling uses the same native scratch admission as borrowed
    /// reasons. Optional terms remain absent, with no invented vocabulary.
    ///
    /// # Errors
    /// Returns malformed RDF reason declarations or original storage refusal.
    pub fn from_parts<S: Admission + ?Sized>(
        cause: T,
        concern: T,
        constraint: Option<T>,
        vault: Option<T>,
        watermark: Option<T>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, ReasonError> {
        let identity = ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: constraint.as_deref(),
            vault: vault.as_deref(),
            watermark: watermark.as_deref(),
        }
        .identity(memory)?;
        Ok(Self {
            cause,
            concern,
            constraint,
            vault,
            watermark,
            identity,
        })
    }

    /// Borrow the original description for lazy ledger rendering.
    #[must_use]
    pub fn description(&self) -> ReasonDescription<'_> {
        ReasonDescription {
            cause: &self.cause,
            concern: &self.concern,
            constraint: self.constraint.as_deref(),
            vault: self.vault.as_deref(),
            watermark: self.watermark.as_deref(),
        }
    }

    /// Canonical content identity computed before publication.
    #[must_use]
    pub const fn identity(&self) -> ReasonId {
        self.identity
    }
}

/// Failure of the original retained reason dictionary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReasonTableError {
    /// A previous mutation failed; the partial table cannot be published.
    Aborted,
    /// The row names a reason which this original table does not own.
    UnknownReason(ReasonId),
    /// Equal identities cannot silently merge different canonical descriptions.
    IdentityCollision(ReasonId),
    /// Original native capacity/layout/admission refusal.
    Storage(StorageError),
}

impl fmt::Display for ReasonTableError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aborted => out.write_str("original reason dictionary was aborted"),
            Self::UnknownReason(id) => write!(
                out,
                "original reason dictionary does not own {}",
                id.digest()
            ),
            Self::IdentityCollision(id) => write!(
                out,
                "different original reasons share identity {}",
                id.digest()
            ),
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for ReasonTableError {}

/// One original description per canonical reason identity, in content order.
///
/// Term carriers already own their individual grants; the table admits its own
/// array before birth through `S`. Duplicate incoming owners are destroyed
/// immediately. The entry array and all retained terms die before the array's
/// admission owner. There is no resident deep clone or query-local published id.
#[derive(Debug)]
pub struct ReasonTable<S: Admission, T> {
    entries: Vec<(ReasonId, RetainedReason<T>)>,
    live: usize,
    aborted: bool,
    admission: S,
}

impl<S: Admission, T: core::ops::Deref<Target = TermValue>> ReasonTable<S, T> {
    /// Retain the original caller's array grant before any entry is born.
    #[must_use]
    pub const fn with_admission(admission: S) -> Self {
        Self {
            entries: Vec::new(),
            live: 0,
            aborted: false,
            admission,
        }
    }

    /// Intern moved original term owners, deduplicating by canonical content.
    ///
    /// # Errors
    /// Returns original storage refusal, a content collision or an aborted table.
    pub fn intern(&mut self, reason: RetainedReason<T>) -> Result<ReasonId, ReasonTableError> {
        if self.aborted {
            return Err(ReasonTableError::Aborted);
        }
        let id = reason.identity();
        let position = match self.entries.binary_search_by_key(&id, |entry| entry.0) {
            Ok(position) => {
                let old = self.entries[position].1.description();
                let new = reason.description();
                if old.cause != new.cause
                    || old.concern != new.concern
                    || old.constraint != new.constraint
                    || old.vault != new.vault
                    || old.watermark != new.watermark
                {
                    self.aborted = true;
                    return Err(ReasonTableError::IdentityCollision(id));
                }
                return Ok(id);
            }
            Err(position) => position,
        };
        let Some(required) = self.entries.len().checked_add(1) else {
            self.aborted = true;
            return Err(ReasonTableError::Storage(StorageError::SizeOverflow));
        };
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = memory.reserve(&mut self.entries, required);
        self.live = memory.admitted_bytes();
        if let Err(error) = result {
            self.aborted = true;
            return Err(ReasonTableError::Storage(error));
        }
        self.entries.insert(position, (id, reason));
        Ok(id)
    }

    /// Borrow one original immutable description for queryable ledger rendering.
    ///
    /// # Errors
    /// Refuses missing reasons or publication from an aborted table.
    pub fn get(&self, id: ReasonId) -> Result<&RetainedReason<T>, ReasonTableError> {
        if self.aborted {
            return Err(ReasonTableError::Aborted);
        }
        self.entries
            .binary_search_by_key(&id, |entry| entry.0)
            .map(|position| &self.entries[position].1)
            .map_err(|_| ReasonTableError::UnknownReason(id))
    }

    /// Borrow the canonical dictionary only when its complete owner is valid.
    ///
    /// # Errors
    /// Refuses an aborted table rather than exposing its partial prefix.
    pub fn entries(&self) -> Result<&[(ReasonId, RetainedReason<T>)], ReasonTableError> {
        if self.aborted {
            Err(ReasonTableError::Aborted)
        } else {
            Ok(&self.entries)
        }
    }
}

impl ReasonDescription<'_> {
    /// Compute the canonical content identity through the original term writer.
    ///
    /// Term spelling is streamed twice: once to count, then into its framed
    /// digest. No complete serialization is copied into a resident string. Both
    /// walks retain their native scratch admission and use the same immutable
    /// term, exact escaping and scoped blank-label spelling.
    ///
    /// # Errors
    /// Refuses malformed declarations or any native spelling/storage failure.
    pub fn identity<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<ReasonId, ReasonError> {
        if !matches!(self.cause, TermValue::Iri(value)
            if purrdf_iri::is_absolute(value).is_ok_and(|absolute| absolute))
        {
            return Err(ReasonError::CauseMustBeIri);
        }
        if !matches!(self.concern, TermValue::Triple { .. }) {
            return Err(ReasonError::ConcernMustBeTriple);
        }
        let mut hash = RecordHasher::new();
        frame_le_into(&mut hash, REASON_DOMAIN.as_bytes());
        for (label, term) in [
            (b"cause".as_slice(), Some(self.cause)),
            (b"concern".as_slice(), Some(self.concern)),
            (b"constraint".as_slice(), self.constraint),
            (b"vault".as_slice(), self.vault),
            (b"watermark".as_slice(), self.watermark),
        ] {
            frame_le_into(&mut hash, label);
            frame_le_into(&mut hash, &[u8::from(term.is_some())]);
            if let Some(term) = term {
                absorb_term(&mut hash, term, memory)?;
            }
        }
        Ok(ReasonId(Digest32::new(*hash.finalize().as_bytes())))
    }
}

struct IdentitySink<F> {
    emit: F,
    error: Option<StorageError>,
}

impl<F: FnMut(&[u8]) -> Result<(), StorageError>> fmt::Write for IdentitySink<F> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.error.is_some() {
            return Err(fmt::Error);
        }
        (self.emit)(text.as_bytes()).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}

impl<F: FnMut(&[u8]) -> Result<(), StorageError>> TextOut for IdentitySink<F> {
    fn push_str(&mut self, text: &str) {
        let _ = self.write_str(text);
    }

    fn push(&mut self, ch: char) {
        let _ = self.write_char(ch);
    }

    fn failed(&self) -> bool {
        self.error.is_some()
    }
}

fn absorb_term<S: Admission + ?Sized>(
    hash: &mut RecordHasher,
    term: &TermValue,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let mut len = 0usize;
    let mut count = IdentitySink {
        emit: |bytes: &[u8]| {
            len = len
                .checked_add(bytes.len())
                .ok_or(StorageError::SizeOverflow)?;
            Ok(())
        },
        error: None,
    };
    write_term_value_with_memory(term, &mut count, memory)?;
    if let Some(error) = count.error {
        return Err(error);
    }
    frame_length_into(hash, len);
    let mut stream = IdentitySink {
        emit: |bytes: &[u8]| {
            hash.update(bytes);
            Ok(())
        },
        error: None,
    };
    write_term_value_with_memory(term, &mut stream, memory)?;
    stream.error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::{REASON_DOMAIN, ReasonDescription, ReasonError};
    use crate::{BlankScope, TermBox, TermValue, write_term_value};
    use purrdf_hash::{blake3, frame::frame_le, hex::Digest32};
    use purrdf_lex::allocation::{Memory, Resident};

    fn concern() -> TermValue {
        TermValue::Triple {
            s: TermBox::new(TermValue::iri("http://example.org/constraint")),
            p: TermBox::new(TermValue::iri("http://example.org/field")),
            o: TermBox::new(TermValue::simple_literal("time\nplace")),
        }
    }

    #[test]
    fn repeated_missing_constraint_has_one_content_identity_without_retained_scratch() {
        let cause = TermValue::iri("http://example.org/missing-value");
        let concern = concern();
        let description = ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: None,
            watermark: None,
        };
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let first = description.identity(&mut memory).unwrap();
        for _ in 0..2279 {
            assert_eq!(description.identity(&mut memory).unwrap(), first);
            assert_eq!(memory.admitted_bytes(), 0);
        }
    }

    #[test]
    fn streamed_identity_matches_independent_materialized_framing_and_preserves_blank_scope() {
        let cause = TermValue::iri("http://example.org/missing-value");
        let concern = concern();
        let vault = TermValue::Blank {
            label: "private label".to_owned(),
            scope: BlankScope(7),
        };
        let watermark = TermValue::simple_literal("\0watermark");
        let description = ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: Some(&vault),
            watermark: Some(&watermark),
        };
        let mut preimage = Vec::new();
        frame_le(&mut preimage, REASON_DOMAIN.as_bytes());
        for (label, term) in [
            (b"cause".as_slice(), Some(&cause)),
            (b"concern".as_slice(), Some(&concern)),
            (b"constraint".as_slice(), None),
            (b"vault".as_slice(), Some(&vault)),
            (b"watermark".as_slice(), Some(&watermark)),
        ] {
            frame_le(&mut preimage, label);
            frame_le(&mut preimage, &[u8::from(term.is_some())]);
            if let Some(term) = term {
                let mut spelling = String::new();
                write_term_value(term, &mut spelling);
                frame_le(&mut preimage, spelling.as_bytes());
            }
        }
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let actual = description.identity(&mut memory).unwrap();
        assert_eq!(
            actual.digest(),
            Digest32::new(*blake3::hash(&preimage).as_bytes())
        );
        let other_vault = TermValue::Blank {
            label: "private label".to_owned(),
            scope: BlankScope(8),
        };
        let other = ReasonDescription {
            vault: Some(&other_vault),
            ..description
        };
        assert_ne!(actual, other.identity(&mut memory).unwrap());
        let withheld = ReasonDescription {
            watermark: None,
            ..description
        };
        assert_ne!(actual, withheld.identity(&mut memory).unwrap());
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn invalid_reification_or_cause_is_a_hard_declaration_error() {
        let cause = TermValue::iri("http://example.org/cause");
        let plain = TermValue::simple_literal("not a triple term");
        let concern = concern();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let invalid = ReasonDescription {
            cause: &cause,
            concern: &plain,
            constraint: None,
            vault: None,
            watermark: None,
        };
        assert_eq!(
            invalid.identity(&mut memory),
            Err(ReasonError::ConcernMustBeTriple)
        );
        let invalid = ReasonDescription {
            cause: &plain,
            concern: &concern,
            ..invalid
        };
        assert_eq!(
            invalid.identity(&mut memory),
            Err(ReasonError::CauseMustBeIri)
        );
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
