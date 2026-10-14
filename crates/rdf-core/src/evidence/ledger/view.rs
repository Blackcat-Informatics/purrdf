// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ledger is a borrowed, lazy native dataset, queried by the same engine.

use super::{BorrowedLedgerTerms, LedgerError, LedgerStatement, LedgerTermId};
use crate::dataset_view::lock_read_state;
use crate::{DatasetView, GraphMatch, QuadIds, TermRef, TermValue, WorkspaceReservation};
use purrdf_lex::allocation::{Admission, StorageError};
use std::sync::Mutex;

/// An immutable original answer rendering. Each enumeration must yield the same
/// validated statements exactly once, including shared dictionary descriptions
/// only once. As with DatasetView's statement partition, the producer owns this
/// invariant. Rendering borrows original row/reason/frame owners, never a second
/// resident graph. No named graph is introduced.
pub trait LedgerSource<'a>: core::fmt::Debug {
    /// Enumerate original validated statements without minting term payloads.
    fn statements(&self) -> impl Iterator<Item = Result<LedgerStatement<'a>, LedgerError>>;
}

impl<'a, L: LedgerSource<'a> + ?Sized> LedgerSource<'a> for &L {
    fn statements(&self) -> impl Iterator<Item = Result<LedgerStatement<'a>, LedgerError>> {
        (**self).statements()
    }
}

/// Typed invalid ledger declaration, foreign id or native operational refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerReadError {
    /// An original rendering declaration is malformed.
    Declaration(LedgerError),
    /// An id does not belong to this immutable dictionary.
    ForeignId,
    /// The source changed after its dictionary was established.
    SourceChanged,
    /// Original native layout, allocator or admission refusal.
    Storage(StorageError),
}

impl core::fmt::Display for LedgerReadError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Declaration(error) => error.fmt(out),
            Self::ForeignId => out.write_str("foreign ledger dictionary id"),
            Self::SourceChanged => {
                out.write_str("ledger source changed after dictionary admission")
            }
            Self::Storage(error) => error.fmt(out),
        }
    }
}
impl std::error::Error for LedgerReadError {}

/// A query-local ledger view retaining its original physical index admission.
/// Quads are rendered on demand; only borrowed term descriptors are indexed.
#[derive(Debug)]
pub struct LedgerDataset<'a, S: Admission, L: LedgerSource<'a>> {
    source: L,
    terms: BorrowedLedgerTerms<'a, S>,
    error: Mutex<Option<LedgerReadError>>,
}

impl<'a, S: Admission, L: LedgerSource<'a>> LedgerDataset<'a, S, L> {
    /// Admit the original answer's term dictionary before exposing the view.
    ///
    /// # Errors
    /// Refuses malformed declarations or original native physical allocation.
    pub fn new(source: L, admission: S) -> Result<Self, LedgerReadError> {
        for statement in source.statements() {
            let statement = statement.map_err(LedgerReadError::Declaration)?;
            super::reifier(statement.subject).map_err(LedgerReadError::Declaration)?;
            if !purrdf_iri::is_absolute(statement.predicate).is_ok_and(|absolute| absolute) {
                return Err(LedgerReadError::Declaration(LedgerError::InvalidPredicate));
            }
        }
        let terms = BorrowedLedgerTerms::try_new(
            source.statements().flat_map(|statement| match statement {
                Ok(statement) => [Ok(statement.subject), Ok(statement.object)],
                Err(error) => [Err(error), Err(error)],
            }),
            source
                .statements()
                .map(|statement| statement.map(|statement| statement.predicate)),
            admission,
        )
        .map_err(|error| match error {
            super::terms::TermBuildError::Storage(error) => LedgerReadError::Storage(error),
            super::terms::TermBuildError::Source(error) => LedgerReadError::Declaration(error),
        })?;
        Ok(Self {
            source,
            terms,
            error: Mutex::new(None),
        })
    }

    fn fail<T>(&self, error: LedgerReadError) -> Result<T, LedgerReadError> {
        let mut status = lock_read_state(&self.error);
        let first = status.unwrap_or(error);
        *status = Some(first);
        drop(status);
        Err(first)
    }

    fn statement_ids(
        &self,
        statement: LedgerStatement<'a>,
    ) -> Result<QuadIds<LedgerTermId>, LedgerReadError> {
        let Some(s) = self.term_id_by_value(statement.subject)? else {
            return self.fail(LedgerReadError::SourceChanged);
        };
        let predicate = self.terms.iri(statement.predicate);
        let Some(p) = predicate else {
            return self.fail(LedgerReadError::SourceChanged);
        };
        let Some(o) = self.term_id_by_value(statement.object)? else {
            return self.fail(LedgerReadError::SourceChanged);
        };
        Ok(QuadIds { s, p, o, g: None })
    }
}

