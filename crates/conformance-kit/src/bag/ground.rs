// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admitted ground keys shared by exact and value-normalized result comparison.

use std::borrow::Cow;
use std::cmp::Ordering;

use purrdf_core::{CanonError, CanonicalizationControl, RdfTextDirection, StopSignal, TermValue};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use purrdf_lex::walk::{WorkList, try_sort_unstable_by};

use crate::GradeError;

/// Borrowed coordinates of one complete result.
#[derive(Clone, Copy, Debug)]
pub struct SolutionRows<'a> {
    /// Original result-variable vector.
    pub variables: &'a [String],
    /// Original rows, including unbound cells and empty rows.
    pub rows: &'a [Vec<Option<TermValue>>],
}

/// The caller's established header contract.
#[derive(Clone, Copy, Debug)]
pub enum HeaderLaw {
    /// The same unique names, independently of column order.
    ByName,
    /// The identical variable vector, including its order.
    IdenticalVector,
}

/// The caller's established row cardinality and ordering contract.
#[derive(Clone, Copy, Debug)]
pub enum RowLaw {
    /// Row positions are observable.
    Ordered,
    /// Row order is irrelevant; every multiplicity is observable.
    Multiset,
    /// Every expected distinct row occurs at least once, at most its expected count.
    Reduced,
}

/// Literal identity remains explicit at the shared comparison boundary.
#[derive(Clone, Copy, Debug)]
pub enum LiteralLaw {
    /// Preserve lexical identity, datatype, language identity and base direction.
    Exact,
    /// Apply the caller's one same-datatype numeric normalizer.
    ByValue,
}

/// The complete law selected by this caller, with no implicit semantic default.
#[derive(Clone, Copy, Debug)]
pub struct ComparisonLaw {
    /// Header correspondence.
    pub headers: HeaderLaw,
    /// Row correspondence.
    pub rows: RowLaw,
    /// Literal correspondence.
    pub literals: LiteralLaw,
}

/// Actual ground preparation and comparison observations, separate from RDFC work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComparisonWork {
    /// Complete row keys built, once per input row.
    pub row_keys: usize,
    /// Structural key parts retained across those rows.
    pub key_parts: usize,
    /// Actual row comparator invocations, including sort and run counting.
    pub row_comparisons: u64,
}

/// A completed ground screening or comparison; no canonical identity is invented.
#[derive(Clone, Copy, Debug)]
pub struct GroundComparison {
    /// Equality, or `None` when a global blank-node bijection is required.
    pub matches: Option<bool>,
    /// Work actually performed before that result.
    pub work: ComparisonWork,
}

fn poll(stop: Option<&dyn StopSignal>) -> Result<(), GradeError> {
    if purrdf_core::governor::is_stopped(stop) {
        Err(CanonError::Stopped.into())
    } else {
        Ok(())
    }
}

/// Reject malformed result shape through the same door used before trace encoding.
///
/// # Errors
/// Returns an explicit duplicate-header, row-width or physical refusal.
pub fn validate_solution_shape(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
) -> Result<(), GradeError> {
    shape_columns(
        SolutionRows { variables, rows },
        None,
        &mut Memory::new(&mut Resident),
    )?;
    Ok(())
}

