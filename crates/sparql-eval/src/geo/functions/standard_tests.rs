// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fmt::Write as _, sync::Arc};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_geo_kernel::{AxisOrder, Crs, GeoProfile, GeoVocab, GeographicReference};
use purrdf_iri::vocab::ogc;

use crate::{
    Arity, NativeSparqlEngine, QueryGovernors, QueryOptions, UserFunctionRegistry, Volatility,
};

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[derive(Default)]
struct OutputLedger {
    calls: u32,
    work: u64,
    peak: u64,
    refuse_call: Option<u32>,
}

impl purrdf_geo_kernel::MetricWorkObserver for OutputLedger {
    fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), purrdf_geo_kernel::GeoError> {
        self.calls += 1;
        if self.refuse_call == Some(self.calls) {
            return Err(purrdf_geo_kernel::GeoError::Cancelled);
        }
        self.work += work;
        self.peak += growth;
        Ok(())
    }
}

#[test]
fn carrier_term_moves_admitted_text_and_charges_its_remaining_owned_storage() {
    for (kind, datatype, lexical) in [
        (
            purrdf_geo_kernel::GeoTerm::WktLiteral,
            ogc::geo::WKT_LITERAL,
            "POINT(1 2)",
        ),
        (
            purrdf_geo_kernel::GeoTerm::GeoJsonLiteral,
            ogc::geo::GEO_JSON_LITERAL,
            "{\"type\":\"Point\",\"coordinates\":[1,2]}",
        ),
    ] {
        let lexical = lexical.to_owned();
        let lexical_bytes = lexical.capacity() as u64;
        let mut context = purrdf_geo_kernel::MetricContext::wgs84().unwrap();
        context.set_preparation_work(17).unwrap();
        context.set_retained_workspace(256).unwrap();
        context.admit_workspace(lexical_bytes).unwrap();
        let mut receipt = OutputLedger {
            work: context.work_items(),
            peak: context.workspace_peak(),
            ..OutputLedger::default()
        };
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let term =
            super::geographic_carrier_term(lexical, kind, &mut context, &mut receipt).unwrap();
        let allocations = window.close();
        assert_eq!(allocations.allocations, 1);
        assert_eq!(allocations.requested_bytes, datatype.len() as u64);
        let result_bytes = (size_of::<TermValue>() + datatype.len()) as u64;
        assert_eq!(receipt.work, context.work_items());
        assert_eq!(receipt.peak, context.workspace_peak());
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes - 256 - lexical_bytes - result_bytes
        );
        drop(term);
        context
            .release_workspace(lexical_bytes + result_bytes)
            .unwrap();
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes - 256
        );

        let lexical = "POINT(1 2)".to_owned();
        context.admit_workspace(lexical.capacity() as u64).unwrap();
        let mut receipt = OutputLedger {
            work: context.work_items(),
            peak: context.workspace_peak(),
            refuse_call: Some(1),
            ..OutputLedger::default()
        };
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refusal = super::geographic_carrier_term(lexical, kind, &mut context, &mut receipt);
        let allocations = window.close();
        assert_eq!(refusal, Err(purrdf_geo_kernel::GeoError::Cancelled));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes - 256
        );
    }
}

fn dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("empty dataset")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn distance_query(a: &str, b: &str) -> String {
    format!(
        "SELECT (<{}>(\"{a}\"^^<{}>,\"{b}\"^^<{}>) AS ?distance) WHERE {{}}",
        ogc::geof::METRIC_DISTANCE,
        ogc::geo::WKT_LITERAL,
        ogc::geo::WKT_LITERAL
    )
}

fn distance(result: SparqlResult) -> TermValue {
    match result {
        SparqlResult::Solutions { rows, .. } => rows[0][0].clone().expect("a bound metric"),
        other => panic!("expected SELECT solutions, received {other:?}"),
    }
}