impl<'a, S: Admission, L: LedgerSource<'a>> DatasetView for LedgerDataset<'a, S, L> {
    type Id = LedgerTermId;
    type ReadError = LedgerReadError;
    type TermGuard<'b>
        = TermRef<'b, LedgerTermId>
    where
        Self: 'b;
    type ProbePlan = ();

    fn read_error(&self) -> Option<Self::ReadError> {
        *lock_read_state(&self.error)
    }

    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError> {
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        let mut reservation = LedgerReservation {
            terms: &self.terms,
            error: &self.error,
            bytes: 0,
        };
        reservation.resize(bytes)?;
        Ok(reservation)
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.source.statements().map_while(|statement| {
            if lock_read_state(&self.error).is_some() {
                return None;
            }
            match statement {
                Ok(statement) => self.statement_ids(statement).ok(),
                Err(error) => self
                    .fail::<QuadIds<Self::Id>>(LedgerReadError::Declaration(error))
                    .ok(),
            }
        })
    }

    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        let term = self.terms.resolve(id);
        match term {
            Some(term) => Ok(term),
            None => self.fail(LedgerReadError::ForeignId),
        }
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        let result = self.terms.lookup(value);
        match result {
            Ok(id) => Ok(id),
            Err(error) => self.fail(LedgerReadError::Storage(error)),
        }
    }

    crate::fixed_store_capabilities!(quoted_triples);

    fn term_count(&self) -> u64 {
        self.terms.len() as u64
    }

    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch<Self::Id>) -> Self::ProbePlan {}

    fn quads_for_pattern_with_plan(
        &self,
        (): &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }
}

struct LedgerReservation<'v, 'a, S: Admission> {
    terms: &'v BorrowedLedgerTerms<'a, S>,
    error: &'v Mutex<Option<LedgerReadError>>,
    bytes: usize,
}

impl<S: Admission> WorkspaceReservation for LedgerReservation<'_, '_, S> {
    type Error = LedgerReadError;

    fn resize(&mut self, bytes: u64) -> Result<(), Self::Error> {
        let error = *lock_read_state(self.error);
        if let Some(error) = error {
            return Err(error);
        }
        let result = usize::try_from(bytes)
            .map_err(|_| StorageError::SizeOverflow)
            .and_then(|bytes| {
                self.terms.resize_workspace(self.bytes, bytes)?;
                self.bytes = bytes;
                Ok(())
            });
        result.map_err(|error| {
            let error = LedgerReadError::Storage(error);
            let mut status = lock_read_state(self.error);
            let first = status.unwrap_or(error);
            *status = Some(first);
            drop(status);
            first
        })
    }
}

impl<S: Admission> Drop for LedgerReservation<'_, '_, S> {
    fn drop(&mut self) {
        // The caller destroys its payload before dropping this original grant.
        let result = self.terms.resize_workspace(self.bytes, 0);
        if let Err(error) = result {
            let mut status = lock_read_state(self.error);
            if status.is_none() {
                *status = Some(LedgerReadError::Storage(error));
            }
        }
    }
}

impl<'a, S: Admission, L: LedgerSource<'a>> crate::FallibleDatasetView for LedgerDataset<'a, S, L> {
    type Error = LedgerReadError;
    type Evidence = usize;

    fn operation_status(&self) -> crate::ViewOperationStatus<Self::Error, Self::Evidence> {
        let status = lock_read_state(&self.error);
        let evidence = self.terms.admitted_bytes();
        match *status {
            Some(error) => crate::ViewOperationStatus::Failed { error, evidence },
            None => crate::ViewOperationStatus::Ready { evidence },
        }
    }
}
