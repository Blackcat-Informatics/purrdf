// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Tests for the prepared-product surface: the writer, the two restore seams and
//! the cold certification path.
//!
//! Every refusal below is executed alongside a NEIGHBOURING VALID case. A refusal
//! is a claim, and an over-refusal — turning away a product that is actually fine —
//! hides perfectly: every test still passes and the strictness looks correct, right
//! up until a user loads the product that should restore and doesn't.
//!
//! Deliberately function-and-fixture-only where it can be: the product-model
//! census scans every `crates/shapes/src/**/*.rs` file, so nothing here is named
//! after a model type.

use std::sync::Arc;

use ::purrdf::{RdfDataset, TermValue};
use purrdf_core::artifact::{ArtifactBuilder, ArtifactView, Identity};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, Arity, BindingPattern, CustomAggregate, EvalError,
    GovernorState, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    QueryGovernors, UserFunctionRegistry, Volatility,
};

use super::{
    HostBindings, ProductDimension, SECTION_AST, SECTION_DATASET, SECTION_IDENTITY, SPEC, STAGE_ID,
    ShapesProduct, ShapesProfile,
};
use crate::engine::{PreparedShapes, parse_shapes};
use crate::model::BoxRoleVocab;
use crate::report::ValidationReport;
use crate::shapes::{Shapes, from_dataset_with_config_and_graph};
use crate::text_ingest::{extract_prefixes, parse_turtle_to_dataset};

// ── Fixtures (example.org, per the repository's fixture rule) ───────────────────

/// The prefix header every shapes fixture opens with.
const PREFIXES: &str = r"
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix ex:     <http://example.org/ns#> .
@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

/// A shapes graph with a class target and a property shape.
const PLAIN_SHAPES: &str = r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] .
";

/// Two `ex:Person` nodes, one of which is missing its `ex:name`.
const PLAIN_DATA: &str = r#"
ex:alice a ex:Person ; ex:name "Alice" .
ex:bob   a ex:Person .
"#;

/// A DIFFERENT data graph, with a focus node the producer's graph never had.
const OTHER_DATA: &str = r"
ex:carol a ex:Person .
";

/// The box-role vocabulary namespace the box-role fixtures run under. PurRDF mints
/// no vocabulary IRIs, so this is caller-supplied configuration.
const ROLE_NS: &str = "http://example.org/roles#";

/// A shapes graph annotated with a box role under [`ROLE_NS`].
const ROLE_SHAPES: &str = r"
@prefix roles: <http://example.org/roles#> .
ex:PersonShape a sh:NodeShape ;
    roles:graphBoxRole roles:boxTBox ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
";

/// SHACL 1.2 SPARQL Extensions §7.2: a list-parameter declaration, which a product
/// CAN carry — the neighbouring valid case for the `sh:SPARQLFunction` refusal.
const EXPRESSION_FN_SHAPES: &str = r"
ex:incomeTotal a sh:ListParameterExpressionFunction ;
  rdfs:subClassOf sh:ListParameterExpression ;
  sh:bodyExpression [ shnex:sum [ shnex:pathValues ex:income ; shnex:focusNode [ shnex:arg 0 ] ] ] ;
  sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .

ex:S a sh:NodeShape ;
  sh:targetNode ex:alice, ex:bob ;
  sh:expression [ sparql:equals ( [ ex:incomeTotal ( sh:this ) ] 30 ) ] .
";

/// The data the expression-function fixture validates over.
const EXPRESSION_FN_DATA: &str = r"
ex:alice ex:income 10, 20 .
ex:bob   ex:income 5 .
";

/// SHACL-AF §5: a `sh:SPARQLFunction` declaration whose body a `sh:sparql`
/// constraint actually CALLS, so the verdict depends on the function resolving.
///
/// A round-trip over a graph whose function never fires proves nothing — the
/// restored registry could be empty and every assertion would still hold. Here a
/// restore that lost `ex:double` cannot produce the same report, because the filter
/// that decides the verdict is the call.
const SPARQL_FN_SHAPES: &str = r#"
ex:double a sh:SPARQLFunction ;
  sh:parameter [ sh:path ex:arg ; sh:datatype xsd:integer ] ;
  sh:returnType xsd:integer ;
  sh:select """SELECT ?result WHERE { BIND(?arg * 2 AS ?result) }""" .

ex:CapShape a sh:NodeShape ;
  sh:targetClass ex:Thing ;
  sh:sparql [
    a sh:SPARQLConstraint ;
    sh:message "the doubled value exceeds the cap" ;
    sh:select """SELECT $this ?value WHERE { $this ex:n ?value . FILTER (ex:double(?value) > 10) }""" ;
  ] .
"#;

/// Data on which `ex:double` fires for `ex:high` and not for `ex:low`.
const SPARQL_FN_DATA: &str = r#"
ex:low  a ex:Thing ; ex:n "3"^^xsd:integer .
ex:high a ex:Thing ; ex:n "7"^^xsd:integer .
"#;

