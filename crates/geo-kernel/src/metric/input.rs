// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Numeric radius arguments use the shared XSD promotion and exact IEEE homes.

use crate::context::WorkProgress;
use crate::{GeoError, Metres, MetricContext, MetricWorkObserver, Rat, XsdDoubleMetres};
use purrdf_core::TermValue;
use purrdf_xsd::XsdDatatype;
use purrdf_xsd::ieee::dyadic::Binary64Dyadic;
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};

/// Read a finite numeric radius under SPARQL's double promotion, then apply its
/// explicitly declared positive linear unit factor without intermediate rounding.
/// # Errors
/// Refuses wrong numeric types/lexicals, nonfinite values and operational limits.
pub fn numeric_radius(
    value: &TermValue,
    metres_per_unit: &Rat,
    context: &mut MetricContext,
) -> Result<Metres, GeoError> {
    radius(
        value,
        metres_per_unit,
        context,
        &mut WorkProgress::new(None),
    )
}

/// Read the identical numeric radius with bounded external work/cancellation.
/// # Errors
/// Preserves argument, numerical admission and observer refusals.
pub fn numeric_radius_metered(
    value: &TermValue,
    metres_per_unit: &Rat,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Metres, GeoError> {
    radius(
        value,
        metres_per_unit,
        context,
        &mut WorkProgress::new(Some(observer)),
    )
}

fn radius(
    value: &TermValue,
    metres_per_unit: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Metres, GeoError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        ..
    } = value
    else {
        return Err(GeoError::literal(
            "a buffer radius must be a numeric literal",
        ));
    };
    let datatype = XsdDatatype::from_iri(datatype)
        .filter(|datatype| datatype.is_numeric())
        .ok_or_else(|| GeoError::literal("a buffer radius must have a numeric XSD datatype"))?;
    // The shared exact-decimal preflight also dominates the fixed-width XSD
    // numeric scanner and conversion; reject long input before its first scan.
    let cost =
        ExactArithmeticCost::decimal(lexical_form.len(), 0).ok_or(GeoError::WorkExhausted {
            limit: context.policy().limits().max_work_items,
        })?;
    let promoted = progress.exact(context, cost, || {
        let numeric = purrdf_xsd::parse(lexical_form, datatype)
            .map_err(|error| GeoError::literal(error.to_string()))?;
        XsdDoubleMetres::from_xsd(&numeric)
    })?;
    let decoded =
        Binary64Dyadic::decode(promoted.reported()).expect("validated finite numeric radius");
    let numerator_bits = u64::from(64 - decoded.significand().leading_zeros())
        + u64::from(decoded.exponent().max(0).unsigned_abs());
    let denominator_bits = 1 + u64::from(decoded.exponent().min(0).unsigned_abs());
    let cost = ExactArithmeticCost::for_rational_operands(
        ExactOperation::RationalReduce,
        [(numerator_bits, denominator_bits)],
        1,
    )
    .ok_or(GeoError::WorkExhausted {
        limit: context.policy().limits().max_work_items,
    })?;
    let exact = progress.exact(context, cost, || {
        Ok(Rat::from_binary64(promoted.reported()).expect("validated finite numeric radius"))
    })?;
    super::unit::to_metres(&exact, metres_per_unit, context, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ExecutionPolicy, GeographicReference, Int};

    #[test]
    fn radius_promotes_once_then_applies_the_exact_declared_unit() {
        let mut context = MetricContext::wgs84().unwrap();
        let integer = TermValue::typed_literal("2", purrdf_xsd::datatype::XSD_INTEGER);
        assert_eq!(
            numeric_radius(&integer, &Rat::from_i64(1_000), &mut context)
                .unwrap()
                .exact(),
            &Rat::from_i64(2_000)
        );
        let decimal = TermValue::typed_literal("0.1", purrdf_xsd::datatype::XSD_DECIMAL);
        let expected = Rat::new(
            Int::from_i64(10_808_639_105_689_191),
            Int::from_i64(36_028_797_018_963_968),
        )
        .unwrap();
        assert_eq!(
            numeric_radius(&decimal, &Rat::from_i64(3), &mut context)
                .unwrap()
                .exact(),
            &expected
        );
        let negative = TermValue::typed_literal("-2", purrdf_xsd::datatype::XSD_INTEGER);
        assert_eq!(
            numeric_radius(&negative, &Rat::one(), &mut context)
                .unwrap()
                .exact(),
            &Rat::from_i64(-2)
        );
    }

    #[test]
    fn invalid_or_large_numeric_sources_refuse_before_unbounded_scanning() {
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(ExecutionLimits {
                max_work_items: 1_024,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        )
        .unwrap();
        let long = TermValue::typed_literal("1".repeat(32_768), purrdf_xsd::datatype::XSD_INTEGER);
        assert!(matches!(
            numeric_radius(&long, &Rat::one(), &mut context),
            Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(context.work_items(), 0);
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes
        );
        let mut context = MetricContext::wgs84().unwrap();
        for lexical in ["NaN", "INF", "-INF"] {
            let value = TermValue::typed_literal(lexical, purrdf_xsd::datatype::XSD_DOUBLE);
            assert!(matches!(
                numeric_radius(&value, &Rat::one(), &mut context),
                Err(GeoError::NonFiniteThreshold { .. })
            ));
        }
        let string = TermValue::typed_literal("2", purrdf_xsd::datatype::XSD_STRING);
        assert!(matches!(
            numeric_radius(&string, &Rat::one(), &mut context),
            Err(GeoError::Literal(_))
        ));
    }
}
