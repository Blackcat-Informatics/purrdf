// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one failure channel this crate has, and the single site that maps it onto
//! the evaluator's.
//!
//! Every refusal in `purrdf-geo` reduces to one of five kinds, and each kind
//! answers a different question about *whose* mistake it was:
//!
//! * [`GeoError::Arity`] — the **call as written** supplies the wrong number of
//!   arguments. Nothing about the data or the wiring is wrong; the query text is.
//!   It is its own kind rather than a flavour of the four below precisely because
//!   it belongs to nobody else: it cannot be repaired by fixing a literal, by
//!   declaring a vocabulary term, or by implementing a function.
//! * [`GeoError::Config`] — the **caller's wiring** is unusable as written: a
//!   vocabulary with an empty IRI, a registration missing the datatype it is
//!   supposed to recognize. Nothing about the data or the query is wrong yet.
//! * [`GeoError::Literal`] — a **geometry literal** is not the thing its datatype
//!   claims. A malformed WKT token, a GeoJSON object with no `type`, a ring that
//!   does not close. This is bad input, and it is refused rather than repaired,
//!   because a repaired geometry answers a question nobody asked.
//! * [`GeoError::Unsupported`] — the function is **spec-defined and genuinely not
//!   implemented here**. This is the one variant that is a statement about
//!   `purrdf-geo` rather than about its inputs, and it exists so that the gap is
//!   *loud*. A `geof:` call this crate cannot answer must abort the query; it must
//!   never return a plausible-looking wrong geometry, and it must never return
//!   `false` — a topological predicate that answers `false` because it was not
//!   implemented is indistinguishable from one that answers `false` because the
//!   geometries genuinely do not relate, and that is the silent-wrong-answer
//!   channel this crate exists to keep closed.
//! * [`GeoError::Domain`] — the arguments are individually well-formed but the
//!   operation is undefined on them: two geometries in different coordinate
//!   reference systems (this crate reprojects nothing, so it refuses rather than
//!   pretending the coordinates are comparable), a measure asked of an empty
//!   geometry, an exponent past the parser's cap.
//!
//! # Why the mapping onto [`EvalError`] lives here and only here
//!
//! A failure that reaches a caller wearing two different labels depending on
//! which call path produced it is a diagnostic that cannot be relied on. So the
//! `From<GeoError> for EvalError` impl below is *the* mapping site: no other
//! module in this crate constructs an [`EvalError`] directly, and every seam
//! (scalar function, relation, cursor) reduces its failure to a [`GeoError`]
//! first.
//!
//! # Two questions, not one
//!
//! *Which label* a failure wears (`From<GeoError> for EvalError`) and *how far it
//! travels* ([`GeoError::is_expression_error`]) are separate decisions, and both are
//! made here. SPARQL 1.1 has two failure distances: a per-solution **expression
//! error**, which a `FILTER` turns into a dropped row and a `BIND` into an unbound
//! variable, and a query-fatal error. Two of the five kinds above are the first
//! ([`GeoError::Literal`], [`GeoError::Domain`] — both statements about *these
//! arguments*) and three are the second ([`GeoError::Arity`],
//! [`GeoError::Unsupported`], [`GeoError::Config`] — all conditions that hold for
//! every solution alike, so answering "no value" would empty a result set and call
//! it an answer). Putting the split in this module keeps it from being re-litigated
//! once per call site, which is exactly how the two would drift.

use purrdf_sparql_eval::EvalError;

/// Why a `purrdf-geo` operation refused.
///
/// See the module docs for what distinguishes the five kinds; the split is by
/// *whose mistake it was*, not by which function raised it, and it is what
/// [`Self::is_expression_error`] reads to decide how far a refusal travels.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GeoError {
    /// The call supplies the wrong number of arguments for the function named.
    Arity(String),
    /// The caller's wiring is unusable as written (an empty IRI, a vocabulary
    /// missing a term a registered function needs).
    Config(String),
    /// A geometry literal is not well-formed for its datatype.
    Literal(String),
    /// A spec-defined operation this crate does not implement. Always loud,
    /// never a default answer.
    Unsupported(String),
    /// Well-formed arguments on which the operation is undefined (mixed
    /// coordinate reference systems, a measure of an empty geometry).
    Domain(String),
}

