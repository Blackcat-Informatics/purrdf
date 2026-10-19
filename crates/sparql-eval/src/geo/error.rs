// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one conversion from shared geometry refusals to evaluator failures.

use crate::EvalError;
use purrdf_geo_kernel::GeoError;

/// The original geographic owner with the evaluator's existing presentation.
struct GeographicFailure(GeoError);
purrdf_hash::debug_non_exhaustive!(GeographicFailure {});

impl purrdf_hash::error::StructuredError for GeographicFailure {
    purrdf_hash::error_visit! {
        @tuple self (_error) => "geo-evaluator-failure" { "cause" => Cause(&self.0) }
    }
}
impl core::fmt::Display for GeographicFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0.original() {
            GeoError::SourceRead(cause) | GeoError::Arity(cause) => {
                core::fmt::Display::fmt(cause, f)
            }
            GeoError::Config(message) => f.write_str(message),
            GeoError::NoRelationFamilies => f.write_str(&self.0.detail()),
            GeoError::MissingNativeObserver { .. } => {
                f.write_str("native entry requires its owned observer")
            }
            GeoError::IndexConfiguration(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::IndexBinding(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::SourcePreparation(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::ContextConfiguration(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::TransverseInput(cause) if !cause.is_domain_error() => {
                core::fmt::Display::fmt(&**cause, f)
            }
            GeoError::AffineInput(cause) if !cause.is_domain_error() => {
                core::fmt::Display::fmt(&**cause, f)
            }
            GeoError::ProjectionInput(cause) if !cause.is_domain_error() => {
                core::fmt::Display::fmt(&**cause, f)
            }
            GeoError::PolynomialInput(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::ModelInput(cause) if !cause.is_domain_error() => {
                core::fmt::Display::fmt(&**cause, f)
            }
            GeoError::MetricUnit(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::MatrixConfiguration(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::BindingConfiguration(cause) => core::fmt::Display::fmt(&**cause, f),
            GeoError::Vocabulary(cause) if !cause.is_expression_error() => {
                core::fmt::Display::fmt(&**cause, f)
            }
            GeoError::CarrierInput(cause) if cause.is_configuration() => {
                core::fmt::Display::fmt(&**cause, f)
            }
            GeoError::CrsConfiguration(cause) => core::fmt::Display::fmt(&**cause, f),
            other => core::fmt::Display::fmt(other, f),
        }
    }
}
impl std::error::Error for GeographicFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// Move the original cause and owner; no intermediate human message is made.
pub(super) fn native_error(error: GeoError, workspace: &crate::WorkspaceCapability) -> EvalError {
    convert(
        error,
        workspace,
        crate::NativeDiagnosticKind::FunctionOperational,
    )
}

fn convert(
    error: GeoError,
    workspace: &crate::WorkspaceCapability,
    function: crate::NativeDiagnosticKind,
) -> EvalError {
    use crate::NativeDiagnosticKind;
    let original = error.original();
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    if let GeoError::Storage(error) = original {
        return frame.storage_error(*error, "native geographic producer");
    }
    if let GeoError::NumericalScratch(purrdf_xsd::integer::LimbScratchError::Storage(error)) =
        original
    {
        return frame.storage_error(*error, "native geographic numerical storage");
    }
    let kind = match original {
        GeoError::Vocabulary(cause) if !cause.is_expression_error() => NativeDiagnosticKind::Config,
        GeoError::CarrierInput(cause) => {
            if cause.is_configuration() {
                NativeDiagnosticKind::Config
            } else {
                NativeDiagnosticKind::Data
            }
        }
        GeoError::SourceRead(_) => NativeDiagnosticKind::SourceRead,
        GeoError::Config(_)
        | GeoError::NoRelationFamilies
        | GeoError::CrsConfiguration(_)
        | GeoError::IndexConfiguration(_)
        | GeoError::IndexBinding(_)
        | GeoError::SourcePreparation(_)
        | GeoError::ContextConfiguration(_)
        | GeoError::PolynomialInput(_)
        | GeoError::MetricUnit(_)
        | GeoError::MatrixConfiguration(_)
        | GeoError::BindingConfiguration(_)
        | GeoError::UnregisteredCrs(_)
        | GeoError::MissingOperation { .. }
        | GeoError::InvalidEllipsoid(_)
        | GeoError::InvalidExecutionPolicy(_)
        | GeoError::InvalidCoverPolicy(_)
        | GeoError::InvalidPointIndexPolicy(_)
        | GeoError::MissingNativeObserver { .. }
        | GeoError::InvalidNativeStorage(_) => NativeDiagnosticKind::Config,
        GeoError::TransverseInput(cause) if !cause.is_domain_error() => {
            NativeDiagnosticKind::Config
        }
        GeoError::AffineInput(cause) if !cause.is_domain_error() => NativeDiagnosticKind::Config,
        GeoError::ProjectionInput(cause) if !cause.is_domain_error() => {
            NativeDiagnosticKind::Config
        }
        GeoError::ModelInput(cause) if !cause.is_domain_error() => NativeDiagnosticKind::Config,
        GeoError::Literal(_)
        | GeoError::ArgumentInput(_)
        | GeoError::NumericLiteral(_)
        | GeoError::IndexSerialization(_)
        | GeoError::GeometryShape(_)
        | GeoError::GeoJsonShape(_)
        | GeoError::PatternSyntax(_)
        | GeoError::CrsLiteral { .. }
        | GeoError::WktSyntax(_)
        | GeoError::JsonSyntax(_) => NativeDiagnosticKind::Data,
        _ => function,
    };
    crate::RetainedNativeFailure::error(kind, GeographicFailure(error), workspace)
}

impl From<GeoError> for EvalError {
    /// The single site that decides which evaluator label a geo failure wears.
    ///
    /// Non-storage refusals become [`EvalError::RetainedNativeFailure`], retaining
    /// the original [`GeoError`] in the geographic adapter. The diagnostic kind
    /// records configuration, malformed data, source-read or function failure;
    /// an already-retained function refusal uses `FunctionOperational`.
    ///
    /// Native storage refusals, including integer scratch storage, keep the
    /// evaluator's original storage failure. Presentation is separate from the
    /// retained cause and does not replace it with a rendered message.
    fn from(err: GeoError) -> Self {
        let function = if matches!(&err, GeoError::Retained(_)) {
            crate::NativeDiagnosticKind::FunctionOperational
        } else {
            crate::NativeDiagnosticKind::Function
        };
        convert(err, &crate::WorkspaceCapability::resident(), function)
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
        for per_solution in [
            GeoError::literal("x"),
            GeoError::domain("x"),
            purrdf_geo_kernel::Pattern::parse("FFFFFFFF?").unwrap_err(),
        ] {
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
            EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Function
        ));
    }

    #[test]
    fn actual_geographic_arity_failure_retains_declared_and_supplied_counts() {
        use crate::geo::functions::GeofFunction;
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let error = GeofFunction::Distance.check_argument_count(1).unwrap_err();
        let fields = Capture::read(&error)
            .child("cause")
            .child("cause")
            .child("cause");
        assert_eq!(fields.code, "geo-function-arity");
        assert!(matches!(
            fields.field("function"),
            ErrorValue::Code("distance")
        ));
        assert!(matches!(fields.field("actual"), ErrorValue::Unsigned(1)));
        let expected = fields.child("expected");
        assert_eq!(expected.code, "function-arity-exact");
        assert!(matches!(expected.field("count"), ErrorValue::Unsigned(3)));
        assert_eq!(
            error.to_string(),
            "wrong argument count: geof:distance expects exactly 3 argument(s), got 1"
        );
        let error = EvalError::from(error);
        assert!(
            matches!(&error, EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Function)
        );
        let fields = Capture::read(&error)
            .child("cause")
            .child("cause")
            .child("cause")
            .child("cause")
            .child("cause")
            .child("cause");
        assert!(matches!(fields.field("actual"), ErrorValue::Unsigned(1)));
    }

    #[test]
    fn source_read_failure_remains_fatal_and_preserves_its_detail() {
        let converted = EvalError::from(GeoError::source_read("storage checksum mismatch"));
        assert!(
            matches!(&converted, EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::SourceRead)
        );
        assert!(converted.to_string().contains("storage checksum mismatch"));
    }

    #[test]
    fn native_source_transport_never_formats_the_original_cause() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::{CallerRefusal, Capture};

        let original = GeoError::native_source(CallerRefusal { limit: u128::MAX });
        for error in [
            EvalError::from(original.clone()),
            super::native_error(original, &crate::WorkspaceCapability::resident()),
        ] {
            assert!(error.is_native_storage_failure());
            let cloned = error.clone();
            drop(error);
            let fields = Capture::read(&cloned)
                .child("cause")
                .child("cause")
                .child("cause")
                .child("cause")
                .child("cause")
                .child("cause");
            assert_eq!(fields.code, "caller-refused");
            assert!(matches!(
                fields.field("limit"),
                ErrorValue::Unsigned(u128::MAX)
            ));
        }
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
            EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Config
        ));
        assert!(matches!(
            EvalError::from(GeoError::literal("x")),
            EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Data
        ));
        let index = purrdf_geo_kernel::index::GeoIndexConfig::new(
            Vec::new(),
            purrdf_geo_kernel::index::GraphSelector::Any,
        )
        .unwrap_err();
        let matrix = purrdf_geo_kernel::IntersectionMatrix::parse("FFFFFFFF").unwrap_err();
        for error in [index, matrix] {
            assert!(
                matches!(EvalError::from(error), EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Config)
            );
        }
        let pattern = purrdf_geo_kernel::Pattern::parse("FFFFFFFF?").unwrap_err();
        assert!(
            matches!(EvalError::from(pattern), EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Data)
        );
    }

    /// An unimplemented operation must reach the caller as a hard failure, never
    /// as a default answer: a `false` from an unimplemented predicate is
    /// indistinguishable from an honest `false`.
    #[test]
    fn conversion_maps_unsupported_and_domain_to_function() {
        assert!(matches!(
            EvalError::from(GeoError::unsupported("geof:transform")),
            EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Function
        ));
        assert!(matches!(
            EvalError::from(GeoError::domain("mixed CRS")),
            EvalError::RetainedNativeFailure(cause) if cause.kind() == crate::NativeDiagnosticKind::Function
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
