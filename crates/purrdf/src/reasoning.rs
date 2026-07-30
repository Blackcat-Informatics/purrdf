// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entailment-aware SPARQL orchestration over the native PurRDF engines.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_datalog::seminaive::BudgetReport;
use purrdf_entail::{
    EntailError, Materialization, QNode, QTriple, ReasoningReport, Regime, RuleSet,
    materialize_combined,
};
use purrdf_rdf::{
    RdfDataset, RdfDiagnostic, RdfTextDirection, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    BaseDirection, GraphPattern, Literal, NamedNodePattern, Query, TermPattern,
};
use purrdf_sparql_eval::NativeSparqlEngine;

/// Entailment behavior applied before evaluating one SPARQL query.
///
/// Every W3C `sparql:entailmentRegime` this repository implements is here, because a
/// regime that is materializable everywhere else and unreachable from the query surface is
/// a capability the caller cannot use: `entailment/D` is a regime of the SPARQL 1.1
/// Entailment Regimes recommendation exactly as `entailment/RDFS` is, and
/// [`purrdf_entail::materialize`] serves it like any other rule table.
#[derive(Debug, Clone, Copy)]
pub enum QueryEntailment<'a> {
    /// Query asserted data directly.
    Simple,
    /// Materialize RDF entailment.
    Rdf,
    /// Materialize RDFS entailment.
    Rdfs,
    /// Materialize OWL 2 RL entailment.
    OwlRl,
    /// Materialize `entailment/D` — Simple entailment plus the five `dt-*` rules of
    /// OWL 2 Profiles §4.3 Table 8.
    D,
    /// Perform query-directed OWL Direct-Semantics augmentation.
    OwlDirect,
    /// Materialize the supplied RIF-Core rule set.
    Rif(&'a RuleSet),
}

/// Failure from entailment-aware query preparation or evaluation.
#[derive(Debug)]
pub enum ReasoningError {
    /// SPARQL parsing or evaluation failed.
    Query(RdfDiagnostic),
    /// Entailment or rule materialization failed.
    Entailment(EntailError),
}

impl std::fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(error) => write!(f, "SPARQL query failed: {error}"),
            Self::Entailment(error) => write!(f, "entailment failed: {error}"),
        }
    }
}

impl std::error::Error for ReasoningError {}

impl From<RdfDiagnostic> for ReasoningError {
    fn from(value: RdfDiagnostic) -> Self {
        Self::Query(value)
    }
}

impl From<EntailError> for ReasoningError {
    fn from(value: EntailError) -> Self {
        Self::Entailment(value)
    }
}

