// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Completed public geodesy, operation, metric, buffer, cover and index bytes.
//! The same target executes on native, portable wasm and SIMD wasm. Invocation
//! work, precision and tighter proof receipts are deliberately absent from the
//! frozen completed-output digest.

use purrdf_geo_kernel::cells::{
    ClosedBox, CoverLevels, CubeHilbertQ62V1, MixedCoverLimits, NativeGridProfile, PointCellIndex,
    PointIndexLimits, PointIndexPoint,
};
use purrdf_geo_kernel::geodesic::DirectOutputGrid;
use purrdf_geo_kernel::operation::{
    Applicability, CoordinateOperation, CoordinateUnit, Hemisphere, OperationModel, OperationPoint,
    OperationReference, TransverseMercator, ZoneFamily,
};
use purrdf_geo_kernel::{
    Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, GeographicReference, LonLat, Metres,
    MetricContext, OffsetRegion, PreparedGeodesic, PreparedGeometry, Rat, XsdDoubleMetres,
    ellipsoidal, wkt,
};
use purrdf_hash::{Backend as _, fnv, frame::frame_le, hex::Digest32};
use purrdf_iri::vocab::ogc;
use purrdf_testkit::harness::report_digest;
use purrdf_xsd::math::FloatProductBackend;
use std::sync::Arc;

// Certificates bind complete two-gauge area, actual selected-stratum
// length/perimeter and the original matched-TM continuous-image laws.
// Against the previous 0x3133ce37aba5ecb2 corpus of 65 records, the 65 shared
// records differ only at the two box-cover law identities (records 46/55): the
// cover law now also binds the version-2 physical-disk classifier. Every point
// inverse, distance, direct, operation, metric, buffer, cover cell, index
// identity and search key is byte-identical. Eight records are new: the
// near-antipodal GeodTest pair's inverse and distance certificates under both
// native references, and a street-scale default-limit disk cover's law and
// complete cell list under both native grids.
const GOLDEN_DIGEST: u64 = 0x60a7_5046_1b26_a1ea;
const CORPUS_LEN: usize = 73;

#[derive(Default)]
struct Replay {
    bytes: Vec<u8>,
    count: usize,
}
impl Replay {
    fn record(&mut self, bytes: &[u8]) {
        frame_le(&mut self.bytes, bytes);
        self.count += 1;
        // A field audit names exactly which completed records moved.
        if let Some(path) = std::env::var_os("PURRDF_GEO_REPLAY_DUMP") {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("replay dump file");
            writeln!(
                file,
                "{} {:016x} {}",
                self.count,
                fnv::fold(fnv::BASIS, bytes),
                String::from_utf8_lossy(bytes).escape_debug()
            )
            .expect("replay dump line");
        }
    }
}

fn point(longitude: &str, latitude: &str) -> LonLat {
    LonLat::new(
        Rat::parse_decimal(longitude).unwrap(),
        Rat::parse_decimal(latitude).unwrap(),
    )
    .unwrap()
}