/// The NEGATIVE CONTROL for [`SPARQL_FN_DATA`]: the same two nodes, with their
/// values moved to the other side of the cap, so the verdict flips.
const SPARQL_FN_DATA_FLIPPED: &str = r#"
ex:low  a ex:Thing ; ex:n "9"^^xsd:integer .
ex:high a ex:Thing ; ex:n "2"^^xsd:integer .
"#;

/// A `sh:ListParameterExpressionFunction` no NODE EXPRESSION calls, called instead
/// from `sh:sparql` query TEXT.
///
/// The declarative model is what a product carries, and a declaration nothing in
/// that model reaches is not in it — while the `sh:select` body above still names
/// the function by IRI, so a restore would resolve that call to nothing. This is the
/// construct a product genuinely cannot carry.
const UNREACHABLE_FN_SHAPES: &str = r#"
ex:incomeTotal a sh:ListParameterExpressionFunction ;
  rdfs:subClassOf sh:ListParameterExpression ;
  sh:bodyExpression [ shnex:sum [ shnex:pathValues ex:income ; shnex:focusNode [ shnex:arg 0 ] ] ] ;
  sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .

ex:S a sh:NodeShape ;
  sh:targetNode ex:alice ;
  sh:sparql [ sh:select """SELECT $this WHERE { FILTER (ex:incomeTotal($this) > 30) }""" ] .
"#;

/// The `AGG(<iri>, …)` IRI the custom-aggregate fixture registers under.
const AGG_IRI: &str = "http://example.org/ns#sum";

/// The implementation identity the fixtures that inject host implementations bind:
/// an opaque byte string standing in for whatever a real host uses to tell its own
/// builds of its native code apart.
const IMPLEMENTATION_ID: &[u8] = b"example.org/host-build-1";

/// A shapes graph whose `sh:sparql` body resolves an `AGG(<iri>, …)` call. It is
/// only satisfiable when the host's aggregate registry is actually INSTALLED on the
/// restored shapes — a restore that merely fingerprinted it fails here with "no
/// custom aggregate is registered".
fn aggregate_shapes() -> String {
    format!(
        r#"
ex:AggShape a sh:NodeShape ;
    sh:targetClass ex:Thing ;
    sh:sparql ex:AggConstraint .
ex:AggConstraint sh:select """
    SELECT $this WHERE {{ $this <http://example.org/ns#amount> ?v }}
    GROUP BY $this
    HAVING (AGG(<{AGG_IRI}>, ?v) < 0)
""" .
"#
    )
}

/// One node whose amounts sum negative (must violate) and one whose do not.
const AGGREGATE_DATA: &str = r#"
ex:v0 a ex:Thing ; ex:amount "-1"^^xsd:integer .
ex:v1 a ex:Thing ; ex:amount "1"^^xsd:integer .
"#;

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Parse a shapes graph from text, under the prefix header every fixture shares.
fn shapes_of(body: &str) -> Shapes {
    parse_shapes(&format!("{PREFIXES}{body}"), None).expect("fixture shapes parse")
}

/// Freeze a data graph from text.
fn data_of(body: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{body}"), None).expect("fixture data parses")
}

/// Prepare a shapes graph.
fn prepare(body: &str) -> PreparedShapes {
    PreparedShapes::new(Arc::new(shapes_of(body)))
}

/// Write a product for `body` under the only profile this build implements.
fn product_of(body: &str) -> Vec<u8> {
    prepare(body)
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable")
}

/// Validate `data` with `prepared`, returning the report's canonical N-Triples.
///
/// The report's RDF form is the comparison surface, not a field-by-field walk: two
/// restores are equal exactly when the graphs they produce are the same bytes.
fn report_nt(prepared: &PreparedShapes, data: &Arc<RdfDataset>) -> String {
    validate(prepared, data).to_ntriples()
}

/// Validate `data` with `prepared`.
fn validate(prepared: &PreparedShapes, data: &Arc<RdfDataset>) -> ValidationReport {
    prepared
        .bind_shared_dataset(Arc::clone(data))
        .expect("binding the data graph")
        .validate()
        .expect("validation runs")
}

/// Rewrite a product's sections into a NEW, fully self-consistent container.
///
/// Every digest is recomputed, so the result is a product the envelope accepts —
/// which is the point. A tamper that left the envelope refusing would prove
/// nothing about the checks that come after it.
fn repack(bytes: &[u8], mutate: impl FnOnce(&mut [Vec<u8>; 3])) -> Vec<u8> {
    let view = ArtifactView::from_bytes(SPEC, bytes).expect("the fixture product opens");
    let mut identity = Identity::new();
    for component in view.identity().components() {
        identity.push(component.label(), component.value());
    }
    let mut sections = [
        view.section(SECTION_IDENTITY).expect("identity").to_vec(),
        view.section(SECTION_DATASET).expect("dataset").to_vec(),
        view.section(SECTION_AST).expect("ast").to_vec(),
    ];
    mutate(&mut sections);

    let mut builder = ArtifactBuilder::new(SPEC);
    builder
        .identity(identity)
        .section(SECTION_IDENTITY, &sections[0])
        .section(SECTION_DATASET, &sections[1])
        .section(SECTION_AST, &sections[2]);
    builder.build_bytes().expect("the repack frames")
}

