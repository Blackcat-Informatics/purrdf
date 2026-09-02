// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The property-function seam, end to end, from the vantage a host has.
//!
//! One query text is carried through every stage the seam touches — parse under a
//! configured namespace, the algebra node the parse produced, evaluation against a
//! host-injected relation, the answers — and then through the serializer and back,
//! byte-exactly. Nothing here reaches into the crate: a seam whose stages only line up
//! from inside is a seam a host cannot use.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    GraphPattern, Query, SparqlParser, TermPattern, Variable, pattern_to_select_query,
};
use purrdf_sparql_eval::{
    BindingPattern, ChargePoint, EvalError, GovernedOutcome, GovernedUpdateOutcome, GovernorState,
    MemoryRelation, NativeSparqlEngine, NodeCharges, ParserOptions, PfArgs, PfArity, PfCursor,
    PfRow, PropertyFunction, PropertyFunctionRegistry, QueryGovernors, QueryOptions,
    ResourceDimension, TrippedGovernor, Volatility,
};

/// The namespace this host configured. PurRDF mints none: without this line in the
/// parser options, `rel:memberOf` is an ordinary predicate and the query below reads
/// the graph instead of the relation.
const REL_NS: &str = "https://example.org/rel/";

/// The data namespace of the fixture terms.
const EX: &str = "https://example.org/d/";

/// The one query text every stage below is given.
const QUERY: &str = "PREFIX rel: <https://example.org/rel/>\n\
                     SELECT ?person ?team WHERE { ?person rel:memberOf ?team }\n";

fn options() -> ParserOptions {
    ParserOptions {
        extension_fn_namespaces: Vec::new(),
        property_fn_namespaces: vec![REL_NS.to_owned()],
        property_fn_iris: Vec::new(),
    }
}

/// The host's relation: three (person, team) pairs, held in host memory and reachable
/// from no graph.
fn relations() -> PropertyFunctionRegistry {
    let iri = |local: &str| TermValue::iri(format!("{EX}{local}"));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        format!("{REL_NS}memberOf"),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![
                    vec![iri("ada"), iri("alpha")],
                    vec![iri("brian"), iri("alpha")],
                    vec![iri("chen"), iri("beta")],
                ],
            )
            .expect("every row is two values wide"),
        ),
    );
    registry
}

/// A dataset holding one unrelated triple: the answers below come from the relation,
/// and this is what makes that observable rather than merely stated.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}unrelated"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let o = builder.intern_iri(&format!("{EX}o"));
    builder.push_quad(s, p, o, None);
    builder.freeze().expect("freeze fixture")
}

fn variable(name: &str) -> TermPattern {
    TermPattern::Variable(Variable::new(name.to_owned()))
}

/// The call node in the parsed query, with the shape the parser is contracted to
/// produce: the call joins through a `Lateral` onto whatever was written before it,
/// which for a lone call is the empty basic graph pattern (the identity table).
fn call_of(query: &Query) -> &purrdf_sparql_algebra::PropertyFunctionCall {
    let Query::Select { pattern, .. } = query else {
        panic!("the acceptance query is a SELECT");
    };
    let GraphPattern::Project { inner, .. } = pattern else {
        panic!("a SELECT's algebra root is a Project, got {pattern:?}");
    };
    let GraphPattern::Lateral { left, right } = &**inner else {
        panic!("a call joins through a Lateral, got {inner:?}");
    };
    assert!(
        matches!(&**left, GraphPattern::Bgp { patterns } if patterns.is_empty()),
        "a call written first is driven by the identity table, got {left:?}"
    );
    let GraphPattern::PropertyFunction(call) = &**right else {
        panic!("the Lateral's right operand is the call, got {right:?}");
    };
    call
}