fn point_geodesy(replay: &mut Replay) {
    let pairs = [
        (point("0", "0"), point("1", "0")),
        (point("179.5", "89"), point("-179.5", "89")),
        (point("0", "0"), point("180", "0")),
        (point("10", "20"), point("10", "20")),
        (point("23", "-12"), point("24", "-11")),
        (point("31", "89"), point("0", "90")),
        // A near-antipodal, near-conjugate public CC0 GeodTest row.
        (
            point("0", "21.004101257892"),
            point("179.43641220015808594", "-21.004101257877442853"),
        ),
    ];
    let paths = FloatProductBackend::all_available().collect::<Vec<_>>();
    println!(
        "completed point replay arithmetic paths: {}",
        paths
            .iter()
            .map(|path| path.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert!(paths.contains(&FloatProductBackend::Simd128));
    for reference in [
        GeographicReference::wgs84(),
        GeographicReference::cgcs2000(),
    ] {
        let mut context =
            MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
        let prepared = PreparedGeodesic::prepare(reference.clone(), &mut context).unwrap();
        let mut expected = Vec::new();
        for (a, b) in &pairs {
            let inverse = prepared.inverse(a, b, &mut context).unwrap();
            let certificate = inverse.certificate_bytes();
            replay.record(&certificate);
            expected.push(certificate);
        }
        assert_eq!(
            prepared
                .distance(&pairs[0].0, &pairs[0].1, &mut context)
                .unwrap()
                .value()
                .exact(),
            &Rat::parse_decimal("111319.490793").unwrap(),
            "independent equatorial shortest-distance vector",
        );
        assert!(
            prepared
                .distance(&pairs[6].0, &pairs[6].1, &mut context)
                .unwrap()
                .value()
                .exact()
                .sub(&Rat::parse_decimal("19974623.3063849").unwrap())
                .abs()
                <= Rat::parse_decimal("0.0000006").unwrap()
                || reference != GeographicReference::wgs84(),
            "public near-antipodal GeodTest distance",
        );
        let mut output = [const { None }; 7];
        prepared
            .distance_batch_borrowed(pairs.iter().map(|(a, b)| (a, b)), &mut output, &mut context)
            .unwrap();
        for result in output {
            replay.record(&result.unwrap().certificate_bytes());
        }
        for (start, azimuth, distance, grid) in [
            (point("0", "0"), 90, 1_000, DirectOutputGrid::DEGREE15),
            (point("73", "90"), 123, 10_000, DirectOutputGrid::DEGREE15),
            (point("179.999", "0"), 90, 1_000, DirectOutputGrid::new(18)),
        ] {
            replay.record(
                &prepared
                    .direct_with_grid(
                        &start,
                        &Rat::from_i64(azimuth),
                        &Metres::new(Rat::from_i64(distance)),
                        grid,
                        &mut context,
                    )
                    .unwrap()
                    .certificate_bytes(),
            );
        }
        for &backend in &paths {
            let mut worker =
                MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
            worker.set_binary64_backend(backend).unwrap();
            for ((a, b), certificate) in pairs.iter().zip(&expected) {
                assert_eq!(
                    prepared
                        .inverse(a, b, &mut worker)
                        .unwrap()
                        .certificate_bytes(),
                    *certificate,
                    "forced completed inverse bytes: {}",
                    backend.name(),
                );
            }
        }
    }
}

fn operations(replay: &mut Replay) {
    let source = OperationReference {
        realization: GeographicReference::wgs84().id().digest(),
        unit: CoordinateUnit::Degrees,
        swapped_axes: false,
    };
    let input = OperationPoint {
        x: Rat::parse_decimal("121.47").unwrap(),
        y: Rat::parse_decimal("31.23").unwrap(),
        z: Some(Rat::from_i64(10)),
        epoch: None,
    };
    let applicability = Applicability::new(
        Rat::from_i64(121),
        Rat::from_i64(122),
        Rat::from_i64(31),
        Rat::from_i64(32),
    )
    .unwrap();
    let models = [
        OperationModel::GcjRationalHarmonicV1(applicability),
        OperationModel::Bd09LlV1,
        OperationModel::BaiduMercatorAnalyticV1,
        OperationModel::WebMercator {
            radius: Rat::from_i64(6_378_137),
        },
    ];
    let mut context = MetricContext::wgs84().unwrap();
    for (index, model) in models.into_iter().enumerate() {
        let target = OperationReference {
            realization: Digest32::new([index as u8 + 1; 32]),
            unit: if index < 2 {
                CoordinateUnit::Degrees
            } else {
                CoordinateUnit::Metres
            },
            swapped_axes: false,
        };
        let operation = CoordinateOperation::compile(source, target, model).unwrap();
        let result = operation.apply(&input, &mut context).unwrap();
        replay.record(&result.certificate_bytes());
        let inverse = operation
            .inverse()
            .unwrap_or_else(|error| panic!("compile inverse model {index}: {error}"));
        replay.record(
            &inverse
                .apply(result.point(), &mut context)
                .unwrap_or_else(|error| panic!("apply inverse model {index}: {error}"))
                .certificate_bytes(),
        );
    }
    for reference in [
        GeographicReference::wgs84(),
        GeographicReference::cgcs2000(),
    ] {
        let source = OperationReference {
            realization: reference.id().digest(),
            ..source
        };
        let mut context =
            MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
        let operation = CoordinateOperation::compile(
            source,
            OperationReference {
                realization: Digest32::new([9; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            },
            OperationModel::TransverseMercator(Box::new(TransverseMercator {
                ellipsoid: reference.ellipsoid().clone(),
                family: ZoneFamily::GaussKruger3,
                zone: 40,
                central_meridian: Rat::from_i64(120),
                scale: Rat::one(),
                false_easting: Rat::from_i64(500_000),
                false_northing: Rat::zero(),
                hemisphere: Hemisphere::North,
                zone_prefix: true,
            })),
        )
        .unwrap();
        let result = operation.apply(&input, &mut context).unwrap();
        replay.record(&result.certificate_bytes());
        replay.record(
            &operation
                .inverse()
                .unwrap()
                .apply(result.point(), &mut context)
                .unwrap()
                .certificate_bytes(),
        );
    }
}

fn spatial_outputs(replay: &mut Replay) {
    let profile = GeoProfile::standard();
    let crs = Crs::new(ogc::CRS84).unwrap();
    let literal = wkt::parse("POLYGON((0 0,0.001 0,0.001 0.001,0 0.001,0 0))", &crs).unwrap();
    let region = PreparedGeometry::from_literal(&literal, &profile).unwrap();
    let mut context = MetricContext::wgs84().unwrap();
    for result in [
        ellipsoidal::length(&region, &mut context).unwrap(),
        ellipsoidal::perimeter(&region, &mut context).unwrap(),
        ellipsoidal::area(&region, &mut context).unwrap(),
    ] {
        replay.record(&result.certificate_bytes());
    }
    let levels = CoverLevels::new(0, 1).unwrap();
    let center = point("0", "0");
    for (native, reference) in [
        (NativeGridProfile::Wgs84, GeographicReference::wgs84()),
        (NativeGridProfile::Cgcs2000, GeographicReference::cgcs2000()),
    ] {
        let grid = CubeHilbertQ62V1::new(native);
        let bounds = ClosedBox::new(
            Rat::from_i64(170),
            Rat::from_i64(-5),
            Rat::from_i64(-170),
            Rat::from_i64(5),
        )
        .unwrap();
        let cover = grid
            .cover_box_mixed(&bounds, levels, MixedCoverLimits::DEFAULT)
            .unwrap();
        replay.record(cover.law_id().digest().as_bytes());
        for cell in cover.cells() {
            replay.record(&cell.to_be_bytes());
        }
        // A street-scale physical disk under default limits: its law and every
        // emitted mixed-level cell, in canonical order, as one record.
        let disk = grid
            .cover_disk_mixed(
                &point("116", "40"),
                &Metres::new(Rat::from_i64(1_000)),
                CoverLevels::new(0, 16).unwrap(),
                MixedCoverLimits::DEFAULT,
            )
            .unwrap();
        replay.record(disk.law_id().digest().as_bytes());
        replay.record(
            &disk
                .cells()
                .iter()
                .flat_map(|cell| cell.to_be_bytes())
                .collect::<Vec<_>>(),
        );
        let index = PointCellIndex::new(
            grid,
            // Coarse buckets place the origin and one-degree neighbour together:
            // the positive query must apply the actual reported-distance law.
            0,
            reference,
            None,
            vec![
                PointIndexPoint {
                    key: 19,
                    point: center.clone(),
                },
                PointIndexPoint {
                    key: 7,
                    point: point("1", "0"),
                },
                PointIndexPoint {
                    key: 23,
                    point: point("179", "89"),
                },
            ],
            PointIndexLimits::DEFAULT,
        )
        .unwrap();
        replay.record(index.id().digest().as_bytes());
        let result = index
            .search_reported(
                &center,
                XsdDoubleMetres::from_integer(1_000).unwrap(),
                MixedCoverLimits::DEFAULT,
                PointIndexLimits::DEFAULT,
            )
            .unwrap();
        assert_eq!(result, [19]);
        for key in result {
            replay.record(&key.to_be_bytes());
        }
    }
    let source = wkt::parse("POINT(0 0)", &crs).unwrap();
    let source = Arc::new(PreparedGeometry::from_literal(&source, &profile).unwrap());
    let offset =
        OffsetRegion::new(source, Metres::new(Rat::one()), ExecutionPolicy::geometry()).unwrap();
    // The complete one-metre ring has an explicit invocation admission. Its
    // completed output law does not bind the numerical proof cost.
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1_048_576,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
    let materialized = offset
        .materialize_points(&profile, &crs, &mut context)
        .unwrap();
    replay.record(&materialized.certificate_bytes());
    replay.record(wkt::write(materialized.literal(), 15).as_bytes());
}

fn public_geo_capability_replay() {
    let digest = report_digest("public_geo_capability_replay", CORPUS_LEN, || {
        let mut replay = Replay::default();
        point_geodesy(&mut replay);
        operations(&mut replay);
        spatial_outputs(&mut replay);
        assert_eq!(
            replay.count, CORPUS_LEN,
            "freeze complete capability coverage"
        );
        fnv::fold(fnv::BASIS, &replay.bytes)
    });
    assert_eq!(
        digest, GOLDEN_DIGEST,
        "completed public capability bytes changed"
    );
}

purrdf_testkit::harness_main!(public_geo_capability_replay);