/// The pack container header's `rdfc_digest` field: 32 bytes at offset 32.
///
/// Hard-coded from the pack's documented fixed layout because the tamper below has
/// to hit that field and NOTHING else — every pack section digest must stay intact
/// for `certify_is_not_reachable_from_admit` to mean what it claims.
const RDFC_DIGEST_OFFSET: usize = 32;

/// A `Commutative` `SUM`-alike over one numeric-lexical argument.
#[derive(Debug)]
struct SumAccumulator {
    /// The running total of the integer lexical forms folded so far.
    total: i64,
}

impl AggregateAccumulator for SumAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        )))
    }
}

/// The host-side custom aggregate the `AGG(<iri>, …)` fixture resolves.
#[derive(Debug)]
struct SumAggregate;

impl CustomAggregate for SumAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> purrdf_sparql_eval::AlgebraicClass {
        purrdf_sparql_eval::AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(SumAccumulator { total: 0 })
    }
}

/// A registry with [`SumAggregate`] registered under [`AGG_IRI`].
fn sum_aggregates() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(AGG_IRI, Arc::new(SumAggregate));
    registry
}

// ── The happy path ─────────────────────────────────────────────────────────────

/// An admitted product validates EXACTLY as the parsed shapes graph it was written
/// from: the same report, byte for byte.
#[test]
fn admit_yields_reports_identical_to_parsed() {
    let data = data_of(PLAIN_DATA);
    let parsed = prepare(PLAIN_SHAPES);
    let expected = report_nt(&parsed, &data);
    assert!(
        expected.contains("http://example.org/ns#bob"),
        "the fixture must actually report a violation, or this property is vacuous: {expected}",
    );

    let bytes = product_of(PLAIN_SHAPES);
    let restored = ShapesProduct::open(&bytes)
        .expect("the product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product admits");

    assert_eq!(report_nt(&restored, &data), expected);
}

/// The two restore seams answer identically over one product.
#[test]
fn admit_equals_rebuild() {
    let data = data_of(PLAIN_DATA);
    let bytes = product_of(PLAIN_SHAPES);

    let admitted = ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");
    let rebuilt = ShapesProduct::open(&bytes)
        .expect("opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("rebuilds");

    let from_memo = report_nt(&admitted, &data);
    assert!(!from_memo.is_empty(), "the fixture reports something");
    assert_eq!(
        report_nt(&rebuilt, &data),
        from_memo,
        "rebuild re-derives the shapes graph from the carried dataset, so it must answer exactly \
         what the memo answers — a difference means the memo and the primitive disagree",
    );
}

/// `HostBindings::empty()` suffices for a product of the CORE profile: every
/// capability comes from the shapes graph itself.
#[test]
fn empty_host_bindings_suffice_under_core_profile() {
    for fixture in [PLAIN_SHAPES, EXPRESSION_FN_SHAPES, SPARQL_FN_SHAPES] {
        let bytes = product_of(fixture);
        ShapesProduct::open(&bytes)
            .expect("opens")
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .expect("a CORE product needs no host bindings");
    }

    // ...including the one whose validation actually evaluates a declared
    // expression-bodied SPARQL function, so the claim is about capability and not
    // only about loading.
    let bytes = product_of(EXPRESSION_FN_SHAPES);
    let restored = ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");
    let report = validate(&restored, &data_of(EXPRESSION_FN_DATA));
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(
        focus,
        vec!["<http://example.org/ns#bob>".to_owned()],
        "the restored declaration must still compute ex:alice's total as 30 and ex:bob's as 5",
    );
}

// ── Installing, not merely fingerprinting ──────────────────────────────────────

/// A restored product resolves the host's `AGG(<iri>, …)` calls, which is only true
/// if the aggregate registry was INSTALLED on the restored `Shapes`.
///
/// A restore that fingerprinted the registry and left the restored value's own
/// empty one in place passes every identity check and then fails here with "no
/// custom aggregate is registered" — a capability loss no fingerprint can catch,
/// because the fingerprints agreed.
#[test]
fn admit_installs_host_aggregates() {
    let ttl = aggregate_shapes();
    let aggregates = sum_aggregates();

    let mut shapes = shapes_of(&ttl);
    shapes.aggregates = Arc::new(aggregates.clone());
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let bytes = prepared
        .to_product_with_implementation_identity(&ShapesProfile::CORE, IMPLEMENTATION_ID)
        .expect("representable");

    let functions = UserFunctionRegistry::new();
    let property_functions = PropertyFunctionRegistry::new();
    let host = HostBindings::without_declarations(
        &functions,
        &aggregates,
        &property_functions,
        IMPLEMENTATION_ID,
    );

    let restored = ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &host)
        .expect("admits under the same registry it was prepared against");

    let report = validate(&restored, &data_of(AGGREGATE_DATA));
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(
        focus,
        vec!["<http://example.org/ns#v0>".to_owned()],
        "exactly the focus node whose amounts sum negative violates, which requires the \
         aggregate to have actually run",
    );
}

