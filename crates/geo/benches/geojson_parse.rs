// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! GeoJSON reading through the one shared JSON reader (`purrdf_lex::json`).
//!
//! **Report-only, and never a gate.** Wall-clock samples from a shared
//! development host are not acceptance evidence; behaviour is fixed by the test
//! suite.
//!
//! * `geojson_json/parse` and `geojson_json/write` read and re-render a roughly
//!   1 MB `FeatureCollection` (polygons with fractional coordinates, plus
//!   properties holding clean and escape-bearing strings) through
//!   `purrdf_geo::json`, the JSON tree.
//! * `geojson_literal/parse` reads one large `MultiPolygon` GeoJSON literal into
//!   the exact geometry model straight from the JSON reader's events, so the
//!   reader and the coordinate ingest are measured together.
//!
//! The corpus is generated deterministically from the constants below with the
//! workspace's SplitMix64 (`purrdf_testkit::rng`).

use core::fmt::Write as _;

use purrdf_geo::{Crs, geojson, json};
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_testkit::rng::SplitMix64;

/// Features in the collection; sized so the document is about one megabyte.
const FEATURES: usize = 1_400;
/// Vertices per polygon ring, closing vertex included.
const RING: usize = 17;
/// Polygons in the `MultiPolygon` literal.
const POLYGONS: usize = 2_000;

fn coordinate(rng: &mut SplitMix64) -> String {
    let whole = rng.next_u64() % 360;
    let fraction = rng.next_u64() % 1_000_000;
    format!("{}.{fraction:06}", whole as i64 - 180)
}

fn ring(rng: &mut SplitMix64, out: &mut String) {
    out.push('[');
    let mut first = String::new();
    for i in 0..RING {
        if i > 0 {
            out.push(',');
        }
        let vertex = if i + 1 == RING {
            first.clone()
        } else {
            format!("[{},{}]", coordinate(rng), coordinate(rng))
        };
        if i == 0 {
            first.clone_from(&vertex);
        }
        out.push_str(&vertex);
    }
    out.push(']');
}

fn feature_collection() -> String {
    let mut rng = SplitMix64::new(0x6E0_1500);
    let mut out = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
    for i in 0..FEATURES {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"type\":\"Feature\",\"id\":{i},\"properties\":{{\"name\":\"parcel {i}\",\
             \"note\":\"line one\\nline \\\"two\\\" caf\\u00e9 \\ud83d\\udc31\",\"area\":{}.5}},\
             \"geometry\":{{\"type\":\"Polygon\",\"coordinates\":[",
            i * 3
        );
        ring(&mut rng, &mut out);
        out.push_str("]}}");
    }
    out.push_str("]}");
    out
}

fn multi_polygon() -> String {
    let mut rng = SplitMix64::new(0x6E0_2600);
    let mut out = String::from("{\"type\":\"MultiPolygon\",\"coordinates\":[");
    for i in 0..POLYGONS {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        ring(&mut rng, &mut out);
        out.push(']');
    }
    out.push_str("]}");
    out
}

fn bench_geojson(c: &mut Bench) {
    let collection = feature_collection();
    let tree = json::parse(&collection).expect("the generated collection is valid JSON");
    assert_eq!(json::count(&tree, "type"), 1);
    let multi = multi_polygon();
    let crs = Crs::new("http://example.org/crs/planar").expect("a non-empty IRI");
    geojson::parse(&multi, &crs).expect("the generated MultiPolygon is a valid literal");

    let mut group = c.benchmark_group("geojson_json");
    group.throughput(Throughput::Bytes(collection.len() as u64));
    group.bench_function("parse", |bencher| {
        bencher.iter(|| black_box(json::parse(black_box(&collection)).expect("parses")));
    });
    group.bench_function("write", |bencher| {
        bencher.iter(|| black_box(json::write(black_box(&tree))));
    });
    group.finish();

    let mut group = c.benchmark_group("geojson_literal");
    group.throughput(Throughput::Bytes(multi.len() as u64));
    group.bench_function("parse", |bencher| {
        bencher.iter(|| black_box(geojson::parse(black_box(&multi), &crs).expect("parses")));
    });
    group.finish();
}

bench_group!(benches, bench_geojson);
bench_main!(benches);