/// Text → parse → algebra → evaluation → answers, in one pass.
#[test]
fn a_configured_predicate_parses_to_a_call_and_answers_from_the_injected_relation() {
    // 1. Parse, with the namespace configured.
    let query = SparqlParser::new()
        .parse_query_with(QUERY, &options())
        .expect("the query parses under the configured namespace");

    // 2. The algebra carries the call, with the predicate IRI byte-exact and the two
    //    argument vectors in written order.
    let call = call_of(&query);
    assert_eq!(call.iri, format!("{REL_NS}memberOf"));
    assert_eq!(call.subject_args, vec![variable("person")]);
    assert_eq!(call.object_args, vec![variable("team")]);

    // 3. Evaluate against the injected relation. The engine derives the parse-time
    //    namespace from the registry itself, so a host that registers a relation does
    //    not also have to configure the parser for it.
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: &relations(),
                ..QueryOptions::EMPTY
            },
        )
        .expect("the call resolves and evaluates");

    // 4. The answers are the relation's rows, in the relation's emission order.
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("a SELECT returns solutions");
    };
    assert_eq!(variables, vec!["person".to_owned(), "team".to_owned()]);
    let rendered: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(TermValue::as_iri)
                        .expect("every cell is a bound IRI")
                        .to_owned()
                })
                .collect()
        })
        .collect();
    assert_eq!(
        rendered,
        vec![
            vec![format!("{EX}ada"), format!("{EX}alpha")],
            vec![format!("{EX}brian"), format!("{EX}alpha")],
            vec![format!("{EX}chen"), format!("{EX}beta")],
        ]
    );
}

/// The same query, serialized out of the algebra and read back in: the round trip is
/// exact in both directions — the algebra survives it, and so do the bytes.
#[test]
fn the_call_serializes_and_re_parses_byte_exactly() {
    let query = SparqlParser::new()
        .parse_query_with(QUERY, &options())
        .expect("parse");
    let Query::Select { pattern, .. } = &query else {
        panic!("the acceptance query is a SELECT");
    };
    let GraphPattern::Project { inner, .. } = pattern else {
        panic!("a SELECT's algebra root is a Project");
    };

    let serialized = pattern_to_select_query(inner);
    assert!(
        serialized.contains(&format!("<{REL_NS}memberOf>")),
        "the emitted text carries the predicate IRI as written, never a fabricated \
         prefix: {serialized}"
    );

    let reparsed = SparqlParser::new()
        .parse_query_with(&serialized, &options())
        .expect("the serialized text parses under the same options");
    let Query::Select {
        pattern: reparsed_pattern,
        ..
    } = &reparsed
    else {
        panic!("the serialized text is a SELECT");
    };
    let GraphPattern::Project {
        inner: reparsed_inner,
        ..
    } = reparsed_pattern
    else {
        panic!("a SELECT's algebra root is a Project");
    };
    assert_eq!(
        &**reparsed_inner, &**inner,
        "the algebra survives the round trip"
    );
    assert_eq!(
        pattern_to_select_query(reparsed_inner),
        serialized,
        "and so do the bytes"
    );
    assert_eq!(
        call_of(&reparsed).iri,
        call_of(&query).iri,
        "the predicate IRI is byte-exact on the way back"
    );
}

/// Without the namespace, the very same text is an ordinary triple pattern reading the
/// graph — the seam is configuration, and there is no default that turns it on.
#[test]
fn the_same_text_without_the_namespace_is_an_ordinary_triple_pattern() {
    let query = SparqlParser::new()
        .parse_query(QUERY)
        .expect("the text is valid SPARQL either way");
    let Query::Select { pattern, .. } = &query else {
        panic!("the acceptance query is a SELECT");
    };
    let GraphPattern::Project { inner, .. } = pattern else {
        panic!("a SELECT's algebra root is a Project");
    };
    let GraphPattern::Bgp { patterns } = &**inner else {
        panic!("with no namespace configured the body is a basic graph pattern, got {inner:?}");
    };
    assert_eq!(patterns.len(), 1);

    // And it answers from the graph, which holds no such triple.
    let result = NativeSparqlEngine::new()
        .query(
            &dataset(),
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("an unconfigured engine evaluates it as data");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions");
    };
    assert!(
        rows.is_empty(),
        "the dataset holds no rel:memberOf triple: {rows:?}"
    );
}