/// Evaluate SPARQL under an explicit native entailment regime, and say what the reasoner
/// did.
///
/// Returns the query's answer AND the [`ReasoningReport`] of the run that produced the
/// dataset it was answered over.
///
/// # The certificate travels with the answer
///
/// A SPARQL result set carries no reasoning metadata, and this function used to take that
/// as permission to drop the report: the closure was computed, the evidence was bound to
/// `_`, and the caller received rows with no way to learn that "OWL Direct-Semantics
/// answers" had been computed over an ontology holding an `owl:propertyChainAxiom` the
/// reverse mapping could not read, or that most of their input sat in named graphs the lane
/// never opened. That is not a constraint of [`SparqlResult`]; it is a missing return
/// value, and a pair is the smallest honest fix. A caller who does not want it binds
/// `(result, _)`.
///
/// [`QueryEntailment::Simple`] asks no reasoner to run, so its report is the one
/// [`purrdf_entail::materialize`] returns for [`Regime::Simple`] — an identity closure has
/// no rule table, meets no boundary and consumes no ceiling — assembled directly rather
/// than by copying the dataset to obtain it. The equality is asserted in this module's
/// tests rather than asserted in prose.
///
/// # Errors
///
/// Returns [`ReasoningError::Query`] for SPARQL failures and
/// [`ReasoningError::Entailment`] for malformed or inconsistent knowledge bases.
pub fn query_with_entailment(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    request: SparqlRequest<'_>,
    entailment: QueryEntailment<'_>,
) -> Result<(SparqlResult, ReasoningReport), ReasoningError> {
    // Parse first so invalid queries fail before potentially expensive closure work.
    // OWL Direct also inspects this same cached plan, avoiding a second parse/cache lookup.
    let prepared_query = engine.prepare_query(request.query, request.base_iri)?;
    // Every lane hands back a `ReasoningReport` alongside the closure, and every one of
    // them is carried out of this function rather than dropped at this call site.
    // `collect_query_bgp` is bound outside the match because the OWL-Direct plan BORROWS
    // it; it is computed for that mode alone.
    let pattern = match entailment {
        QueryEntailment::OwlDirect => collect_query_bgp(&prepared_query.query),
        _ => Vec::new(),
    };
    // Populated only when the OWL-Direct lane answered through the COMBINED APPROACH
    // (`purrdf_entail::materialize_combined`) rather than the whole-vocabulary
    // augmentation: the set of blank terms its restricted chase minted as existential
    // witnesses, which the filter below removes from any DISTINGUISHED (projected)
    // variable's bindings before the answer is returned. A witness is not a certain
    // answer for a distinguished variable — the regime draws its answers from the
    // scoping graph, and a minted witness is not in it.
    let mut combined_surrogates = None;
    // ONE call, seven modes. `purrdf_entail::materialize` is total over
    // `Materialization`, so this lane no longer splits into "the regimes that
    // materialize" and "the two that need their own entry point".
    let (prepared, report) = match entailment {
        QueryEntailment::Simple => (Arc::clone(dataset), simple_report()),
        QueryEntailment::Rdf => purrdf_entail::materialize(dataset, Materialization::Rdf)?,
        QueryEntailment::Rdfs => purrdf_entail::materialize(dataset, Materialization::Rdfs)?,
        QueryEntailment::OwlRl => purrdf_entail::materialize(dataset, Materialization::OwlRl)?,
        QueryEntailment::D => purrdf_entail::materialize(dataset, Materialization::D)?,
        QueryEntailment::OwlDirect => match materialize_combined(dataset, &pattern)? {
            // The ontology's TBox is in the certified Horn fragment: answer through the
            // combined approach (restricted-chase witnesses for the anonymous part,
            // filtered below) rather than the whole-vocabulary augmentation, which is
            // silently incomplete for a query's non-distinguished variable.
            Some(combined) => {
                combined_surrogates = Some(combined.surrogates);
                (combined.dataset, combined.report)
            }
            // Outside that fragment: the pre-existing augmentation, boundary and all,
            // exactly as before.
            None => purrdf_entail::materialize(dataset, Materialization::OwlDirect(&pattern))?,
        },
        QueryEntailment::Rif(ruleset) => {
            purrdf_entail::materialize(dataset, Materialization::Rif(ruleset))?
        }
    };
    let mut result = engine.query_prepared(&prepared, &prepared_query, request.substitutions)?;
    if let Some(surrogates) = combined_surrogates {
        let distinguished = distinguished_variables(&prepared_query.query);
        withhold_surrogate_bindings(&mut result, &surrogates, distinguished.as_ref());
    }
    Ok((result, report))
}

/// The query's DISTINGUISHED (projected) variable names, or `None` if the query has no
/// projection to read one from (`ASK`, `CONSTRUCT`, `DESCRIBE`). [`withhold_surrogate_bindings`]
/// reads that `None` as "every variable is distinguished" — the conservative, maximally
/// filtering reading, and the correct one: none of those three query forms lets a caller
/// read a query variable's binding the way `SELECT` does, but a `CONSTRUCT` template or an
/// `ASK` verdict can still be built FROM one, so a witness reaching any of their variables
/// must be withheld rather than assumed harmless.
fn distinguished_variables(query: &Query) -> Option<BTreeSet<String>> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    };
    find_projection(pattern)
}

/// The variable list of the first [`GraphPattern::Project`] reached by peeling off solution
/// modifiers — `SELECT`'s own root pattern is exactly that, wrapped by
/// `Slice`/`OrderBy`/`Distinct`/`Reduced`/`Group` and the like. `None` if none is found
/// (there is no `SELECT` projection to read).
fn find_projection(pattern: &GraphPattern) -> Option<BTreeSet<String>> {
    match pattern {
        GraphPattern::Project { variables, .. } => {
            Some(variables.iter().map(|v| v.as_str().to_owned()).collect())
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => find_projection(inner),
        _ => None,
    }
}

/// Drop every solution row that binds a DISTINGUISHED variable to a chase-minted witness.
///
/// `distinguished`'s `None` reading is "every variable is distinguished" (see
/// [`distinguished_variables`]), which is the conservative default for `ASK`/`CONSTRUCT`/
/// `DESCRIBE` — it filters at least as much as the precise answer would, never less, so it
/// can never let a witness leak.
///
/// A no-op for anything but [`SparqlResult::Solutions`]: `ASK` and `CONSTRUCT`/`DESCRIBE`
/// results are booleans and graphs respectively, and a witness reaching a `CONSTRUCT`
/// template is a property of the template, not of a variable binding this filter can name.
fn withhold_surrogate_bindings(
    result: &mut SparqlResult,
    surrogates: &BTreeSet<String>,
    distinguished: Option<&BTreeSet<String>>,
) {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        return;
    };
    let is_distinguished = |name: &str| match distinguished {
        Some(names) => names.contains(name),
        None => true,
    };
    let is_surrogate = |value: &TermValue| match value {
        TermValue::Blank { label, .. } => surrogates.contains(label),
        TermValue::Iri(_) | TermValue::Literal { .. } | TermValue::Triple { .. } => false,
    };
    rows.retain(|row| {
        !row.iter()
            .zip(variables.iter())
            .any(|(cell, name)| is_distinguished(name) && cell.as_ref().is_some_and(is_surrogate))
    });
}

