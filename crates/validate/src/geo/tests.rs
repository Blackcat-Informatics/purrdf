// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_geo_kernel::{GeoError, LonLat, MetricContext, PreparedGeodesic, Rat};
use purrdf_lex::json::{self, Object, Value};

use super::{
    GeoCallError, GeoProfile, GeoRequest, GeoSession, profile_from_str, profile_to_string,
};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

const DISTANCE: &str = r#"{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"1","latitude":"0"}}"#;

fn response(session: &GeoSession, request: &str) -> Value {
    json::read(&session.call_string(request)).expect("canonical JSON response")
}

fn similarity_profile() -> GeoProfile {
    let source = "00".repeat(32);
    let target = "01".repeat(32);
    let text = format!(
        r#"{{"version":1,"operations":[{{"name":"http://example.org/operation","source_crs":"http://example.org/source","target_crs":"http://example.org/target","chain":[{{"source":{{"realization":"{source}","unit":"metres","swapped_axes":false}},"target":{{"realization":"{target}","unit":"metres","swapped_axes":false}},"model":{{"law":"similarity2d-v1","translation_metres":["2","4"],"scale":"2","rotation_degrees":"0","convention":"position-vector","inverse":false}}}}]}}]}}"#
    );
    profile_from_str(&text).expect("explicit calibrated operation")
}

#[test]
fn typed_source_collection_capacity_refuses_before_any_call_allocation() {
    use purrdf_geo_kernel::{Crs, ExecutionLimits, TransformOutputGrid, cells::NativeGridProfile};
    use purrdf_sparql_eval::geo::functions::GeofFunction;

    let session = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_workspace_bytes: 2_048,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
    let requests = [
        GeoRequest::TransformBatch {
            name: Crs::new("http://example.org/operation").unwrap(),
            points: Vec::with_capacity(16_384),
            grid: TransformOutputGrid::DEFAULT,
        },
        GeoRequest::DistanceBatch {
            crs: crs.clone(),
            pairs: Vec::with_capacity(16_384),
        },
        GeoRequest::InverseBatch {
            crs: crs.clone(),
            pairs: Vec::with_capacity(16_384),
        },
        GeoRequest::DirectBatch {
            crs,
            inputs: Vec::with_capacity(16_384),
            grid: purrdf_geo_kernel::geodesic::DirectOutputGrid::DEGREE15,
        },
        GeoRequest::Geometry {
            function: GeofFunction::Dimension,
            arguments: Vec::with_capacity(16_384),
        },
        GeoRequest::CellBatch {
            grid: NativeGridProfile::Wgs84,
            points: Vec::with_capacity(16_384),
            level: 0,
        },
    ];
    for request in requests {
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = session.call(&request);
        let allocations = window.close();
        assert!(matches!(
            result,
            Err(GeoCallError::Engine(GeoError::MemoryExhausted {
                limit: 2_048
            }))
        ));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
    }
}

