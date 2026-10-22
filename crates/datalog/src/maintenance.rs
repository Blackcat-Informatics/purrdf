// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native signed transactions supplied by the host's committed change stream.
//!
//! A batch retains its caller-owned watermark and original memory admission. Its
//! terms are IDs in the receiving store's dictionary, never a second lexical
//! interner. Consolidation is exact before narrowing to signed 64-bit weights:
//! cancellation cannot fail merely because an intermediate sum exceeded `i64`.
//! These transactions describe input changes; constructing one does not advance
//! a maintained relation's applied watermark or certify a new least model.

use core::fmt;

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::dictionary::TermIdentity;
use crate::id::TermId;
use crate::store::{Arity, StoreShape};

/// A signed transaction could not be admitted or exactly consolidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeError {
    /// The final multiplicity of one original row does not fit `i64`.
    MultiplicityOutOfRange {
        /// Exact consolidated multiplicity, before narrowing.
        weight: i128,
    },
    /// Original layout, admission or physical allocation refusal.
    Storage(StorageError),
}

impl purrdf_hash::error::StructuredError for ChangeError {
    purrdf_hash::error_visit! {
        MultiplicityOutOfRange { weight } => "datalog-change-multiplicity" { "weight" => Signed(*weight) },
        Storage(cause) => "datalog-change-storage" { "cause" => Cause(cause) },
    }
}

impl fmt::Display for ChangeError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MultiplicityOutOfRange { weight } => {
                write!(
                    out,
                    "consolidated input multiplicity {weight} does not fit i64"
                )
            }
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for ChangeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::MultiplicityOutOfRange { .. } => None,
        }
    }
}

purrdf_lex::variant_from!(ChangeError { Storage(StorageError) });

/// One committed host transaction, with duplicate rows consolidated and zeros removed.
///
/// Rows are grouped by their original runtime IDs for lookup, not presentation.
/// Neither IDs nor that grouping order define a persistent content identity.
/// A supersede is an ordinary batch containing a negative and a positive row.
/// An empty consolidated batch still retains its host watermark.
///
/// `S` owns the caller's grant and releases it on destruction, after the rows and
/// watermark have died. The batch cannot clone or detach that original owner.
#[derive(Debug)]
pub struct ChangeBatch<M, S: Admission, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    rows: Vec<([I; N], i64)>,
    watermark: M,
    admitted_bytes: usize,
    admission: S,
}

impl<M, S: Admission, I: TermIdentity, const N: usize> ChangeBatch<M, S, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Admit the original transaction and consolidate its complete signed input.
    ///
    /// All buffers are constructed through the supplied admission. Overflow or
    /// refusal publishes no batch and changes neither the source nor a model.
    /// Input iterator and watermark payloads retain their host-owned allocation.
    ///
    /// # Errors
    /// Returns an exact final weight overflow or the original storage refusal.
    pub fn new(
        watermark: M,
        changes: impl IntoIterator<Item = ([I; N], i64)>,
        admission: S,
    ) -> Result<Self, ChangeError> {
        // Establish payload-before-grant destruction even if construction fails.
        let mut batch = Self {
            rows: Vec::new(),
            watermark,
            admitted_bytes: 0,
            admission,
        };
        let mut memory = Memory::new(&mut batch.admission);
        batch.rows = memory.try_scope(|memory| {
            let mut rows = memory.collect(changes)?;
            rows.sort_unstable_by_key(|(row, _)| *row);
            let mut read = 0;
            let mut write = 0;
            while read < rows.len() {
                let row = rows[read].0;
                let mut total = 0_i128;
                while read < rows.len() && rows[read].0 == row {
                    // A resident Vec holds at most isize::MAX elements, each
                    // contributing an i64, so its exact sum fits i128 on every
                    // supported 32/64-bit target, independently of input order.
                    total += i128::from(rows[read].1);
                    read += 1;
                }
                let weight = i64::try_from(total)
                    .map_err(|_| ChangeError::MultiplicityOutOfRange { weight: total })?;
                if weight != 0 {
                    rows[write] = (row, weight);
                    write += 1;
                }
            }
            rows.truncate(write);
            Ok::<_, ChangeError>(rows)
        })?;
        batch.admitted_bytes = memory.admitted_bytes();
        Ok(batch)
    }

    /// Original host position, including for an empty transaction.
    pub const fn watermark(&self) -> &M {
        &self.watermark
    }

    /// Unique nonzero native input changes, grouped in runtime lookup order.
    pub fn changes(&self) -> &[([I; N], i64)] {
        &self.rows
    }

    /// Original requested buffer layout retained by this batch, including spare capacity.
    pub const fn admitted_bytes(&self) -> usize {
        self.admitted_bytes
    }
}