#[test]
fn actual_new_default_empty_and_prepared_use_the_ellipsoidal_kernel() {
    let dataset = dataset();
    let query = distance_query("POINT(0 0)", "POINT(1 0)");
    let expected = TermValue::typed_literal("1.11319490793E5", purrdf_xsd::datatype::XSD_DOUBLE);
    for engine in [NativeSparqlEngine::new(), NativeSparqlEngine::default()] {
        assert_eq!(
            distance(
                engine
                    .query(&dataset, request(&query))
                    .expect("ordinary raw-engine API")
            ),
            expected
        );
        let ordinary = engine
            .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
            .expect("standard functions are installed");
        assert_eq!(distance(ordinary), expected);
        let prepared = engine
            .prepare_query_with_options(&query, None, QueryOptions::EMPTY)
            .expect("standard prepare");
        assert_eq!(
            prepared.geo_identity(),
            purrdf_geo_kernel::binding::STANDARD_PROFILE.query_identity()
        );
        let replay = engine
            .query_prepared_view(&*dataset, &prepared, &[], QueryOptions::EMPTY)
            .expect("standard prepared call");
        assert_eq!(distance(replay), expected);
    }
}

#[test]
fn standard_geographic_rows_match_in_sequential_and_parallel_workers() {
    let data = dataset();
    let engine = NativeSparqlEngine::new();
    let mut values = String::new();
    for index in 0..64 {
        write!(
            values,
            " ({index} \"POINT({} 0)\"^^<{}>)",
            index % 2,
            ogc::geo::WKT_LITERAL
        )
        .unwrap();
    }
    let query = format!(
        "SELECT ?row (<{}>(?start,\"POINT(1 0)\"^^<{}>) AS ?distance) WHERE {{ VALUES (?row ?start) {{{values}}} }} ORDER BY ?row",
        ogc::geof::METRIC_DISTANCE,
        ogc::geo::WKT_LITERAL,
    );
    let run = |parallel| {
        let _guard = crate::parallel::force_parallel_for_test(parallel);
        let SparqlResult::Solutions {
            variables,
            rows,
            aux,
        } = engine
            .query_with_options_view(&*data, request(&query), QueryOptions::EMPTY)
            .unwrap()
        else {
            panic!("the geographic row query must produce SELECT solutions");
        };
        assert_eq!(aux.quad_count(), 0);
        (variables, rows)
    };
    let sequential = run(false);
    assert_eq!(run(true), sequential);
    let (_, rows) = sequential;
    assert_eq!(rows.len(), 64);
    let expected = [
        TermValue::typed_literal("1.11319490793E5", purrdf_xsd::datatype::XSD_DOUBLE),
        TermValue::typed_literal("0.0E0", purrdf_xsd::datatype::XSD_DOUBLE),
    ];
    assert!(
        rows.iter()
            .enumerate()
            .all(|(index, row)| row[1].as_ref() == Some(&expected[index % 2]))
    );
}

#[test]
fn standard_buffers_use_complete_native_materialization_and_governed_admission() {
    let data = dataset();
    let engine = NativeSparqlEngine::new();
    let expected = purrdf_geo_kernel::wkt::parse(
        "MULTIPOINT(0.1234567890123456789 0)",
        purrdf_geo_kernel::standard_vocabulary().default_wkt_crs(),
    )
    .unwrap();
    for (function, extra) in [
        (ogc::geof::METRIC_BUFFER, String::new()),
        (ogc::geof::BUFFER, format!(",<{}>", ogc::uom::METRE)),
    ] {
        let query = format!(
            "SELECT (<{function}>(\"POINT(0.1234567890123456789 0)\"^^<{}>,0{extra}) AS ?buffer) WHERE {{}}",
            ogc::geo::WKT_LITERAL
        );
        let result = engine
            .query_with_options_view(&*data, request(&query), QueryOptions::EMPTY)
            .unwrap();
        let term = distance(result);
        let actual = purrdf_geo_kernel::carrier::geometry_arg(
            purrdf_geo_kernel::standard_vocabulary(),
            &term,
        )
        .unwrap();
        assert_eq!(actual, expected);
        let outcome = engine
            .query_governed(
                &data,
                request(&query),
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(100),
            )
            .unwrap();
        assert!(matches!(
            outcome,
            crate::GovernedOutcome::BudgetExhausted(_)
        ));
        assert!(outcome.evidence().tripped().is_some());
    }
    let identity = purrdf_geo_kernel::binding::STANDARD_PROFILE.query_identity();
    assert_eq!(
        identity.point_buffer_materialization_law,
        purrdf_geo_kernel::buffer::BufferMaterialization::output_law_id()
    );
    assert_eq!(
        identity.buffer_materialization_law,
        purrdf_geo_kernel::buffer::BufferMaterialization::region_output_law_id()
    );
    assert_eq!(
        identity.global_point_buffer_materialization_law,
        purrdf_geo_kernel::buffer::BufferMaterialization::global_output_law_id()
    );
    assert_eq!(
        identity.global_buffer_materialization_law,
        purrdf_geo_kernel::buffer::BufferMaterialization::global_region_output_law_id()
    );
}