fn shape_columns<S: Admission + ?Sized>(
    input: SolutionRows<'_>,
    stop: Option<&dyn StopSignal>,
    memory: &mut Memory<'_, S>,
) -> Result<Vec<usize>, GradeError> {
    poll(stop)?;
    let mut columns = memory.collect(0..input.variables.len())?;
    try_sort_unstable_by(&mut columns, |left, right| {
        poll(stop)?;
        Ok::<_, GradeError>(input.variables[*left].cmp(&input.variables[*right]))
    })?;
    for pair in columns.windows(2) {
        poll(stop)?;
        if input.variables[pair[0]] == input.variables[pair[1]] {
            return Err(GradeError::Malformed(memory.format_with_progress(
                &format_args!("duplicate result variable {:?}", input.variables[pair[0]]),
            )?));
        }
    }
    for (index, row) in input.rows.iter().enumerate() {
        poll(stop)?;
        if row.len() != input.variables.len() {
            return Err(GradeError::Malformed(memory.format_with_progress(
                &format_args!(
                    "row {index} has {} cells for {} variables",
                    row.len(),
                    input.variables.len(),
                ),
            )?));
        }
    }
    Ok(columns)
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Part<'a> {
    Unbound,
    Iri(&'a str),
    Literal {
        datatype: &'a str,
        lexical: Cow<'a, str>,
        language: Option<Cow<'a, str>>,
        direction: Option<RdfTextDirection>,
    },
    Triple,
}

fn row_keys<'a, S: Admission + ?Sized>(
    input: SolutionRows<'a>,
    columns: &[usize],
    law: LiteralLaw,
    normalize: &mut impl FnMut(&str, &str, &mut Memory<'_, S>) -> Result<Option<String>, CanonError>,
    stop: Option<&dyn StopSignal>,
    memory: &mut Memory<'_, S>,
    work: &mut ComparisonWork,
) -> Result<Vec<Vec<Part<'a>>>, GradeError> {
    let mut keys = Vec::new();
    memory.reserve(&mut keys, input.rows.len())?;
    for row in input.rows {
        poll(stop)?;
        let mut key = Vec::new();
        memory.reserve(&mut key, columns.len())?;
        for &column in columns {
            let Some(value) = &row[column] else {
                memory.push(&mut key, Part::Unbound)?;
                continue;
            };
            let mut pending = WorkList::<&TermValue, 16>::new();
            let result = (|| {
                let mut next = Some(value);
                while let Some(value) = next.take().or_else(|| pending.pop()) {
                    poll(stop)?;
                    let part = match value {
                        TermValue::Iri(iri) => Part::Iri(iri),
                        TermValue::Literal {
                            lexical_form,
                            datatype,
                            language,
                            direction,
                        } => {
                            let lexical = match law {
                                LiteralLaw::Exact => Cow::Borrowed(lexical_form.as_str()),
                                LiteralLaw::ByValue => normalize(lexical_form, datatype, memory)?
                                    .map_or(Cow::Borrowed(lexical_form.as_str()), Cow::Owned),
                            };
                            let language = language
                                .as_deref()
                                .map(|language| {
                                    if purrdf_iri::langtag::is_identity_folded(language) {
                                        Ok(Cow::Borrowed(language))
                                    } else {
                                        purrdf_iri::langtag::identity_fold_with_memory(
                                            language, memory,
                                        )
                                        .map(Cow::Owned)
                                    }
                                })
                                .transpose()?;
                            Part::Literal {
                                datatype,
                                lexical,
                                language,
                                direction: *direction,
                            }
                        }
                        TermValue::Triple { s, p, o } => {
                            pending.try_push_admitted(&**o, memory)?;
                            pending.try_push_admitted(&**p, memory)?;
                            next = Some(&**s);
                            Part::Triple
                        }
                        TermValue::Blank { .. } => {
                            return Err(GradeError::Malformed(memory.string(
                                "ground key reached a blank after complete blank screening",
                            )?));
                        }
                    };
                    memory.push(&mut key, part)?;
                }
                Ok(())
            })();
            let cleanup = pending.release_admitted(memory).map_err(GradeError::from);
            result.and(cleanup)?;
        }
        work.key_parts = work
            .key_parts
            .checked_add(key.len())
            .ok_or(StorageError::SizeOverflow)?;
        memory.push(&mut keys, key)?;
    }
    work.row_keys = work
        .row_keys
        .checked_add(keys.len())
        .ok_or(StorageError::SizeOverflow)?;
    Ok(keys)
}

