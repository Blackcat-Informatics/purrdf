// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact numeric facet bounds, beside the public schema AST.
//!
//! [`NumericLiteral`] holds a `MININCLUSIVE`/`MINEXCLUSIVE`/`MAXINCLUSIVE`/
//! `MAXEXCLUSIVE` bound as an `i64` or an `f64`, the ShExJ wire encoding. A bound
//! written as an `INTEGER` or `DECIMAL` that neither holds — `MAXINCLUSIVE
//! 1000000000000000000000000000000000000000001`, or a decimal with more digits
//! than a double carries — loses digits there, and a value one past the bound
//! would then compare equal to it.
//!
//! [`parse_shexc_exact`] and [`parse_shexj_exact`] return the same [`Schema`] the
//! plain parsers do, together with an [`ExactFacets`] holding the exact value of
//! every such bound, and [`validate_exact`] compares each value node against those
//! exact bounds. A bound the `i64`/`f64` already holds exactly needs no entry; a
//! bound written as a `DOUBLE` (`1.5E3`) is a double and keeps its binary value.

use purrdf_core::{RdfDataset, TermValue};
use purrdf_xsd::XsdValue;
use purrdf_xsd::exact::Decimal;

use crate::ast::{NumericLiteral, Schema};
use crate::error::{Result, ShexError};
use crate::validate::{ResultShapeMap, ShapeSelector, ValidationOptions};

/// The identity of a lossy bound: its `i64`, or its `f64`'s bit pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    Integer(i64),
    Fractional(u64),
}

impl Key {
    fn of(bound: NumericLiteral) -> Self {
        match bound {
            NumericLiteral::Integer(value) => Self::Integer(value),
            NumericLiteral::Fractional(value) => Self::Fractional(value.to_bits()),
        }
    }
}

/// The exact values of the numeric facet bounds of one parsed schema whose
/// [`NumericLiteral`] does not hold them exactly, keyed by that `NumericLiteral`.
///
/// Built by [`parse_shexc_exact`] / [`parse_shexj_exact`] and read by
/// [`validate_exact`]; empty when every bound of the schema is exact already.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExactFacets {
    bounds: Vec<(Key, Decimal)>,
    /// A key two different exact bounds share (they round to one `i64`/`f64`).
    conflict: Option<String>,
}

impl ExactFacets {
    /// The exact value of `bound` when it lost digits as a [`NumericLiteral`].
    #[must_use]
    pub fn bound(&self, bound: NumericLiteral) -> Option<&Decimal> {
        let key = Key::of(bound);
        self.bounds
            .iter()
            .find(|(entry, _)| *entry == key)
            .map(|(_, exact)| exact)
    }

    /// How many bounds carry an exact value here.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bounds.len()
    }

    /// Whether every bound of the schema is exact as a [`NumericLiteral`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    /// Add every bound of `other` (an imported schema's) to these.
    ///
    /// # Errors
    ///
    /// Two different exact bounds that share one `i64`/`f64`, as
    /// [`parse_shexc_exact`] refuses within one schema.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        for (key, exact) in &other.bounds {
            self.insert(*key, exact.clone());
        }
        if let Some(message) = &other.conflict {
            self.conflict.get_or_insert_with(|| message.clone());
        }
        self.clone().finish().map(|_| ())
    }

    /// Record the bound `lossy`, read from `lexical` written in an exact
    /// (`INTEGER`/`DECIMAL`) syntax.
    pub(crate) fn record(&mut self, lossy: NumericLiteral, lexical: &str) {
        let Ok(exact) = lexical.parse::<Decimal>() else {
            return;
        };
        let held = match lossy {
            NumericLiteral::Integer(value) => Decimal::from(i128::from(value)),
            NumericLiteral::Fractional(value) => match Decimal::from_f64(value) {
                Ok(held) => held,
                Err(_) => return,
            },
        };
        if held == exact {
            return;
        }
        self.insert(Key::of(lossy), exact);
    }

    /// Record `exact` under `key`, noting a conflict with a different recorded value.
    fn insert(&mut self, key: Key, exact: Decimal) {
        match self.bounds.iter().find(|(entry, _)| *entry == key) {
            Some((_, recorded)) if *recorded == exact => {}
            Some((_, recorded)) => {
                if self.conflict.is_none() {
                    self.conflict = Some(format!(
                        "the numeric facet bounds {recorded} and {exact} share one binary \
                         approximation, so their exact values cannot both be kept"
                    ));
                }
            }
            None => self.bounds.push((key, exact)),
        }
    }

    /// The facet bound as an XSD value: its exact value when one is recorded,
    /// the `NumericLiteral` itself otherwise.
    pub(crate) fn value_of(this: Option<&Self>, bound: NumericLiteral) -> XsdValue {
        if let Some(exact) = this.and_then(|facets| facets.bound(bound)) {
            return XsdValue::from_exact_decimal(exact.clone());
        }
        match bound {
            NumericLiteral::Integer(value) => XsdValue::Integer {
                value: i128::from(value),
                datatype: purrdf_xsd::XsdDatatype::Integer,
            },
            NumericLiteral::Fractional(value) => XsdValue::Double(value),
        }
    }

    /// Refuse a schema two of whose distinct bounds share a key.
    fn finish(self) -> Result<Self> {
        match self.conflict {
            Some(message) => Err(ShexError::shexj(message)),
            None => Ok(self),
        }
    }
}

/// [`crate::parse_shexc`], also returning the exact value of every numeric
/// facet bound its [`NumericLiteral`] does not hold.
///
/// # Errors
///
/// Every error [`crate::parse_shexc`] raises; and a schema with two different
/// `INTEGER`/`DECIMAL` bounds that round to one `i64`/`f64` (their exact values
/// cannot both be keyed by it), which [`crate::parse_shexc`] parses lossily.
///
/// ```
/// use purrdf_shex::exact_facets::parse_shexc_exact;
///
/// let (_, facets) = parse_shexc_exact(
///     "<http://example.org/S> { <http://example.org/n> \
///      MAXINCLUSIVE 1000000000000000000000000000000000000000001 }",
///     None,
/// )?;
/// assert_eq!(facets.len(), 1);
/// # Ok::<(), purrdf_shex::ShexError>(())
/// ```
pub fn parse_shexc_exact(input: &str, base: Option<&str>) -> Result<(Schema, ExactFacets)> {
    let (schema, facets) = crate::parser::parse_shexc_recording(input, base)?;
    Ok((schema, facets.finish()?))
}

/// [`crate::parse_shexj`], also returning the exact value of every numeric
/// facet bound written as a JSON number without an exponent that its
/// [`NumericLiteral`] does not hold.
///
/// # Errors
///
/// As [`parse_shexc_exact`], for the ShExJ document.
pub fn parse_shexj_exact(input: &str, base: Option<&str>) -> Result<(Schema, ExactFacets)> {
    let (schema, facets) = crate::shexj::parse_shexj_recording(input, base)?;
    Ok((schema, facets.finish()?))
}

/// [`crate::validate_with`], comparing every value node against the exact
/// numeric facet bounds in `facets` — the ones parsed with `schema` by
/// [`parse_shexc_exact`] or [`parse_shexj_exact`].
#[must_use]
pub fn validate_exact(
    schema: &Schema,
    facets: &ExactFacets,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
) -> ResultShapeMap {
    crate::validate::validate_with_facets(schema, data, map, options, Some(facets))
}