#[test]
fn standard_relations_identify_geographic_poles_and_longitude_seams() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    for (a, b) in [
        ("POINT(17 90)", "POINT(-23 90)"),
        ("POINT(180 5)", "POINT(-180 5)"),
    ] {
        let query = format!(
            "SELECT (<{}>(\"{a}\"^^<{}>,\"{b}\"^^<{}>) AS ?equal) WHERE {{}}",
            ogc::geof::SF_EQUALS,
            ogc::geo::WKT_LITERAL,
            ogc::geo::WKT_LITERAL
        );
        assert_eq!(
            distance(engine.query(&data, request(&query)).unwrap()),
            TermValue::boolean(true)
        );
    }
    assert_eq!(
        purrdf_geo_kernel::binding::STANDARD_PROFILE
            .query_identity()
            .topology_law,
        purrdf_geo_kernel::atlas::topology_law_id()
    );
}

#[test]
fn standard_relations_use_completed_dimensions_for_constant_source_curves() {
    let query = format!(
        "SELECT (<{}>(\"MULTILINESTRING((0 0,0 0),(1 0,1 0))\"^^<{}>,\"MULTILINESTRING((0 0,0 0),(2 0,2 0))\"^^<{}>) AS ?overlap) WHERE {{}}",
        ogc::geof::SF_OVERLAPS,
        ogc::geo::WKT_LITERAL,
        ogc::geo::WKT_LITERAL
    );
    assert_eq!(
        distance(
            NativeSparqlEngine::new()
                .query(&dataset(), request(&query))
                .unwrap()
        ),
        TermValue::boolean(true)
    );
}

#[test]
fn complete_length_and_exact_declared_unit_reach_standard_dispatch() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let mut profile = GeoProfile::standard();
    let unit = Crs::new("http://example.org/kilometre").expect("unit IRI");
    profile
        .register_linear_unit(unit.clone(), purrdf_geo_kernel::Rat::from_i64(1000))
        .expect("exact output unit");
    let query = format!(
        "SELECT (<{}>(\"LINESTRING(0 0,1 0)\"^^<{}>, <{}>) AS ?distance) WHERE {{}}",
        ogc::geof::LENGTH,
        ogc::geo::WKT_LITERAL,
        unit
    );
    let answer = engine
        .query_with_options_view(
            &*data,
            request(&query),
            QueryOptions::EMPTY.with_geo(&profile),
        )
        .expect("geographic length with declared unit");
    assert_eq!(
        distance(answer),
        TermValue::typed_literal("1.11319490793E2", purrdf_xsd::datatype::XSD_DOUBLE)
    );
    let too_small = GeoProfile::STANDARD
        .with_limits(purrdf_geo_kernel::ExecutionLimits {
            max_precision_bits: 16,
            ..purrdf_geo_kernel::ExecutionLimits::GEOMETRY
        })
        .expect("admitted low precision");
    let query = format!(
        "SELECT ?length WHERE {{ BIND (<{}>(\"LINESTRING(0 0,1 0)\"^^<{}>) AS ?length) }}",
        ogc::geof::METRIC_LENGTH,
        ogc::geo::WKT_LITERAL
    );
    assert!(
        engine
            .query_with_options_view(
                &*data,
                request(&query),
                QueryOptions::EMPTY.with_geo(&too_small)
            )
            .is_err(),
        "operational precision refusal propagates through BIND"
    );
}