#[test]
fn typed_geometry_term_spare_capacity_refuses_before_carrier_allocation() {
    use purrdf_core::TermValue;
    use purrdf_geo_kernel::ExecutionLimits;
    use purrdf_sparql_eval::geo::functions::GeofFunction;

    let session = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_workspace_bytes: 8_192,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    let mut lexical_form = String::with_capacity(131_072);
    lexical_form.push_str("POINT(0 0)");
    let request = GeoRequest::Geometry {
        function: GeofFunction::Dimension,
        arguments: vec![TermValue::Literal {
            lexical_form,
            datatype: purrdf_iri::vocab::ogc::geo::WKT_LITERAL.to_owned(),
            language: None,
            direction: None,
        }],
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = session.call(&request);
    let allocations = window.close();
    assert!(
        matches!(
            result,
            Err(GeoCallError::Engine(GeoError::MemoryExhausted { limit })) if limit <= 8_192
        ),
        "original carrier phase must refuse its remaining storage: {result:?}"
    );
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn typed_dispatch_identifier_spare_capacity_is_retained_before_lookup() {
    use purrdf_geo_kernel::{Crs, ExecutionLimits};
    let session = GeoSession::new(
        similarity_profile()
            .with_limits(ExecutionLimits {
                max_workspace_bytes: 8_192,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    let mut name = String::with_capacity(131_072);
    name.push_str("http://example.org/operation");
    let request = GeoRequest::Operation {
        name: Crs::new(name).unwrap(),
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = session.call(&request);
    let allocations = window.close();
    assert!(matches!(
        result,
        Err(GeoCallError::Engine(GeoError::MemoryExhausted {
            limit: 8_192
        }))
    ));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn typed_invalid_datatype_text_is_admitted_before_error_encoding() {
    use purrdf_core::TermValue;
    use purrdf_geo_kernel::ExecutionLimits;
    use purrdf_sparql_eval::geo::functions::GeofFunction;

    let session = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_work_items: 512,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    let request = GeoRequest::Geometry {
        function: GeofFunction::Dimension,
        arguments: vec![TermValue::typed_literal("POINT(0 0)", "x".repeat(131_072))],
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = session.call(&request);
    let allocations = window.close();
    assert!(matches!(
        result,
        Err(GeoCallError::Engine(GeoError::WorkExhausted { limit })) if limit <= 512
    ));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn typed_unknown_identifiers_refuse_before_lookup_error_copies() {
    use super::{GeometryInput, GeometryParts, RegionInput};
    use purrdf_core::TermValue;
    use purrdf_geo_kernel::{Crs, ExecutionLimits, ellipsoidal::GeometryMetricLaw};

    let session = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_work_items: 512,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    let unknown = Crs::new(format!("http://example.org/{}", "x".repeat(131_072))).unwrap();
    let requests = [
        GeoRequest::Operation {
            name: unknown.clone(),
        },
        GeoRequest::GeometryMetric {
            metric: GeometryMetricLaw::Length,
            geometry: GeometryInput::Image {
                operation: unknown.clone(),
                geometry: TermValue::typed_literal(
                    "POINT(0 0)",
                    purrdf_iri::vocab::ogc::geo::WKT_LITERAL,
                ),
                epoch: None,
            },
        },
        GeoRequest::GeometryMetric {
            metric: GeometryMetricLaw::Length,
            geometry: GeometryInput::Prepared(Box::new(GeometryParts {
                crs: unknown,
                points: Vec::new(),
                curves: Vec::new(),
                region: RegionInput::Empty,
            })),
        },
    ];
    for request in requests {
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = session.call(&request);
        let allocations = window.close();
        assert!(matches!(
            result,
            Err(GeoCallError::Engine(GeoError::WorkExhausted { limit })) if limit <= 512
        ));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
    }
}

#[test]
fn transform_batch_admits_original_input_and_output_slots_before_allocating() {
    use purrdf_geo_kernel::{
        Crs, ExecutionLimits, OperationPoint, TransformOutputGrid, TransformResult,
    };

    let points = vec![
        OperationPoint {
            x: Rat::zero(),
            y: Rat::zero(),
            z: None,
            epoch: None,
        };
        512
    ];
    let source_bytes = points.capacity() * size_of::<OperationPoint>();
    let output_bytes = points.len() * size_of::<Option<TransformResult>>();
    assert!(output_bytes > 65_536);
    let request = GeoRequest::TransformBatch {
        name: Crs::new("http://example.org/operation").unwrap(),
        points,
        grid: TransformOutputGrid::DEFAULT,
    };
    for limits in [
        ExecutionLimits {
            max_workspace_bytes: source_bytes as u64 + 65_536,
            ..ExecutionLimits::GEOMETRY
        },
        ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        },
    ] {
        let session = GeoSession::new(similarity_profile().with_limits(limits).unwrap());
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = session.call(&request);
        let allocations = window.close();
        assert!(matches!(
            result,
            Err(GeoCallError::Engine(
                GeoError::MemoryExhausted { .. } | GeoError::WorkExhausted { .. }
            ))
        ));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
    }
}

#[test]
fn identity_record_members_are_all_in_the_response_admission_schema() {
    let identity = GeoSession::default().identity().unwrap();
    let record = super::encode::identity(&identity);
    let fields = record.as_object().unwrap();
    assert_eq!(fields.len() as u64, super::encode::IDENTITY_MEMBER_COUNT);
    for (name, value) in fields {
        assert!(name.len() <= 64);
        assert_eq!(value.as_str().unwrap().len(), 64);
    }
    // Profile repeats that full identity inside the admitted response envelope.
    // A one-byte-short allowance must refuse before either Object allocates.
    let required = super::output::envelope()
        .unwrap()
        .with_child(super::output::identity().unwrap())
        .unwrap()
        .workspace_bytes()
        .unwrap();
    let session = GeoSession::new(
        GeoProfile::standard()
            .with_limits(purrdf_geo_kernel::ExecutionLimits {
                max_workspace_bytes: required - 1,
                ..purrdf_geo_kernel::ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = session.call(&GeoRequest::Profile);
    let allocations = window.close();
    assert!(matches!(
        result,
        Err(GeoCallError::Engine(GeoError::MemoryExhausted { limit })) if limit == required - 1
    ));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn prepared_relation_records_preserve_physical_aliases_original_paths_and_native_law() {
    use purrdf_geo_kernel::{ExecutionLimits, atlas};

    let session = GeoSession::default();
    for (a, b, expected) in [
        (
            r#"{"kind":"prepared","points":[{"x":"180","y":"1"}]}"#,
            r#"{"kind":"literal","value":"POINT(-180 1)","datatype":"http://www.opengis.net/ont/geosparql#wktLiteral"}"#,
            "0FFFFFFF2",
        ),
        (
            r#"{"kind":"prepared","curves":[[{"law":"source-linear","start":{"x":"170","y":"0"},"end":{"x":"-170","y":"0"}}]]}"#,
            r#"{"kind":"literal","value":"POINT(0 0)","datatype":"http://www.opengis.net/ont/geosparql#wktLiteral"}"#,
            "0F1FF0FF2",
        ),
    ] {
        let text = format!(r#"{{"version":1,"operation":"geometry-relate","a":{a},"b":{b}}}"#);
        let answer = response(&session, &text);
        let GeoRequest::GeometryRelate { a, b } = GeoRequest::parse(&text).unwrap() else {
            panic!("the strict relation record must retain both original sources");
        };
        let a = a.prepare(session.profile()).unwrap();
        let b = b.prepare(session.profile()).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let native = atlas::relate(&a, &b, &mut context).unwrap();
        assert_eq!(answer["result"]["matrix"], expected);
        assert_eq!(answer["result"]["matrix"], native.to_string());
        assert_eq!(
            answer["result"]["law"],
            atlas::topology_law_id().digest().to_string()
        );
        assert_eq!(
            answer["result"]["binding"],
            a.reference().id().digest().to_string()
        );
        assert_eq!(answer["result"]["source_a"], a.id().to_string());
        assert_eq!(answer["result"]["source_b"], b.id().to_string());
        assert!(
            GeoRequest::parse(&text.replacen("\"version\":1", "\"version\":1,\"unknown\":true", 1))
                .is_err()
        );
        for limits in [
            ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            },
            ExecutionLimits {
                max_workspace_bytes: 1,
                ..ExecutionLimits::GEOMETRY
            },
        ] {
            let limited = GeoSession::new(GeoProfile::standard().with_limits(limits).unwrap());
            let refused = response(&limited, &text);
            assert!(refused.get("result").is_none());
            assert!(matches!(
                refused["error"]["code"].as_str(),
                Some("work-exhausted" | "memory-exhausted")
            ));
        }
    }
}

#[test]
fn native_polygon_records_keep_selected_curved_sides_and_complete_union_complement() {
    use purrdf_geo_kernel::{
        ExecutionLimits, ExecutionPolicy, PreparedEdge, PreparedRegion, Set, atlas,
    };

    let ring = r#"[[{"law":"shortest-geodesic","start":{"x":"0","y":"0"},"end":{"x":"1","y":"0"}},{"law":"shortest-geodesic","start":{"x":"1","y":"0"},"end":{"x":"0","y":"1"}},{"law":"shortest-geodesic","start":{"x":"0","y":"1"},"end":{"x":"0","y":"0"}}]]"#;
    // Native winding and Jordan construction use the caller's explicit admission.
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 2_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let profile = GeoProfile::standard()
        .with_limits(*policy.limits())
        .unwrap();
    let point = LonLat::new(
        Rat::parse_decimal("0.1").unwrap(),
        Rat::parse_decimal("0.1").unwrap(),
    )
    .unwrap();
    for (side, interior, complement, expected) in [
        ("left", "written", false, Set::Interior),
        ("right", "written", false, Set::Exterior),
        ("left", "complement", false, Set::Exterior),
        ("left", "written", true, Set::Exterior),
    ] {
        let text = format!(
            r#"{{"kind":"prepared","region":{{"kind":"native","polygons":[{{"side":"{side}","interior":"{interior}","rings":{ring}}}],"complement":{complement}}}}}"#
        );
        let input = super::GeometryInput::from_value(&json::read(&text).unwrap()).unwrap();
        let prepared = input.prepare(&profile).unwrap();
        let polygons = match prepared.region() {
            PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
                polygons
            }
            other => panic!("native rings must retain their region: {other:?}"),
        };
        assert!(polygons[0].chart().is_none());
        assert!(
            polygons[0].rings()[0]
                .edges()
                .iter()
                .all(|edge| matches!(edge, PreparedEdge::ShortestGeodesic(_)))
        );
        let mut context = MetricContext::new(prepared.reference().clone(), policy).unwrap();
        assert_eq!(
            atlas::locate(&point, prepared.region(), &mut context).unwrap(),
            expected
        );
        assert!(
            super::GeometryInput::from_value(
                &json::read(&text.replace("\"side\":", "\"unknown\":true,\"side\":")).unwrap()
            )
            .is_err()
        );
    }
}

#[test]
fn explicit_prepared_empty_container_capacity_refuses_before_output_allocation() {
    use super::{GeometryInput, GeometryParts, RegionInput};
    use purrdf_geo_kernel::{Crs, ExecutionLimits};

    let parts = GeometryParts {
        crs: Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap(),
        points: Vec::new(),
        curves: Vec::with_capacity(65_536),
        region: RegionInput::Empty,
    };
    let input = GeometryInput::Prepared(Box::new(parts));
    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_workspace_bytes: 1_048_576,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = input.prepare(&profile);
    let allocations = window.close();
    assert!(matches!(
        result,
        Err(GeoCallError::Engine(GeoError::MemoryExhausted {
            limit: 1_048_576
        }))
    ));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn scalar_distance_carries_the_same_native_completed_certificate() {
    let session = GeoSession::default();
    let record = response(&session, DISTANCE);
    assert_eq!(record, response(&session, DISTANCE));
    let start = LonLat::new(Rat::zero(), Rat::zero()).expect("start");
    let end = LonLat::new(Rat::one(), Rat::zero()).expect("end");
    let mut context = MetricContext::wgs84().expect("worker environment");
    let native = PreparedGeodesic::new(purrdf_geo_kernel::GeographicReference::wgs84())
        .distance(&start, &end, &mut context)
        .expect("certified native metric");
    assert_eq!(record["result"]["metres"].as_str(), Some("111319.490793"));
    assert_eq!(
        record["result"]["certificate"].as_str(),
        Some(purrdf_hash::hex::encode(&native.certificate_bytes()).as_str())
    );
    assert_eq!(
        record["identity"]["distance_law"].as_str(),
        Some(native.law_id().digest().to_string().as_str())
    );
    assert!(
        record
            .as_object()
            .expect("response record")
            .get("error")
            .is_none()
    );
}

#[test]
fn direct_declared_grids_preserve_native_certificates_and_complete_batches() {
    use purrdf_geo_kernel::{Metres, geodesic::DirectOutputGrid};
    let session = GeoSession::default();
    let input = r#""start":{"longitude":"0.1234567891234567","latitude":"0"},"azimuth_degrees":"90","length_metres":"1000""#;
    let default = response(
        &session,
        &format!(r#"{{"version":1,"operation":"direct",{input}}}"#),
    );
    let start = LonLat::new(
        Rat::parse_decimal("0.1234567891234567").unwrap(),
        Rat::zero(),
    )
    .unwrap();
    let prepared = PreparedGeodesic::new(purrdf_geo_kernel::GeographicReference::wgs84());
    for places in [12, 15, 18] {
        let scalar = response(
            &session,
            &format!(
                r#"{{"version":1,"operation":"direct",{input},"angular_decimal_places":{places}}}"#
            ),
        );
        let mut context = MetricContext::wgs84().unwrap();
        let native = prepared
            .direct_with_grid(
                &start,
                &Rat::from_i64(90),
                &Metres::new(Rat::from_i64(1000)),
                DirectOutputGrid::new(places),
                &mut context,
            )
            .unwrap();
        assert_eq!(scalar["result"], super::encode::direct(&native));
        assert_eq!(
            scalar["result"]["surface_error_bound_metres"].as_str(),
            Some("0.000001")
        );
        if places == 15 {
            assert_eq!(scalar["result"], default["result"]);
        } else {
            assert_ne!(scalar["result"]["law"], default["result"]["law"]);
        }
        let batch = response(
            &session,
            &format!(
                r#"{{"version":1,"operation":"direct-batch","angular_decimal_places":{places},"inputs":[{{{input}}},{{{input}}}]}}"#
            ),
        );
        assert_eq!(
            batch["result"],
            Value::Array(vec![scalar["result"].clone(); 2])
        );
    }
    for places in [0, u32::MAX] {
        let refusal = response(
            &session,
            &format!(
                r#"{{"version":1,"operation":"direct",{input},"angular_decimal_places":{places}}}"#
            ),
        );
        assert_eq!(
            refusal["error"]["code"].as_str(),
            Some("precision-exhausted")
        );
        assert!(refusal.get("result").is_none());
    }
}

#[test]
fn complete_carrier_metrics_include_polygon_boundaries_and_si_certificates() {
    let wkt = purrdf_iri::vocab::ogc::geo::WKT_LITERAL;
    let session = GeoSession::default();
    let request = |metric: &str, lexical: &str| {
        format!(
            r#"{{"version":1,"operation":"geometry-metric","metric":"{metric}","geometry":{{"kind":"literal","value":"{lexical}","datatype":"{wkt}"}}}}"#
        )
    };
    let line = response(&session, &request("length", "LINESTRING(0 0,1 0)"));
    assert_eq!(line["result"]["value"].as_str(), Some("111319.490793"));
    assert_eq!(line["result"]["unit"].as_str(), Some("metres"));
    assert!(line["result"]["certificate"].as_str().is_some());
    let polygon = "POLYGON((0 0,1 0,1 1,0 1,0 0))";
    let length = response(&session, &request("length", polygon));
    let perimeter = response(&session, &request("perimeter", polygon));
    assert_eq!(length["result"]["value"], perimeter["result"]["value"]);
    let value = Rat::parse_decimal(
        length["result"]["value"]
            .as_str()
            .expect("complete polygon length"),
    )
    .expect("exact metric");
    assert!(
        value > Rat::from_i64(400_000),
        "every areal boundary contributes"
    );
    assert_ne!(length["result"]["law"], perimeter["result"]["law"]);
    let empty = response(&session, &request("area", "POLYGON EMPTY"));
    assert_eq!(empty["result"]["value"].as_str(), Some("0"));
    assert_eq!(empty["result"]["unit"].as_str(), Some("square-metres"));
    assert_eq!(empty["result"]["error_bound"].as_str(), Some("0.1"));
}

#[test]
fn strict_unknown_duplicate_nested_and_version_refusals() {
    for request in [
        r#"{"version":1,"operation":"profile","extra":true}"#,
        r#"{"version":1,"version":1,"operation":"profile"}"#,
        r#"{"version":2,"operation":"profile"}"#,
        r#"{"version":1,"operation":"cell","grid":"wgs84","level":0,"point":{"longitude":"0","latitude":"0","latitude":"0"}}"#,
        r#"{"version":1,"operation":"cell","grid":"wgs84","level":0,"point":{"longitude":0,"latitude":"0"}}"#,
    ] {
        let record = response(&GeoSession::default(), request);
        assert_eq!(record["error"]["code"].as_str(), Some("invalid-record"));
        assert!(record.as_object().expect("record").get("result").is_none());
    }
    for profile in [
        r#"{"version":1,"unknown":0}"#,
        r#"{"version":1,"limits":{"max_work_items":1,"max_work_items":2}}"#,
    ] {
        assert!(profile_from_str(profile).is_err());
    }
}

#[test]
fn explicit_prepared_curve_laws_and_complete_region_covers_cross_the_boundary() {
    let session = GeoSession::default();
    let source = r#"{"version":1,"operation":"geometry-metric","metric":"length","geometry":{"kind":"prepared","curves":[[{"law":"source-linear","start":{"x":"0","y":"0","z":"7","m":"8"},"end":{"x":"1","y":"0","z":"9","m":"10"}}]]}}"#;
    let answer = response(&session, source);
    assert_eq!(answer["result"]["value"].as_str(), Some("111319.490793"));
    let arc = r#"{"version":1,"operation":"geometry-metric","metric":"length","geometry":{"kind":"prepared","curves":[[{"law":"azimuth-length","start":{"x":"0","y":"0"},"azimuth_degrees":"90","length_metres":"1000"}]]}}"#;
    assert_eq!(
        response(&session, arc)["result"]["value"].as_str(),
        Some("1000")
    );
    let integral = response(
        &session,
        &arc.replace("\"length\"", "\"geodesic-area-integral\""),
    );
    assert_eq!(integral["result"]["value"].as_str(), Some("0"));
    assert_eq!(integral["result"]["unit"].as_str(), Some("square-metres"));
    assert_eq!(
        integral["result"]["law"].as_str(),
        Some(
            purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::GeodesicAreaIntegral
                .id()
                .digest()
                .to_string()
                .as_str()
        )
    );
    assert!(
        GeoRequest::parse(&arc.replace(
            "\"length_metres\":\"1000\"",
            "\"length_metres\":\"1000\",\"end\":{\"x\":\"1\",\"y\":\"0\"}"
        ))
        .is_err()
    );
    let cover = r#"{"version":1,"operation":"cover-region","grid":"wgs84","region_kind":"whole","mode":"mixed","levels":{"min":0,"max":2},"limits":{"max_emitted_cells":6}}"#;
    let whole = response(&session, cover);
    assert_eq!(whole["result"]["cells"].as_array().unwrap().len(), 6);
    let limited = response(
        &session,
        &cover.replace("\"max_emitted_cells\":6", "\"max_emitted_cells\":5"),
    );
    assert!(limited.get("result").is_none());
    assert_eq!(
        limited["error"]["code"].as_str(),
        Some("cover-cells-exhausted")
    );
    let empty = response(&session, &cover.replace("\"whole\"", "\"empty\""));
    assert_eq!(
        empty["result"]["cells"].as_array().unwrap().as_slice(),
        [] as [Value; 0]
    );
}

#[test]
fn inverse_zero_retains_absent_azimuths_and_separate_completed_law() {
    let record = response(
        &GeoSession::default(),
        r#"{"version":1,"operation":"inverse","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"}}"#,
    );
    assert_eq!(record["result"]["distance"]["metres"].as_str(), Some("0"));
    assert_eq!(record["result"]["forward_azimuth_degrees"], Value::Null);
    assert_eq!(record["result"]["final_azimuth_degrees"], Value::Null);
    assert_eq!(record["result"]["scale12"].as_str(), Some("1"));
    assert_eq!(record["result"]["scale21"].as_str(), Some("1"));
    assert_eq!(record["result"]["law"], record["identity"]["inverse_law"]);
    assert_ne!(record["result"]["law"], record["identity"]["distance_law"]);
}

#[test]
fn exact_original_range_and_ieee_refusals_remain_typed() {
    let request = r#"{"version":1,"operation":"cell","grid":"wgs84","level":0,"point":{"longitude":"0","latitude":"90.0000000000000000000001"}}"#;
    assert!(matches!(
        GeoRequest::parse(request),
        Err(GeoCallError::Engine(GeoError::CoordinateOutOfRange {
            axis: "latitude",
            ..
        }))
    ));
    let ieee = r#"{"version":1,"operation":"cell","grid":"wgs84","level":0,"point":{"encoding":"ieee64","longitude":"7ff0000000000000","latitude":"0000000000000000"}}"#;
    assert_eq!(
        response(&GeoSession::default(), ieee)["error"]["code"].as_str(),
        Some("nonfinite-coordinate")
    );
}

#[test]
fn profile_roundtrip_preserves_reference_and_policy_ids() {
    let profile = profile_from_str(&format!(r#"{{"version":1,"references":[{{"crs":"{}","axes":"lat-lon","ellipsoid":"wgs84"}}],"limits":{{"max_precision_bits":256}}}}"#, purrdf_iri::vocab::ogc::EPSG4326)).expect("explicit reference");
    let encoded = profile_to_string(&profile).expect("exact profile");
    let replay = profile_from_str(&encoded).expect("canonical replay");
    assert_eq!(replay, profile);
    assert_eq!(replay.query_identity(), profile.query_identity());
    assert_ne!(
        profile.binding_id(),
        purrdf_geo_kernel::binding::STANDARD_PROFILE.binding_id()
    );
}

#[test]
fn exact_unit_registrations_are_normalized_protected_and_identity_bound() {
    let text = r#"{"version":1,"units":[{"iri":"http://example.org/kilometre","metres_per_unit":"1000.00"}]}"#;
    let profile = profile_from_str(text).expect("explicit linear unit");
    let unit = purrdf_geo_kernel::Crs::new("http://example.org/kilometre").expect("unit IRI");
    assert_eq!(
        profile.metres_per_unit(&unit).expect("factor"),
        Rat::from_i64(1000)
    );
    assert_ne!(
        profile.binding_id(),
        purrdf_geo_kernel::binding::STANDARD_PROFILE.binding_id()
    );
    let replay = profile_from_str(&profile_to_string(&profile).expect("canonical units"))
        .expect("strict replay");
    assert_eq!(profile, replay);
    assert_eq!(profile.binding_id(), replay.binding_id());
    assert_eq!(
        profile_from_str(&text.replace("1000.00", "1e3")).expect("equivalent factor"),
        profile
    );
    for factor in ["0", "-1"] {
        assert!(profile_from_str(&text.replace("1000.00", factor)).is_err());
    }
    let official = format!(
        r#"{{"version":1,"units":[{{"iri":"{}","metres_per_unit":"2"}}]}}"#,
        purrdf_iri::vocab::ogc::uom::METRE
    );
    assert!(profile_from_str(&official).is_err());
    let identical = profile_from_str(&official.replace("\"2\"", "\"1\""))
        .expect("identical immutable metre coalesces");
    assert_eq!(identical, purrdf_geo_kernel::binding::STANDARD_PROFILE);
    assert_eq!(
        identical.binding_id(),
        purrdf_geo_kernel::binding::STANDARD_PROFILE.binding_id()
    );
}

#[test]
fn both_threshold_laws_are_separate_and_negative_reported_is_false() {
    let reported = r#"{"version":1,"operation":"within","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"decimal","value":"-1"}}"#;
    assert_eq!(
        response(&GeoSession::default(), reported)["result"],
        Value::Bool(false)
    );
    let physical = r#"{"version":1,"operation":"within-physical","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"},"threshold_metres":"0"}"#;
    assert_eq!(
        response(&GeoSession::default(), physical)["result"],
        Value::Bool(true)
    );
    let nonfinite = reported.replace(
        r#""decimal","value":"-1""#,
        r#""double-bits","value":"7ff8000000000000""#,
    );
    assert!(GeoRequest::parse(&nonfinite).is_err());
}

#[test]
fn batch_refusal_publishes_no_partial_results() {
    let session =
        GeoSession::from_profile_str(r#"{"version":1,"limits":{"max_output_elements":1}}"#)
            .expect("one-element cap");
    let pair = json::read(DISTANCE).expect("request");
    let pairs = vec![
        Object::new()
            .with("a", pair["a"].clone())
            .with("b", pair["b"].clone()),
        Object::new()
            .with("a", pair["a"].clone())
            .with("b", pair["b"].clone()),
    ];
    let request = json::write_compact(
        &Object::new()
            .with("version", 1_u32)
            .with("operation", "distance-batch")
            .with(
                "pairs",
                Value::Array(pairs.into_iter().map(Value::from).collect()),
            )
            .into(),
    );
    let record = response(&session, &request);
    assert_eq!(record["error"]["code"].as_str(), Some("output-exhausted"));
    assert!(record.as_object().expect("record").get("result").is_none());
}

#[test]
fn integer_cell_encoding_and_physical_level_selection_cross_the_codec() {
    use purrdf_geo_kernel::{ExecutionLimits, cells::NativeGridProfile};

    let request = r#"{"version":1,"operation":"cell","grid":"wgs84","level":30,"point":{"longitude":"0","latitude":"90"}}"#;
    let cell = response(&GeoSession::default(), request);
    assert_eq!(
        cell["result"]["face"]
            .as_number()
            .and_then(json::Number::as_u64),
        Some(2)
    );
    assert_eq!(cell["result"]["key"], cell["result"]["big_endian"]);
    let scale = r#"{"version":1,"operation":"cell-scale","grid":"wgs84","level":16,"maximum_edge_metres":"150"}"#;
    let scale_response = response(&GeoSession::default(), scale);
    assert_eq!(
        scale_response["result"]["selected_level"]
            .as_number()
            .and_then(json::Number::as_u64),
        Some(17),
        "{}",
        json::write_compact(&scale_response)
    );
    let request = GeoRequest::CellScale {
        grid: NativeGridProfile::Wgs84,
        level: 16,
        maximum_edge: Some(purrdf_geo_kernel::Metres::new(Rat::from_i64(150))),
    };
    for (limits, work_failure) in [
        (
            ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            },
            true,
        ),
        (
            ExecutionLimits {
                max_workspace_bytes: 64,
                ..ExecutionLimits::GEOMETRY
            },
            false,
        ),
    ] {
        let limited = GeoSession::new(GeoProfile::standard().with_limits(limits).unwrap());
        let error = limited.call(&request).unwrap_err();
        if work_failure {
            assert!(matches!(
                error,
                GeoCallError::Engine(GeoError::WorkExhausted { .. })
            ));
        } else {
            assert!(matches!(
                error,
                GeoCallError::Engine(GeoError::MemoryExhausted { .. })
            ));
        }
    }
}

#[test]
fn explicit_similarity_chain_compiles_roundtrips_and_applies_once() {
    let profile = similarity_profile();
    assert_eq!(
        profile_from_str(&profile_to_string(&profile).expect("canonical profile")).expect("replay"),
        profile
    );
    let session = GeoSession::new(profile);
    let request = r#"{"version":1,"operation":"transform","name":"http://example.org/operation","point":{"x":"7","y":"-3"}}"#;
    let record = response(&session, request);
    assert_eq!(record["result"]["x"].as_str(), Some("16"));
    assert_eq!(record["result"]["y"].as_str(), Some("-2"));

    let point = purrdf_geo_kernel::OperationPoint {
        x: Rat::parse_decimal("0.6172839456172839455").unwrap(),
        y: Rat::zero(),
        z: None,
        epoch: None,
    };
    let name = purrdf_geo_kernel::Crs::new("http://example.org/operation").unwrap();
    let binding = session.profile().operation(&name).unwrap();
    let default_request = r#"{"version":1,"operation":"transform","name":"http://example.org/operation","point":{"x":"0.6172839456172839455","y":"0"}}"#;
    for places in [6, 9, 18] {
        let request = format!(
            r#"{{"version":1,"operation":"transform","name":"http://example.org/operation","point":{{"x":"0.6172839456172839455","y":"0"}},"metric_decimal_places":{places}}}"#
        );
        let record = response(&session, &request);
        let mut context = MetricContext::wgs84().unwrap();
        let native = binding
            .chain()
            .apply_with_grid(
                &point,
                purrdf_geo_kernel::TransformOutputGrid::new(15, places),
                &mut context,
            )
            .unwrap();
        assert_eq!(
            record["result"]["x"].as_str(),
            Some(
                super::profile::exact_decimal(&native.point().x)
                    .unwrap()
                    .as_str()
            )
        );
        assert_eq!(
            record["result"]["law"].as_str(),
            Some(native.law_id().digest().to_string().as_str())
        );
        assert_eq!(
            record["result"]["certificate"].as_str(),
            Some(purrdf_hash::hex::encode(&native.certificate_bytes()).as_str())
        );
        if places == 6 {
            assert_eq!(
                session.call_string(&request),
                session.call_string(default_request)
            );
        }
        let batch = format!(
            r#"{{"version":1,"operation":"transform-batch","name":"http://example.org/operation","points":[{{"x":"0.6172839456172839455","y":"0"}},{{"x":"0.6172839456172839455","y":"0"}}],"metric_decimal_places":{places}}}"#
        );
        let batch = response(&session, &batch);
        assert_eq!(
            batch["result"],
            Value::Array(vec![record["result"].clone(), record["result"].clone()])
        );
    }
    for places in [5, u32::MAX] {
        let request =
            default_request.replacen('}', &format!("}},\"metric_decimal_places\":{places}"), 1);
        let record = response(&session, &request);
        assert_eq!(record["error"]["code"], "precision-exhausted");
        assert!(record.get("result").is_none());
    }
    let geometry = format!(
        r#"{{"version":1,"operation":"transform-geometry","name":"http://example.org/operation","geometry":{{"kind":"literal","value":"<http://example.org/source> LINESTRING(0 0,1 0)","datatype":"{}"}}}}"#,
        purrdf_iri::vocab::ogc::geo::WKT_LITERAL
    );
    let answer = response(&session, &geometry);
    let term = super::request::term(&answer["result"]["geometry"]).expect("typed output");
    let literal =
        purrdf_geo_kernel::carrier::geometry_arg(purrdf_geo_kernel::standard_vocabulary(), &term)
            .expect("actual target carrier");
    let expected = purrdf_geo_kernel::wkt::parse(
        "<http://example.org/target> LINESTRING(2 4,4 4)",
        purrdf_geo_kernel::standard_vocabulary().default_wkt_crs(),
    )
    .expect("expected image");
    assert_eq!(literal, expected);
    assert_eq!(answer["result"]["error_bound_metres"].as_str(), Some("0.1"));
    assert_eq!(
        answer["result"]["source_vertices"]
            .as_number()
            .and_then(json::Number::as_u64),
        Some(2)
    );
    let source_literal = purrdf_geo_kernel::wkt::parse(
        "<http://example.org/source> LINESTRING(0 0,1 0)",
        purrdf_geo_kernel::standard_vocabulary().default_wkt_crs(),
    )
    .expect("original native source");
    let mut context = MetricContext::new(
        purrdf_geo_kernel::GeographicReference::wgs84(),
        session.profile().policy(),
    )
    .expect("native image context");
    let native = session
        .profile()
        .transform_literal_named(
            &purrdf_geo_kernel::Crs::new("http://example.org/operation").expect("operation IRI"),
            &source_literal,
            None,
            &mut context,
        )
        .expect("native complete image");
    assert_eq!(
        answer["result"]["source"].as_str(),
        Some(native.source_id().to_string().as_str())
    );
    assert_eq!(
        answer["result"]["law"].as_str(),
        Some(native.law_id().digest().to_string().as_str())
    );
    assert_eq!(
        answer["result"]["certificate"].as_str(),
        Some(purrdf_hash::hex::encode(&native.certificate_bytes()).as_str())
    );
    let dispatch = format!(
        r#"{{"version":1,"operation":"geometry","function":"transform","arguments":[{{"kind":"literal","value":"<http://example.org/source> LINESTRING(0 0,1 0)","datatype":"{}"}},{{"kind":"iri","value":"http://example.org/target"}}]}}"#,
        purrdf_iri::vocab::ogc::geo::WKT_LITERAL
    );
    assert_eq!(
        response(&session, &dispatch)["result"],
        answer["result"]["geometry"]
    );
}

#[test]
fn inverse_models_roundtrip_strict_profiles_without_reminting_bindings() {
    let source = "00".repeat(32);
    let target = "01".repeat(32);
    let models = [
        (
            "degrees",
            "degrees",
            r#"{"law":"gcj-rational-harmonic-inverse-v1","applicability":{"west":"72.004","east":"137.8347","south":"0.8293","north":"55.8271"}}"#,
        ),
        ("degrees", "degrees", r#"{"law":"bd09ll-inverse-v1"}"#),
        (
            "metres",
            "metres",
            r#"{"law":"polynomial2d-inverse","basis":"monomial","order":"x-power-y-power","degree":1,"origin_metres":["0","0"],"normalization_metres":["1","1"],"terms":[{"x_power":1,"y_power":0,"x_coefficient_metres":"1","y_coefficient_metres":"0"},{"x_power":0,"y_power":1,"x_coefficient_metres":"0","y_coefficient_metres":"1"}],"source_domain":{"min_x_metres":"0.1","max_x_metres":"3","min_y_metres":"0.2","max_y_metres":"4"}}"#,
        ),
        (
            "metres",
            "metres",
            r#"{"law":"bilinear-grid-inverse","origin_metres":["0.1","0.2"],"spacing_metres":["1","1"],"columns":2,"rows":2,"nodes":[{"x_metres":"0","y_metres":"0"},{"x_metres":"0","y_metres":"0"},{"x_metres":"0","y_metres":"0"},{"x_metres":"0","y_metres":"0"}],"source_domain":{"min_x_metres":"0.1","max_x_metres":"1.1","min_y_metres":"0.2","max_y_metres":"1.2"}}"#,
        ),
        (
            "degrees",
            "metres",
            r#"{"law":"ellipsoidal-mercator-v1","semimajor_metres":"6378137","eccentricity_squared":"0"}"#,
        ),
        (
            "metres",
            "degrees",
            r#"{"law":"transverse-mercator-inverse","ellipsoid":{"profile":"cgcs2000"},"family":"utm","zone":50,"central_meridian_degrees":"117","scale":"0.9996","false_easting_metres":"500000","false_northing_metres":"0","hemisphere":"north","zone_prefix":false}"#,
        ),
    ];
    for (input_unit, output_unit, model) in models {
        let text = format!(
            r#"{{"version":1,"operations":[{{"name":"http://example.org/operation","source_crs":"http://example.org/source","target_crs":"http://example.org/target","chain":[{{"source":{{"realization":"{source}","unit":"{input_unit}","swapped_axes":false}},"target":{{"realization":"{target}","unit":"{output_unit}","swapped_axes":false}},"model":{model}}}]}}]}}"#
        );
        let profile = profile_from_str(&text).expect("explicit model");
        let canonical = profile_to_string(&profile).expect("canonical model");
        let replay = profile_from_str(&canonical).expect("strict replay");
        assert_eq!(profile.binding_id(), replay.binding_id());
        assert_eq!(profile.query_identity(), replay.query_identity());
        let unknown = text.replace(r#""model":{"law""#, r#""model":{"unrecognized":true,"law""#);
        assert!(profile_from_str(&unknown).is_err());
    }
}

#[test]
fn point_buffer_carrier_preserves_closure_grid_and_certificate() {
    let session = GeoSession::default();
    let request = |radius: &str| {
        format!(
            r#"{{"version":1,"operation":"buffer-points","geometry":{{"kind":"prepared","points":[{{"x":"0.1234567890123456789","y":"0"}}]}},"radius_metres":"{radius}"}}"#,
        )
    };
    let zero = response(&session, &request("0"));
    assert_eq!(
        zero,
        response(&session, &request("0").replace("buffer-points", "buffer"))
    );
    let term = super::request::term(&zero["result"]["geometry"]).expect("complete buffer carrier");
    let literal =
        purrdf_geo_kernel::carrier::geometry_arg(purrdf_geo_kernel::standard_vocabulary(), &term)
            .unwrap();
    let expected = purrdf_geo_kernel::wkt::parse(
        "MULTIPOINT(0.1234567890123456789 0)",
        purrdf_geo_kernel::standard_vocabulary().default_wkt_crs(),
    )
    .unwrap();
    assert_eq!(literal, expected);
    assert_eq!(zero["result"]["outward_error_metres"].as_str(), Some("0.1"));
    assert!(zero["result"]["certificate"].as_str().is_some());
    let positive = response(&session, &request("0.001"));
    let term = super::request::term(&positive["result"]["geometry"]).unwrap();
    let circle =
        purrdf_geo_kernel::carrier::geometry_arg(purrdf_geo_kernel::standard_vocabulary(), &term)
            .unwrap();
    assert_eq!(
        purrdf_geo_kernel::wkt::write_exact(&circle).unwrap(),
        purrdf_geo_kernel::wkt::write(&circle, 15)
    );
    assert_eq!(positive["result"]["law"], zero["result"]["law"]);
    let negative = response(&session, &request("-1"));
    let term = super::request::term(&negative["result"]["geometry"]).unwrap();
    let empty =
        purrdf_geo_kernel::carrier::geometry_arg(purrdf_geo_kernel::standard_vocabulary(), &term)
            .unwrap();
    assert!(
        matches!(empty.geometry().body(), purrdf_geo_kernel::GeometryBody::GeometryCollection(parts) if parts.is_empty())
    );
    let unknown = request("0").replace(
        r#""operation":"buffer-points""#,
        r#""operation":"buffer-points","unexpected":true"#,
    );
    assert_eq!(
        response(&session, &unknown)["error"]["code"].as_str(),
        Some("invalid-record")
    );
}

#[test]
fn physical_offset_membership_keeps_negative_zero_and_reference_laws() {
    let session = GeoSession::default();
    let source = r#"{"kind":"prepared","points":[{"x":"0","y":"0","z":"7","m":"9"}]}"#;
    let request = |radius: &str, longitude: &str| {
        format!(
            r#"{{"version":1,"operation":"offset-contains","geometry":{source},"radius_metres":"{radius}","point":{{"longitude":"{longitude}","latitude":"0"}}}}"#
        )
    };
    assert_eq!(
        response(&session, &request("-1", "0"))["result"]["inside"],
        Value::Bool(false)
    );
    assert_eq!(
        response(&session, &request("0", "0"))["result"]["inside"],
        Value::Bool(true)
    );
    assert_eq!(
        response(&session, &request("0", "1"))["result"]["inside"],
        Value::Bool(false)
    );
    assert_eq!(
        response(&session, &request("200000", "1"))["result"]["inside"],
        Value::Bool(true)
    );
    let altered =
        GeoSession::from_profile_str(r#"{"version":1,"limits":{"max_work_items":1000000}}"#)
            .expect("adequate different policy");
    assert_eq!(
        response(&session, &request("0", "0"))["result"]["offset"],
        response(&altered, &request("0", "0"))["result"]["offset"]
    );
    let mixed = request("0", "0").replace(
        r#""geometry":{"kind":"prepared""#,
        r#""geometry":{"kind":"prepared","crs":"http://example.org/cgcs""#,
    );
    let cgcs = GeoSession::from_profile_str(r#"{"version":1,"references":[{"crs":"http://example.org/cgcs","ellipsoid":"cgcs2000","axes":"lon-lat"}]}"#).expect("explicit CGCS");
    assert_eq!(
        response(&cgcs, &mixed)["error"]["code"].as_str(),
        Some("missing-operation")
    );
}

#[test]
fn closed_whole_surface_fixed_and_mixed_covers_share_canonical_ranges() {
    let session = GeoSession::default();
    let fixed = r#"{"version":1,"operation":"cover-box","grid":"wgs84","mode":"fixed","level":0,"box":{"west":"-180","south":"-90","east":"180","north":"90"}}"#;
    let mixed = r#"{"version":1,"operation":"cover-box","grid":"wgs84","mode":"mixed","levels":{"min":0,"max":0},"box":{"west":"-180","south":"-90","east":"180","north":"90"}}"#;
    let fixed = response(&session, fixed);
    let mixed = response(&session, mixed);
    assert!(fixed.get("error").is_none(), "whole fixed cover: {fixed:?}");
    assert!(mixed.get("error").is_none(), "whole mixed cover: {mixed:?}");
    assert_eq!(fixed["result"]["cells"], mixed["result"]["cells"]);
    assert_eq!(fixed["result"]["ranges"], mixed["result"]["ranges"]);
    assert_eq!(
        fixed["result"]["cells"]
            .as_array()
            .expect("whole surface cells")
            .len(),
        6
    );
    let refusal = r#"{"version":1,"operation":"cover-box","grid":"wgs84","mode":"mixed","levels":{"min":0,"max":0},"limits":{"max_emitted_cells":5},"box":{"west":"-180","south":"-90","east":"180","north":"90"}}"#;
    let refusal = response(&session, refusal);
    assert_eq!(refusal["error"]["code"], "cover-cells-exhausted");
    assert!(refusal.get("result").is_none());
}

#[test]
fn completed_cell_record_replays_and_inconsistent_metadata_refuses() {
    let session = GeoSession::default();
    let assigned = response(
        &session,
        r#"{"version":1,"operation":"cell","grid":"wgs84","level":4,"point":{"longitude":"0","latitude":"0"}}"#,
    );
    let mut request = Object::new()
        .with("version", 1_u32)
        .with("operation", "cell-hierarchy")
        .with("cell", assigned["result"].clone());
    let replay = response(&session, &json::write_compact(&request.clone().into()));
    assert_eq!(replay["result"]["cell"], assigned["result"]);
    let mut invalid = assigned["result"].clone();
    invalid["level"] = Value::from(5_u8);
    request.insert("cell", invalid);
    assert_eq!(
        response(&session, &json::write_compact(&request.into()))["error"]["code"],
        "invalid-record"
    );
}

#[test]
fn reusable_index_content_identity_and_both_comparison_laws_survive_codec() {
    let session = GeoSession::default();
    let source = r#"{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}},{"key":"11","point":{"longitude":"1","latitude":"0"}}]}"#;
    let index = session
        .point_index(source)
        .expect("complete reusable index");
    let mut reordered = json::read(source).expect("source record");
    reordered["points"]
        .as_array_mut()
        .expect("points")
        .reverse();
    let alternate = session
        .point_index(&json::write_compact(&reordered))
        .expect("reordered source");
    assert_eq!(index.native().id(), alternate.native().id());
    let physical = r#"{"version":1,"operation":"search-physical","center":{"longitude":"0","latitude":"0"},"radius_metres":"0"}"#;
    let result = json::read(&index.call_string(physical)).expect("response");
    assert_eq!(
        result["result"]["keys"],
        Value::Array(vec![Value::from("7")]),
        "{result:?}"
    );
    let reported = r#"{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"integer","value":"-1"}}"#;
    let negative = json::read(&index.call_string(reported)).expect("response");
    assert_eq!(negative["result"]["keys"], Value::Array(Vec::new()));
    assert_eq!(index.native().len(), 2);
    let duplicate = source.replace("\"11\"", "\"7\"");
    assert!(matches!(
        session.point_index(&duplicate),
        Err(GeoCallError::Engine(GeoError::DuplicatePointKey(7)))
    ));
    let query = Object::new()
        .with("version", 1_u32)
        .with("operation", "point-index-query")
        .with("index", json::read(source).expect("source"))
        .with("search", json::read(reported).expect("search"));
    assert_eq!(
        response(&session, &json::write_compact(&query.into()))["result"]["keys"],
        negative["result"]["keys"]
    );
}

#[test]
fn original_scientific_strings_are_admitted_before_integer_construction() {
    for text in [
        r#"{"version":1,"operation":"distance","a":{"longitude":"1e100000","latitude":"0"},"b":{"longitude":"0","latitude":"0"}}"#,
        r#"{"version":1,"operation":"cell","grid":"wgs84","level":0,"point":{"longitude":"1e100000","latitude":"0"}}"#,
    ] {
        let (bytes, complete) = GeoSession::default().response(text);
        assert!(!complete);
        let value = json::read(&bytes).expect("canonical refusal");
        assert_eq!(value["error"]["code"].as_str(), Some("work-exhausted"));
        assert!(value.get("result").is_none());
    }
    let error = profile_from_str(
        r#"{"version":1,"units":[{"iri":"http://example.org/unit","metres_per_unit":"1e100000"}]}"#,
    )
    .expect_err("profile decimal construction is admitted too");
    assert!(matches!(
        error,
        GeoCallError::Engine(GeoError::WorkExhausted { .. })
    ));
}

#[test]
fn decimal_admission_respects_payload_roles_before_strict_decoding() {
    for text in [
        r#"{"iri":"1e100000"}"#,
        r#"{"geometry":{"kind":"literal","value":"1e100000"}}"#,
        r#"{"encoding":"ieee64","longitude":"1000000000000000","latitude":"0000000000000000"}"#,
    ] {
        let value = super::read_record(text).expect("nondecimal payload is not constructed as Rat");
        assert_eq!(value, json::read(text).expect("the same lexical record"));
    }
    let unknown = GeoRequest::parse(r#"{"version":1,"operation":"profile","iri":"1e100000"}"#)
        .expect_err("payload role handling still preserves strict unknown-field refusal");
    assert!(matches!(unknown, GeoCallError::Decode(_)));
}

#[test]
fn explicit_continuous_image_records_use_original_projected_metric_sources() {
    use purrdf_geo_kernel::operation::{CoordinateUnit, OperationModel, OperationReference};
    use purrdf_geo_kernel::{CoordinateOperation, Crs, OperationChain};
    use purrdf_hash::hex::Digest32;
    use purrdf_iri::vocab::ogc;

    let mut profile = GeoProfile::standard();
    let source = Crs::new("http://example.org/projected").unwrap();
    let target = Crs::new(ogc::CRS84).unwrap();
    let operation = CoordinateOperation::compile(
        OperationReference {
            realization: Digest32::new([7; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        },
        profile.operation_reference(&target).unwrap(),
        OperationModel::MercatorToGeographic {
            radius: Rat::from_i64(6_378_137),
            eccentricity_squared: Rat::zero(),
            square_domain: true,
        },
    )
    .unwrap();
    profile
        .register_operation(
            Crs::new("http://example.org/inverse-projection").unwrap(),
            source,
            target,
            OperationChain::compile(vec![operation]).unwrap(),
        )
        .unwrap();
    let session = GeoSession::new(profile);
    let request = format!(
        r#"{{"version":1,"operation":"geometry-metric","metric":"length","geometry":{{"kind":"image","operation":"http://example.org/inverse-projection","geometry":{{"kind":"literal","value":"<http://example.org/projected> LINESTRING ZM(0 0 7 9,1000 0 8 10)","datatype":"{}"}}}}}}"#,
        ogc::geo::WKT_LITERAL
    );
    let answer = response(&session, &request);
    assert_eq!(answer["result"]["value"].as_str(), Some("1000"));
    assert_eq!(answer["result"]["unit"].as_str(), Some("metres"));
    assert!(answer["result"]["certificate"].as_str().is_some());
    let edge_record = r#"{"law":"operation-image","operation":"http://example.org/inverse-projection","start":{"x":"0","y":"0","z":"7","m":"9"},"end":{"x":"1000","y":"0","z":"8","m":"10"},"epoch_decimal_year":"2026.5"}"#;
    let original = format!(r#"{{"kind":"prepared","curves":[[{edge_record}]]}}"#);
    let input = super::GeometryInput::from_value(&json::read(&original).unwrap()).unwrap();
    let geometry = input.prepare(session.profile()).unwrap();
    let purrdf_geo_kernel::PreparedEdge::Transformed(image) = &geometry.curves()[0].edges()[0]
    else {
        panic!("explicit transformed records must retain the complete original curve");
    };
    assert_eq!(image.source_endpoints().1.x(), &Rat::from_i64(1000));
    assert_eq!(image.source_endpoints().1.m(), Some(&Rat::from_i64(10)));
    let metric_record = format!(
        r#"{{"version":1,"operation":"geometry-metric","metric":"length","geometry":{original}}}"#
    );
    let metric = response(&session, &metric_record);
    assert_eq!(metric["result"]["value"].as_str(), Some("1000"));
    assert!(
        super::GeometryInput::from_value(
            &json::read(&original.replace("\"law\":", "\"extra\":true,\"law\":")).unwrap()
        )
        .is_err()
    );
    let export = format!(
        r#"{{"version":1,"operation":"transform-geometry","name":"http://example.org/inverse-projection","geometry":{{"kind":"literal","value":"<http://example.org/projected> POINT(1000 0)","datatype":"{}"}}}}"#,
        ogc::geo::WKT_LITERAL,
    );
    let exported = response(&session, &export);
    let term = super::request::term(&exported["result"]["geometry"]).unwrap();
    let literal =
        purrdf_geo_kernel::carrier::geometry_arg(purrdf_geo_kernel::standard_vocabulary(), &term)
            .unwrap();
    let purrdf_geo_kernel::GeometryBody::Point(Some(coordinate)) = literal.geometry().body() else {
        panic!("one complete point image");
    };
    let mut context = MetricContext::wgs84().unwrap();
    let native = session
        .profile()
        .operation(&Crs::new("http://example.org/inverse-projection").unwrap())
        .unwrap()
        .chain()
        .apply(
            &purrdf_geo_kernel::OperationPoint {
                x: Rat::from_i64(1000),
                y: Rat::zero(),
                z: None,
                epoch: None,
            },
            &mut context,
        )
        .unwrap();
    assert_eq!(coordinate.x(), &native.point().x);
    assert_ne!(
        coordinate.x(),
        &Rat::from_decimal(coordinate.x().round_to_scale(6), 6)
    );
    let unknown = request.replace(
        r#""kind":"image","operation""#,
        r#""kind":"image","unexpected":true,"operation""#,
    );
    assert_eq!(
        response(&session, &unknown)["error"]["code"].as_str(),
        Some("invalid-record")
    );
}

#[test]
fn explicit_prepared_sources_admit_exact_copies_before_large_range_arithmetic() {
    use super::geometry::{GeometryInput, GeometryParts, RegionInput};
    use purrdf_geo_kernel::{Coord, Crs, ExecutionLimits, Int};

    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: 1_024,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let tiny = Rat::new(Int::one(), Int::one().shl(32_768)).unwrap();
    let input = GeometryInput::Prepared(Box::new(GeometryParts {
        crs: Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap(),
        points: vec![Coord::xy(tiny, Rat::zero())],
        curves: Vec::new(),
        region: RegionInput::Empty,
    }));
    assert!(matches!(
        input.prepare(&profile),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { .. }))
    ));
}

#[test]
fn cell_batches_borrow_exact_sources_and_refuse_complete_invocation_limits() {
    use purrdf_geo_kernel::{
        ExecutionLimits,
        cells::{CubeHilbertQ62V1, NativeGridProfile},
    };

    let point = super::PointInput(LonLat::new(Rat::from_i64(12), Rat::from_i64(34)).unwrap());
    let grid = NativeGridProfile::Wgs84;
    let scalar = GeoRequest::Cell {
        grid,
        point: point.clone(),
        level: 17,
    };
    let batch = GeoRequest::CellBatch {
        grid,
        points: vec![point.clone(), point.clone()],
        level: 17,
    };
    let native = super::encode::cell(CubeHilbertQ62V1::new(grid).assign(&point.0, 17).unwrap());
    let session = GeoSession::default();
    assert_eq!(session.call(&scalar).unwrap(), native);
    assert_eq!(session.call(&batch).unwrap(), Value::Array(vec![native; 2]));

    let work = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    assert!(matches!(
        work.call(&scalar),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { .. }))
    ));
    assert!(matches!(
        work.call(&batch),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { .. }))
    ));
    let memory = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_workspace_bytes: 1,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    assert!(matches!(
        memory.call(&batch),
        Err(GeoCallError::Engine(GeoError::MemoryExhausted { .. }))
    ));
    let output = GeoSession::new(
        GeoProfile::standard()
            .with_limits(ExecutionLimits {
                max_output_elements: 1,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
    );
    assert!(matches!(
        output.call(&batch),
        Err(GeoCallError::Engine(GeoError::OutputExhausted { .. }))
    ));
}

#[test]
fn scratch_policy_and_refusals_preserve_explicit_host_fields() {
    use purrdf_xsd::integer::LimbScratchError;
    let default = GeoProfile::standard();
    let changed =
        profile_from_str(r#"{"version":1,"limits":{"max_scratch_destinations":1024}}"#).unwrap();
    assert_eq!(changed.policy().limits().max_scratch_destinations, 1024);
    assert_ne!(default.policy().id(), changed.policy().id());
    assert_eq!(default.binding_id(), changed.binding_id());
    assert_eq!(
        profile_from_str(&profile_to_string(&changed).unwrap()).unwrap(),
        changed
    );
    assert!(profile_from_str(r#"{"version":1,"limits":{"max_scratch_destinations":0}}"#).is_err());
    for (error, reason) in [
        (
            LimbScratchError::Capacity {
                required_limbs: 40,
                admitted_limbs: 32,
            },
            "capacity",
        ),
        (
            LimbScratchError::Exhausted { destinations: 768 },
            "destinations",
        ),
        (
            LimbScratchError::Retained {
                held_destinations: 1,
                destinations: 768,
            },
            "retained-destinations",
        ),
        (LimbScratchError::SizeOverflow, "size-overflow"),
    ] {
        let error = GeoError::NumericalScratch(error);
        assert!(!error.is_expression_error());
        let record = super::encode::refusal(&GeoCallError::Engine(error));
        assert_eq!(record["code"].as_str(), Some("numerical-scratch"));
        assert_eq!(record["reason"].as_str(), Some(reason));
    }
}

#[test]
fn taylor_scratch_refusal_is_operational_and_preserves_its_cause() {
    let error = GeoError::NumericalTaylorScratch(purrdf_xsd::math::TaylorScratchError::InUse);
    assert!(!error.is_expression_error());
    let record = super::encode::refusal(&GeoCallError::Engine(error));
    assert_eq!(record["code"].as_str(), Some("numerical-taylor-scratch"));
    assert_eq!(record["reason"].as_str(), Some("in-use"));
}

#[test]
fn floating_environment_refusals_preserve_the_exact_register_or_probe_evidence() {
    use purrdf_xsd::ieee::environment::{FloatEnvironmentError, FloatEnvironmentEvidence};
    let register = FloatEnvironmentEvidence::Register {
        name: "MXCSR",
        bits: 0x9fc0,
    };
    let probe = FloatEnvironmentEvidence::Probe {
        operation: "original binary64 half-ulp addition",
        expected: 0x3ff0_0000_0000_0000,
        observed: 0x3ff0_0000_0000_0001,
    };
    for (error, reason, expected) in [
        (
            FloatEnvironmentError::TrapsEnabled { evidence: register },
            "traps-enabled",
            register,
        ),
        (
            FloatEnvironmentError::FlushToZero { evidence: register },
            "flush-to-zero",
            register,
        ),
        (
            FloatEnvironmentError::RoundingMode { evidence: register },
            "rounding-mode",
            register,
        ),
        (
            FloatEnvironmentError::DoubleRounding { evidence: probe },
            "double-rounding",
            probe,
        ),
    ] {
        let record =
            super::encode::refusal(&GeoCallError::Engine(GeoError::FloatEnvironment(error)));
        assert_eq!(record["code"], "floating-environment");
        assert_eq!(record["reason"], reason);
        assert_eq!(record["expression_error"], false);
        match expected {
            FloatEnvironmentEvidence::Register { name, bits } => {
                assert_eq!(record["evidence"]["kind"], "register");
                assert_eq!(record["evidence"]["name"], name);
                assert_eq!(record["evidence"]["bits"], super::encode::key(bits));
            }
            FloatEnvironmentEvidence::Probe {
                operation,
                expected,
                observed,
            } => {
                assert_eq!(record["evidence"]["kind"], "probe");
                assert_eq!(record["evidence"]["operation"], operation);
                assert_eq!(
                    record["evidence"]["expected_bits"],
                    super::encode::key(expected)
                );
                assert_eq!(
                    record["evidence"]["observed_bits"],
                    super::encode::key(observed)
                );
            }
            _ => unreachable!("known test evidence"),
        }
    }
}

#[test]
fn scale_and_configuration_refusals_keep_machine_readable_original_parameters() {
    use purrdf_geo_kernel::Int;
    let third = Rat::new(Int::one(), Int::from_i128(3)).unwrap();
    let seventh = Rat::new(Int::one(), Int::from_i128(7)).unwrap();
    let record = super::encode::refusal(&GeoCallError::Engine(GeoError::UnattainableEdgeLength {
        target: third,
        minimum: Box::new(seventh),
    }));
    assert_eq!(record["code"], "unattainable-edge-length");
    assert_eq!(record["target_metres"]["numerator"], "1");
    assert_eq!(record["target_metres"]["denominator"], "3");
    assert_eq!(record["minimum_metres"]["numerator"], "1");
    assert_eq!(record["minimum_metres"]["denominator"], "7");
    let record = super::encode::refusal(&GeoCallError::Engine(GeoError::NonPositiveEdgeLength(
        Rat::from_i64(-2),
    )));
    assert_eq!(record["target_metres"], "-2");
    for (error, field, detail) in [
        (
            GeoError::InvalidEllipsoid("original axes must be positive"),
            "reason",
            "original axes must be positive",
        ),
        (
            GeoError::InvalidExecutionPolicy("precision must be positive"),
            "reason",
            "precision must be positive",
        ),
        (
            GeoError::ArithmeticOverflow("complete output count"),
            "operation",
            "complete output count",
        ),
    ] {
        let record = super::encode::refusal(&GeoCallError::Engine(error));
        assert_eq!(record[field], detail);
    }
    let session = GeoSession::default();
    let record = response(
        &session,
        r#"{"version":1,"operation":"cell-scale","grid":"wgs84","level":16,"maximum_edge_metres":"0"}"#,
    );
    assert_eq!(record["error"]["code"], "nonpositive-edge-length");
    assert_eq!(record["error"]["target_metres"], "0");
    assert!(record.get("result").is_none());
    let record = response(
        &session,
        r#"{"version":1,"operation":"cell-scale","grid":"wgs84","level":16,"maximum_edge_metres":"0.0000000001"}"#,
    );
    assert_eq!(record["error"]["code"], "unattainable-edge-length");
    assert_eq!(record["error"]["target_metres"], "0.0000000001");
    assert!(record["error"].get("minimum_metres").is_some());
    assert!(record.get("result").is_none());
}

#[test]
fn complete_json_output_storage_refuses_before_large_result_construction() {
    use purrdf_geo_kernel::{ExecutionLimits, cells::NativeGridProfile};
    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: u64::MAX,
            max_workspace_bytes: 65_536,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let points = vec![super::PointInput(LonLat::new(Rat::zero(), Rat::zero()).unwrap()); 500];
    let request = GeoRequest::CellBatch {
        grid: NativeGridProfile::Wgs84,
        points,
        level: 0,
    };
    assert!(matches!(
        GeoSession::new(profile).call(&request),
        Err(GeoCallError::Engine(GeoError::MemoryExhausted {
            limit: 65_536
        }))
    ));
    let adequate = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: u64::MAX,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let admitted = GeoSession::new(adequate).call(&request).unwrap();
    assert_eq!(admitted.as_array().unwrap().len(), 500);
    let expected = purrdf_geo_kernel::cells::CubeHilbertQ62V1::new(NativeGridProfile::Wgs84)
        .assign(&LonLat::new(Rat::zero(), Rat::zero()).unwrap(), 0)
        .unwrap();
    assert!(
        admitted
            .as_array()
            .unwrap()
            .iter()
            .all(|value| *value == super::encode::cell(expected))
    );

    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: u64::MAX,
            max_workspace_bytes: 262_144,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let request = r#"{"version":1,"operation":"cover-region","grid":"wgs84","region_kind":"whole","mode":"fixed","level":4}"#;
    let refusal = response(&GeoSession::new(profile), request);
    assert_eq!(refusal["error"]["code"].as_str(), Some("memory-exhausted"));
    assert!(refusal.get("result").is_none());
}

#[test]
fn immutable_identity_has_distinct_checked_configuration_admission() {
    use purrdf_geo_kernel::{Crs, ExecutionLimits, ExecutionPolicy, Int};
    let low_compile = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    assert!(matches!(
        GeoSession::try_new(GeoProfile::standard(), low_compile),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { limit: 1 }))
    ));
    let invocation = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let session = GeoSession::try_new(invocation.clone(), ExecutionPolicy::geometry()).unwrap();
    let refused = response(&session, DISTANCE);
    assert_eq!(refused["error"]["code"].as_str(), Some("work-exhausted"));
    assert_eq!(
        refused["identity"]["binding"].as_str(),
        Some(invocation.binding_id().digest().to_string().as_str())
    );
    let mut profile = GeoProfile::standard();
    profile
        .register_linear_unit_in_budget(
            Crs::new("http://example.org/large-exact-unit").unwrap(),
            Rat::new(Int::one().shl(16_384), Int::one()).unwrap(),
            &mut purrdf_geo_kernel::PreparationBudget::new(
                ExecutionPolicy::new(ExecutionLimits {
                    max_work_items: u64::MAX,
                    ..ExecutionLimits::GEOMETRY
                })
                .unwrap(),
            ),
        )
        .unwrap();
    let retained_refusal = GeoSession::new(profile.clone());
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let copied_refusal = retained_refusal.clone();
    let cached_identity = copied_refusal.identity();
    let stats = window.close();
    assert!(matches!(
        cached_identity,
        Err(GeoError::WorkExhausted { .. })
    ));
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.requested_bytes, 0);
    let refused = response(&retained_refusal, DISTANCE);
    assert_eq!(refused["error"]["code"].as_str(), Some("work-exhausted"));
    assert_eq!(refused["identity"], Value::Null);
    let raised = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: u64::MAX,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let admitted = GeoSession::try_new(profile.clone(), raised).unwrap();
    assert_eq!(admitted.identity().unwrap(), profile.query_identity());
}

#[test]
fn configuration_parser_keeps_cumulative_work_and_separate_invocation_limits() {
    use purrdf_geo_kernel::{ExecutionLimits, ExecutionPolicy};
    let text = r#"{"version":1,"limits":{"max_work_items":1},"units":[{"iri":"https://example.org/unit/km","metres_per_unit":"1000"}]}"#;
    let session = GeoSession::from_profile_str(text).unwrap();
    assert_eq!(session.profile().policy().limits().max_work_items, 1);
    let low = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    assert!(matches!(
        GeoSession::from_profile_str_with_policy(text, low),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { limit: 1 }))
    ));
    let raised = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let admitted = GeoSession::from_profile_str_with_policy(text, raised).unwrap();
    assert_eq!(admitted.identity().unwrap(), session.identity().unwrap());
    let request = GeoRequest::parse(DISTANCE).unwrap();
    assert!(matches!(
        session.call(&request),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { limit: 1 }))
    ));
}

#[test]
fn profile_exports_share_complete_configuration_admission_and_preserve_bytes() {
    use purrdf_geo_kernel::{ExecutionLimits, ExecutionPolicy};
    let profile = similarity_profile();
    let expected = profile_to_string(&profile).unwrap();
    let raised = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    assert_eq!(
        super::profile_to_string_with_policy(&profile, raised).unwrap(),
        expected
    );
    for limits in [
        ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        },
        ExecutionLimits {
            max_workspace_bytes: 1,
            ..ExecutionLimits::GEOMETRY
        },
    ] {
        let policy = ExecutionPolicy::new(limits).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = super::profile_to_string_with_policy(&profile, policy);
        let stats = window.close();
        assert!(matches!(
            result,
            Err(GeoCallError::Engine(
                GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. }
            ))
        ));
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.requested_bytes, 0);
    }
    let separate = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
    let exported = profile_to_string(&separate).unwrap();
    let value = json::read(&exported).unwrap();
    assert_eq!(value["limits"]["max_work_items"], Value::from(1_u64));
}

#[test]
fn immutable_profile_snapshots_keep_large_source_ownership_without_copying() {
    use purrdf_geo_kernel::{Crs, ExecutionLimits, ExecutionPolicy, Int, PreparationBudget};
    let generous = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 100_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut profile = GeoProfile::standard();
    profile
        .register_linear_unit_in_budget(
            Crs::new("http://example.org/snapshot-unit").unwrap(),
            Rat::from_int(Int::one().shl(4_096)),
            &mut PreparationBudget::new(generous),
        )
        .unwrap();
    let session = GeoSession::try_new(profile, generous).unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let first = session.profile_snapshot();
    let second = session.profile_snapshot();
    let allocations = window.close();
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    let expected = session.identity().unwrap();
    drop(session);
    assert_eq!(first.query_identity(), expected);
    assert_eq!(
        second.linear_units()[0]
            .metres_per_unit()
            .numerator()
            .bit_len(),
        4_097
    );
}

#[test]
fn borrowed_session_configuration_keeps_copy_and_compile_in_one_budget() {
    use purrdf_geo_kernel::{ExecutionLimits, ExecutionPolicy, PreparationBudget};
    let profile = similarity_profile();
    let mut copy_budget = PreparationBudget::new(ExecutionPolicy::geometry());
    drop(profile.clone_in_budget(&mut copy_budget).unwrap());
    let copy_only = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: copy_budget.work_items(),
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    assert!(matches!(
        GeoSession::try_from_profile(&profile, copy_only),
        Err(GeoCallError::Engine(GeoError::WorkExhausted { .. }))
    ));
    for limits in [
        ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        },
        ExecutionLimits {
            max_workspace_bytes: 1,
            ..ExecutionLimits::GEOMETRY
        },
    ] {
        let policy = ExecutionPolicy::new(limits).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refused = GeoSession::try_from_profile(&profile, policy);
        let stats = window.close();
        assert!(matches!(
            refused,
            Err(GeoCallError::Engine(
                GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. }
            ))
        ));
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.requested_bytes, 0);
    }
    let borrowed = GeoSession::try_from_profile(&profile, ExecutionPolicy::geometry()).unwrap();
    let owned = GeoSession::try_new(profile, ExecutionPolicy::geometry()).unwrap();
    assert_eq!(borrowed.identity().unwrap(), owned.identity().unwrap());
    assert_eq!(borrowed.call_string(DISTANCE), owned.call_string(DISTANCE));
}
