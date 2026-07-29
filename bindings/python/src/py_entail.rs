// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 bindings for **entailment-regime materialization** and the **OWL 2
//! Direct-Semantics reasoning services** — the `purrdf.entail` submodule.
//!
//! Two lanes, and they are not interchangeable:
//!
//! * the **chase** — [`materialize`] / [`materialize_nt`] close a document under
//!   `Simple` / `RDF` / `RDFS` / `OWL-RL` / `D` and return the closure with a report
//!   whose completeness is `exact` / `sound-incomplete <n>`;
//! * the **tableau** — [`consistency`], [`classify`], [`realize`], [`instances`],
//!   [`entails`], [`profile`], [`extract_module`], [`justify`] and
//!   [`explain_conclusion`] each return `(answer, certificate)` where the
//!   certificate's completeness is `decided` / `decided-within-boundaries` /
//!   `budget-exhausted`. The DL lane has no rule table to subtract, so reusing the
//!   chase's notion would report "exact" for a search that ran out of budget.
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
    MATERIALIZABLE_REGIME_NAMES, REGIME_NAMES, classify_to_string, consistency_to_string,
    entails_to_string, explain_conclusion_to_string, extract_module_to_string,
    implemented_rules_string, instances_to_string, justify_to_string, materialize_to_nquads_string,
    parse_regime, profile_to_string, realize_to_string, regime_name, render_reasoning_report,
    rules_string,
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

// ── The Description-Logic reasoning services ────────────────────────────────────
//
// Nine entry points, one shape: `(answer, certificate)`, both `str`. The pair is a
// tuple rather than an object for the same reason `materialize` returns
// `(closure, report)`: a caller must UNPACK both, so the evidence for how completely
// a question was decided cannot be dropped by not asking for it.
//
// Each is the same convert → detach → call the boundary → map the error sequence
// `materialize_nt` is. Nothing here reasons; `purrdf_validate::regime` does, and
// the C-ABI and WASM hosts call the very same functions.