#[test]
fn nested_sparql_bodies_and_update_where_resolve_the_same_standard_set() {
    let engine = NativeSparqlEngine::new();
    let mut dataset = dataset();
    let expression = format!(
        "<{}>(\"POINT(0 0)\"^^<{}>,\"POINT(1 0)\"^^<{}>)",
        ogc::geof::METRIC_DISTANCE,
        ogc::geo::WKT_LITERAL,
        ogc::geo::WKT_LITERAL
    );
    let mut declarations = UserFunctionRegistry::default();
    declarations.insert(
        "http://example.org/function/nested",
        crate::UserFunction {
            params: Vec::new(),
            required: 0,
            body: format!("SELECT ({expression} AS ?distance) WHERE {{}}").into(),
            kind: crate::UserFnBody::Select,
            return_constraint: crate::TypeConstraint::default(),
        },
    );
    let functions = engine
        .bind_functions(declarations, crate::ExtensionEnv::empty())
        .expect("nested body admission");
    let nested = engine
        .query_with_options_view(
            &*dataset,
            request("SELECT (<http://example.org/function/nested>() AS ?distance) WHERE {}"),
            QueryOptions::EMPTY.with_functions(&functions),
        )
        .expect("nested standard resolution");
    assert_eq!(
        distance(nested),
        TermValue::typed_literal("1.11319490793E5", purrdf_xsd::datatype::XSD_DOUBLE)
    );
    let update = format!(
        "INSERT {{ <http://example.org/s> <http://example.org/distance> ?distance }} WHERE {{ BIND({expression} AS ?distance) }}"
    );
    engine
        .update_with_options(&mut dataset, request(&update), QueryOptions::EMPTY)
        .expect("UPDATE standard resolution");
    let read = engine.query(&dataset, request("SELECT ?distance WHERE { <http://example.org/s> <http://example.org/distance> ?distance }")).expect("stored metric result");
    assert_eq!(
        distance(read),
        TermValue::typed_literal("1.11319490793E5", purrdf_xsd::datatype::XSD_DOUBLE)
    );
}

#[test]
fn every_overlay_kind_refuses_standard_shadow_before_parsing_a_body() {
    let engine = NativeSparqlEngine::new();
    let mut sparql = UserFunctionRegistry::default();
    sparql.insert(
        ogc::geof::IS_EMPTY,
        crate::UserFunction {
            params: Vec::new(),
            required: 0,
            body: "intentionally invalid query".into(),
            kind: crate::UserFnBody::Ask,
            return_constraint: crate::TypeConstraint::default(),
        },
    );
    let mut expression = UserFunctionRegistry::default();
    expression.register_expr(
        ogc::geof::IS_EMPTY,
        Arity::Exact(1),
        Arc::new(|_| panic!("a shadow is never entered")),
    );
    for declarations in [sparql, expression] {
        let error = engine
            .bind_functions(declarations, crate::ExtensionEnv::empty())
            .expect_err("sealed function conflict outranks body parsing");
        assert_eq!(error.code, "native-sparql-standard-function-conflict");
    }
}

#[test]
fn a_real_precision_exhaustion_remains_fatal_in_filter_and_bind() {
    let engine = NativeSparqlEngine::new();
    let dataset = dataset();
    let mut limits = purrdf_geo_kernel::ExecutionLimits::GEOMETRY;
    limits.max_precision_bits = 16;
    let profile = GeoProfile::STANDARD
        .with_limits(limits)
        .expect("positive inadequate precision is admitted");
    let expression = format!(
        "<{}>(\"POINT(0 0)\"^^<{}>,\"POINT(1 0)\"^^<{}>)",
        ogc::geof::METRIC_DISTANCE,
        ogc::geo::WKT_LITERAL,
        ogc::geo::WKT_LITERAL
    );
    for clause in [
        format!("FILTER({expression} > 0)"),
        format!("BIND({expression} AS ?distance)"),
    ] {
        let query = format!("SELECT * WHERE {{ {clause} }}");
        let error = engine
            .query_with_options_view(
                &*dataset,
                request(&query),
                QueryOptions::EMPTY.with_geo(&profile),
            )
            .expect_err("a numerical refusal cannot drop or unbind a row");
        assert!(error.message.contains("precision"));
    }
}