/// A relation that yields no rows; it exists to be DECLARED.
#[derive(Debug)]
struct EmptyRelation {
    /// The single binding pattern this relation admits.
    modes: [BindingPattern; 1],
}

/// A cursor over no rows; see [`EmptyRelation`].
#[derive(Debug)]
struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

impl PropertyFunction for EmptyRelation {
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
        0
    }
    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(EmptyCursor))
    }
}

/// The CORE profile binds the EMPTY property-function registry — host relations are
/// wiring no shapes graph can describe, so a product cannot carry them — and a host
/// that wires one is told so rather than executing a product prepared without it.
///
/// Its neighbour is the case that matters: any EMPTY registry admits, including a
/// freshly constructed one whose instance id differs from the writer's. The binding
/// is over the registry's CONTENT, so an over-strict check that compared instances
/// would refuse every restore in a different process.
#[test]
fn core_profile_binds_the_empty_property_function_registry() {
    let bytes = product_of(PLAIN_SHAPES);
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();

    let mut wired = PropertyFunctionRegistry::new();
    wired.register(
        "http://example.org/ns#rel",
        Arc::new(EmptyRelation {
            modes: [BindingPattern::from_code("bf")],
        }),
    );
    let error = ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(
            &ShapesProfile::CORE,
            &HostBindings::without_declarations(&functions, &aggregates, &wired, &[]),
        )
        .expect_err("a CORE product was not prepared against any host relation");
    assert_eq!(
        error.dimension(),
        ProductDimension::PropertyFunctionRegistry
    );

    let fresh = PropertyFunctionRegistry::new();
    ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(
            &ShapesProfile::CORE,
            &HostBindings::without_declarations(&functions, &aggregates, &fresh, &[]),
        )
        .expect("a DIFFERENT but equally empty registry must still admit");
}

/// The neighbouring refusal: a host that supplies a DIFFERENT aggregate registry
/// than the product was prepared against is refused, on the aggregate dimension.
///
/// The implementation identity is held FIXED across both halves, so the aggregate
/// registry is the only moving input and the dimension the refusal names is the
/// dimension that actually moved. A host that also got its identity wrong would
/// disagree on all three host rows at once, and the codec reports the first — which
/// is a true statement about a different question.
#[test]
fn admit_refuses_a_different_aggregate_registry() {
    let ttl = aggregate_shapes();
    let mut shapes = shapes_of(&ttl);
    shapes.aggregates = Arc::new(sum_aggregates());
    let bytes = PreparedShapes::new(Arc::new(shapes))
        .to_product_with_implementation_identity(&ShapesProfile::CORE, IMPLEMENTATION_ID)
        .expect("representable");

    let functions = UserFunctionRegistry::new();
    let property_functions = PropertyFunctionRegistry::new();
    let none = AggregateRegistry::new();
    let error = ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(
            &ShapesProfile::CORE,
            &HostBindings::without_declarations(
                &functions,
                &none,
                &property_functions,
                IMPLEMENTATION_ID,
            ),
        )
        .expect_err("an empty aggregate registry is not the one this product was prepared with");
    assert_eq!(error.dimension(), ProductDimension::AggregateRegistry);

    // ...and the valid neighbour still succeeds, so the strictness is not refusing
    // every host.
    let aggregates = sum_aggregates();
    ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(
            &ShapesProfile::CORE,
            &HostBindings::without_declarations(
                &functions,
                &aggregates,
                &property_functions,
                IMPLEMENTATION_ID,
            ),
        )
        .expect("the registry it WAS prepared against admits");
}

// ── The box-role vocabulary ────────────────────────────────────────────────────