fn compare_rows(
    left: &[Part<'_>],
    right: &[Part<'_>],
    stop: Option<&dyn StopSignal>,
    work: &mut ComparisonWork,
) -> Result<Ordering, GradeError> {
    poll(stop)?;
    work.row_comparisons = work
        .row_comparisons
        .checked_add(1)
        .ok_or(StorageError::SizeOverflow)?;
    for (left, right) in left.iter().zip(right) {
        poll(stop)?;
        let order = left.cmp(right);
        if !order.is_eq() {
            return Ok(order);
        }
    }
    Ok(left.len().cmp(&right.len()))
}

fn run_end(
    keys: &[Vec<Part<'_>>],
    order: &[usize],
    start: usize,
    stop: Option<&dyn StopSignal>,
    work: &mut ComparisonWork,
) -> Result<usize, GradeError> {
    let mut end = start + 1;
    while end < order.len()
        && compare_rows(&keys[order[start]], &keys[order[end]], stop, work)?.is_eq()
    {
        end += 1;
    }
    Ok(end)
}

fn ground_body<S: Admission + ?Sized>(
    left: SolutionRows<'_>,
    right: SolutionRows<'_>,
    law: ComparisonLaw,
    normalize: &mut impl FnMut(&str, &str, &mut Memory<'_, S>) -> Result<Option<String>, CanonError>,
    stop: Option<&dyn StopSignal>,
    memory: &mut Memory<'_, S>,
) -> Result<GroundComparison, GradeError> {
    use purrdf_core::ir::canon::screen_comparison_terms_with_memory;
    let left_blanks = screen_comparison_terms_with_memory(
        left.rows.iter().flatten().filter_map(Option::as_ref),
        stop,
        memory,
    )?;
    let right_blanks = screen_comparison_terms_with_memory(
        right.rows.iter().flatten().filter_map(Option::as_ref),
        stop,
        memory,
    )?;
    let left_columns = shape_columns(left, stop, memory)?;
    let right_columns = shape_columns(right, stop, memory)?;
    let mut work = ComparisonWork::default();
    if left.variables.len() != right.variables.len() {
        return Ok(GroundComparison {
            matches: Some(false),
            work,
        });
    }
    for (column, (&left_column, &right_column)) in
        left_columns.iter().zip(&right_columns).enumerate()
    {
        poll(stop)?;
        let (left_name, right_name) = match law.headers {
            HeaderLaw::ByName => (&left.variables[left_column], &right.variables[right_column]),
            HeaderLaw::IdenticalVector => (&left.variables[column], &right.variables[column]),
        };
        if left_name != right_name {
            return Ok(GroundComparison {
                matches: Some(false),
                work,
            });
        }
    }
    if left_blanks || right_blanks {
        return Ok(GroundComparison {
            matches: None,
            work,
        });
    }
    let left_keys = row_keys(
        left,
        &left_columns,
        law.literals,
        normalize,
        stop,
        memory,
        &mut work,
    )?;
    let right_keys = row_keys(
        right,
        &right_columns,
        law.literals,
        normalize,
        stop,
        memory,
        &mut work,
    )?;
    let equal = match law.rows {
        RowLaw::Ordered => {
            let mut same = left_keys.len() == right_keys.len();
            for (left, right) in left_keys.iter().zip(&right_keys) {
                same &= compare_rows(left, right, stop, &mut work)?.is_eq();
            }
            same
        }
        RowLaw::Multiset | RowLaw::Reduced => {
            let mut left_order = memory.collect(0..left_keys.len())?;
            let mut right_order = memory.collect(0..right_keys.len())?;
            try_sort_unstable_by(&mut left_order, |a, b| {
                compare_rows(&left_keys[*a], &left_keys[*b], stop, &mut work)
            })?;
            try_sort_unstable_by(&mut right_order, |a, b| {
                compare_rows(&right_keys[*a], &right_keys[*b], stop, &mut work)
            })?;
            let (mut a, mut b) = (0, 0);
            let mut same = true;
            while a < left_order.len() && b < right_order.len() {
                if !compare_rows(
                    &left_keys[left_order[a]],
                    &right_keys[right_order[b]],
                    stop,
                    &mut work,
                )?
                .is_eq()
                {
                    same = false;
                    break;
                }
                let a_end = run_end(&left_keys, &left_order, a, stop, &mut work)?;
                let b_end = run_end(&right_keys, &right_order, b, stop, &mut work)?;
                let actual = a_end - a;
                let expected = b_end - b;
                same &= match law.rows {
                    RowLaw::Reduced => actual <= expected,
                    _ => actual == expected,
                };
                a = a_end;
                b = b_end;
            }
            same && a == left_order.len() && b == right_order.len()
        }
    };
    Ok(GroundComparison {
        matches: Some(equal),
        work,
    })
}

/// Screen both complete inputs, then compare ground values under an explicit law.
///
/// The normalizer returns an originally admitted canonical string only for a
/// modeled numeric value; `None` preserves nonnumeric or ill-typed lexical identity.
/// Its native storage/control refusal must remain an error. Left input is actual
/// for REDUCED, and right input carries the permitted expected multiplicities.
/// Scratch dies before refund; returned diagnostic payload keeps its original account.
///
/// # Errors
/// Returns malformed shape or the original native domain, stop or storage refusal.
pub fn compare_ground_with_memory<S: Admission + ?Sized>(
    left: SolutionRows<'_>,
    right: SolutionRows<'_>,
    law: ComparisonLaw,
    normalize: &mut impl FnMut(&str, &str, &mut Memory<'_, S>) -> Result<Option<String>, CanonError>,
    memory: &mut Memory<'_, S>,
    control: &mut CanonicalizationControl<'_>,
) -> Result<GroundComparison, GradeError> {
    purrdf_core::ir::canon::comparison_preparation(control, |stop| {
        let baseline = memory.admitted_bytes();
        let result = ground_body(left, right, law, normalize, stop, memory);
        let retained = match &result {
            Err(
                GradeError::Malformed(text)
                | GradeError::Mismatch(text)
                | GradeError::ProjectionLimit(text),
            ) => Some(text.capacity()),
            Err(GradeError::Canonicalization(CanonError::Invalid(cause))) => cause.retained_bytes(),
            Err(GradeError::Canonicalization(CanonError::ReservedVocabulary(cause))) => {
                cause.retained_bytes()
            }
            _ => Some(0),
        };
        let Some(released) = retained
            .and_then(|bytes| baseline.checked_add(bytes))
            .and_then(|live| memory.admitted_bytes().checked_sub(live))
        else {
            drop(result);
            if let Some(bytes) = memory.admitted_bytes().checked_sub(baseline) {
                let _ = memory.release_bytes(bytes);
            }
            return Err(GradeError::Storage(StorageError::SizeOverflow));
        };
        let cleanup = memory.release_bytes(released);
        if result.is_ok() {
            cleanup?;
        }
        result
    })
}