#[test]
fn explicit_epsg_registration_swaps_source_axes_and_preserves_crs84() {
    let dataset = dataset();
    let engine = NativeSparqlEngine::new();
    let query = distance_query(&format!("<{}> POINT(0 1)", ogc::EPSG4326), "POINT(0 0)");
    let refused = engine
        .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
        .expect_err("EPSG:4326 is not implicit");
    assert!(refused.message.contains("unregistered"));
    let mut profile = GeoProfile::STANDARD;
    profile
        .register_reference(
            Crs::new(ogc::EPSG4326).expect("IRI"),
            GeographicReference::wgs84().with_axes(AxisOrder::LatLon),
        )
        .expect("explicit reference");
    let answer = engine
        .query_with_options_view(
            &*dataset,
            request(&query),
            QueryOptions::EMPTY.with_geo(&profile),
        )
        .expect("declared axes");
    assert_eq!(
        distance(answer),
        TermValue::typed_literal("1.11319490793E5", purrdf_xsd::datatype::XSD_DOUBLE)
    );
}

#[test]
fn sealed_identical_registration_coalesces_and_arbitrary_shadow_refuses_before_use() {
    let engine = NativeSparqlEngine::new();
    let dataset = dataset();
    let mut identical = UserFunctionRegistry::default();
    super::register(&mut identical, &GeoVocab::standard());
    let identical = engine
        .bind_functions(identical, crate::ExtensionEnv::empty())
        .expect("sealed identical set");
    let query = distance_query("POINT(0 0)", "POINT(0 0)");
    let result = engine
        .query_with_options_view(
            &*dataset,
            request(&query),
            QueryOptions::EMPTY.with_functions(&identical),
        )
        .expect("coalesced");
    assert_eq!(
        distance(result),
        TermValue::typed_literal("0.0E0", purrdf_xsd::datatype::XSD_DOUBLE)
    );
    for iri in [ogc::geof::METRIC_DISTANCE, ogc::geof::IS_EMPTY] {
        let mut shadow = UserFunctionRegistry::default();
        shadow.register_native(
            iri,
            Arity::Exact(2),
            Volatility::Stable,
            Arc::new(|_| panic!("a shadow must never execute")),
        );
        let error = engine
            .bind_functions(shadow, crate::ExtensionEnv::empty())
            .expect_err("standard collision");
        assert_eq!(error.code, "native-sparql-standard-function-conflict");
        assert!(error.message.contains(iri));
    }
}

#[test]
fn prepared_cache_and_admission_bind_reference_and_policy_independently() {
    let engine = NativeSparqlEngine::new();
    let dataset = dataset();
    let mut profile = GeoProfile::STANDARD;
    profile
        .register_reference(
            Crs::new(ogc::EPSG4326).expect("IRI"),
            GeographicReference::wgs84().with_axes(AxisOrder::LatLon),
        )
        .expect("reference");
    let plain = engine
        .prepare_query_with_options("ASK {}", None, QueryOptions::EMPTY)
        .expect("plain plan");
    let bound = engine
        .prepare_query_with_options("ASK {}", None, QueryOptions::EMPTY.with_geo(&profile))
        .expect("bound plan");
    assert!(!Arc::ptr_eq(&plain, &bound));
    let error = engine
        .query_prepared_view(
            &*dataset,
            &plain,
            &[],
            QueryOptions::EMPTY.with_geo(&profile),
        )
        .expect_err("no stale profile replay");
    assert_eq!(error.code, "native-sparql-geo-profile-mismatch");
    assert_ne!(plain.geo_identity().binding, bound.geo_identity().binding);
    assert_eq!(
        plain.geo_identity().distance_law,
        bound.geo_identity().distance_law
    );
    assert_eq!(plain.geo_identity().policy, bound.geo_identity().policy);
}

#[test]
fn a_governor_trip_inside_geodesy_withholds_the_filter_or_bind_answer() {
    let engine = NativeSparqlEngine::new();
    let dataset = dataset();
    let query = distance_query("POINT(0 0)", "POINT(1 0)");
    let answer = engine
        .query_governed(
            &dataset,
            request(&query),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_fuel(100),
        )
        .expect("a governor trip is an outcome");
    assert!(matches!(answer, crate::GovernedOutcome::BudgetExhausted(_)));
}