/// THE data-predicate-hijack regression (GAP-3): registering a relation must not
/// turn a merely prefix-sharing, unregistered, LONGER predicate into a
/// hard-erroring property-function call.
///
/// `NativeSparqlEngine::prepare_for` derives parse-time recognition from the
/// registry (see the doc comment there): it used to push each registered
/// relation's exact IRI into the parser's PREFIX namespace set, so registering
/// `{REL_NS}a` made the ordinary, unrelated data predicate `{REL_NS}ab` parse as
/// an (unregistered) property-function call and hard-error — a previously
/// working query breaking with a diagnostic that names the wrong cause. A
/// registry's keys are exact IRIs, not namespaces, and exact match is the only
/// rule that respects that.
#[test]
fn registering_a_relation_does_not_hijack_a_longer_sibling_data_predicate() {
    let short_iri = format!("{REL_NS}a");
    let long_predicate = format!("{REL_NS}ab");

    // Register a relation under the SHORT IRI only.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        short_iri,
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri(format!("{EX}from_relation_x")),
                    TermValue::iri(format!("{EX}from_relation_y")),
                ]],
            )
            .expect("one row, two values wide"),
        ),
    );

    // The dataset holds an ordinary triple under the LONGER, unregistered IRI —
    // it merely shares the short IRI's characters as a prefix.
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}subject"));
    let p = builder.intern_iri(&long_predicate);
    let o = builder.intern_iri(&format!("{EX}object"));
    builder.push_quad(s, p, o, None);
    let dataset = builder.freeze().expect("freeze fixture");

    let query = format!("SELECT ?s ?o WHERE {{ ?s <{long_predicate}> ?o }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: &registry,
                ..QueryOptions::EMPTY
            },
        )
        .expect(
            "an unregistered, merely-prefix-sharing predicate must parse and evaluate as an \
             ordinary data triple, never a hard-erroring call",
        );

    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("a SELECT returns solutions");
    };
    assert_eq!(
        rows.len(),
        1,
        "the triple under the longer predicate is read from the graph as an ordinary BGP \
         triple, not routed through the relation: {rows:?}"
    );
    // A relation fire would also produce exactly one row (`MemoryRelation` above has a
    // single row), so `rows.len() == 1` alone does not distinguish "read from the graph"
    // from "hijacked by the relation" — only the bound CELLS do. The graph triple binds
    // `?s`/`?o` to `{EX}subject`/`{EX}object`; the relation's row would instead bind
    // `{EX}from_relation_x`/`{EX}from_relation_y`.
    let s_index = variables
        .iter()
        .position(|v| v == "s")
        .expect("?s is projected");
    let o_index = variables
        .iter()
        .position(|v| v == "o")
        .expect("?o is projected");
    assert_eq!(
        rows[0][s_index],
        Some(TermValue::iri(format!("{EX}subject"))),
        "?s must be bound from the graph triple's subject, not the relation's row: {rows:?}"
    );
    assert_eq!(
        rows[0][o_index],
        Some(TermValue::iri(format!("{EX}object"))),
        "?o must be bound from the graph triple's object, not the relation's row: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// The GOVERNED lane (GAP-2)
// ---------------------------------------------------------------------------
//
// Every test below drives `NativeSparqlEngine::query_governed` — the headline governed
// entry — with the registry carried in its `QueryOptions`. Before that parameter existed
// the governed entries could not be handed a registry at all: they parsed under the
// engine's bare `ParserOptions`, so a registered relation's predicate stayed an ORDINARY
// triple pattern, matched nothing, and the query answered the empty bag with no
// diagnostic. These tests are what makes that shape unreachable.

/// The one query text the governed tests below carry, spelled with the relation IRI
/// written out so nothing but the registry decides whether it is a call.
const GOVERNED_QUERY: &str = "PREFIX rel: <https://example.org/rel/>\n\
                              SELECT ?person ?team WHERE { ?person rel:memberOf ?team }\n";

/// A relation that declares a huge upper bound and emits one row.
///
/// The declaration is what admission control reads (see
/// `PropertyFunction::rows_per_invocation`), so this is a relation an intermediate-cell
/// ceiling must REFUSE before it runs — the estimate half of the seam, which is
/// unreachable from an entry that cannot be handed a registry.
#[derive(Debug)]
struct DeclaredHugeRelation {
    modes: Vec<BindingPattern>,
}

impl DeclaredHugeRelation {
    fn new() -> Self {
        Self {
            modes: vec![BindingPattern::from_code("ff")],
        }
    }
}

impl PropertyFunction for DeclaredHugeRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1_000_000
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(OneRowCursor { emitted: false }))
    }
}

#[derive(Debug)]
struct OneRowCursor {
    emitted: bool,
}

