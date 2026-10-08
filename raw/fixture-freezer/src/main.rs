// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use purrdf_lex::json::{self, Value};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let source = json::read(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mut target = json::read(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let Value::Array(fixtures) = &mut target else { panic!("fixture array required") };
    let names: Vec<_> = fixtures.iter().flat_map(|fixture| fixture["cases"].as_array().unwrap()).map(|case| case["name"].as_str().unwrap()).collect();
    for case in source["cases"].as_array().unwrap() {
        assert!(!names.contains(&case["name"].as_str().unwrap()), "duplicate case");
    }
    let mut named = Vec::new();
    let mut plain = Vec::new();
    for case in source["cases"].as_array().unwrap() {
        if case["dataset"].as_str() == Some("named") { named.push(case.clone()); }
        else { plain.push(case.clone()); }
    }
    if !plain.is_empty() {
        fixtures.push(Value::Object(vec![
            ("triples".into(), source["triples"].clone()),
            ("cases".into(), Value::Array(plain)),
        ].into()));
    }
    if !named.is_empty() {
        fixtures.push(Value::Object(vec![
            ("triples".into(), source["triples"].clone()),
            ("quads".into(), source["quads"].clone()),
            ("cases".into(), Value::Array(named)),
        ].into()));
    }
    std::fs::write(&args[2], target.to_string()).unwrap();
    println!("froze {} primary oracle answers, preserving ordered rows and RDF terms", source["cases"].as_array().unwrap().len());
}