/// Parse `ROLE_SHAPES` under a non-empty box-role vocabulary.
fn role_shapes() -> Shapes {
    let ttl = format!("{PREFIXES}{ROLE_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl, None).expect("role fixture parses");
    from_dataset_with_config_and_graph(
        &dataset,
        &extract_prefixes(&ttl),
        Some(BoxRoleVocab::for_namespace(ROLE_NS)),
        None,
    )
    .expect("role fixture shapes parse")
}

/// Every top-level shape's box roles, as rendered IRIs.
fn box_roles(shapes: &Shapes) -> Vec<Vec<String>> {
    shapes
        .node_shapes
        .iter()
        .map(|shape| {
            shape
                .box_roles
                .iter()
                .map(|role| role.as_str().to_owned())
                .collect()
        })
        .collect()
}

/// A product prepared under a non-empty box-role vocabulary restores that
/// vocabulary AND the roles it collected.
///
/// The vocabulary is caller-supplied configuration with no fabricated default, so a
/// restore that dropped it would produce a `Shapes` that loads, verifies and
/// validates with every role lookup silently disabled.
#[test]
fn admit_restores_box_role_vocab() {
    let parsed = role_shapes();
    let expected_vocab = parsed.box_role_vocab.clone();
    let expected_roles = box_roles(&parsed);
    assert_eq!(
        expected_roles,
        vec![vec![format!("{ROLE_NS}boxTBox")]],
        "the fixture must actually collect a role, or this property is vacuous",
    );

    let bytes = PreparedShapes::new(Arc::new(parsed))
        .to_product(&ShapesProfile::CORE)
        .expect("representable");
    let view = ShapesProduct::open(&bytes).expect("opens");
    assert_eq!(
        view.declared_provenance().box_role_vocab(),
        expected_vocab.as_ref(),
        "the recorded parse inputs carry the vocabulary a rebuild needs",
    );

    let restored = view
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");
    let restored_shapes = restored_shapes_of(&restored);
    assert_eq!(restored_shapes.box_role_vocab, expected_vocab);
    assert_eq!(box_roles(&restored_shapes), expected_roles);
}

/// The PAIRED case: a product prepared with NO vocabulary restores none, and
/// collects no roles.
///
/// Absent and present-and-empty are different configurations, and a codec that
/// conflated them would let one product open against the other's inputs.
#[test]
fn admit_under_none_vocab_restores_none() {
    let ttl = format!("{PREFIXES}{ROLE_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl, None).expect("parses");
    let parsed = from_dataset_with_config_and_graph(&dataset, &extract_prefixes(&ttl), None, None)
        .expect("shapes parse");
    assert!(parsed.box_role_vocab.is_none());
    assert_eq!(
        box_roles(&parsed),
        vec![Vec::<String>::new()],
        "with the feature inactive the SAME source document collects no roles",
    );

    let bytes = PreparedShapes::new(Arc::new(parsed))
        .to_product(&ShapesProfile::CORE)
        .expect("representable");
    let view = ShapesProduct::open(&bytes).expect("opens");
    assert!(view.declared_provenance().box_role_vocab().is_none());

    let restored = view
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");
    let restored_shapes = restored_shapes_of(&restored);
    assert!(restored_shapes.box_role_vocab.is_none());
    assert_eq!(box_roles(&restored_shapes), vec![Vec::<String>::new()]);
}

/// The shapes graph a preparation holds. In-crate reach, so the restored value's
/// fields can be observed directly rather than inferred from a report.
fn restored_shapes_of(prepared: &PreparedShapes) -> Arc<Shapes> {
    Arc::clone(prepared.shapes())
}

// ── Forward compatibility ──────────────────────────────────────────────────────

/// A product whose stage id this build does not know refuses `admit` — the memo
/// describes a model this build no longer has — and still `rebuild`s, because the
/// dataset it carries is authenticated and self-sufficient.
#[test]
fn stage_id_mismatch_refuses_admit_but_rebuild_succeeds() {
    let data = data_of(PLAIN_DATA);
    let original = product_of(PLAIN_SHAPES);
    let expected = report_nt(
        &ShapesProduct::open(&original)
            .expect("opens")
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .expect("admits"),
        &data,
    );

    let foreign = repack(&original, |sections| {
        // A stage id from some other build. Every digest is recomputed by the
        // repack, so the envelope itself is beyond reproach: what is being tested
        // is the stage check, not the container.
        sections[0][..32].copy_from_slice(&[0xAB; 32]);
    });

    let view = ShapesProduct::open(&foreign).expect("a foreign stage id still OPENS");
    assert_eq!(view.stage_id(), &[0xAB; 32]);

    let error = ShapesProduct::open(&foreign)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect_err("a memo written against another model must not be trusted");
    assert_eq!(error.dimension(), ProductDimension::StageId);

    let rebuilt = ShapesProduct::open(&foreign)
        .expect("opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the carried dataset is enough to re-derive the preparation");
    assert_eq!(
        report_nt(&rebuilt, &data),
        expected,
        "the rescued preparation must answer what the original did",
    );
}

/// The paired valid case: the stage id this build DOES write admits.
#[test]
fn matching_stage_id_admits() {
    let bytes = product_of(PLAIN_SHAPES);
    let view = ShapesProduct::open(&bytes).expect("opens");
    assert_eq!(view.stage_id(), &STAGE_ID);
    view.admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the stage id this build writes is the one it admits");
}

// ── Inspection without admission ───────────────────────────────────────────────

/// A product's declared identity is readable without admitting it — and readable
/// even for a product this build refuses to admit, which is the case that matters.
#[test]
fn declared_identity_readable_without_admission() {
    let bytes = product_of(PLAIN_SHAPES);
    let view = ShapesProduct::open(&bytes).expect("opens");

    let labels: Vec<&str> = view
        .declared_identity()
        .components()
        .iter()
        .map(purrdf_core::artifact::IdentityComponent::label)
        .collect();
    assert_eq!(
        labels,
        vec![
            "source-dataset",
            "shapes-graph",
            "doc-prefixes",
            "base",
            "profile",
            "box-role-vocab",
            "user-functions-declared",
            "user-functions-injected",
            "aggregate-registry",
            "property-function-registry",
            "class-catalog",
            "parse-configuration",
        ],
        "the binding is DECODABLE, which is what lets a caller see which input moved",
    );
    assert_eq!(
        view.declared_identity().component("profile"),
        Some(ShapesProfile::CORE.id().as_bytes()),
    );
    assert_eq!(view.format_version(), 1);

    // The inspection surface is the same on a product this build will not admit.
    let foreign = repack(&bytes, |sections| {
        sections[0][..32].copy_from_slice(&[0xCD; 32]);
    });
    let refused = ShapesProduct::open(&foreign).expect("opens");
    assert_eq!(
        refused.declared_identity().component("profile"),
        Some(ShapesProfile::CORE.id().as_bytes()),
        "a product that cannot be admitted must still be able to explain itself",
    );
}

// ── A declared SPARQL function survives the round trip ─────────────────────────

/// A `sh:SPARQLFunction` the shapes graph declares is reinstated by a restore, so a
/// product answers EXACTLY what the document answers — including on data that flips
/// the verdict.
///
/// The declaration is re-derived from the shapes dataset the product already
/// carries, which is the same move the class catalog makes. Nothing about it reaches
/// the encoded model, so this property is about a capability rather than about a new
/// field: a restore that skipped the re-derivation would resolve `ex:double(?value)`
/// to nothing, the filter would never hold, and every report below would come back
/// conforming.
#[test]
fn a_declared_sparql_function_survives_the_round_trip() {
    let data = data_of(SPARQL_FN_DATA);
    let document = prepare(SPARQL_FN_SHAPES);
    let expected = report_nt(&document, &data);
    assert!(
        expected.contains("http://example.org/ns#high")
            && !expected.contains("http://example.org/ns#low"),
        "the function must actually decide the verdict, or this property is vacuous: {expected}",
    );

    let bytes = prepare(SPARQL_FN_SHAPES)
        .to_product(&ShapesProfile::CORE)
        .expect("a sh:SPARQLFunction declaration is representable");
    let admitted = ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");
    assert_eq!(
        report_nt(&admitted, &data),
        expected,
        "an admitted product must answer what the document answers, byte for byte",
    );

    let rebuilt = ShapesProduct::open(&bytes)
        .expect("opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("rebuilds");
    assert_eq!(
        report_nt(&rebuilt, &data),
        expected,
        "the two restore seams must agree about a declared SPARQL function too",
    );

    // THE NEGATIVE CONTROL. Move both values across the cap: the verdict must flip,
    // and it must flip IDENTICALLY through both routes. Without this the assertions
    // above would hold just as well for two routes that both silently report nothing.
    let flipped = data_of(SPARQL_FN_DATA_FLIPPED);
    let flipped_expected = report_nt(&document, &flipped);
    assert!(
        flipped_expected.contains("http://example.org/ns#low")
            && !flipped_expected.contains("http://example.org/ns#high"),
        "the control must genuinely flip which node the function condemns: {flipped_expected}",
    );
    assert_ne!(flipped_expected, expected);
    assert_eq!(report_nt(&admitted, &flipped), flipped_expected);
    assert_eq!(report_nt(&rebuilt, &flipped), flipped_expected);
}

// ── The writer refuses before it emits ─────────────────────────────────────────

/// A declared function the product's model cannot reach is refused at WRITE time,
/// with nothing emitted.
///
/// The construct is a `sh:ListParameterExpressionFunction` that no node expression
/// calls: nothing in the declarative model reaches the declaration, so the model
/// cannot carry it — while a `sh:sparql` body still names it, so a restore would
/// resolve that call site to nothing and validate green. The refusal is reached
/// through the registry fingerprint comparison rather than through a probe for this
/// case, which is what makes it hold for a kind nobody has thought of yet.
#[test]
fn to_product_refuses_before_emitting_bytes() {
    let prepared = prepare(UNREACHABLE_FN_SHAPES);
    let error = prepared
        .to_product(&ShapesProfile::CORE)
        .expect_err("a declaration the model cannot reach is not representable");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
    assert!(
        error.message().contains("validate green"),
        "the refusal must name the harm it prevents: {error}",
    );

    // The NEIGHBOURING valid cases. Without these the check above would pass just as
    // well for a writer that refused every function-bearing shapes graph — which is
    // exactly the over-refusal this pairing exists to catch.
    //
    // A §7.3 list-parameter declaration a node expression DOES call:
    let bytes = prepare(EXPRESSION_FN_SHAPES)
        .to_product(&ShapesProfile::CORE)
        .expect("an expression-bodied declaration the model reaches is representable");
    ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");

    // …and a SHACL-AF §5 `sh:SPARQLFunction`, which the shapes dataset states and a
    // restore re-derives:
    let bytes = prepare(SPARQL_FN_SHAPES)
        .to_product(&ShapesProfile::CORE)
        .expect("a SPARQL-bodied declaration is representable");
    ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");
}

// ── Certification is the cold path ─────────────────────────────────────────────

/// A product whose stored canonical digest has been tampered with — every section
/// digest intact — ADMITS and FAILS certification.
///
/// This is the test that catches someone moving certification onto the common path:
/// the moment `admit` re-canonicalizes the shapes dataset, the first assertion
/// fails.
#[test]
fn certify_is_not_reachable_from_admit() {
    let bytes = product_of(PLAIN_SHAPES);
    ShapesProduct::open(&bytes)
        .expect("opens")
        .certify()
        .expect("a freshly written product certifies");

    let tampered = repack(&bytes, |sections| {
        let window = &mut sections[1][RDFC_DIGEST_OFFSET..RDFC_DIGEST_OFFSET + 32];
        assert_ne!(
            window,
            &[0xEE_u8; 32][..],
            "the tamper must actually change the stored digest",
        );
        window.copy_from_slice(&[0xEE; 32]);
    });

    let data = data_of(PLAIN_DATA);
    let restored = ShapesProduct::open(&tampered)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admit must NOT canonicalize, so a tampered canonical digest is invisible to it");
    assert_eq!(
        report_nt(&restored, &data),
        report_nt(
            &ShapesProduct::open(&bytes)
                .expect("opens")
                .admit(&ShapesProfile::CORE, &HostBindings::empty())
                .expect("admits"),
            &data,
        ),
        "the tamper touched an identity field, not the data, so the restore is unaffected",
    );

    let error = ShapesProduct::open(&tampered)
        .expect("opens")
        .certify()
        .expect_err("certification recomputes, so it sees what admission cannot");
    assert_eq!(error.dimension(), ProductDimension::DatasetIdentity);
}

/// A section altered WITHOUT repacking is caught by the envelope, so the tamper
/// above is exercising the identity tier rather than a hole in the container.
#[test]
fn a_raw_section_edit_is_refused_by_the_envelope() {
    let mut bytes = product_of(PLAIN_SHAPES);
    ShapesProduct::open(&bytes).expect("the untouched product opens");
    let offset = bytes.len() / 2;
    bytes[offset] ^= 0xFF;

    let error = ShapesProduct::open(&bytes).expect_err("an edited section is refused");
    assert!(
        matches!(
            error.dimension(),
            ProductDimension::SectionDigest | ProductDimension::ContainerDigest
        ),
        "an in-place edit is corruption, reported as such: {error}",
    );
}

// ── Targets are not persisted ──────────────────────────────────────────────────

/// A product carries no target-resolution section, and a restored preparation bound
/// to a DIFFERENT dataset resolves that dataset's targets.
///
/// Target resolution is a property of a data graph, never of a shapes graph. A
/// product that cached it would validate a new snapshot against the focus nodes of
/// an old one: green reports about nodes that are no longer there, and silence
/// about the ones that are.
#[test]
fn targets_not_persisted() {
    let bytes = product_of(PLAIN_SHAPES);
    let view = ShapesProduct::open(&bytes).expect("opens");
    assert_eq!(
        view.section_kinds(),
        vec![SECTION_IDENTITY, SECTION_DATASET, SECTION_AST],
        "the directory is closed and total: identity, dataset, ast, and nothing else",
    );
    let restored = view
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");

    let producer_report = validate(&restored, &data_of(PLAIN_DATA));
    let producer_focus: Vec<String> = producer_report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(
        producer_focus,
        vec!["<http://example.org/ns#bob>".to_owned()]
    );

    let other_report = validate(&restored, &data_of(OTHER_DATA));
    let other_focus: Vec<String> = other_report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(
        other_focus,
        vec!["<http://example.org/ns#carol>".to_owned()],
        "the second binding resolves the SECOND dataset's targets, not the producer's",
    );
}

// ── The governed decode ────────────────────────────────────────────────────────

/// A scratch ceiling below the product's own declared size refuses the restore at
/// ADMISSION, before anything is decoded — and a ceiling above it admits.
///
/// The paired case is the whole point: an estimator that refused a product that
/// would have fit is over-refusal, which looks exactly like correct strictness.
#[test]
fn governed_decode_refuses_over_the_ceiling_and_admits_under_it() {
    let bytes = product_of(PLAIN_SHAPES);
    let declared = {
        let view = ShapesProduct::open(&bytes).expect("opens");
        (view.section(SECTION_IDENTITY).expect("identity").len()
            + view.section(SECTION_DATASET).expect("dataset").len()
            + view.section(SECTION_AST).expect("ast").len()) as u64
    };
    assert!(declared > 0);

    {
        let governors = QueryGovernors::UNBOUNDED.with_max_scratch_bytes(declared - 1);
        let state = Arc::new(GovernorState::new(&governors));
        let _scope = crate::sparql::enter_governor_scope(Arc::clone(&state));
        let error = ShapesProduct::open(&bytes)
            .expect("opens")
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .expect_err("a ceiling below the product's own declared size refuses it");
        assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
        assert!(
            error.message().contains("scratch"),
            "the refusal must name the governor that refused it: {error}",
        );
        assert!(
            state
                .tripped()
                .is_some_and(|tripped| tripped.label() == "scratch-admission-refused"),
            "the trip is recorded as an ADMISSION refusal, not a spent budget",
        );
    }

    {
        // The neighbouring valid case, at the exact boundary: the ceiling is
        // INCLUSIVE, so a product whose declared size equals it must restore.
        let governors = QueryGovernors::UNBOUNDED.with_max_scratch_bytes(declared);
        let state = Arc::new(GovernorState::new(&governors));
        let _scope = crate::sparql::enter_governor_scope(state);
        ShapesProduct::open(&bytes)
            .expect("opens")
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .expect("a product that fits the ceiling must restore");
    }

    // ...and an ungoverned thread pays nothing and restores.
    ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("an ungoverned restore is unaffected");
}

// ── Structural refusals, each with its valid neighbour ──────────────────────────

/// Bytes that were never a product are refused on the magic, while the real product
/// they were derived from still opens.
#[test]
fn foreign_bytes_refuse_on_the_magic() {
    // Long enough that the buffer is not refused for its LENGTH first: the claim
    // is about the magic, so the bytes have to get as far as the magic check.
    let foreign = vec![0x5A_u8; 512];
    let error = ShapesProduct::open(&foreign).expect_err("foreign bytes refuse");
    assert_eq!(error.dimension(), ProductDimension::Magic);

    let bytes = product_of(PLAIN_SHAPES);
    ShapesProduct::open(&bytes).expect("the real product opens");
}

/// A truncated product is refused as truncated, and the untruncated original opens.
#[test]
fn a_truncated_product_refuses() {
    let bytes = product_of(PLAIN_SHAPES);
    let error = ShapesProduct::open(&bytes[..bytes.len() - 1]).expect_err("truncation refuses");
    assert!(
        matches!(
            error.dimension(),
            ProductDimension::Truncated | ProductDimension::Trailer
        ),
        "a short buffer is an interrupted write: {error}",
    );
    ShapesProduct::open(&bytes).expect("the whole buffer opens");
}

/// An identity section with bytes appended is refused: a writer that produced that
/// section produced exactly its bytes, and ignoring a remainder is how a partial
/// write passes for a whole one.
#[test]
fn trailing_bytes_in_the_identity_section_refuse() {
    let bytes = product_of(PLAIN_SHAPES);
    let padded = repack(&bytes, |sections| sections[0].push(0));
    let error = ShapesProduct::open(&padded).expect_err("a padded section refuses");
    assert_eq!(error.dimension(), ProductDimension::Malformed);

    // The neighbour: the same repack WITHOUT the extra byte still opens, so the
    // refusal is about the padding and not about repacking.
    let repacked = repack(&bytes, |_| {});
    assert_eq!(
        repacked, bytes,
        "a repack of untouched sections is the identity"
    );
    ShapesProduct::open(&repacked).expect("opens");
}

/// A product whose model and whose recorded parse inputs name different
/// shapes-graph IRIs is refused rather than restored inconsistently.
#[test]
fn disagreeing_carriers_refuse() {
    let ttl = format!("{PREFIXES}{PLAIN_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl, None).expect("parses");
    let shapes = from_dataset_with_config_and_graph(
        &dataset,
        &extract_prefixes(&ttl),
        None,
        Some("http://example.org/shapes".to_owned()),
    )
    .expect("shapes parse");
    let bytes = PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("representable");

    // The valid neighbour first: the untouched product restores.
    ShapesProduct::open(&bytes)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("admits");

    // Now rewrite ONLY the identity section, from a parse that names no shapes
    // graph. Both halves are internally well formed; they simply describe two
    // different parses.
    let other = shapes_of(PLAIN_SHAPES);
    let preamble = super::encode_preamble(&other);
    let split = repack(&bytes, |sections| sections[0] = preamble);

    let error = ShapesProduct::open(&split)
        .expect("opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect_err("two carriers describing two parses must not restore");
    assert!(
        matches!(
            error.dimension(),
            ProductDimension::Malformed | ProductDimension::ShapesGraph
        ),
        "got: {error}",
    );
}

/// Writing is deterministic: two preparations of the same shapes graph produce
/// byte-identical products.
#[test]
fn writing_is_deterministic() {
    assert_eq!(product_of(PLAIN_SHAPES), product_of(PLAIN_SHAPES));
}