impl PfCursor for OneRowCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.emitted {
            return Ok(None);
        }
        self.emitted = true;
        Ok(Some(vec![
            TermValue::iri(format!("{EX}ada")),
            TermValue::iri(format!("{EX}alpha")),
        ]))
    }
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

/// The options a host with relations in scope hands a governed entry.
fn with_relations(registry: &PropertyFunctionRegistry) -> QueryOptions<'_> {
    QueryOptions {
        property_functions: registry,
        ..QueryOptions::EMPTY
    }
}

/// Render a solution result as `[[iri, iri], ..]` for comparison.
fn rows_of(result: &SparqlResult) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions");
    };
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(TermValue::as_iri)
                        .expect("every cell is a bound IRI")
                        .to_owned()
                })
                .collect()
        })
        .collect()
}

/// The relation's three rows come back through `query_governed`, and the execution
/// charges the two engine-observed property-function points on the way. (The third,
/// `property-function-work`, is reported by the relation; this fixture reports none.)
#[test]
fn a_governed_query_answers_from_the_relation_and_charges_it() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();

    let outcome = engine
        .query_governed(
            &dataset,
            request(GOVERNED_QUERY),
            with_relations(&registry),
            &QueryGovernors::METERED,
        )
        .expect("the call resolves and evaluates under governors");
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = outcome
    else {
        panic!("METERED bounds nothing, so this must complete");
    };
    assert!(evidence.is_complete());
    assert_eq!(
        rows_of(&result),
        vec![
            vec![format!("{EX}ada"), format!("{EX}alpha")],
            vec![format!("{EX}brian"), format!("{EX}alpha")],
            vec![format!("{EX}chen"), format!("{EX}beta")],
        ],
        "the governed entry must answer from the relation, not from the graph"
    );

    // The charge ledger for the very same plan, read through the explanation entry:
    // one invocation, and one charge per row the relation emitted. These are the two
    // charge points that are unreachable from an entry with no registry — a run that
    // silently degraded the call to a BGP triple would spend zero at both.
    let explanation = engine
        .explain_query_with_options(&dataset, GOVERNED_QUERY, None, with_relations(&registry))
        .expect("explain");
    let invocations: u64 = explanation
        .ledger()
        .iter()
        .map(|node| node.fuel_at(ChargePoint::PropertyFunctionInvocation))
        .sum();
    let relation_rows: u64 = explanation
        .ledger()
        .iter()
        .map(|node| node.fuel_at(ChargePoint::PropertyFunctionRow))
        .sum();
    assert_eq!(invocations, 1, "one call into the host relation");
    assert_eq!(relation_rows, 3, "one charge per row the relation emitted");

    // And the governed run spent exactly what the ledger accounts for: the two entries
    // price the same execution, so the seam's charge points are live on THIS lane and
    // not merely on the explain lane.
    assert_eq!(
        evidence.consumed.get(ResourceDimension::Fuel),
        explanation
            .ledger()
            .iter()
            .map(NodeCharges::fuel_total)
            .sum::<u64>(),
        "the governed entry and the explanation must price the same run"
    );
}

/// A fuel ceiling one unit below the metered spend trips on the governed entry, which
/// is only possible if the relation's own charges are being spent there.
#[test]
fn a_governed_relation_query_trips_one_unit_below_its_metered_spend() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();

    let measure = |governors: &QueryGovernors| {
        engine
            .query_governed(
                &dataset,
                request(GOVERNED_QUERY),
                with_relations(&registry),
                governors,
            )
            .expect("a governor trip is an outcome, never an error")
    };

    let GovernedOutcome::Complete { evidence, .. } = measure(&QueryGovernors::METERED) else {
        panic!("METERED bounds nothing");
    };
    let spend = evidence.consumed.get(ResourceDimension::Fuel);
    assert!(spend > 0, "a relation call is not free");

    assert!(
        matches!(
            measure(&QueryGovernors::UNBOUNDED.with_fuel(spend)),
            GovernedOutcome::Complete { .. }
        ),
        "the measured spend is exactly affordable"
    );
    assert!(
        matches!(
            measure(&QueryGovernors::UNBOUNDED.with_fuel(spend - 1)),
            GovernedOutcome::BudgetExhausted(_)
        ),
        "one unit less is not"
    );
}

