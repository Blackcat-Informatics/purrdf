// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one conversion from shared geometry refusals to evaluator failures.

use crate::EvalError;
use purrdf_geo_kernel::GeoError;

impl From<GeoError> for EvalError {
    /// The single site that decides which evaluator label a geo failure wears.
    ///
    /// * `Arity` becomes [`EvalError::function`]: the call could not be invoked as
    ///   written, which is the same thing the evaluator's own pre-dispatch arity
    ///   check reports through that label.
    /// * `Config` keeps its name: it is a statement about the host's wiring, which
    ///   is exactly what [`EvalError::config`] means.
    /// * `Literal` becomes [`EvalError::data`]: a geometry literal is *dataset
    ///   content*, and a malformed one is bad data rather than a bad query or a
    ///   bad registration — even when it arrives as a constant written in the
    ///   query text, because the same lexical form would have been equally
    ///   malformed had it come from a triple.
    /// * `Unsupported` and `Domain` become [`EvalError::function`], which is the
    ///   evaluator's label for "the callee could not be invoked as written". The
    ///   message is re-prefixed because `EvalError::function` supplies its own
    ///   framing rather than this type's `Display`.
    fn from(err: GeoError) -> Self {
        match err {
            GeoError::SourceRead(msg) => Self::SourceRead(msg),
            GeoError::Arity(msg) => Self::function(msg),
            GeoError::Config(msg) => Self::config(msg),
            GeoError::Literal(msg) => Self::data(format!("malformed geometry literal: {msg}")),
            GeoError::Unsupported(msg) => {
                Self::function(format!("unsupported GeoSPARQL operation: {msg}"))
            }
            GeoError::Domain(msg) => Self::function(format!("geometry domain error: {msg}")),
            other @ (GeoError::UnregisteredCrs(_)
            | GeoError::MissingOperation { .. }
            | GeoError::InvalidEllipsoid(_)
            | GeoError::InvalidExecutionPolicy(_)) => Self::config(other.to_string()),
            other => Self::function(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GeoError;
    use crate::EvalError;

    #[test]
    fn each_constructor_builds_its_own_variant() {
        assert!(matches!(GeoError::arity("z"), GeoError::Arity(_)));
        assert!(matches!(GeoError::config("a"), GeoError::Config(_)));
        assert!(matches!(GeoError::literal("b"), GeoError::Literal(_)));
        assert!(matches!(
            GeoError::unsupported("c"),
            GeoError::Unsupported(_)
        ));
        assert!(matches!(GeoError::domain("d"), GeoError::Domain(_)));
    }

    /// The two kinds that are statements about *these arguments* travel one
    /// solution; the three that hold for every solution alike abort the query.
    ///
    /// Both halves are asserted, because each guards a different failure. If a
    /// `Literal` or `Domain` refusal became fatal, one malformed geometry in a
    /// dataset would fail every query that scanned past it. If an `Unsupported`,
    /// `Config` or `Arity` refusal became per-solution, a `FILTER` would drop every
    /// row and the caller would read that as an honest empty answer.
    #[test]
    fn only_the_argument_level_kinds_are_per_solution_expression_errors() {
        for per_solution in [GeoError::literal("x"), GeoError::domain("x")] {
            assert!(
                per_solution.is_expression_error(),
                "{per_solution} is about one call's arguments, so it must not fail the query"
            );
        }
        for fatal in [
            GeoError::arity("x"),
            GeoError::unsupported("x"),
            GeoError::config("x"),
            GeoError::source_read("storage checksum mismatch"),
        ] {
            assert!(
                !fatal.is_expression_error(),
                "{fatal} holds for every solution, so answering 'no value' would silently empty \
                 the result set"
            );
        }
    }

    /// A wrong argument count reaches the evaluator as a function error — the same
    /// label the evaluator's own pre-dispatch arity check uses.
    #[test]
    fn conversion_maps_arity_to_a_function_error() {
        assert!(matches!(
            EvalError::from(GeoError::arity(
                "geof:sfEquals expects exactly 2 argument(s), got 1"
            )),
            EvalError::Function(_)
        ));
    }

    #[test]
    fn source_read_failure_remains_fatal_and_preserves_its_detail() {
        let converted = EvalError::from(GeoError::source_read("storage checksum mismatch"));
        assert!(
            matches!(&converted, EvalError::SourceRead(message) if message == "storage checksum mismatch")
        );
    }

    #[test]
    fn display_carries_the_detail_and_its_own_prefix() {
        for error in [
            GeoError::arity("the detail"),
            GeoError::config("the detail"),
            GeoError::literal("the detail"),
            GeoError::unsupported("the detail"),
            GeoError::domain("the detail"),
        ] {
            let rendered = error.to_string();
            assert!(
                rendered.contains("the detail"),
                "the detail must survive rendering: {rendered}"
            );
            assert_eq!(error.detail(), "the detail", "detail() strips the prefix");
            assert!(
                rendered.len() > "the detail".len(),
                "a prefix names the kind: {rendered}"
            );
        }
    }

    #[test]
    fn conversion_maps_config_to_its_namesake_and_a_literal_to_data() {
        assert!(matches!(
            EvalError::from(GeoError::config("x")),
            EvalError::Config(_)
        ));
        assert!(matches!(
            EvalError::from(GeoError::literal("x")),
            EvalError::Data(_)
        ));
    }

    /// An unimplemented operation must reach the caller as a hard failure, never
    /// as a default answer: a `false` from an unimplemented predicate is
    /// indistinguishable from an honest `false`.
    #[test]
    fn conversion_maps_unsupported_and_domain_to_function() {
        assert!(matches!(
            EvalError::from(GeoError::unsupported("geof:transform")),
            EvalError::Function(_)
        ));
        assert!(matches!(
            EvalError::from(GeoError::domain("mixed CRS")),
            EvalError::Function(_)
        ));
    }

    #[test]
    fn conversion_preserves_the_detail() {
        for error in [
            GeoError::arity("the detail"),
            GeoError::config("the detail"),
            GeoError::literal("the detail"),
            GeoError::unsupported("the detail"),
            GeoError::domain("the detail"),
        ] {
            let rendered = EvalError::from(error).to_string();
            assert!(
                rendered.contains("the detail"),
                "the detail must survive the conversion: {rendered}"
            );
        }
    }
}
