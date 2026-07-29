// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 bindings for **entailment-regime materialization** — the `purrdf.entail`
//! submodule: close an RDF document under `Simple` / `RDF` / `RDFS` / `OWL-RL` /
//! `D` and get back both the closure and a report of what the run actually did.
//!
//! # Not to be confused with `purrdf.shapes.entail`
//!
//! The two names sit one namespace apart and both say "entail", so the
//! distinction is stated in full:
//!
//! * `purrdf.shapes.entail(shapes_ttl, data_nt)` — **SHACL-AF `sh:rule`**
//!   entailment ([`crate::shacl`]). It needs a *shapes* graph and applies the
//!   rules that graph declares.
//! * `purrdf.entail.materialize(dataset, regime)` — **SPARQL entailment-regime**
//!   materialization (this module). It takes no shapes at all: it closes a
//!   document under the regime's own specification rule table.
//!
//! # One boundary, three bindings
//!
//! Nothing here reimplements the parse → close → serialize sequence. Every entry
//! point routes through `purrdf_validate::regime`, the same string boundary the
//! C-ABI and WASM hosts call, so a byte difference between the three hosts is one
//! shared golden vector failing rather than three surfaces that quietly stopped
//! agreeing. This module converts Python values to plain Rust data, releases the
//! GIL, calls that boundary, and maps its `Result<_, String>` onto `ValueError`.
//! That is its whole job.
//!
//! # Why both [`rules`] and [`implemented_rules`]
//!
//! `rules(regime)` is the rule table the specification *defines* the regime by;
//! `implemented_rules(regime)` is the subset this workspace's chase currently
//! fires. The difference is the honest gap — for `OWL-RL` and for `D` it is now
//! EMPTY, and where it is not empty it is the existential-head patterns that mint
//! a fresh blank node. Both are exposed so a caller can MEASURE the gap instead of
//! trusting a docstring, which is the whole point: a number written here would be
//! stale the day a rule lands, and the pair never is.
//! The same gap appears as the `missing` lines of the rendered report
//! that every materialization returns; the report is never optional here, for the
//! reason [`purrdf_validate::regime`] documents at length.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use purrdf_validate::regime::{
    MATERIALIZABLE_REGIME_NAMES, REGIME_NAMES, implemented_rules_string,
    materialize_to_nquads_string, parse_regime, regime_name, render_reasoning_report, rules_string,
};

use crate::entail::{EntailError, Regime, materialize as materialize_closure};
use crate::py_gts_dataset::PyRdfDataset;

// ── The regime enum ─────────────────────────────────────────────────────────────

/// The SPARQL entailment regimes, as a Python enum (`Regime.OWL_RL`).
///
/// Mirrors [`PyRdfFormat`](crate::py_store::PyRdfFormat): the SCREAMING_SNAKE
/// member spellings ARE the Python-visible names, and [`Self::to_native`] is the
/// total map onto [`Regime`]. Every entry point in this module also accepts the
/// regime's CLI spelling as a plain `str` (`"owl-rl"`), so one spelling works from
/// the command line, through the C ABI, through WASM and from Python.
#[pyclass(name = "Regime", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[allow(
    clippy::upper_case_acronyms,
    reason = "the SCREAMING_SNAKE variant spellings ARE the Python-visible enum members (`Regime.OWL_RL`), so they must not be renamed"
)]
pub(crate) enum PyRegime {
    /// `entailment/Simple` — no entailment; the graph is its own closure.
    SIMPLE,
    /// `entailment/RDF` — RDF entailment.
    RDF,
    /// `entailment/RDFS` — RDFS entailment.
    RDFS,
    /// `entailment/OWL-RL` — OWL 2 RL.
    OWL_RL,
    /// `entailment/OWL-Direct` — OWL 2 DL; not forward-materializable.
    OWL_DIRECT,
    /// `entailment/RIF` — needs a caller-supplied rule set; not materializable here.
    RIF,
    /// `entailment/D` — datatype entailment, realized as the five `dt-*` rules of
    /// OWL 2 Profiles §4.3 Table 8; forward-materializable.
    D,
}

impl PyRegime {
    /// The native regime this member selects.
    ///
    /// The match is exhaustive with no wildcard arm, so the compiler — not a
    /// test — is what forces this map to be revisited when either enum grows.
    fn to_native(self) -> Regime {
        match self {
            Self::SIMPLE => Regime::Simple,
            Self::RDF => Regime::Rdf,
            Self::RDFS => Regime::Rdfs,
            Self::OWL_RL => Regime::OwlRl,
            Self::OWL_DIRECT => Regime::OwlDirect,
            Self::RIF => Regime::Rif,
            Self::D => Regime::D,
        }
    }
}