/// The report for the identity closure — what `materialize(ds, Materialization::Simple)` returns.
///
/// Assembled rather than obtained by calling it, because that call COPIES the dataset to
/// produce a closure this lane already has as an `Arc`. Every field is a property of the
/// regime and not of the data: `Simple` has no rule table (so nothing can be missing), it
/// copies every quad of every graph faithfully (so it meets no boundary), and it evaluates
/// no program (so it consumes none of the three ceilings), and it invents no term (so it
/// has no termination obligation to discharge). The contract hash is derived inside
/// [`ReasoningReport::new`] from the regime itself.
fn simple_report() -> ReasoningReport {
    ReasoningReport::new(
        Regime::Simple,
        Vec::new(),
        Vec::new(),
        BudgetReport::new(0, 0, 0),
        None,
        0,
        None,
    )
}

fn collect_query_bgp(query: &Query) -> Vec<QTriple> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    };
    let mut triples = Vec::new();
    collect_bgp(pattern, &mut triples);
    triples
}

fn collect_bgp(pattern: &GraphPattern, output: &mut Vec<QTriple>) {
    match pattern {
        GraphPattern::Bgp { patterns } => output.extend(patterns.iter().filter_map(|pattern| {
            Some(QTriple {
                s: term_to_qnode(&pattern.subject)?,
                p: named_node_pattern_to_qnode(&pattern.predicate),
                o: term_to_qnode(&pattern.object)?,
            })
        })),
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            collect_bgp(left, output);
            collect_bgp(right, output);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => collect_bgp(inner, output),
        GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}
    }
}

fn term_to_qnode(term: &TermPattern) -> Option<QNode> {
    Some(match term {
        TermPattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
        TermPattern::NamedNode(node) => QNode::Term(TermValue::iri(node.as_str())),
        TermPattern::BlankNode(node) => QNode::Term(TermValue::blank(node.as_str())),
        TermPattern::Literal(literal) => QNode::Term(literal_to_term_value(literal)),
        TermPattern::Triple(_) => return None,
    })
}

fn named_node_pattern_to_qnode(pattern: &NamedNodePattern) -> QNode {
    match pattern {
        NamedNodePattern::NamedNode(node) => QNode::Term(TermValue::iri(node.as_str())),
        NamedNodePattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
    }
}

fn literal_to_term_value(literal: &Literal) -> TermValue {
    match literal.language() {
        Some(language) => TermValue::Literal {
            lexical_form: literal.value().to_owned(),
            datatype: literal.datatype().as_str().to_owned(),
            language: Some(language.to_ascii_lowercase()),
            direction: literal.direction().map(|direction| match direction {
                BaseDirection::Ltr => RdfTextDirection::Ltr,
                BaseDirection::Rtl => RdfTextDirection::Rtl,
            }),
        },
        None => TermValue::typed_literal(literal.value(), literal.datatype().as_str()),
    }
}

#[cfg(test)]
mod tests {
    use purrdf_entail::{Atom, RifTerm, Rule, RuleSet};
    use purrdf_rdf::{BlankScope, RdfDatasetBuilder, TermValue};

