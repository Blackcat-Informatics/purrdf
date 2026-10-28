// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conservative value intervals for annotation candidate scans. Exact FILTER
//! evaluation remains the consumer's responsibility, including type errors.

use std::cmp::Ordering;

use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_xsd::{XsdDatatype, XsdValue};

use crate::{DatasetView, QuadIds, RdfDataset, TermGuard, TermId, TermRef};

/// Original caller admission for transient range parsing and numeric comparison.
/// The caller retains and checks its first typed failure after consuming a range
/// iterator. Backends may conservatively omit pruning when no owner is supplied.
pub trait AnnotationRangeWorkspace: std::fmt::Debug + Sync {
    /// Resize the live range scratch allocation before native allocation.
    /// # Errors
    /// Returns physical refusal while retaining the original caller error.
    fn resize(&self, bytes: usize) -> Result<(), StorageError>;
    /// Admit the native transient working set against the caller's compute meter.
    /// False records a governed stop, distinct from physical source failure.
    fn admit_transient(&self, bytes: usize) -> bool;
    /// Whether the original compute governor has stopped this range scan.
    fn stopped(&self) -> bool;
    /// Retain the first physical failure, including actual allocator refusal.
    fn record_failure(&self, error: StorageError);
}

pub(crate) struct RangeMemory<'a>(pub(crate) &'a dyn AnnotationRangeWorkspace);
impl Admission for RangeMemory<'_> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.0.resize(bytes)
    }
}
impl Drop for RangeMemory<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.0.resize(0) {
            self.0.record_failure(error);
        }
    }
}

struct ObservedMemory<'a, 'm, S: Admission + ?Sized> {
    memory: &'a mut Memory<'m, S>,
    workspace: Option<&'a dyn AnnotationRangeWorkspace>,
    live: usize,
}
impl<S: Admission + ?Sized> Admission for ObservedMemory<'_, '_, S> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.live
            && self
                .workspace
                .is_some_and(|owner| !owner.admit_transient(bytes))
        {
            return Err(StorageError::AdmissionFailed);
        }
        if bytes >= self.live {
            self.memory.add_bytes(bytes - self.live)?;
        } else {
            self.memory.release_bytes(self.live - bytes)?;
        }
        self.live = bytes;
        Ok(())
    }
}
impl<S: Admission + ?Sized> Drop for ObservedMemory<'_, '_, S> {
    fn drop(&mut self) {
        if let Err(error) = self.memory.release_bytes(self.live)
            && let Some(owner) = self.workspace
        {
            owner.record_failure(error);
        }
    }
}

/// One value-space endpoint, borrowed from the admitted query expression.
#[derive(Debug, Clone, Copy)]
pub struct AnnotationValueBound<'a> {
    /// Exact parsed endpoint; no lexical rounding or dictionary ordering applies.
    pub value: &'a XsdValue,
    /// Whether equality to this endpoint satisfies the bound.
    pub inclusive: bool,
}

/// A conjunctive interval used only to remove impossible storage candidates.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnnotationValueRange<'a> {
    /// Original caller-owned scratch admission. A paged resident carrier only
    /// prunes when this owner is present; no unmetered parser is substituted.
    pub workspace: Option<&'a dyn AnnotationRangeWorkspace>,
    /// Lower endpoint, or no lower restriction.
    pub lower: Option<AnnotationValueBound<'a>>,
    /// Upper endpoint, or no upper restriction.
    pub upper: Option<AnnotationValueBound<'a>>,
}