/// The host supplies committed transactions in its own watermark order.
///
/// The consumer must finish an update before reporting its watermark as applied.
/// `None` means no committed batch is presently available, not that a relation
/// has reached an unspecified latest snapshot. Source failures remain typed.
pub trait ChangeSource<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// Original dictionary identity type used by every yielded transaction.
    type Id: TermIdentity;
    /// The host's opaque committed position; no numerical ordering is fabricated.
    type Watermark;
    /// Original owned memory admission retained by each transaction.
    type Admission: Admission;
    /// Original host read or transaction-construction failure.
    type Error;

    /// Yield the next committed transaction without acknowledging its application.
    ///
    /// # Errors
    /// Preserves the source's original error; it is never treated as an empty stream.
    fn next_change(&mut self) -> ChangePoll<Self, N>;
}

/// Applying a host stream failed at its original source or model boundary.
#[derive(Debug)]
pub enum ChangeApplyError<D, C> {
    /// Original host polling failure; no batch or acknowledgement was fabricated.
    Source(C),
    /// Original model refusal, including a transaction already in progress.
    Model(crate::seminaive::MaintenanceError<D>),
}

impl<D: fmt::Display, C: fmt::Display> fmt::Display for ChangeApplyError<D, C> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(out),
            Self::Model(error) => error.fmt(out),
        }
    }
}

impl<D: std::error::Error + 'static, C: std::error::Error + 'static> std::error::Error
    for ChangeApplyError<D, C>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Model(error) => Some(error),
        }
    }
}

impl<D: purrdf_hash::error::StructuredError, C: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for ChangeApplyError<D, C>
{
    purrdf_hash::error_visit! {
        Source(cause) => "datalog-change-source" { "cause" => Cause(cause) },
        Model(cause) => "datalog-change-model" { "cause" => Cause(cause) },
    }
}

/// One poll of a host source, retaining its native batch and original error type.
pub type ChangePoll<T, const N: usize = 4> = Result<
    Option<
        ChangeBatch<
            <T as ChangeSource<N>>::Watermark,
            <T as ChangeSource<N>>::Admission,
            <T as ChangeSource<N>>::Id,
            N,
        >,
    >,
    <T as ChangeSource<N>>::Error,