    use super::*;

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    fn hierarchy() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let cat = builder.intern_iri("https://example.org/Cat");
        let animal = builder.intern_iri("https://example.org/Animal");
        let lillith = builder.intern_iri("https://example.org/lillith");
        let rdf_type = builder.intern_iri(RDF_TYPE);
        let subclass = builder.intern_iri(RDFS_SUBCLASS);
        builder.push_quad(cat, subclass, animal, None);
        builder.push_quad(lillith, rdf_type, cat, None);
        builder.freeze().unwrap()
    }

    /// Run the fixture ASK under `mode`, returning both halves of the answer.
    fn ask_reported(mode: QueryEntailment<'_>) -> (SparqlResult, ReasoningReport) {
        let query = "ASK { <https://example.org/lillith> a <https://example.org/Animal> }";
        query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            mode,
        )
        .unwrap()
    }

    fn ask(mode: QueryEntailment<'_>) -> SparqlResult {
        ask_reported(mode).0
    }

    #[test]
    fn rdfs_query_sees_derived_type() {
        assert!(matches!(
            ask(QueryEntailment::Rdfs),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn owl_rl_query_sees_derived_type() {
        assert!(matches!(
            ask(QueryEntailment::OwlRl),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn owl_direct_query_uses_the_query_bgp() {
        assert!(matches!(
            ask(QueryEntailment::OwlDirect),
            SparqlResult::Boolean(true)
        ));
    }

    /// `entailment/D` IS SELECTABLE, and it is the regime it says it is.
    ///
    /// It is a W3C SPARQL entailment regime and `materialize` serves it like any other rule
    /// table; a query surface that could not name it was withholding a capability the
    /// library has.
    #[test]
    fn the_d_regime_is_reachable_from_the_query_surface() {
        let query = "ASK { <http://www.w3.org/2001/XMLSchema#integer> a \
                     <http://www.w3.org/2000/01/rdf-schema#Datatype> }";
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::D,
        )
        .unwrap();
        // `dt-type1` is premise-free, so every supported datatype is typed in every closure.
        assert!(matches!(result, SparqlResult::Boolean(true)));
        assert_eq!(report.regime(), Regime::D);
        // Simple entailment does NOT derive it, so the answer is the regime's and not the
        // data's.
        assert!(matches!(
            query_with_entailment(
                &NativeSparqlEngine::new(),
                &hierarchy(),
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryEntailment::Simple,
            )
            .unwrap()
            .0,
            SparqlResult::Boolean(false)
        ));
    }

    /// EVERY MODE CARRIES ITS CERTIFICATE OUT, and each names its own regime.
    #[test]
    fn every_mode_returns_the_report_of_the_run_it_made() {
        let rules = RuleSet::new();
        for (mode, regime) in [
            (QueryEntailment::Simple, Regime::Simple),
            (QueryEntailment::Rdf, Regime::Rdf),
            (QueryEntailment::Rdfs, Regime::Rdfs),
            (QueryEntailment::OwlRl, Regime::OwlRl),
            (QueryEntailment::D, Regime::D),
            (QueryEntailment::OwlDirect, Regime::OwlDirect),
            (QueryEntailment::Rif(&rules), Regime::Rif),
        ] {
            let (_, report) = ask_reported(mode);
            assert_eq!(report.regime(), regime, "{regime:?}");
        }
    }

    /// The `Simple` report is EXACTLY the one `materialize` returns for that regime —
    /// assembled without paying for the copy, and checked rather than claimed.
    #[test]
    fn the_simple_report_equals_the_materialized_one() {
        let dataset = hierarchy();
        let (_, from_materialize) =
            purrdf_entail::materialize(&dataset, Materialization::Simple).expect("simple");
        let (_, from_query) = ask_reported(QueryEntailment::Simple);
        assert_eq!(format!("{from_query:?}"), format!("{from_materialize:?}"));
    }

    #[test]
    fn rdf_query_types_predicates_as_properties() {
        let query = format!(
            "ASK {{ <{RDFS_SUBCLASS}> a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }}"
        );
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::Rdf,
        )
        .unwrap();
        assert!(matches!(result, SparqlResult::Boolean(true)));
        assert_eq!(report.regime(), Regime::Rdf);
    }

    #[test]
    fn simple_query_does_not_invent_closure() {
        assert!(matches!(
            ask(QueryEntailment::Simple),
            SparqlResult::Boolean(false)
        ));
    }

    #[test]
    fn rif_query_sees_rule_derived_fact() {
        let mut rules = RuleSet::new();
        rules.push_rule(Rule {
            body: vec![Atom {
                s: RifTerm::Var("subject".to_owned()),
                p: RifTerm::Const(TermValue::iri(RDF_TYPE)),
                o: RifTerm::Const(TermValue::iri("https://example.org/Cat")),
            }],
            head: vec![Atom {
                s: RifTerm::Var("subject".to_owned()),
                p: RifTerm::Const(TermValue::iri(RDF_TYPE)),
                o: RifTerm::Const(TermValue::iri("https://example.org/Animal")),
            }],
        });
        assert!(matches!(
            ask(QueryEntailment::Rif(&rules)),
            SparqlResult::Boolean(true)
        ));
    }

    // ── The combined approach: a non-distinguished variable, answered correctly ────────

    const COMBINED_NS: &str = "https://example.org/combined#";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
    const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";

    /// `A ⊑ ∃r.B`, `a : A` — the classic shape a query-independent, whole-vocabulary
    /// augmentation cannot answer correctly for a non-distinguished variable, because no
    /// NAMED individual need be `r`-related to anything: the axiom only entails that SOME
    /// element is.
    fn some_values_from_ontology() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let class = b.intern_iri(OWL_CLASS);
        let subclass_of = b.intern_iri(RDFS_SUBCLASS);
        let a = b.intern_iri(&format!("{COMBINED_NS}A"));
        let big_b = b.intern_iri(&format!("{COMBINED_NS}B"));
        let r = b.intern_iri(&format!("{COMBINED_NS}r"));
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
        let restriction_class = b.intern_iri(OWL_RESTRICTION);
        let on_property = b.intern_iri(OWL_ON_PROPERTY);
        let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(restriction, ty, restriction_class, None);
        b.push_quad(restriction, on_property, r, None);
        b.push_quad(restriction, some_values_from, big_b, None);
        b.push_quad(a, subclass_of, restriction, None);
        b.push_quad(little_a, ty, a, None);
        b.freeze().expect("freeze")
    }

    /// HALF ONE: `a` IS a certain answer of `SELECT ?x WHERE { ?x r ?y . ?y a B }`, even
    /// though no triple — asserted or in the whole-vocabulary augmentation — ever states
    /// that any named individual is `r`-related to anything. Only the combined approach's
    /// restricted-chase witness makes the match possible.
    #[test]
    fn the_combined_approach_finds_the_certain_answer_a_whole_vocabulary_augmentation_misses() {
        let query = format!("SELECT ?x WHERE {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}");
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &some_values_from_ontology(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            panic!("expected a solution sequence");
        };
        let x = variables
            .iter()
            .position(|v| v == "x")
            .expect("?x is projected");
        let bindings: Vec<&TermValue> = rows
            .iter()
            .map(|row| row[x].as_ref().expect("?x is bound"))
            .collect();
        assert_eq!(
            bindings,
            vec![&TermValue::iri(format!("{COMBINED_NS}a"))],
            "a is a certain answer: every model has SOME r-successor of a typed B"
        );
    }

    /// HALF TWO: no chase-minted Skolem surrogate ever leaks as a binding for a
    /// DISTINGUISHED variable. `?y` is now the projected variable, and the only "value"
    /// `?y` could take is the witness the restricted chase invented for the existential —
    /// which is not a certain answer (the axiom does not name which element it is), so the
    /// solution set must be EMPTY rather than surfacing the internal witness.
    #[test]
    fn no_chase_witness_leaks_as_a_binding_for_a_distinguished_variable() {
        let query = format!(
            "SELECT ?y WHERE {{ <{COMBINED_NS}a> <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}"
        );
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &some_values_from_ontology(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        let SparqlResult::Solutions { rows, .. } = result else {
            panic!("expected a solution sequence");
        };
        assert!(
            rows.is_empty(),
            "a chase-minted witness must never bind the distinguished ?y: {rows:?}"
        );
    }

    /// The wiring itself: an ontology outside the combined approach's Horn fragment (here,
    /// `owl:equivalentClass`) still answers through the pre-existing whole-vocabulary
    /// augmentation, unchanged.
    #[test]
    fn an_ontology_outside_the_horn_fragment_still_uses_the_whole_vocabulary_augmentation() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let class = b.intern_iri(OWL_CLASS);
        let equiv = b.intern_iri("http://www.w3.org/2002/07/owl#equivalentClass");
        let a = b.intern_iri(&format!("{COMBINED_NS}A"));
        let big_b = b.intern_iri(&format!("{COMBINED_NS}B"));
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(a, equiv, big_b, None);
        b.push_quad(little_a, ty, a, None);
        let dataset = b.freeze().expect("freeze");

        let query = format!("ASK {{ <{COMBINED_NS}a> a <{COMBINED_NS}B> }}");
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        assert!(matches!(result, SparqlResult::Boolean(true)));
    }
}