/// Decode the `regime` argument: a [`PyRegime`] member, or its CLI spelling.
///
/// An unrecognized value raises `ValueError` naming the whole accepted set, so a
/// caller two language boundaries away can fix the call without reading this
/// source.
fn native_regime(regime: &Bound<'_, PyAny>) -> PyResult<Regime> {
    if let Ok(member) = regime.extract::<PyRegime>() {
        return Ok(member.to_native());
    }
    let spelling: String = regime.extract().map_err(|_| {
        PyValueError::new_err(format!(
            "regime must be a purrdf.entail.Regime member or one of: {}",
            REGIME_NAMES.join(", ")
        ))
    })?;
    parse_regime(&spelling).map_err(PyValueError::new_err)
}

/// The message a regime that cannot be forward-materialized is refused with.
///
/// The wording mirrors `purrdf_validate::regime`'s own refusal and the accepted
/// set is read from [`MATERIALIZABLE_REGIME_NAMES`] rather than re-typed, so the
/// dataset path and the string path can never come to list different regimes.
fn refusal(regime: Regime) -> String {
    format!(
        "entailment regime \"{}\" cannot be forward-materialized \
         (owl-direct needs the query's class expressions, \
         rif needs a parsed rule set); materializable regimes: {}",
        regime_name(regime),
        MATERIALIZABLE_REGIME_NAMES.join(", ")
    )
}

// ── Materialization ─────────────────────────────────────────────────────────────

/// Close a frozen `RdfDataset` under `regime`, returning `(closure, report)`.
///
/// `dataset` is a native `purrdf.RdfDataset`, so a document already parsed on the
/// Python side is closed WITHOUT a round trip back through text. The closure is a
/// new `RdfDataset` holding every input quad plus every triple the regime's
/// implemented rules infer; read it with `closure.to_nquads()`.
///
/// The report is the second element and is never optional — the same discipline
/// the Rust and C-ABI surfaces enforce. It names, in a byte-stable rendering,
/// which rules fired and how often, which specification rules did NOT fire, which
/// constructs were left at a boundary, and the calculus's contract hash. All
/// seventy-eight OWL 2 RL rules now run, so the report's job under `OWL-RL` has
/// shifted from naming the missing rules to naming the CONSTRUCTS still at a
/// boundary — a complete rule table is not a complete closure, and reporting the
/// first as if it were the second is exactly the overclaim the report prevents.
///
/// Raises `ValueError` for an unknown regime spelling (naming the accepted set),
/// for a regime that cannot be forward-materialized (`owl-direct`, `rif`),
/// and for an exhausted evaluation ceiling.
#[pyfunction]
#[pyo3(signature = (dataset, regime))]
fn materialize(
    py: Python<'_>,
    dataset: &PyRdfDataset,
    regime: &Bound<'_, PyAny>,
) -> PyResult<(PyRdfDataset, String)> {
    // Arguments become plain Rust data (a native regime, an owned `Arc` handle)
    // BEFORE the GIL is released.
    let native = native_regime(regime)?;
    let data = dataset.dataset();
    // Chase + report rendering run detached (GIL released); the Python objects
    // are built after the GIL is reacquired.
    let (closure, report) = py
        .detach(|| {
            let (closure, report) =
                materialize_closure(data.as_ref(), native).map_err(|error| match error {
                    EntailError::Unsupported(_) => refusal(native),
                    other => format!("entailment regime \"{}\": {other}", regime_name(native)),
                })?;
            Ok::<_, String>((closure, render_reasoning_report(&report)))
        })
        .map_err(PyValueError::new_err)?;
    Ok((PyRdfDataset::from_arc(closure), report))
}

/// Close an N-Quads (or N-Triples) document under `regime`, returning
/// `(canonical_nquads, report)`.
///
/// The text-in/text-out twin of [`materialize`], for callers holding a document
/// rather than a parsed dataset. It is a thin wrapper over
/// [`purrdf_validate::regime::materialize_to_nquads_string`] — the SAME boundary
/// call the C-ABI and WASM hosts make against the SAME golden vector — and not a
/// second engine path, which is what makes byte-identity across the three hosts a
/// meaningful claim.
///
/// The closure is serialized through the RDFC-1.0 canonical flat serializer, so
/// repeated calls on equal input produce byte-identical output.
///
/// Raises `ValueError` on a malformed document, an unknown regime spelling
/// (naming the accepted set), or a regime that cannot be forward-materialized.
#[pyfunction]
#[pyo3(signature = (data, regime))]
fn materialize_nt(
    py: Python<'_>,
    data: &str,
    regime: &Bound<'_, PyAny>,
) -> PyResult<(String, String)> {
    let name = regime_name(native_regime(regime)?);
    // Parse + chase + canonical serialization + report rendering run detached
    // (GIL released).
    let closure = py
        .detach(|| materialize_to_nquads_string(name, data))
        .map_err(PyValueError::new_err)?;
    Ok(closure.into_parts())
}