impl AnnotationValueRange<'_> {
    /// Whether a certified interval can contain an answer. Incomparable bounds
    /// cannot justify pruning. Inputs belong to one homogeneous total-order group.
    pub(crate) fn may_overlap<S: Admission + ?Sized>(
        self,
        minimum: &XsdValue,
        maximum: &XsdValue,
        memory: &mut Memory<'_, S>,
    ) -> Result<bool, StorageError> {
        if let Some(lower) = self.lower {
            let order = compare(maximum, lower.value, memory)?;
            if order == Some(Ordering::Less) || (!lower.inclusive && order == Some(Ordering::Equal))
            {
                return Ok(false);
            }
        }
        if let Some(upper) = self.upper {
            let order = compare(minimum, upper.value, memory)?;
            if order == Some(Ordering::Greater)
                || (!upper.inclusive && order == Some(Ordering::Equal))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn compare<S: Admission + ?Sized>(
    left: &XsdValue,
    right: &XsdValue,
    memory: &mut Memory<'_, S>,
) -> Result<Option<Ordering>, StorageError> {
    let mut peak = 0;
    let answer = purrdf_xsd::ops::value_cmp_admitted(left, right, &mut |layout| {
        let needed = layout.required_bytes();
        if needed > peak {
            memory
                .add_bytes(needed - peak)
                .map_err(purrdf_xsd::bigint::LimbScratchError::Storage)?;
            peak = needed;
        }
        Ok(())
    });
    memory.release_bytes(peak)?;
    answer.map_err(|error| match error {
        purrdf_xsd::bigint::LimbScratchError::Storage(error) => error,
        purrdf_xsd::bigint::LimbScratchError::SizeOverflow => StorageError::SizeOverflow,
        purrdf_xsd::bigint::LimbScratchError::AllocationFailed => StorageError::AllocationFailed,
        _ => StorageError::AdmissionFailed,
    })
}

/// Endpoints name original dictionary values; None means an unorderable row is
/// present for the predicate and forbids pruning that predicate's unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValueInterval<I> {
    pub(crate) predicate: I,
    pub(crate) endpoints: Option<(I, I)>,
}

pub(crate) enum RangeReadError<E> {
    Read(E),
    Storage(StorageError),
}

fn read_value<D: DatasetView, S: Admission + ?Sized>(
    view: &D,
    id: D::Id,
    memory: &mut Memory<'_, S>,
) -> Result<Option<XsdValue>, RangeReadError<D::ReadError>> {
    let guard = view.resolve(id).map_err(RangeReadError::Read)?;
    let TermRef::Literal {
        lexical,
        datatype,
        language: None,
        direction: None,
    } = guard.term()
    else {
        return Ok(None);
    };
    let datatype_guard = view.resolve(datatype).map_err(RangeReadError::Read)?;
    let TermRef::Iri(iri) = datatype_guard.term() else {
        return Ok(None);
    };
    let Some(datatype) = XsdDatatype::from_iri(iri) else {
        return Ok(None);
    };
    Ok(
        match purrdf_xsd::value::try_parse_with_memory(lexical, datatype, false, memory)
            .map_err(RangeReadError::Storage)?
        {
            purrdf_xsd::value::ParsedValue::Value(value) => Some(value),
            purrdf_xsd::value::ParsedValue::Invalid(_) => None,
        },
    )
}

pub(crate) fn interval_may_match<D: DatasetView, S: Admission + ?Sized>(
    view: &D,
    endpoints: Option<(D::Id, D::Id)>,
    range: AnnotationValueRange<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<bool, RangeReadError<D::ReadError>> {
    if range
        .workspace
        .is_some_and(AnnotationRangeWorkspace::stopped)
    {
        return Ok(false);
    }
    let Some((low, high)) = endpoints else {
        return Ok(true);
    };
    let mut observed = ObservedMemory {
        memory,
        workspace: range.workspace,
        live: 0,
    };
    let mut memory = Memory::new(&mut observed);
    let Some(low) = read_value(view, low, &mut memory)? else {
        return Ok(true);
    };
    let Some(high) = read_value(view, high, &mut memory)? else {
        return Ok(true);
    };
    range
        .may_overlap(&low, &high, &mut memory)
        .map_err(RangeReadError::Storage)
}

/// Derive intervals only from actual resident rows during sealing/certification.
/// Groups never mix numeric promotion strata or zoned/unzoned temporal values.
pub(crate) fn seal_intervals(
    dataset: &RdfDataset,
    rows: impl Iterator<Item = QuadIds<TermId>>,
) -> Result<Vec<ValueInterval<TermId>>, StorageError> {
    seal_intervals_with(rows, |id| Ok(dataset.resolve(id)))
}

pub(crate) fn seal_intervals_with<'a, I: crate::ViewTermId + 'a, E: From<StorageError>>(
    rows: impl Iterator<Item = QuadIds<I>>,
    resolve: impl Fn(I) -> Result<TermRef<'a, I>, E>,
) -> Result<Vec<ValueInterval<I>>, E> {
    let mut resident = purrdf_lex::allocation::Resident;
    let mut memory = Memory::new(&mut resident);
    let mut values: Vec<(I, &'static str, bool, I, XsdValue)> = Vec::new();
    let mut unknown = Vec::new();
    for row in rows {
        let TermRef::Literal {
            lexical,
            datatype,
            language: None,
            direction: None,
        } = resolve(row.o)?
        else {
            memory.push(&mut unknown, row.p)?;
            continue;
        };
        let TermRef::Iri(iri) = resolve(datatype)? else {
            memory.push(&mut unknown, row.p)?;
            continue;
        };
        let Some(datatype) = XsdDatatype::from_iri(iri) else {
            memory.push(&mut unknown, row.p)?;
            continue;
        };
        let purrdf_xsd::value::ParsedValue::Value(value) =
            purrdf_xsd::value::try_parse_with_memory(lexical, datatype, false, &mut memory)?
        else {
            memory.push(&mut unknown, row.p)?;
            continue;
        };
        let total = matches!(
            &value,
            XsdValue::Integer { .. }
                | XsdValue::Decimal(_)
                | XsdValue::Boolean(_)
                | XsdValue::String(_)
                | XsdValue::DateTime(_)
                | XsdValue::Date(_)
                | XsdValue::Time(_)
        ) || matches!(&value, XsdValue::Float(number) if !number.is_nan())
            || matches!(&value, XsdValue::Double(number) if !number.is_nan());
        if !total {
            memory.push(&mut unknown, row.p)?;
            continue;
        }
        let bytes = lexical.as_bytes();
        let zoned = lexical.ends_with('Z')
            || (bytes.len() >= 6
                && matches!(bytes[bytes.len() - 6], b'+' | b'-')
                && bytes[bytes.len() - 3] == b':');
        memory.push(&mut values, (row.p, datatype.iri(), zoned, row.o, value))?;
    }
    values.sort_unstable_by_key(|value| (value.0, value.1, value.2));
    let mut intervals = Vec::new();
    let mut start = 0;
    while start < values.len() {
        let key = (values[start].0, values[start].1, values[start].2);
        let end =
            start + values[start..].partition_point(|value| (value.0, value.1, value.2) == key);
        let (mut minimum, mut maximum) = (start, start);
        for index in start + 1..end {
            if compare(&values[index].4, &values[minimum].4, &mut memory)? == Some(Ordering::Less) {
                minimum = index;
            }
            if compare(&values[index].4, &values[maximum].4, &mut memory)?
                == Some(Ordering::Greater)
            {
                maximum = index;
            }
        }
        // Separate extremes from the dense interior, without a comparison sort
        // that could conceal a fallible numeric admission inside its comparator.
        let mut middle: Option<(usize, usize)> = None;
        for index in start..end {
            if index == minimum || index == maximum {
                continue;
            }
            match &mut middle {
                None => middle = Some((index, index)),
                Some((low, high)) => {
                    if compare(&values[index].4, &values[*low].4, &mut memory)?
                        == Some(Ordering::Less)
                    {
                        *low = index;
                    }
                    if compare(&values[index].4, &values[*high].4, &mut memory)?
                        == Some(Ordering::Greater)
                    {
                        *high = index;
                    }
                }
            }
        }
        for (low, high) in [Some((minimum, minimum)), middle, Some((maximum, maximum))]
            .into_iter()
            .flatten()
        {
            memory.push(
                &mut intervals,
                ValueInterval {
                    predicate: key.0,
                    endpoints: Some((values[low].3, values[high].3)),
                },
            )?;
        }
        start = end;
    }
    unknown.sort_unstable();
    unknown.dedup();
    for predicate in unknown {
        memory.push(
            &mut intervals,
            ValueInterval {
                predicate,
                endpoints: None,
            },
        )?;
    }
    intervals.sort_unstable_by_key(|interval| (interval.predicate, interval.endpoints));
    intervals.dedup();
    Ok(intervals)
}