impl GeoError {
    /// A [`GeoError::Arity`] with `what` as its detail.
    pub fn arity(what: impl Into<String>) -> Self {
        Self::Arity(what.into())
    }

    /// A [`GeoError::Config`] with `what` as its detail.
    pub fn config(what: impl Into<String>) -> Self {
        Self::Config(what.into())
    }

    /// A [`GeoError::Literal`] with `what` as its detail.
    pub fn literal(what: impl Into<String>) -> Self {
        Self::Literal(what.into())
    }

    /// A [`GeoError::Unsupported`] with `what` as its detail.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported(what.into())
    }

    /// A [`GeoError::Domain`] with `what` as its detail.
    pub fn domain(what: impl Into<String>) -> Self {
        Self::Domain(what.into())
    }

    /// Whether this refusal is a SPARQL **expression error** — scoped to the one
    /// solution being evaluated — rather than a condition that must abort the whole
    /// query.
    ///
    /// This is the second half of the decision the `From<GeoError> for EvalError`
    /// impl below makes, and it lives beside it for the same reason: a failure that
    /// aborts a query on one call path and drops a row on another is a behaviour
    /// nobody can rely on. [`crate::functions::evaluate`] is its only consumer, and
    /// it is what makes the `geof:` seam obey SPARQL 1.1 §17.2 rather than treating
    /// one bad row as a fatal query.
    ///
    /// * [`Domain`](Self::Domain) and [`Literal`](Self::Literal) are expression
    ///   errors. Both are statements about *these arguments*: a geometry literal
    ///   whose lexical form its datatype does not license is exactly SPARQL's
    ///   ill-typed literal, and §17.2's "Functions invoked with an argument of the
    ///   wrong type will produce a type error" puts it in the per-solution channel —
    ///   the same channel `"abc"^^xsd:integer > 1` lands in. Making them fatal would
    ///   mean one malformed geometry anywhere in a dataset kills every query that
    ///   scans past it, which is a much larger claim than the data supports.
    /// * [`Arity`](Self::Arity), [`Unsupported`](Self::Unsupported) and
    ///   [`Config`](Self::Config) are NOT. Each holds for *every* solution alike, so
    ///   answering "no value" would empty a result set and present that as the
    ///   answer, reopening the silent-wrong-answer channel this crate exists to keep
    ///   shut. A wrong argument count is a defect in the query text that no row can
    ///   satisfy. An unimplemented function that answered "no
    ///   value" would be dropped by a `FILTER` and left unbound by a `BIND` — in
    ///   both cases indistinguishable from an honest negative result, with nothing
    ///   downstream able to tell the difference. A `Config` refusal names a
    ///   declaration the host never made — an undeclared linear unit, an absent
    ///   Simple Features namespace — and the host is the only party who can make it;
    ///   PurRDF fabricates no vocabulary defaults to fall back on. A row may be what
    ///   *reveals* the gap, but leaving that row unbound would report "this geometry
    ///   has no area" and hide it.
    #[must_use]
    pub const fn is_expression_error(&self) -> bool {
        match self {
            Self::Domain(_) | Self::Literal(_) => true,
            Self::Arity(_) | Self::Unsupported(_) | Self::Config(_) => false,
        }
    }

    /// The detail message, without this error's own prefix.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Arity(msg)
            | Self::Config(msg)
            | Self::Literal(msg)
            | Self::Unsupported(msg)
            | Self::Domain(msg) => msg,
        }
    }
}

impl core::fmt::Display for GeoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Arity(msg) => write!(f, "wrong argument count: {msg}"),
            Self::Config(msg) => write!(f, "invalid geo configuration: {msg}"),
            Self::Literal(msg) => write!(f, "malformed geometry literal: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported GeoSPARQL operation: {msg}"),
            Self::Domain(msg) => write!(f, "geometry domain error: {msg}"),
        }
    }
}

impl std::error::Error for GeoError {}

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
            GeoError::Arity(msg) => Self::function(msg),
            GeoError::Config(msg) => Self::config(msg),
            GeoError::Literal(msg) => Self::data(format!("malformed geometry literal: {msg}")),
            GeoError::Unsupported(msg) => {
                Self::function(format!("unsupported GeoSPARQL operation: {msg}"))
            }
            GeoError::Domain(msg) => Self::function(format!("geometry domain error: {msg}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GeoError;
    use purrdf_sparql_eval::EvalError;

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