// ── The rule inventories ────────────────────────────────────────────────────────

/// The rule table `regime` is *defined by*, one specification rule name per
/// entry, in specification table order.
///
/// `[]` for a regime with no rule table (`simple`, and the two that are not
/// forward-materializable). `OWL-RL` returns all 78 rules of OWL 2 Profiles
/// §4.3 Tables 4–9 whether or not this workspace fires them — that is the point:
/// compare it with [`implemented_rules`] to measure the gap.
///
/// Raises `ValueError` for an unknown regime spelling, naming the accepted set.
#[pyfunction]
#[pyo3(signature = (regime))]
fn rules(regime: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    // A lookup of a `&'static` table, not a compute path: there is nothing heavy
    // here for the GIL to be released around.
    let table = rules_string(regime_name(native_regime(regime)?)).map_err(PyValueError::new_err)?;
    Ok(table.lines().map(str::to_owned).collect())
}

/// The subset of [`rules`] this workspace's chase actually fires today.
///
/// Always a subsequence of [`rules`] — same order, no additions — and for
/// `OWL-RL` and `D` it is now the WHOLE table, not a strict subset. `rules(r)`
/// minus `implemented_rules(r)` is the regime's measurable gap, the same set the
/// rendered report's `missing` lines name, and it is legitimately empty.
///
/// Raises `ValueError` for an unknown regime spelling, naming the accepted set.
#[pyfunction]
#[pyo3(signature = (regime))]
fn implemented_rules(regime: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    // As in `rules`: a static-table lookup, so no GIL release.
    let table = implemented_rules_string(regime_name(native_regime(regime)?))
        .map_err(PyValueError::new_err)?;
    Ok(table.lines().map(str::to_owned).collect())
}

/// Register the entailment-regime surface on a Python module.
///
/// Called by the unified `purrdf_native` cdylib to populate the
/// `purrdf_native.entail` submodule, which the package shim re-attaches as
/// `purrdf.entail` (mirroring [`crate::py_shex::register`]).
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRegime>()?;
    m.add_function(wrap_pyfunction!(materialize, m)?)?;
    m.add_function(wrap_pyfunction!(materialize_nt, m)?)?;
    m.add_function(wrap_pyfunction!(rules, m)?)?;
    m.add_function(wrap_pyfunction!(implemented_rules, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every Python-visible member maps onto a distinct native regime, and the
    /// seven together cover the whole `Regime` enum.
    #[test]
    fn regime_maps_to_native() {
        const MEMBERS: [PyRegime; 7] = [
            PyRegime::SIMPLE,
            PyRegime::RDF,
            PyRegime::RDFS,
            PyRegime::OWL_RL,
            PyRegime::OWL_DIRECT,
            PyRegime::RIF,
            PyRegime::D,
        ];
        let names: Vec<&str> = MEMBERS
            .iter()
            .map(|member| regime_name(member.to_native()))
            .collect();
        assert_eq!(names, REGIME_NAMES.to_vec());
    }

    /// The dataset path's refusal is byte-identical to the string boundary's, so
    /// the two surfaces cannot describe the same limit in different words.
    ///
    /// Exactly two regimes reach it. `d` used to be a third and is not any more:
    /// it materializes, which the tail of this test pins so the two lists cannot
    /// drift apart again.
    #[test]
    fn refusal_matches_the_string_boundary() {
        for name in ["owl-direct", "rif"] {
            let native = parse_regime(name).expect("an accepted spelling");
            let boundary = materialize_to_nquads_string(name, "").expect_err("not materializable");
            assert_eq!(refusal(native), boundary);
        }
        assert!(materialize_to_nquads_string("d", "").is_ok());
        for name in MATERIALIZABLE_REGIME_NAMES {
            assert!(
                materialize_to_nquads_string(name, "").is_ok(),
                "{name} is listed as materializable"
            );
        }
    }
}