>;

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn signed_transactions_consolidate_before_narrowing_in_every_order() {
        let row = [u128::MAX, 7, 3];
        let other = [u128::MAX - 1, 7, 4];
        // Each i64-boundary cancellation must work regardless of sort tie order.
        let input = [
            (row, i64::MAX),
            (row, 1),
            (row, -1),
            (other, i64::MIN),
            (other, -1),
            (other, 1),
        ];
        for shift in 0..input.len() {
            for reversed in [false, true] {
                let changes = (0..input.len()).map(|index| {
                    let position = if reversed {
                        input.len() - 1 - index
                    } else {
                        index
                    };
                    input[(position + shift) % input.len()]
                });
                let batch = ChangeBatch::new("commit-original", changes, Resident).unwrap();
                assert_eq!(batch.watermark(), &"commit-original");
                assert_eq!(batch.changes(), &[(other, i64::MIN), (row, i64::MAX)]);
                assert!(batch.admitted_bytes() >= size_of_val(&input));
            }
        }
        for (changes, expected) in [
            ([(row, i64::MAX), (row, 1)], i128::from(i64::MAX) + 1),
            ([(row, i64::MIN), (row, -1)], i128::from(i64::MIN) - 1),
        ] {
            assert!(matches!(
                ChangeBatch::new((), changes, Resident),
                Err(ChangeError::MultiplicityOutOfRange { weight }) if weight == expected
            ));
        }
    }

    #[test]
    fn cancelling_quads_keep_the_watermark_and_original_grant() {
        struct Grant {
            live: Rc<Cell<usize>>,
            dropped: Rc<Cell<bool>>,
            watermark_dropped: Rc<Cell<bool>>,
            reject: bool,
        }
        impl Admission for Grant {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if self.reject && bytes != 0 {
                    return Err(StorageError::AdmissionFailed);
                }
                self.live.set(bytes);
                Ok(())
            }
        }
        impl Drop for Grant {
            fn drop(&mut self) {
                assert!(
                    self.watermark_dropped.get(),
                    "the original owner must outlive its watermark"
                );
                self.live.set(0);
                self.dropped.set(true);
            }
        }
        struct Watermark {
            position: u64,
            grant_dropped: Rc<Cell<bool>>,
            dropped: Rc<Cell<bool>>,
        }
        impl Drop for Watermark {
            fn drop(&mut self) {
                assert!(
                    !self.grant_dropped.get(),
                    "the original grant is still alive"
                );
                self.dropped.set(true);
            }
        }
        let live = Rc::new(Cell::new(0));
        let dropped = Rc::new(Cell::new(false));
        let watermark_dropped = Rc::new(Cell::new(false));
        let batch = ChangeBatch::new(
            Watermark {
                position: 42,
                grant_dropped: Rc::clone(&dropped),
                dropped: Rc::clone(&watermark_dropped),
            },
            [([1_u64, 2, 3, 4], 9), ([1, 2, 3, 4], -9)],
            Grant {
                live: Rc::clone(&live),
                dropped: Rc::clone(&dropped),
                watermark_dropped: Rc::clone(&watermark_dropped),
                reject: false,
            },
        )
        .unwrap();
        assert_eq!(batch.changes(), []);
        assert_eq!(batch.watermark().position, 42);
        assert_eq!(live.get(), batch.admitted_bytes());
        assert_ne!(
            live.get(),
            0,
            "the input buffer remains admitted after consolidation"
        );
        assert!(!dropped.get());
        drop(batch);
        assert!(watermark_dropped.get());
        assert!(dropped.get());
        assert_eq!(live.get(), 0);
        for reject in [false, true] {
            dropped.set(false);
            watermark_dropped.set(false);
            let result = ChangeBatch::new(
                Watermark {
                    position: 43,
                    grant_dropped: Rc::clone(&dropped),
                    dropped: Rc::clone(&watermark_dropped),
                },
                [([1_u64, 2, 3, 4], i64::MAX), ([1, 2, 3, 4], 1)],
                Grant {
                    live: Rc::clone(&live),
                    dropped: Rc::clone(&dropped),
                    watermark_dropped: Rc::clone(&watermark_dropped),
                    reject,
                },
            );
            assert!(if reject {
                matches!(
                    result,
                    Err(ChangeError::Storage(StorageError::AdmissionFailed))
                )
            } else {
                matches!(result, Err(ChangeError::MultiplicityOutOfRange { .. }))
            });
            assert!(watermark_dropped.get());
            assert!(dropped.get());
            assert_eq!(live.get(), 0);
        }
    }

    #[test]
    fn input_admission_refuses_before_consuming_a_known_extent() {
        struct Refuse;
        impl Admission for Refuse {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes == 0 {
                    Ok(())
                } else {
                    Err(StorageError::AdmissionFailed)
                }
            }
        }
        let visited = Cell::new(0);
        let changes =
            std::iter::once(([1_u64, 2, 3], 1)).inspect(|_| visited.set(visited.get() + 1));
        assert!(matches!(
            ChangeBatch::new((), changes, Refuse),
            Err(ChangeError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(visited.get(), 0);
        let empty = ChangeBatch::<_, _, u64, 3>::new((), [], Refuse).unwrap();
        assert_eq!(empty.changes(), []);
        assert_eq!(empty.admitted_bytes(), 0);
    }
}
