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
//! # Why all three of [`rules`], [`implemented_rules`] and [`extensions`]
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
//!
//! [`extensions`] is the third inventory, and it is disjoint from both others: the
//! rules this build fires that the specification table does not list. Neither of
//! the other two can express that — both are statements ABOUT the table — so
//! without it a caller had to materialize a dataset and read a report line to
//! learn what a build adds. A non-normative rule that is sound is still a
//! behaviour difference, and a caller who needs strictly table-defined behaviour
//! has to be able to see it before deciding.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use purrdf_validate::regime::{
    PROOF_SERVICE_NAMES, REGIME_NAMES, ReasonerSession, certain_answers_to_string, check_dl_proof,
    classify_to_string, consistency_to_string, entails_to_string, explain_conclusion_to_string,
    extension_rules_string, extract_module_to_string, graph_entails_to_string,
    implemented_rules_string, instances_to_string, justify_to_string, materialize_to_nquads_string,
    parse_regime, profile_to_string, prove_to_string, realize_to_string, regime_name, regime_plan,
    regime_rule_set, render_entail_error, render_reasoning_report, rules_string,
    verify_entailment_to_string,
};

use crate::entail::{Regime, materialize as materialize_closure};
use crate::py_gts_dataset::PyRdfDataset;

// ── The regime enum ─────────────────────────────────────────────────────────────