#[test]
fn a_parser_namespace_cannot_intercept_a_standard_function() {
    let options = purrdf_sparql_algebra::ParserOptions {
        extension_fn_namespaces: vec![ogc::geof::NS.to_owned()],
        ..purrdf_sparql_algebra::ParserOptions::default()
    };
    let environment =
        crate::ExtensionEnv::over_options(options).expect("configured parser registry");
    let error = NativeSparqlEngine::new()
        .prepare_query_with_options(
            "not valid SPARQL",
            None,
            QueryOptions::EMPTY.with_env(&environment),
        )
        .expect_err("namespace collision precedes parsing");
    assert_eq!(error.code, "native-sparql-standard-function-conflict");
}

#[test]
fn forged_closed_function_annotations_refuse_before_plan_admission() {
    use purrdf_sparql_algebra::{Expression, Function, GraphPattern, PurrdfCall, PurrdfFn};
    let pattern = GraphPattern::Filter {
        inner: GraphPattern::Bgp { patterns: vec![] }.into(),
        expr: Expression::FunctionCall(
            Function::Purrdf(PurrdfCall {
                fn_kind: PurrdfFn::ListLength,
                iri: ogc::geof::IS_EMPTY.to_owned(),
            }),
            vec![].into(),
        ),
    };
    let error =
        crate::user_fn::validate_standard_algebra(&pattern).expect_err("forged standard dispatch");
    assert!(
        matches!(error,crate::EvalError::StandardFunctionConflict {iri} if iri==ogc::geof::IS_EMPTY)
    );
}

#[test]
fn geojson_conversion_uses_actual_registered_continuous_chain() {
    use purrdf_geo_kernel::{
        CoordinateOperation, OperationChain, Rat,
        operation::{CoordinateUnit, OperationModel, OperationReference},
    };
    use purrdf_hash::hex::Digest32;
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let source = Crs::new("http://example.org/projected").expect("source carrier");
    let target = Crs::new(ogc::CRS84).expect("standard actual target");
    let inverse = CoordinateOperation::compile(
        OperationReference {
            realization: Digest32::new([7; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        },
        OperationReference {
            realization: GeographicReference::wgs84().id().digest(),
            unit: CoordinateUnit::Degrees,
            swapped_axes: false,
        },
        OperationModel::MercatorToGeographic {
            radius: Rat::from_i64(6_378_137),
            eccentricity_squared: Rat::zero(),
            square_domain: true,
        },
    )
    .expect("actual inverse projection");
    let mut profile = GeoProfile::standard();
    profile
        .register_operation(
            Crs::new("http://example.org/projection-inverse").expect("operation name"),
            source.clone(),
            target,
            OperationChain::compile(vec![inverse]).expect("chain"),
        )
        .expect("explicit carrier bindings");
    let query = format!(
        "SELECT (<{}>(\"<{}> POINT(0 0)\"^^<{}>) AS ?geometry) WHERE {{}}",
        ogc::geof::AS_GEO_JSON,
        source,
        ogc::geo::WKT_LITERAL
    );
    let actual = distance(
        engine
            .query_with_options_view(
                &*data,
                request(&query),
                QueryOptions::EMPTY.with_geo(&profile),
            )
            .expect("actual transformation"),
    );
    let expected = TermValue::typed_literal(
        "{\"type\":\"Point\",\"coordinates\":[0,0]}",
        ogc::geo::GEO_JSON_LITERAL,
    );
    assert_eq!(actual, expected);
    assert!(
        engine.query(&data, request(&query)).is_err(),
        "missing actual operation is fatal"
    );
}

#[test]
fn original_carrier_construction_refusal_is_fatal_in_filter_and_bind() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let expression = format!(
        "<{}>(\"POINT(1e100000 0)\"^^<{}>)",
        ogc::geof::IS_EMPTY,
        ogc::geo::WKT_LITERAL
    );
    for clause in [
        format!("FILTER({expression})"),
        format!("BIND({expression} AS ?empty)"),
    ] {
        let query = format!("SELECT * WHERE {{ {clause} }}");
        let error = engine
            .query(&data, request(&query))
            .expect_err("source construction cannot softly drop or unbind the row");
        assert!(error.message.contains("work"), "{error:?}");
    }
}

#[test]
fn governor_cancellation_during_carrier_construction_preserves_incomplete_evidence() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let expression = format!(
        "<{}>(\"POINT(0 0)\"^^<{}>)",
        ogc::geof::IS_EMPTY,
        ogc::geo::WKT_LITERAL
    );
    for clause in [
        format!("FILTER({expression})"),
        format!("BIND({expression} AS ?empty)"),
    ] {
        let query = format!("SELECT * WHERE {{ {clause} }}");
        let outcome = engine
            .query_governed(
                &data,
                request(&query),
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(100),
            )
            .expect("the parser's governor trip is an outcome");
        assert!(matches!(
            outcome,
            crate::GovernedOutcome::BudgetExhausted(_)
        ));
        assert!(outcome.evidence().tripped().is_some());
    }
}