/// The admission half: a relation that DECLARES a million rows is refused by a small
/// intermediate-cell ceiling, before it runs, through the same governed entry.
///
/// The estimate can only exist because the registry travelled with the options: the
/// planner prices a call from the relation's declared bound, and a governed entry that
/// never saw the registry would have surveyed an ordinary triple pattern instead and
/// admitted it against the dataset's (empty) cardinality.
#[test]
fn a_governed_entry_refuses_a_declared_huge_relation_on_a_small_cell_ceiling() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        format!("{REL_NS}memberOf"),
        Arc::new(DeclaredHugeRelation::new()),
    );

    let outcome = NativeSparqlEngine::new()
        .query_governed(
            &dataset(),
            request(GOVERNED_QUERY),
            with_relations(&registry),
            &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(8),
        )
        .expect("a refusal is an outcome, never an error");

    let GovernedOutcome::BudgetExhausted(exhausted) = outcome else {
        panic!("a declared-huge relation under an 8-cell ceiling must be refused");
    };
    let TrippedGovernor::Refused {
        dimension,
        limit,
        estimate,
    } = exhausted.tripped
    else {
        panic!(
            "admission must REFUSE the plan rather than meter it: {:?}",
            exhausted.tripped
        );
    };
    assert_eq!(dimension, ResourceDimension::IntermediateCells);
    assert_eq!(limit, 8);
    assert!(
        estimate > 8,
        "the estimate comes from the relation's declared bound: {estimate}"
    );
}

/// THE gap, stated as a test: the shape that used to silently answer the empty bag.
///
/// `QueryOptions::EMPTY` is the honest "this host configured no relations" value, and
/// under it the predicate really is ordinary data — which is the pre-existing, correct
/// behaviour for a host that registered nothing. What the options parameter removes is
/// the possibility of reaching that behaviour while HOLDING a registry: there is no
/// `query_governed` overload that takes a registry-configured host and drops it, so a
/// caller cannot get the empty bag by accident. The two runs below are the same text on
/// the same entry, and the only difference between them is the field that used to be
/// unreachable.
#[test]
fn the_governed_entry_cannot_silently_drop_a_registered_relation() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();

    let answered = engine
        .query_governed(
            &dataset,
            request(GOVERNED_QUERY),
            with_relations(&registry),
            &QueryGovernors::METERED,
        )
        .expect("evaluates");
    let GovernedOutcome::Complete { result, .. } = answered else {
        panic!("METERED bounds nothing");
    };
    assert_eq!(
        rows_of(&result).len(),
        3,
        "with the registry in the options the call is dispatched"
    );

    let unconfigured = engine
        .query_governed(
            &dataset,
            request(GOVERNED_QUERY),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("evaluates");
    let GovernedOutcome::Complete { result, .. } = unconfigured else {
        panic!("METERED bounds nothing");
    };
    assert!(
        rows_of(&result).is_empty(),
        "a host that configured NO relations reads the graph, which holds no such triple — \
         this is the behaviour `QueryOptions::EMPTY` names, and the only way to reach it is \
         to name it"
    );
}

/// The re-pointed corpus lane, as a unit test: the natural governed entry and the
/// operation entry must spend IDENTICALLY on the same relation query.
///
/// The frozen governor corpus used to drive its relation cases through
/// `query_governed_in_operation`, because that was the only governed surface that could
/// carry a registry. It now drives `query_governed`. The pinned consumption bytes did not
/// move, and this is the invariant that says why: one governed body, two doors.
#[test]
fn the_per_call_and_operation_governed_entries_spend_identically() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();

    let GovernedOutcome::Complete {
        evidence: per_call, ..
    } = engine
        .query_governed(
            &dataset,
            request(GOVERNED_QUERY),
            with_relations(&registry),
            &QueryGovernors::METERED,
        )
        .expect("evaluates")
    else {
        panic!("METERED bounds nothing");
    };

    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let GovernedOutcome::Complete {
        evidence: in_operation,
        ..
    } = engine
        .query_governed_in_operation(
            &*dataset,
            request(GOVERNED_QUERY),
            with_relations(&registry),
            &state,
        )
        .expect("evaluates")
    else {
        panic!("METERED bounds nothing");
    };

    assert_eq!(
        per_call.consumed, in_operation.consumed,
        "the two governed doors reach the same body, so they must price a run identically"
    );
}