/// The SPARQL entailment regimes, as a Python enum (`Regime.OWL_RL`).
///
/// Mirrors [`PyRdfFormat`](crate::py_store::PyRdfFormat): the SCREAMING_SNAKE
/// member spellings ARE the Python-visible names, and [`Self::to_native`] is the
/// total map onto [`Regime`]. Every entry point in this module also accepts the
/// regime's CLI spelling as a plain `str` (`"owl-rl"`), so one spelling works from
/// the command line, through the C ABI, through WASM and from Python.
#[pyclass(name = "Regime", eq, eq_int, skip_from_py_object)]
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
    /// `entailment/OWL-Direct` — OWL 2 DL via the tableau. A document surface has
    /// no query to direct it, so it runs the query-independent augmentation.
    OWL_DIRECT,
    /// `entailment/RIF` — entails under the caller's rule document (`program`).
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
/// # `program` — the regime's own input, and never optional
///
/// EVERY regime materializes; none is refused for being the regime it is. What two of
/// them need is an INPUT, and `program` is where it goes. `rif` entails under the
/// CALLER's rules, so its `program` is a normative RIF-in-XML rule document; every other
/// regime's rule table is the specification's, so its `program` is `""`, and a non-empty
/// one raises rather than being silently discarded — a caller who passed rules to `rdfs`
/// believes they ran. `owl-direct` takes none either: its extra input is a QUERY's class
/// expressions, and this surface closes a dataset rather than answering a query, so what
/// runs is the query-independent augmentation.
///
/// The argument is required rather than defaulted, and required in the same position on
/// all four hosts, so one call shape works from the command line, through the C ABI,
/// through WASM and from Python.
///
/// # An INCONSISTENT knowledge base raises WITH its certificate
///
/// An inconsistent knowledge base entails every triple, so there is no closure to return
/// — but there WAS a run, and it is described. The `ValueError` message carries the
/// one-line refusal and then the full rendered report on the following lines, whose first
/// line is `purrdf_validate`'s report banner: the rule that refused, the graph whose
/// closure refused, the asserted triples that satisfied the rule in that rule's own
/// premise order (`inconsistency-premise` lines), the rules that had already fired, the
/// budget the evaluation had consumed and the calculus hash. Splitting the message at the
/// banner line yields exactly the report a successful call returns.
///
/// It travels in the message because a raise is the only channel a refusal has, and the
/// alternative was what this surface used to do: render `Display` alone, which reads only
/// the premise COUNT, so the caller whose data was bad was the only caller who got no
/// report at all.
///
/// Raises `ValueError` for an unknown regime spelling (naming the accepted set), for a
/// `program` that is wrong for the regime, for an inconsistent knowledge base, and for an
/// exhausted evaluation ceiling.
#[pyfunction]
#[pyo3(signature = (dataset, regime, program))]
fn materialize(
    py: Python<'_>,
    dataset: &PyRdfDataset,
    regime: &Bound<'_, PyAny>,
    program: &str,
) -> PyResult<(PyRdfDataset, String)> {
    // Arguments become plain Rust data (a native regime, an owned `Arc` handle)
    // BEFORE the GIL is released.
    let native = native_regime(regime)?;
    let name = regime_name(native);
    let data = dataset.dataset();
    // Chase + report rendering run detached (GIL released); the Python objects
    // are built after the GIL is reacquired.
    let (closure, report) = py
        .detach(|| {
            // The SAME two helpers `materialize_nt` reaches through the string boundary,
            // so the dataset path and the text path cannot come to mean different things
            // by the same regime spelling.
            let rules = regime_rule_set(native, name, program)?;
            let (closure, report) = materialize_closure(data.as_ref(), regime_plan(native, &rules))
                .map_err(|error| render_entail_error(name, &error))?;
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
/// `program` is the regime's own rule document, exactly as on [`materialize`].
///
/// Raises `ValueError` on a malformed document, an unknown regime spelling
/// (naming the accepted set), or a `program` that is wrong for the regime — and on an
/// INCONSISTENT knowledge base, whose raise carries the run's full rendered report,
/// witness triples included, exactly as [`materialize`] documents.
#[pyfunction]
#[pyo3(signature = (data, regime, program))]
fn materialize_nt(
    py: Python<'_>,
    data: &str,
    regime: &Bound<'_, PyAny>,
    program: &str,
) -> PyResult<(String, String)> {
    let name = regime_name(native_regime(regime)?);
    // Parse + chase + canonical serialization + report rendering run detached
    // (GIL released).
    let closure = py
        .detach(|| materialize_to_nquads_string(name, data, program))
        .map_err(PyValueError::new_err)?;
    Ok(closure.into_parts())
}

// ── The rule inventories ────────────────────────────────────────────────────────

/// The rule table `regime` is *defined by*, one specification rule name per
/// entry, in specification table order.
///
/// `[]` for a regime with no rule table of its own (`simple`, plus `OWL_DIRECT`, which
/// decides through the tableau, and `RIF`, which entails under the caller's rules — all
/// three still MATERIALIZE). `OWL-RL` returns all 78 rules of OWL 2 Profiles
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

/// The rules this build fires BEYOND `regime`'s specification table.
///
/// Disjoint from both [`rules`] and [`implemented_rules`] by construction: the
/// normative table is a statement about the specification, and it does not move
/// because this workspace fires a sound rule the table happens not to list. A
/// rendered report names the same rules on its `extension` line — this answers
/// the question without materializing a dataset first.
///
/// `[]` for a lane with nothing added to it, which is every lane but `OWL-RL`.
///
/// Raises `ValueError` for an unknown regime spelling, naming the accepted set.
#[pyfunction]
#[pyo3(signature = (regime))]
fn extensions(regime: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    // As in `rules`: a static-table lookup, so no GIL release.
    let table = extension_rules_string(regime_name(native_regime(regime)?))
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
/// `work_cap` narrows the per-decision WORK cap on the same `0`-means-the-knowledge-base's-own-cap
/// rule, and can only NARROW too. It bounds what `step_cap` structurally cannot: a round is a
/// PASS over the completion graph rather than a unit of cost, so an ontology can make every
/// round enormously more expensive without making the search take more rounds. A run that
/// reaches it answers `unknown` with `work` equal to `work-budget` in its certificate.
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
#[pyo3(signature = (data, step_cap = 0, work_cap = 0))]
fn consistency(
    py: Python<'_>,
    data: &str,
    step_cap: u32,
    work_cap: u32,
) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| consistency_to_string(data, step_cap, work_cap))
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
#[pyo3(signature = (data, step_cap = 0, work_cap = 0))]
fn classify(
    py: Python<'_>,
    data: &str,
    step_cap: u32,
    work_cap: u32,
) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| classify_to_string(data, step_cap, work_cap))
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
#[pyo3(signature = (data, step_cap = 0, work_cap = 0))]
fn realize(py: Python<'_>, data: &str, step_cap: u32, work_cap: u32) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| realize_to_string(data, step_cap, work_cap))
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
#[pyo3(signature = (data, class_, step_cap = 0, work_cap = 0))]
fn instances(
    py: Python<'_>,
    data: &str,
    class_: &str,
    step_cap: u32,
    work_cap: u32,
) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| instances_to_string(data, class_, step_cap, work_cap))
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
#[pyo3(signature = (data, axiom, step_cap = 0, work_cap = 0))]
fn entails(
    py: Python<'_>,
    data: &str,
    axiom: &str,
    step_cap: u32,
    work_cap: u32,
) -> PyResult<(String, String)> {
    let answer = py
        .detach(|| entails_to_string(data, axiom, step_cap, work_cap))
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

// ── The conclusion-directed entailment services ─────────────────────────────────

/// Borrow a Python-supplied import table as the boundary's [`ImportList`].
///
/// The three services below all take `imports` as a `Sequence[tuple[str, str]]`, which PyO3
/// materializes as owned `String`s; the boundary takes borrowed pairs. This is that one
/// re-borrow, written once so the three call sites cannot drift, and it preserves the
/// caller's ORDER — the boundary's table is a list rather than a map precisely so the same
/// input always produces the same run.
fn import_list(imports: &[(String, String)]) -> Vec<(&str, &str)> {
    imports
        .iter()
        .map(|(iri, document)| (iri.as_str(), document.as_str()))
        .collect()
}

/// The CERTAIN ANSWERS of a basic graph pattern over `data` under `regime`.
///
/// A certain answer is a substitution the knowledge base ENTAILS the pattern under —
/// true in every model, not merely present in one closure — which is what SPARQL's
/// entailment regimes define the answers to a basic graph pattern to be.
///
/// `pattern` is N-Triples with `?name` in any position, the PREDICATE included. A
/// blank node in it is a NON-DISTINGUISHED variable: constrained by the match, not
/// projected, and not a column, which is what SPARQL says a query blank node is. A
/// variable inside an RDF 1.2 triple term is an ordinary variable — it binds, it is a
/// column, and one NAME is one VARIABLE wherever it was written, so a pattern that
/// uses it above and below the triple-term boundary is joined rather than split. A
/// predicate variable is projected like any other, and under `owl-rl` it also renders a
/// `limit`: it ranges over the whole predicate vocabulary, including the schema
/// predicates and the constructs the mechanisms beyond the rule table decide, and the
/// closure holds neither.
///
/// Returns `(answer, certificate)`. The answer opens `mechanism <name>`, then one
/// `var` line per projected variable, one `row` line per certain answer positionally
/// aligned to them, and a `limit` line per reason the row set may not be EXHAUSTIVE.
/// Every row is sound unconditionally; what needs a precondition is the claim about a
/// row that is NOT there, so no `limit` lines is the claim that the row set is complete.
/// The certificate is the run's `purrdf-reasoning-report 4` block.
///
/// A pattern with a projected variable is `mechanism strict-table`: the five mechanisms
/// beyond the rule table are not run for one, because a projected variable over what any
/// of them decides is a different question. That one of them WOULD have been needed is
/// not silence — it arrives as a `limit` line naming the lane. A pattern with NO
/// projected variable is a conclusion graph, is answered by the same fold
/// `graph_entails` runs, and names whichever of the seven reached it; such an answer is
/// the relation with no columns, so a `yes` is one bare `row` line and a `no` is none.
///
/// # `imports` — the documents the premise says it is not all of, and never optional
///
/// A `Sequence[tuple[str, str]]` of `(ontology_iri, document)` pairs, where `document` is
/// N-Quads (or N-Triples) text exactly like `data`. A premise carrying an `owl:imports` is an
/// ontology stating that its axioms are its own PLUS those of the documents it names, so
/// answering over the premise alone would answer a different question — this is where those
/// documents arrive.
///
/// **PurRDF fetches nothing.** An ontology IRI this sequence does not resolve is a
/// `ValueError` naming the document, never a network access and never a silently empty
/// import. `[]` is the ordinary "imports nothing" case and is required rather than defaulted,
/// in the same position on all four hosts, so one call shape works everywhere.
///
/// Raises `ValueError` on `OWL_DIRECT` or `RIF` — each is defined by an input this
/// signature does not carry, so both are refused by name rather than served by a weaker
/// lane — on a malformed document, pattern or import document, on a duplicate or empty
/// import IRI, on a pattern that names a graph, on a pattern that writes a variable in a
/// literal's DATATYPE (a slot RDF reserves for an IRI, and one a basic graph pattern has no
/// binding to project), on an `owl:imports` `imports` does not resolve, and on an
/// inconsistent premise, whose refusal carries the full report.
#[pyfunction]
#[pyo3(signature = (regime, data, pattern, imports))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn certain_answers(
    py: Python<'_>,
    regime: &Bound<'_, PyAny>,
    data: &str,
    pattern: &str,
    imports: Vec<(String, String)>,
) -> PyResult<(String, String)> {
    let name = regime_name(native_regime(regime)?);
    let table = import_list(&imports);
    let answer = py
        .detach(|| certain_answers_to_string(name, data, pattern, &table))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// Does `premise` entail the conclusion GRAPH under `regime`'s rule table?
///
/// NOT [`entails`], and the collision is real enough to state rather than rename away.
/// [`entails`] asks the OWL 2 Direct-Semantics TABLEAU whether an ontology entails one
/// AXIOM, and its certificate is a `purrdf-dl-certificate 1` block whose completeness
/// counts hypertableau rounds. This asks the regime's RULE TABLE whether a premise
/// entails a conclusion GRAPH, and its certificate is a `purrdf-reasoning-report 4` block
/// whose completeness is the regime's own rule inventory.
///
/// Returns `(answer, certificate)`. The answer opens `mechanism <name>`: WHICH of the seven
/// mechanisms reached the verdict. `strict-table` is the regime's own rule table, run
/// once; `refutation`, `freeze`, `comprehension`, `reflexivity` and `data-range` each
/// exist because that table DECIDES no conclusion of that shape; and `composite` is
/// two or more of those folded over one conclusion.
///
/// Then THREE verdicts, never two. `entailment not-entailed` is a PROOF — the procedure
/// was complete for this premise, so the absence of a mapping is the absence of an
/// entailment — and `entailment undecided` is what an incomplete procedure is entitled to
/// say instead. Reading the third as the second would turn a limitation of this library
/// into a false statement about the caller's data.
///
/// `imports` is [`certain_answers`]'s, and applies to the PREMISE: the conclusion is a graph
/// to match rather than an ontology to close, so an `owl:imports` in it names nothing this
/// service resolves.
///
/// Raises `ValueError` as [`certain_answers`].
#[pyfunction]
#[pyo3(signature = (regime, premise, conclusion, imports))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn graph_entails(
    py: Python<'_>,
    regime: &Bound<'_, PyAny>,
    premise: &str,
    conclusion: &str,
    imports: Vec<(String, String)>,
) -> PyResult<(String, String)> {
    let name = regime_name(native_regime(regime)?);
    let table = import_list(&imports);
    let answer = py
        .detach(|| graph_entails_to_string(name, premise, conclusion, &table))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

/// [`graph_entails`] with the warrant RE-DECIDED, without running a reasoner.
///
/// The re-check re-derives nothing, deliberately: "the closure follows from the premise"
/// is the chase's claim and [`explain_conclusion`] is its checker, while "the conclusion
/// follows from the closure" is this one and is finite and purely combinatorial.
///
/// Returns `(answer, certificate)`. The answer is [`graph_entails`]'s plus
/// `warrant present|absent` and `verified true|false|not-applicable`. `warrant absent` /
/// `verified not-applicable` is a `not-entailed` or an `undecided`: there is no evidence
/// to re-decide, and a `false` there would read as a failed check rather than an absent
/// one.
///
/// `imports` is [`certain_answers`]'s. The re-check runs against the premise AS WRITTEN
/// rather than against its imports closure: a warrant re-decidable from the caller's own
/// document is a stronger check than one only re-decidable against a graph the library
/// assembled.
///
/// Raises `ValueError` as [`certain_answers`].
#[pyfunction]
#[pyo3(signature = (regime, premise, conclusion, imports))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn verify_entailment(
    py: Python<'_>,
    regime: &Bound<'_, PyAny>,
    premise: &str,
    conclusion: &str,
    imports: Vec<(String, String)>,
) -> PyResult<(String, String)> {
    let name = regime_name(native_regime(regime)?);
    let table = import_list(&imports);
    let answer = py
        .detach(|| verify_entailment_to_string(name, premise, conclusion, &table))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_parts())
}

// ── Proofs: opt-in to produce, and a checker to consume ─────────────────────────

/// Answer one Description-Logic service WITH the proof term of the run that answered.
///
/// **The opt-in.** The nine entry points above are unchanged and record nothing, so a caller
/// who does not want a proof runs exactly the search they ran before and gets exactly the
/// two-tuple they got before. This one RECORDS — which costs the completion graph of every
/// tableau run it keeps — and returns a THREE-tuple `(answer, certificate, proof)`, all
/// `str`.
///
/// `service` is one of `consistency`, `class-satisfiability`, `classify`, `realize`,
/// `instances`, `entails`, `extract-module`. `argument` is the question's own input in that
/// service's grammar:
///
/// * `""` for `consistency`, `classify` and `realize` — a non-empty one raises rather than
///   being discarded;
/// * ONE N-Triples term for `class-satisfiability` and `instances`;
/// * ONE triple of the OWL 2 RDF mapping for `entails`;
/// * a `method <bot|top|star>` line then one term per line for `extract-module`.
///
/// `answer` and `certificate` are byte-identical to the same question asked without a proof:
/// recording is an observation the reasoner makes of itself, never a lever it reads. `proof`
/// is the `purrdf-dl-proof 1` document `check_proof` takes.
///
/// `step_cap` and `work_cap` are `consistency`'s.
///
/// Raises `ValueError` on a malformed document, an unknown service, an argument wrong for the
/// service, or whatever that service itself refuses.
#[pyfunction]
#[pyo3(signature = (data, service, argument = "", step_cap = 0, work_cap = 0))]
fn prove(
    py: Python<'_>,
    data: &str,
    service: &str,
    argument: &str,
    step_cap: u32,
    work_cap: u32,
) -> PyResult<(String, String, String)> {
    let answer = py
        .detach(|| prove_to_string(data, service, argument, step_cap, work_cap))
        .map_err(PyValueError::new_err)?;
    Ok(answer.into_proved_parts())
}

/// CHECK a proof against the CALLER's own ontology, question and answer.
///
/// **The checker**, and nothing in it trusts the producer: the ontology is parsed from `data`,
/// the question is re-derived from `service` and `argument`, the claims are read back out of
/// `answer`'s own grammar, and the checking context comes from a reverse mapping this call
/// performs itself. The proof supplies the runs and nothing else, so an `entails` proof for a
/// different axiom, a proof for a different document, and a genuine proof of some OTHER
/// answer are each refused.
///
/// `answer` and `certificate` may each be `""`, and each empty one is a WEAKER check that says
/// so rather than one that quietly passed: with no answer the report reads
/// `answer not-checked`, and with no certificate a proof carrying a stopping receipt is
/// refused, because there is nothing for the receipt to be a receipt of.
///
/// Returns the `purrdf-dl-proof-check 1` report — the digest and input identity it checked,
/// the runs it replayed, and the `attested`/`trusted`/`unattested` counts with the
/// producer-shared components the whole check rests on. There is no `verified` line: a
/// verification that FAILED raises, so a rendered `true` would be a constant rather than a
/// gate.
///
/// Raises `ValueError` for a proof document reading `availability not-recorded` — an answer
/// nobody asked to record is never presented as a verified one — and for every other
/// rejection.
#[pyfunction]
#[pyo3(signature = (data, service, argument, answer, certificate, proof))]
fn check_proof(
    py: Python<'_>,
    data: &str,
    service: &str,
    argument: &str,
    answer: &str,
    certificate: &str,
    proof: &str,
) -> PyResult<String> {
    py.detach(|| check_dl_proof(data, service, argument, answer, certificate, proof))
        .map_err(PyValueError::new_err)
}

/// The seven Description-Logic services a proof term can be about, in the spellings `prove`
/// and `check_proof` accept.
///
/// Exposed for the reason `rules`/`implemented_rules` are: a caller must be able to MEASURE
/// the set rather than trust a docstring that would go stale the day it moved.
#[pyfunction]
fn proof_services() -> Vec<String> {
    PROOF_SERVICE_NAMES
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

// ── The session ─────────────────────────────────────────────────────────────────

/// A reasoning session over one ontology — `purrdf.entail.Reasoner`.
///
/// Every free function above takes the document as a string and rebuilds everything it
/// needs, so asking three questions parses and reverse-maps the ontology three times.
/// This class holds the parsed document instead: constructing it parses once, the first
/// question needing a knowledge base reverse-maps once, and later questions reuse both.
///
/// The methods answer exactly what the free functions of the same name answer — they are
/// the same `purrdf_validate::regime` session those functions now wrap — so moving from
/// one to the other cannot change an answer or a certificate.
///
/// ```python
/// r = purrdf.entail.Reasoner(data)
/// answer, certificate = r.consistency()
/// hierarchy, _ = r.classify()          # no second parse
/// ```
#[pyclass(name = "Reasoner")]
pub(crate) struct PyReasoner {
    /// The shared boundary's session. Every method is a thin call onto it.
    session: ReasonerSession,
}

#[pymethods]
impl PyReasoner {
    /// Parse `data` and open a session over it.
    ///
    /// `data` is an N-Quads (or N-Triples) document. `step_cap` narrows the per-decision
    /// tableau step cap for every question asked through this session; **0 (the default)
    /// means the knowledge base's own cap**, not a cap of zero steps, and it can only
    /// NARROW. `work_cap` narrows the per-decision WORK cap on the same rule — the cap on
    /// the matcher, scan, closure and clone work done INSIDE a round, which a round cap
    /// cannot see.
    ///
    /// Nothing is reverse-mapped here, so an ontology whose knowledge base cannot be
    /// built still constructs — and raises on the first question that needs one. That is
    /// deliberate: `profile`, `extract_module`, `justify` and `explain_conclusion` never
    /// reason, and `profile` answers for any parseable document.
    ///
    /// Raises `ValueError` on a malformed document.
    #[new]
    #[pyo3(signature = (data, step_cap = 0, work_cap = 0, proofs = false))]
    fn new(
        py: Python<'_>,
        data: &str,
        step_cap: u32,
        work_cap: u32,
        proofs: bool,
    ) -> PyResult<Self> {
        let session = py
            .detach(|| {
                if proofs {
                    ReasonerSession::open_with_proofs(data, step_cap, work_cap)
                } else {
                    ReasonerSession::open(data, step_cap, work_cap)
                }
            })
            .map_err(PyValueError::new_err)?;
        Ok(Self { session })
    }

    /// Whether this session records proof terms — `True` only for `Reasoner(..., proofs=True)`.
    ///
    /// The default is `False`, and it is the whole opt-in: a session nobody asked to record
    /// runs the same searches it always ran and keeps no traces.
    #[getter]
    fn records_proofs(&self) -> bool {
        self.session.records_proofs()
    }

    /// Answer `service` about `argument`, with its proof. See `purrdf.entail.prove`.
    ///
    /// Returns `(answer, certificate, proof)`, all `str`. Raises `ValueError` on a session
    /// that records nothing: construct it with `proofs=True`.
    #[pyo3(signature = (service, argument = ""))]
    fn prove(
        &mut self,
        py: Python<'_>,
        service: &str,
        argument: &str,
    ) -> PyResult<(String, String, String)> {
        let answer = py
            .detach(|| self.session.prove(service, argument))
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_proved_parts())
    }

    /// Is the knowledge base consistent? See `purrdf.entail.consistency`.
    fn consistency(&mut self, py: Python<'_>) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.consistency())
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// The entailed subsumption hierarchy. See `purrdf.entail.classify`.
    fn classify(&mut self, py: Python<'_>) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.classify())
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// The entailed types of the named individuals. See `purrdf.entail.realize`.
    fn realize(&mut self, py: Python<'_>) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.realize())
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// The individuals entailed to be instances of `class_`. See `purrdf.entail.instances`.
    #[pyo3(signature = (class_))]
    fn instances(&mut self, py: Python<'_>, class_: &str) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.instances(class_))
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// Does the ontology entail `axiom`? See `purrdf.entail.entails`.
    #[pyo3(signature = (axiom))]
    fn entails(&mut self, py: Python<'_>, axiom: &str) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.entails(axiom))
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// Which OWL 2 profiles the ontology is provably in. See `purrdf.entail.profile`.
    ///
    /// Purely syntactic: this never builds a knowledge base, so it answers even for an
    /// ontology whose other services would raise.
    fn profile(&self, py: Python<'_>) -> (String, String) {
        py.detach(|| self.session.profile()).into_parts()
    }

    /// A module for `signature`. See `purrdf.entail.extract_module`.
    #[pyo3(signature = (signature, method))]
    fn extract_module(
        &self,
        py: Python<'_>,
        signature: &str,
        method: &str,
    ) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.extract_module(signature, method))
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// A justification for `axiom`. See `purrdf.entail.justify`.
    #[pyo3(signature = (axiom))]
    fn justify(&self, py: Python<'_>, axiom: &str) -> PyResult<(String, String)> {
        let answer = py
            .detach(|| self.session.justify(axiom))
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// Why `conclusion` holds under `regime`. See `purrdf.entail.explain_conclusion`.
    #[pyo3(signature = (regime, conclusion))]
    fn explain_conclusion(
        &self,
        py: Python<'_>,
        regime: &Bound<'_, PyAny>,
        conclusion: &str,
    ) -> PyResult<(String, String)> {
        let name = regime_name(native_regime(regime)?);
        let answer = py
            .detach(|| self.session.explain_conclusion(name, conclusion))
            .map_err(PyValueError::new_err)?;
        Ok(answer.into_parts())
    }

    /// The session's shape — how big the document is and whether it has reasoned yet.
    fn __repr__(&self) -> String {
        format!("{:?}", self.session)
    }
}

/// Register the entailment-regime surface on a Python module.
///
/// Called by the unified `purrdf_native` cdylib to populate the
/// `purrdf_native.entail` submodule, which the package shim re-attaches as
/// `purrdf.entail` (mirroring [`crate::py_shex::register`]).
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRegime>()?;
    m.add_class::<PyReasoner>()?;
    m.add_function(wrap_pyfunction!(materialize, m)?)?;
    m.add_function(wrap_pyfunction!(materialize_nt, m)?)?;
    m.add_function(wrap_pyfunction!(rules, m)?)?;
    m.add_function(wrap_pyfunction!(implemented_rules, m)?)?;
    m.add_function(wrap_pyfunction!(extensions, m)?)?;
    m.add_function(wrap_pyfunction!(consistency, m)?)?;
    m.add_function(wrap_pyfunction!(classify, m)?)?;
    m.add_function(wrap_pyfunction!(realize, m)?)?;
    m.add_function(wrap_pyfunction!(instances, m)?)?;
    m.add_function(wrap_pyfunction!(entails, m)?)?;
    m.add_function(wrap_pyfunction!(profile, m)?)?;
    m.add_function(wrap_pyfunction!(extract_module, m)?)?;
    m.add_function(wrap_pyfunction!(justify, m)?)?;
    m.add_function(wrap_pyfunction!(explain_conclusion, m)?)?;
    m.add_function(wrap_pyfunction!(certain_answers, m)?)?;
    m.add_function(wrap_pyfunction!(graph_entails, m)?)?;
    m.add_function(wrap_pyfunction!(verify_entailment, m)?)?;
    m.add_function(wrap_pyfunction!(prove, m)?)?;
    m.add_function(wrap_pyfunction!(check_proof, m)?)?;
    m.add_function(wrap_pyfunction!(proof_services, m)?)?;
    Ok(())
}

// `skip_from_py_object` + this hand-written impl, rather than `from_py_object`.
//
// `#[pyclass(from_py_object)]` generates exactly this impl with
// `Clone::clone(&*guard)` as the body. `PyRegime` is `Copy`, so that clone is a copy
// wearing a `.clone()` -- a real `clippy::clone_on_copy`, and one no `#[allow]`
// on the enum can reach, because the macro emits the impl as a SIBLING item
// outside the enum's attribute scope. Writing the impl out and dereferencing
// through `Copy` removes the clone at its source instead of hiding it.
//
// This is a transcription of the pyo3 0.29 expansion, not a redesign: same
// `Error` type, same `PyClassGuard` extraction, same error path. The
// `INPUT_TYPE` associated const the macro can also emit is gated on pyo3's
// `experimental-inspect` feature, which is off here, so there is nothing else to
// carry over. The Python-visible behaviour is unchanged.
impl<'a, 'py> FromPyObject<'a, 'py> for PyRegime {
    type Error = pyo3::pyclass::PyClassGuardError<'a, 'py>;

    fn extract(
        obj: Borrowed<'a, 'py, PyAny>,
    ) -> Result<Self, <Self as FromPyObject<'a, 'py>>::Error> {
        Ok(*obj.extract::<PyClassGuard<'_, Self>>()?)
    }
}

#[cfg(test)]
mod tests {
    use purrdf_validate::regime::PROGRAM_REGIME_NAMES;

    use super::*;

    /// A normative RIF-in-XML rule document: `?x a ex:A` ⟹ `?x a ex:B`.
    ///
    /// `rif` is the one regime whose calculus is the CALLER's, so it is the one
    /// spelling whose `program` argument is a document rather than the empty string.
    const RIF_PROGRAM: &str = "<Document xmlns=\"http://www.w3.org/2007/rif#\"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/A</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/B</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>";

    /// The PyO3 host's leg of the CROSS-HOST assertion for the PROOF surface.
    ///
    /// The `purrdf-validate` test, the C-ABI test and the WASM host's
    /// `entailCheckProofGoldenVectors` call this SAME checker over the SAME committed
    /// artifact. A rendered proof carries the proof term's own canonical bytes, so a host
    /// producing different bytes has produced a different proof TERM — and a consumer's
    /// pinned digest would have moved under them.
    #[test]
    fn the_dl_proof_golden_vector_matches() {
        purrdf_validate::regime::check_dl_proof_golden_vectors()
            .expect("the DL proof golden vector");
    }

    /// The PyO3 host's leg of the availability assertion: an answer nobody asked to record
    /// is never presentable as a verified one.
    #[test]
    fn an_absent_proof_is_never_presented_as_a_verified_one() {
        purrdf_validate::regime::check_absent_proof_is_not_verifiable()
            .expect("the absent-proof refusal");
    }

    /// The service set this module exposes is `purrdf-validate`'s, not a copy of it.
    #[test]
    fn the_proof_service_set_is_the_boundarys() {
        assert_eq!(proof_services(), PROOF_SERVICE_NAMES.to_vec());
        assert_eq!(
            proof_services().len(),
            7,
            "seven services carry a proof term"
        );
    }

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

    /// EVERY accepted spelling materializes through the string boundary this module
    /// wraps, and the two surfaces agree on which one takes a rule document.
    ///
    /// Falsifiable against the behaviour this replaced: `owl-direct` and `rif` were
    /// refused here, and this test asserted that the dataset path's refusal string was
    /// byte-identical to the string boundary's. There is no refusal to compare now, so
    /// what is compared instead is the ACCEPTANCE.
    #[test]
    fn every_regime_spelling_materializes_through_the_boundary() {
        const SCHEMA: &str = "<http://example.org/x> \
                              <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                              <http://example.org/A> .\n";
        for name in REGIME_NAMES {
            let native = parse_regime(name).expect("an accepted spelling");
            let program = if name == "rif" { RIF_PROGRAM } else { "" };
            let closed = materialize_to_nquads_string(name, SCHEMA, program)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(closed.report().contains(&format!("\nregime {name}\n")));
            // …and the dataset path reaches the same plan for the same spelling.
            let rules = regime_rule_set(native, name, program).expect("a legal program");
            assert_eq!(regime_plan(native, &rules).regime(), native);
        }
        // Exactly one regime takes a rule document, and both paths read that from the
        // same constant rather than re-typing it.
        assert_eq!(PROGRAM_REGIME_NAMES, ["rif"]);
    }
}
