// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real binary requests and explicit query profiles use the shared Rust boundary.

mod support;
use support::{pipe, run, stderr, stdout_utf8, write_file};

const DISTANCE: &str = r#"{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"1","latitude":"0"}}"#;

#[test]
fn scalar_request_and_typed_refusal_are_complete_process_outputs() {
    let answer = pipe(&["geo"], DISTANCE);
    assert!(answer.status.success(), "{}", stderr(&answer));
    let answer = purrdf_lex::json::read(&stdout_utf8(&answer)).expect("canonical response");
    assert_eq!(answer["result"]["metres"], "111319.490793");
    let invalid = pipe(
        &["geo"],
        &DISTANCE.replace("\"1\"", "\"180.0000000000000000000001\""),
    );
    assert_eq!(invalid.status.code(), Some(1));
    let invalid = purrdf_lex::json::read(&stdout_utf8(&invalid)).expect("typed refusal output");
    assert_eq!(invalid["error"]["code"], "coordinate-range");
    assert!(invalid.get("result").is_none());
}

#[test]
fn explicit_latitude_longitude_profile_reaches_query_admission_and_execution() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary profile");
    let data = write_file(dir.path(), "empty.ttl", "");
    let profile = write_file(
        dir.path(),
        "geo.json",
        &format!(
            r#"{{"version":1,"references":[{{"crs":"{}","axes":"lat-lon","ellipsoid":"wgs84"}}]}}"#,
            purrdf_iri::vocab::ogc::EPSG4326
        ),
    );
    let query = format!(
        "SELECT (<{}>(\"<{}> POINT(0 0)\"^^<{}>,\"<{}> POINT(0 1)\"^^<{}>) AS ?distance) WHERE {{}}",
        purrdf_iri::vocab::ogc::geof::METRIC_DISTANCE,
        purrdf_iri::vocab::ogc::EPSG4326,
        purrdf_iri::vocab::ogc::geo::WKT_LITERAL,
        purrdf_iri::vocab::ogc::EPSG4326,
        purrdf_iri::vocab::ogc::geo::WKT_LITERAL
    );
    let refused = run(&["query", "--data", &data, "--results-format", "json", &query]);
    assert_eq!(refused.status.code(), Some(1));
    let allowed = run(&[
        "--geo-profile",
        &profile,
        "query",
        "--data",
        &data,
        "--results-format",
        "json",
        &query,
    ]);
    assert!(allowed.status.success(), "{}", stderr(&allowed));
    let record = purrdf_lex::json::read(&stdout_utf8(&allowed)).expect("SPARQL results");
    assert_eq!(
        record["results"]["bindings"][0]["distance"]["value"],
        "1.11319490793E5"
    );
}

#[test]
fn index_query_preserves_stable_caller_keys_and_negative_reported_law() {
    let request = r#"{"version":1,"operation":"point-index-query","index":{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}}]},"search":{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"integer","value":"-1"}}}"#;
    let output = pipe(&["geo"], request);
    assert!(output.status.success(), "{}", stderr(&output));
    let record = purrdf_lex::json::read(&stdout_utf8(&output)).expect("index response");
    assert_eq!(
        record["result"]["keys"],
        purrdf_lex::json::Value::Array(Vec::new())
    );
}

#[test]
fn explicit_transform_grid_crosses_the_profile_and_request_boundary() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary operation profile");
    let profile = write_file(
        dir.path(),
        "operation.json",
        &format!(
            r#"{{"version":1,"operations":[{{"name":"http://example.org/identity","source_crs":"http://example.org/source","target_crs":"http://example.org/target","chain":[{{"source":{{"realization":"{}","unit":"metres","swapped_axes":false}},"target":{{"realization":"{}","unit":"metres","swapped_axes":false}},"model":{{"law":"similarity2d-v1","translation_metres":["0","0"],"scale":"1","rotation_degrees":"0","convention":"position-vector","inverse":false}}}}]}}]}}"#,
            "00".repeat(32),
            "01".repeat(32),
        ),
    );
    let request = r#"{"version":1,"operation":"transform-batch","name":"http://example.org/identity","points":[{"x":"1.2345678912","y":"2.3456789123"}],"metric_decimal_places":9}"#;
    let answer = pipe(&["--geo-profile", &profile, "geo"], request);
    assert!(answer.status.success(), "{}", stderr(&answer));
    let record = purrdf_lex::json::read(&stdout_utf8(&answer)).expect("exact response");
    assert_eq!(record["result"][0]["x"], "1.234567891");
    assert_eq!(record["result"][0]["y"], "2.345678912");
    assert_eq!(record["result"][0]["metric_decimal_places"], 9_u32);
    let refused = pipe(
        &["--geo-profile", &profile, "geo"],
        &request.replace("\"metric_decimal_places\":9", "\"metric_decimal_places\":5"),
    );
    assert_eq!(refused.status.code(), Some(1));
    let record = purrdf_lex::json::read(&stdout_utf8(&refused)).expect("typed refusal");
    assert_eq!(record["error"]["code"], "precision-exhausted");
    assert!(record.get("result").is_none());
}

#[test]
fn zero_buffer_process_output_preserves_the_original_decimal_closure() {
    let request = r#"{"version":1,"operation":"buffer-points","geometry":{"kind":"prepared","points":[{"x":"0.1234567890123456789","y":"0"}]},"radius_metres":"0"}"#;
    let answer = pipe(&["geo"], request);
    assert!(answer.status.success(), "{}", stderr(&answer));
    let record = purrdf_lex::json::read(&stdout_utf8(&answer)).expect("complete buffer response");
    assert_eq!(
        record["result"]["geometry"]["value"],
        format!(
            "<{}> MULTIPOINT((0.1234567890123456789 0))",
            purrdf_iri::vocab::ogc::CRS84
        )
    );
    assert_eq!(record["result"]["outward_error_metres"], "0.1");
}