/// The prepared-plan lane's residue, closed: a plan parsed WITHOUT the registry cannot
/// be evaluated WITH it.
///
/// This is the last shape that could still answer the empty bag silently. The plan below
/// lowered `rel:memberOf` to an ordinary triple pattern because it was parsed with no
/// registry in scope; attaching the registry at evaluation time would leave the BGP scan
/// in place, find nothing in a dataset that holds no such triple, and report a clean
/// complete run of zero rows. The plan carries the registry identity it was parsed under,
/// so the disagreement is refused instead.
#[test]
fn a_plan_prepared_without_the_registry_is_refused_when_evaluated_with_it() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();

    let stale = engine
        .prepare_query(GOVERNED_QUERY, None)
        .expect("the text parses as ordinary data with no registry in scope");
    let refused = engine.query_prepared_governed_view(
        &*dataset,
        &stale,
        &[],
        with_relations(&registry),
        &QueryGovernors::METERED,
    );
    let error = refused.expect_err("a plan/registry disagreement must be a diagnostic");
    assert_eq!(error.code, "native-sparql-property-function");

    // Prepared under the SAME options, the very same text runs and answers.
    let matched = engine
        .prepare_query_with_options(GOVERNED_QUERY, None, with_relations(&registry))
        .expect("the registry-aware parse lowers the predicate to a call");
    let GovernedOutcome::Complete { result, .. } = engine
        .query_prepared_governed_view(
            &*dataset,
            &matched,
            &[],
            with_relations(&registry),
            &QueryGovernors::METERED,
        )
        .expect("evaluates")
    else {
        panic!("METERED bounds nothing");
    };
    assert_eq!(rows_of(&result).len(), 3);
}

// ---------------------------------------------------------------------------
// The UNGOVERNED prepared and federated entries (FB-B)
// ---------------------------------------------------------------------------
//
// `query_with_source(_view)` and `query_prepared(_view)` used to be the last two
// public entries with no `QueryOptions` parameter at all: the federated pair parsed
// through the bare, registry-free `prepare_with`, and the prepared-plan pair applied
// no options and ran no `check_plan_matches_relations` refusal. Both shapes could
// silently drop a registered relation exactly the way the governed lane's gap (above)
// could. These tests are what makes that shape unreachable on the ungoverned lane too.

/// A registered relation is reachable through `query_with_source_view` once `options`
/// carries it — the federation entry's OUTER pattern now parses and resolves through
/// the SAME registry-aware path every other options-carrying entry uses. No `SERVICE`
/// clause is exercised here (the source has no endpoints registered): the point is
/// that the relation's own predicate, sitting outside any `SERVICE` block, dispatches.
#[test]
fn query_with_source_view_dispatches_a_registered_relation_with_options() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();
    let source = purrdf_sparql_eval::LocalRemoteQuerySource::new();

    let result = engine
        .query_with_source_view(
            &*dataset,
            request(GOVERNED_QUERY),
            &source,
            with_relations(&registry),
        )
        .expect("a registered relation's outer-pattern call evaluates through the federated entry");

    assert_eq!(
        rows_of(&result).len(),
        3,
        "with the registry carried in `options`, the call is dispatched exactly as it is \
         through `query_with_options_view`"
    );
}

/// The ungoverned prepared-plan pair's residue, closed the same way the governed pair's
/// is: a plan parsed WITHOUT the registry is refused rather than silently evaluated with
/// it, and the SAME text prepared WITH the registry answers.
#[test]
fn query_prepared_with_a_mismatched_registry_is_refused_and_the_matched_registry_answers() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let dataset = dataset();

    let stale = engine
        .prepare_query(GOVERNED_QUERY, None)
        .expect("the text parses as ordinary data with no registry in scope");
    let refused = engine.query_prepared(&dataset, &stale, &[], with_relations(&registry));
    let error = refused.expect_err(
        "a plan/registry disagreement must be a diagnostic, not a \
                                     silent empty-bag answer",
    );
    assert_eq!(error.code, "native-sparql-property-function");

    // Prepared under the SAME options, the very same text runs and answers from the
    // relation — never from the graph, which holds no `rel:memberOf` triple to have
    // matched instead.
    let matched = engine
        .prepare_query_with_options(GOVERNED_QUERY, None, with_relations(&registry))
        .expect("the registry-aware parse lowers the predicate to a call");
    let result = engine
        .query_prepared(&dataset, &matched, &[], with_relations(&registry))
        .expect("a plan and options that agree on the registry evaluate");
    assert_eq!(rows_of(&result).len(), 3);
}