#[test]
fn metric_literal_formatting_is_charged_before_allocation_and_keeps_live_output() {
    let mut context = purrdf_geo_kernel::MetricContext::wgs84().unwrap();
    context.set_preparation_work(17).unwrap();
    context.set_retained_workspace(256).unwrap();
    let mut ledger = OutputLedger {
        work: 17,
        peak: 256,
        ..OutputLedger::default()
    };
    let value = purrdf_geo_kernel::Rat::from_i64(1);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let term = super::geographic_double(&value, &mut context, &mut ledger).unwrap();
    let allocations = window.close();
    assert_eq!(allocations.allocations, 2);
    assert_eq!(
        allocations.requested_bytes,
        (purrdf_xsd::numeric::CANONICAL_IEEE_MAX_BYTES + purrdf_xsd::datatype::XSD_DOUBLE.len())
            as u64
    );
    assert_eq!(
        term,
        TermValue::typed_literal("1.0E0", purrdf_xsd::datatype::XSD_DOUBLE)
    );
    assert_eq!(ledger.work, context.work_items());
    assert_eq!(ledger.peak, context.workspace_peak());
    let output_bytes = (size_of::<TermValue>()
        + purrdf_xsd::numeric::CANONICAL_IEEE_MAX_BYTES
        + purrdf_xsd::datatype::XSD_DOUBLE.len()) as u64;
    assert_eq!(
        context.remaining_workspace(),
        context.policy().limits().max_workspace_bytes - 256 - output_bytes
    );
    drop(term);
    context.release_workspace(output_bytes).unwrap();
    assert_eq!(
        context.remaining_workspace(),
        context.policy().limits().max_workspace_bytes - 256
    );

    let mut context = purrdf_geo_kernel::MetricContext::wgs84().unwrap();
    let mut ledger = OutputLedger {
        // The exact ratio phase polls before and after its body. The next
        // callback precedes both final output string allocations.
        refuse_call: Some(3),
        ..OutputLedger::default()
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refusal = super::geographic_double(&value, &mut context, &mut ledger);
    let allocations = window.close();
    assert_eq!(refusal, Err(purrdf_geo_kernel::GeoError::Cancelled));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
    assert_eq!(
        context.remaining_workspace(),
        context.policy().limits().max_workspace_bytes
    );
}

#[test]
fn complete_unit_argument_refuses_before_iri_construction() {
    let profile = GeoProfile::STANDARD;
    let unit = TermValue::iri(format!("http://example.org/{}", "x".repeat(65536)));
    let policy = purrdf_geo_kernel::ExecutionPolicy::new(purrdf_geo_kernel::ExecutionLimits {
        max_work_items: 64,
        ..purrdf_geo_kernel::ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut budget = purrdf_geo_kernel::PreparationBudget::new(policy);
    let mut ledger = OutputLedger::default();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refusal = super::standard_unit_factor(&profile, &unit, &mut budget, &mut ledger);
    let allocations = window.close();
    assert!(matches!(
        refusal,
        Err(purrdf_geo_kernel::GeoError::WorkExhausted { limit: 64 })
    ));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
    assert_eq!(budget.work_items(), 0);
    assert_eq!(ledger.calls, 0);

    let unit = TermValue::iri(ogc::uom::METRE);
    let mut budget = purrdf_geo_kernel::PreparationBudget::new(profile.policy());
    let mut ledger = OutputLedger {
        refuse_call: Some(1),
        ..OutputLedger::default()
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refusal = super::standard_unit_factor(&profile, &unit, &mut budget, &mut ledger);
    let allocations = window.close();
    assert_eq!(refusal, Err(purrdf_geo_kernel::GeoError::Cancelled));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
    assert_eq!(budget.work_items(), 0);
}