/// Is the knowledge base consistent — does it have a model at all?
///
/// `data` is an N-Quads (or N-Triples) document. `step_cap` narrows the
/// per-decision tableau step cap; **0 (the default) means the knowledge base's own
/// cap**, not a cap of zero steps. It can only NARROW, so it cannot make a hard
/// instance answerable — only make the `budget-exhausted` certificate reachable.
///
/// Returns `(answer, certificate)`. The answer is `consistency true|false|unknown`;
/// `unknown` means the search reached its step cap and is NEVER collapsed to
/// `false`. The certificate is the `purrdf-dl-certificate 1` block, whose
/// completeness is the DL lane's own notion — `decided`,
/// `decided-within-boundaries` (some axiom never became a DL clause; the
/// certificate names each construct) or `budget-exhausted`.
///
/// This is the only DL service that answers for an unsatisfiable ontology, because
/// it is the one that detects one.
///
/// Raises `ValueError` on a malformed document or a failed reverse mapping.
#[pyfunction]
#[pyo3(signature = (data, step_cap = 0))]
fn consistency(py: Python<'_>, data: &str, step_cap: u32) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| consistency_to_string(data, step_cap))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// The entailed subsumption hierarchy over the ontology's named classes.
///
/// Returns `(answer, certificate)`. The answer carries `equivalent`, `subclass`
/// (the full transitively-closed relation), `direct` (its transitive reduction) and
/// `unsatisfiable` lines, in that block order. Both subsumption blocks are present
/// because they are different facts: `direct` is "direct as far as this run
/// decided", which weakens under a `budget-exhausted` certificate while every
/// listed pair stays a genuine subsumption.
///
/// Costs one tableau decision per ORDERED pair of named classes plus the
/// consistency check; the certificate's `decisions` line reports it.
///
/// Raises `ValueError` on a malformed document, or on an ontology with no model —
/// every class then subsumes every other and the hierarchy would carry no
/// information.
#[pyfunction]
#[pyo3(signature = (data, step_cap = 0))]
fn classify(py: Python<'_>, data: &str, step_cap: u32) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| classify_to_string(data, step_cap))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// The entailed types of the ontology's named individuals, and the most specific of
/// them.
///
/// Returns `(answer, certificate)`; the answer carries `type` lines followed by
/// `direct-type` lines.
///
/// Raises `ValueError` on a malformed document or an ontology with no model.
#[pyfunction]
#[pyo3(signature = (data, step_cap = 0))]
fn realize(py: Python<'_>, data: &str, step_cap: u32) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| realize_to_string(data, step_cap))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// The named individuals entailed to be instances of `class_`.
///
/// `class_` is ONE N-Triples term — `"<https://example.org/Cat>"`, angle brackets
/// included. A class the ontology never mentions is not an error: nothing
/// constrains it, so the empty answer for it is a real answer.
///
/// Returns `(answer, certificate)`; the answer carries `instance <term>` lines.
///
/// Raises `ValueError` on a malformed document, a `class_` that is not one
/// N-Triples term, or an ontology with no model.
#[pyfunction]
#[pyo3(signature = (data, class_, step_cap = 0))]
fn instances(
    py: Python<'_>,
    data: &str,
    class_: &str,
    step_cap: u32,
) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| instances_to_string(data, class_, step_cap))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// Does the ontology entail `axiom`?
///
/// `axiom` is ONE triple of the OWL 2 RDF mapping, in N-Triples syntax. Seven
/// reserved predicates select the seven named axiom kinds — `rdfs:subClassOf`,
/// `owl:equivalentClass`, `owl:disjointWith`, `rdf:type`, `owl:sameAs`,
/// `owl:differentFrom`, `rdfs:subPropertyOf` — and any other predicate is an
/// object-property assertion. No encoding is invented here: this is the mapping the
/// reasoner's own reverse mapping reads.
///
/// Returns `(answer, certificate)`. The answer is `entails true|false|unknown`
/// followed by the axiom as it was READ (`axiom <kind>` plus one `term` line each),
/// so a caller can see which axiom its predicate selected.
///
/// Raises `ValueError` on a malformed document, an `axiom` that is not one triple,
/// an axiom statement that names a graph, or an ontology with no model.
#[pyfunction]
#[pyo3(signature = (data, axiom, step_cap = 0))]
fn entails(py: Python<'_>, data: &str, axiom: &str, step_cap: u32) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| entails_to_string(data, axiom, step_cap))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// Which OWL 2 profiles the ontology is provably in, and what blocked the others.
///
/// Returns `(answer, certificate)`. The answer is `certified <profile>` lines, most
/// restrictive first (`EL`, `QL`, `RL`, `DL`, `Full`), so
/// `answer.splitlines()[0].removeprefix("certified ")` is the most restrictive
/// profile the ontology is provably in.
///
/// Purely syntactic — no tableau, no closure, no budget — so the certificate is a
/// `purrdf-owl-profile-certificate 1` block rather than a DL one: there is no search
/// whose completeness could be reported, and rendering a fabricated `decided` would
/// be the overclaim the certificates exist to prevent. It ends
/// `one-directional true`: a certification PROVES membership, a violation does NOT
/// prove non-membership.
///
/// Raises `ValueError` on a malformed document.
#[pyfunction]
#[pyo3(signature = (data))]
fn profile(py: Python<'_>, data: &str) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| profile_to_string(data))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// The locality module of the ontology for a seed signature.
///
/// `signature` is one N-Triples term per line (blank lines ignored). `method` is
/// `"bot"`, `"top"` or `"star"`.
///
/// Returns `(answer, certificate)`. The answer is the extracted module as canonical
/// (RDFC-1.0) N-Quads — the same serializer `materialize_nt` uses. The certificate's
/// `conservative` line says whether the module is the minimal one or a sound
/// SUPERSET, which is what a caller sizing a module needs to know.
///
/// Raises `ValueError` on a malformed document, a signature line that is not one
/// N-Triples term, an unknown `method` spelling (naming the accepted set), or a
/// module that cannot be frozen.
#[pyfunction]
#[pyo3(signature = (data, signature, method))]
fn extract_module(
    py: Python<'_>,
    data: &str,
    signature: &str,
    method: &str,
) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| extract_module_to_string(data, signature, method))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// WHY a Description-Logic axiom is entailed: a minimal subset of the ontology that
/// still entails it.
///
/// A tableau performs no derivation steps, so this is a JUSTIFICATION and
/// deliberately not called a proof. [`explain_conclusion`] is the chase lane's
/// genuinely derivational explanation; the two are different KINDS of thing rather
/// than two spellings of one, which is why there is no single `explain`.
///
/// `axiom` is read exactly as [`entails`] reads it.
///
/// Returns `(answer, certificate)`. The answer is the justification's axioms as
/// canonical N-Quads — a justification introduces no term, so it is an ordinary RDF
/// 1.2 dataset of axioms already present in the input. The certificate's
/// `sufficient` and `minimal` lines are **re-decided** over the justification alone
/// and over each of its one-axiom-smaller subsets, so they check the answer rather
/// than restate it.
///
/// Raises `ValueError` if the ontology does not entail the axiom — the empty set
/// reads as "nothing is needed" and means the opposite — or if the tableau could not
/// decide it, leaving no answer to shrink against.
#[pyfunction]
#[pyo3(signature = (data, axiom))]
fn justify(py: Python<'_>, data: &str, axiom: &str) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| justify_to_string(data, axiom))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// WHY one triple of a chase closure holds: which rules, from which premises.
///
/// `conclusion` is ONE N-Quads statement; its graph, if it names one, selects the
/// closure to explain.
///
/// Returns `(answer, certificate)`. The answer carries `asserted`, `steps` and one
/// `rule` line per cited rule. The certificate's `derived-*` lines are what the
/// CHECKER re-derived from the proof term and the clause program — not what the
/// proof claims — so a proof whose stated conclusion its own premises do not license
/// shows up as differing lines rather than a silent `checked true`.
///
/// Raises `ValueError` for `RDF` and `RDFS`, four of whose rules conclude about a
/// FRESH blank node: an existential head has no Datalog semantics, so a "proof" of
/// such a step could only be believed. Also for a conclusion that is neither
/// asserted nor derived — a hard error, because there is nothing to explain and an
/// empty answer would read as though there were.
#[pyfunction]
#[pyo3(signature = (data, regime, conclusion))]
fn explain_conclusion(
    py: Python<'_>,
    data: &str,
    regime: &Bound<'_, PyAny>,
    conclusion: &str,
) -> PyResult<(String, String)> {
    let name = regime_name(native_regime(regime)?);
    let answer = py
        .detach(|| explain_conclusion_to_string(data, name, conclusion))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
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
    m.add_function(wrap_pyfunction!(consistency, m)?)?;
    m.add_function(wrap_pyfunction!(classify, m)?)?;
    m.add_function(wrap_pyfunction!(realize, m)?)?;
    m.add_function(wrap_pyfunction!(instances, m)?)?;
    m.add_function(wrap_pyfunction!(entails, m)?)?;
    m.add_function(wrap_pyfunction!(profile, m)?)?;
    m.add_function(wrap_pyfunction!(extract_module, m)?)?;
    m.add_function(wrap_pyfunction!(justify, m)?)?;
    m.add_function(wrap_pyfunction!(explain_conclusion, m)?)?;
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