/// GAP (registry instance identity — the property-function sibling of the
/// custom-aggregate registry-identity gap): a plan prepared under one relation
/// registry must not be silently executed under a DIFFERENT registry that
/// resolves the SAME IRI to a DIFFERENT relation, even when the two registries
/// declare IDENTICALLY (same arity, volatility, modes, and row bound). Declared
/// metadata alone cannot prove the two registries answer a call the same way.
#[test]
fn a_plan_prepared_under_one_relation_registry_refuses_to_execute_under_a_different_one_with_identical_declarations()
 {
    let engine = NativeSparqlEngine::new();
    let registry_a = relations();

    // Registry B: the SAME IRI resolves to a DIFFERENT relation — the SAME row
    // COUNT (three), so `describe()` reports byte-identically to registry A's
    // (arity, volatility, modes, and the row-count-derived bound all match), but
    // entirely different row content.
    let iri = |local: &str| TermValue::iri(format!("{EX}{local}"));
    let mut registry_b = PropertyFunctionRegistry::new();
    registry_b.register(
        format!("{REL_NS}memberOf"),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![
                    vec![iri("dan"), iri("gamma")],
                    vec![iri("erin"), iri("gamma")],
                    vec![iri("frank"), iri("gamma")],
                ],
            )
            .expect("every row is two values wide"),
        ),
    );

    // The reproduction only means what it claims if the two registries' DECLARED
    // metadata is byte-identical for this IRI — confirm that first.
    assert_eq!(
        registry_a.describe().expect("no panic"),
        registry_b.describe().expect("no panic"),
        "the two registries must declare identically for this to be a meaningful \
         reproduction of the declaration-only fingerprint gap"
    );

    let dataset = dataset();
    let prepared = engine
        .prepare_query_with_options(GOVERNED_QUERY, None, with_relations(&registry_a))
        .expect("registry A admits and lowers the predicate to a call");

    let error = engine
        .query_prepared(&dataset, &prepared, &[], with_relations(&registry_b))
        .expect_err(
            "a plan prepared under registry A must be REFUSED under registry B, never silently \
             executed against B's different relation",
        );
    assert_eq!(
        error.code, "native-sparql-property-function",
        "the refusal must be attributable to the property-function registry identity check: \
         {error:?}"
    );

    // The non-regression twin: the SAME registry instance at both prepare and
    // execute must still work.
    let result = engine
        .query_prepared(&dataset, &prepared, &[], with_relations(&registry_a))
        .expect("the SAME registry instance must be accepted at execution");
    assert_eq!(rows_of(&result).len(), 3);
}

// ── UPDATE: the same seam, on the mutation surface ─────────────────────────────────
//
// A property-function call reaches an UPDATE's `WHERE` exactly the way it reaches a
// `SELECT`'s: `?person rel:memberOf ?team` is admitted, feasibility-ordered, evaluated
// against the injected relation, and DELETE/INSERT templates run over the rows it
// answers with — never over an empty scan of the graph.

/// `INSERT { ?person ex:member ?team } WHERE { ?person rel:memberOf ?team }`: the
/// template text every UPDATE test below carries.
const UPDATE_TEXT: &str = "PREFIX ex: <https://example.org/d/>\n\
                           PREFIX rel: <https://example.org/rel/>\n\
                           INSERT { ?person ex:member ?team } WHERE { ?person rel:memberOf ?team }\n";

/// Reads back what [`UPDATE_TEXT`] should have written, as ordinary (non-call) data.
const CHECK_QUERY: &str = "PREFIX ex: <https://example.org/d/>\n\
                           SELECT ?person ?team WHERE { ?person ex:member ?team } ORDER BY ?person\n";

/// The ungoverned entry: `NativeSparqlEngine::update_with_options`, the UPDATE sibling of
/// [`NativeSparqlEngine::query_with_options_view`]. It inserts EXACTLY the
/// relation's three rows — not a subset, and nothing from the graph, which holds no
/// `rel:memberOf` triple to have matched instead.
#[test]
fn an_update_where_inserts_exactly_the_relations_rows() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();
    let mut ds = dataset();
    let before = ds.quad_count();

    engine
        .update_with_options(&mut ds, request(UPDATE_TEXT), with_relations(&registry))
        .expect("the call resolves, evaluates, and the mutation applies");

    assert_eq!(
        ds.quad_count(),
        before + 3,
        "exactly the relation's three rows were inserted — the pre-existing unrelated \
         triple is untouched and nothing else was added"
    );

    let result = engine
        .query(&ds, request(CHECK_QUERY))
        .expect("the inserted triples are ordinary data now, readable with no registry");
    assert_eq!(
        rows_of(&result),
        vec![
            vec![format!("{EX}ada"), format!("{EX}alpha")],
            vec![format!("{EX}brian"), format!("{EX}alpha")],
            vec![format!("{EX}chen"), format!("{EX}beta")],
        ]
    );
}

/// The governed sibling: `update_governed` spends fuel on the relation's dispatch (the
/// same [`ChargePoint::PropertyFunctionInvocation`] / [`ChargePoint::PropertyFunctionRow`]
/// points a governed `SELECT` spends — there is no ledger reader for an UPDATE, so this
/// reads the same evidence the query-side test does, through
/// [`purrdf_core::GovernorEvidence::consumed`]), and a ceiling one unit below that
/// measured spend trips deterministically while a ceiling exactly at it applies.
#[test]
fn a_governed_update_where_charges_the_relation_and_trips_on_fuel() {
    let engine = NativeSparqlEngine::new();
    let registry = relations();

    let run = |governors: &QueryGovernors| -> (Arc<RdfDataset>, GovernedUpdateOutcome) {
        let mut ds = dataset();
        let outcome = engine
            .update_governed(
                &mut ds,
                request(UPDATE_TEXT),
                with_relations(&registry),
                governors,
            )
            .expect("a governor trip is an outcome, never an update error");
        (ds, outcome)
    };

    let (metered_ds, metered) = run(&QueryGovernors::METERED);
    let GovernedUpdateOutcome::Applied { evidence } = metered else {
        panic!("METERED bounds nothing, so this must apply");
    };
    assert_eq!(
        metered_ds.quad_count(),
        dataset().quad_count() + 3,
        "the metered run applied exactly the relation's rows"
    );
    let spend = evidence.consumed.get(ResourceDimension::Fuel);
    assert!(
        spend > 0,
        "dispatching the relation into an INSERT is not free"
    );

    let (_, exact) = run(&QueryGovernors::UNBOUNDED.with_fuel(spend));
    assert!(
        matches!(exact, GovernedUpdateOutcome::Applied { .. }),
        "the measured spend is exactly affordable"
    );

    let (short_ds, short) = run(&QueryGovernors::UNBOUNDED.with_fuel(spend - 1));
    let GovernedUpdateOutcome::BudgetExhausted { .. } = short else {
        panic!("one unit less than the measured spend must trip");
    };
    // A trip applies nothing: the base handed to this run is untouched.
    assert_eq!(
        short_ds.quad_count(),
        dataset().quad_count(),
        "a tripped UPDATE must not have inserted any of the relation's rows"
    );
}

/// A call node reaching evaluation with nothing to resolve against is a hard error, not
/// an empty `WHERE` — the same distinction [`resolve`](property_fn_eval) draws on the
/// query lane. The engine here declares `rel:` as a property-function NAMESPACE (so the
/// predicate parses to a call regardless of what registry, if any, is supplied to
/// evaluation), and this run supplies none.
#[test]
fn an_update_where_call_with_no_registry_hard_errors_precisely() {
    let engine = NativeSparqlEngine::new().with_parser_options(options());
    let mut ds = dataset();
    let before = ds.quad_count();

    let error = engine
        .update(&mut ds, request(UPDATE_TEXT))
        .expect_err("a call with nothing to resolve against must not read the graph instead");
    assert_eq!(error.code, "native-sparql-property-function");
    assert!(
        error.message.contains("no property function is registered"),
        "got: {}",
        error.message
    );
    // The failed request must not have mutated the dataset handle at all.
    assert_eq!(ds.quad_count(), before);
}
