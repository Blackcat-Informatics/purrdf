// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared-product native semantics and two WASM carrier probes.
//!
//! Native `cargo test -p purrdf-shapes --test product_wasm` owns the complete
//! product lifecycle and the vendored W3C SHACL 1.2 subset registered here.
//! The shared `harness = false` runner retains every body and registration.
//!
//! `make wasm-test` selects only encoding against committed native bytes and
//! restoration of that native golden. These exercise the product's integer
//! fields and byte layout on wasm32, qualifying a product prepared by a host
//! for restoration in a browser. The golden is committed under `tests/fixtures/`
//! and produced by a native build; each target compares with those same bytes.
//! Named selections and native owners are documented in
//! [WASM test ownership](../../../docs/WASM_TESTING.md).
//!
//! The W3C cases cover list details, `sh:closed sh:ByTypes`,
//! `sh:uniqueValuesFor`, standalone node expressions, SPARQL 1.2 RL and merged
//! SHACL vocabularies. They remain complete native conformance coverage.
//!
//! # No filesystem
//!
//! Shapes, data and the golden product are compile-time constants. Neither
//! carrier probe opens a file, so it measures the codec rather than runner
//! filesystem shims.

mod product_fixture;

use purrdf_shapes::product::{ShapesProduct, ShapesProfile};

/// The bytes this target writes for the fixture are the bytes the host wrote.
///
/// The golden was produced by a native build and committed; the wasm run encodes
/// the same shapes graph from source and compares. A target whose pointer width,
/// endianness or hash seeding reached the writer renders a different product here
/// rather than shipping a cache two engines disagree about.
fn encoding_matches_the_committed_bytes_on_this_target() {
    let bytes = product_fixture::encode();

    assert_eq!(
        bytes.len(),
        product_fixture::GOLDEN.len(),
        "this target writes a {}-byte product for the fixture and the committed golden is {} \
         bytes; a size difference is a layout difference, not a rounding one",
        bytes.len(),
        product_fixture::GOLDEN.len(),
    );
    assert!(
        bytes == product_fixture::GOLDEN,
        "this target writes different product bytes than the committed golden (first difference \
         at byte {:?}); the prepared-product codec must be a pure function of the shapes graph \
         on every target, or a product prepared on a host cannot be restored in a browser",
        bytes
            .iter()
            .zip(product_fixture::GOLDEN.iter())
            .position(|(a, b)| a != b),
    );
}

/// Encode, open, admit and validate — the entire lifecycle, on this target.
///
/// The byte comparison above proves the *writer* agrees across targets; it says
/// nothing about the reader, and a product this target can write but not restore
/// is still a broken cache. This runs the round trip end to end and checks the
/// answer, so a decoder that mis-read a field on one target is caught by the
/// report rather than by the bytes.
fn encode_open_admit_validate_on_this_target() {
    let expected = product_fixture::expected_report_nt();

    let bytes = product_fixture::encode();
    let restored = ShapesProduct::open(&bytes)
        .expect("the product this target just wrote opens on this target")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect("the product this target just wrote admits on this target");

    let restored_nt = product_fixture::report_nt(&restored);
    product_fixture::assert_non_vacuous(&restored_nt);
    assert_eq!(
        restored_nt, expected,
        "a product restored on this target validates differently from a fresh parse of the same \
         shapes graph on this target",
    );
}

/// The committed golden — bytes this target did not produce — restores and
/// validates here.
///
/// This is the cross-target *restore* direction: the product travelled from a
/// native build into this target's memory as a constant, and it has to become a
/// working validator. Encoding and decoding could both be target-dependent in the
/// same way and still pass the round trip above; only a product from another
/// target separates them.
fn the_committed_golden_restores_on_this_target() {
    let restored = ShapesProduct::open(product_fixture::GOLDEN)
        .expect("the committed golden opens on this target")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect("the committed golden admits on this target");

    let restored_nt = product_fixture::report_nt(&restored);
    product_fixture::assert_non_vacuous(&restored_nt);
    assert_eq!(
        restored_nt,
        product_fixture::expected_report_nt(),
        "the committed golden restored on this target to a validator that answers differently \
         from a fresh parse of the same shapes graph",
    );
}

// ── The W3C SHACL 1.2 subset ──────────────────────────────────────────────────

mod shacl12_subset {
    use std::sync::Arc;

    use purrdf_rdf::RdfDataset;
    use purrdf_shapes::data::{GraphFilter, native_quads};
    use purrdf_shapes::engine::validate_dataset_with_shapes_graph;
    use purrdf_shapes::free_expression::{FreeExpression, evaluate};
    use purrdf_shapes::shacl_corpora::named as iri;
    use purrdf_shapes::shapes::from_dataset_with_config_and_graph;
    use purrdf_shapes::srl::{self, InferOptions};
    use purrdf_shapes::term::Term;
    use purrdf_shapes::text_ingest::parse_turtle_document;

    use purrdf_iri::vocab::rdf::NS as RDF;
    use purrdf_iri::vocab::sh::NS as SH;
    const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
    const SHT: &str = "http://www.w3.org/ns/shacl-test#";

    /// The base every vendored file parses under: the files name themselves with `<>`
    /// and their entries relatively, and no filesystem path exists on wasm32.
    fn base(name: &str) -> String {
        format!("http://example.org/shacl12/{name}")
    }

    fn objects(dataset: &RdfDataset, subject: &Term, predicate: &str) -> Vec<Term> {
        native_quads(
            dataset,
            Some(subject),
            Some(&iri(predicate)),
            None,
            GraphFilter::AnyGraph,
        )
        .into_iter()
        .map(|(_, _, object)| object)
        .collect()
    }

    fn one(dataset: &RdfDataset, subject: &Term, predicate: &str) -> Option<Term> {
        let mut found = objects(dataset, subject, predicate);
        assert!(found.len() <= 1, "{subject} has several {predicate}");
        found.pop()
    }

    /// A term as a grading key: a blank node is `_` (its label is the parser's, not the
    /// test's), an absent one `-`.
    fn key(term: Option<&Term>) -> String {
        match term {
            None => "-".to_owned(),
            Some(Term::BlankNode(_)) => "_".to_owned(),
            Some(term) => term.to_string(),
        }
    }

    /// The members of the RDF list headed by `head`.
    fn list(dataset: &RdfDataset, head: &Term) -> Vec<Term> {
        let mut out = Vec::new();
        let mut cursor = head.clone();
        while cursor != iri(&format!("{RDF}nil")) {
            out.push(one(dataset, &cursor, &format!("{RDF}first")).expect("a list cell"));
            cursor = one(dataset, &cursor, &format!("{RDF}rest")).expect("a list cell");
        }
        out
    }

    /// Validate an `sht:Validate` file against itself and grade the report's top-level
    /// results — focus, component, severity, source shape, value and path — and its
    /// `sh:conforms` against the file's own `mf:result`.
    fn grade_validate(name: &str, text: &str, entry: &str) {
        let document = parse_turtle_document(text, Some(&base(name))).expect("the file parses");
        let shapes =
            from_dataset_with_config_and_graph(&document.dataset, &document.prefixes, None, None)
                .unwrap_or_else(|e| panic!("{name}: the shapes graph loads on this target: {e}"));
        let report = validate_dataset_with_shapes_graph(&document.dataset, &shapes, None)
            .unwrap_or_else(|e| panic!("{name}: validation runs on this target: {e}"));
        let mut produced: Vec<String> = report
            .results
            .iter()
            .map(|result| {
                format!(
                    "{} {} <{}> {} {} {}",
                    key(Some(&result.focus_node)),
                    result.source_constraint_component,
                    result.severity.iri(),
                    key(Some(&result.source_shape)),
                    key(result.value.as_ref()),
                    key(result.result_path.as_ref()),
                )
            })
            .collect();
        produced.sort();

        let dataset = &document.dataset;
        let entry = iri(&base(entry));
        let expected_report = one(dataset, &entry, &format!("{MF}result")).expect("mf:result");
        let conforms = one(dataset, &expected_report, &format!("{SH}conforms"))
            .expect("sh:conforms")
            .to_string();
        let mut expected: Vec<String> = objects(dataset, &expected_report, &format!("{SH}result"))
            .iter()
            .map(|result| {
                let field = |local: &str| one(dataset, result, &format!("{SH}{local}"));
                format!(
                    "{} {} {} {} {} {}",
                    key(field("focusNode").as_ref()),
                    key(field("sourceConstraintComponent").as_ref()),
                    key(field("resultSeverity").as_ref()),
                    key(field("sourceShape").as_ref()),
                    key(field("value").as_ref()),
                    key(field("resultPath").as_ref()),
                )
            })
            .collect();
        expected.sort();
        assert!(!expected.is_empty(), "{name}: a non-vacuous expectation");
        assert_eq!(produced, expected, "{name}: the results on this target");
        assert_eq!(
            conforms.contains("true"),
            report.conforms,
            "{name}: sh:conforms on this target"
        );
    }

    /// SHACL 1.2 Core `sh:memberShape`, with `sh:detail` results.
    pub(super) fn member_shape_001_on_this_target() {
        grade_validate(
            "core/node/memberShape-001.ttl",
            include_str!("../../../vectors/shacl12/tests/core/node/memberShape-001.ttl"),
            "core/node/memberShape-001",
        );
    }

    /// SHACL 1.2 Core `sh:closed sh:ByTypes`.
    pub(super) fn closed_by_types_003_on_this_target() {
        grade_validate(
            "core/node/closed-003.ttl",
            include_str!("../../../vectors/shacl12/tests/core/node/closed-003.ttl"),
            "core/node/closed-003",
        );
    }

    /// SHACL 1.2 Core `sh:uniqueValuesFor`.
    pub(super) fn unique_values_for_001_on_this_target() {
        grade_validate(
            "core/node/uniqueValuesFor-001.ttl",
            include_str!("../../../vectors/shacl12/tests/core/node/uniqueValuesFor-001.ttl"),
            "core/node/uniqueValuesFor-001",
        );
    }

    /// Every `sht:EvalNodeExpr` entry of `shnex/concat.ttl`, through the standalone
    /// evaluator every host reaches, graded term for term and in order against its
    /// `mf:result` list.
    pub(super) fn node_expr_concat_on_this_target() {
        let name = "node-expr/shnex/concat.ttl";
        let document = parse_turtle_document(
            include_str!("../../../vectors/shacl12/tests/node-expr/shnex/concat.ttl"),
            Some(&base(name)),
        )
        .expect("the file parses");
        let dataset: &Arc<RdfDataset> = &document.dataset;
        let manifest = iri(&base(name));
        let entries = list(
            dataset,
            &one(dataset, &manifest, &format!("{MF}entries")).expect("mf:entries"),
        );
        assert_eq!(entries.len(), 3, "the file's three entries");
        for entry in entries {
            let action = one(dataset, &entry, &format!("{MF}action")).expect("mf:action");
            let root = one(dataset, &action, &format!("{SHT}nodeExpr")).expect("sht:nodeExpr");
            // No `sht:focusNode`: a blank node the graph never mentions, as the native
            // harness uses.
            let focus = Term::blank("absent-focus-node");
            let produced = evaluate(&FreeExpression {
                shapes: dataset,
                prefixes: &document.prefixes,
                root: &root,
                data: dataset,
                focus: &focus,
                scope: &[],
                imports: &purrdf_shapes::ShapesImports::new(),
            })
            .unwrap_or_else(|e| panic!("{entry}: evaluates on this target: {e}"))
            .outputs;
            let expected = list(
                dataset,
                &one(dataset, &entry, &format!("{MF}result")).expect("mf:result"),
            );
            assert_eq!(produced, expected, "{entry} on this target");
        }
    }

    /// A SPARQL 1.2 RL evaluation entry: the inference graph equals the results file.
    pub(super) fn srl_eval_filter_01_on_this_target() {
        let document = srl::parse_and_check(
            include_str!("../../../vectors/shacl12/tests/sparql-rl/eval/eval-filter-01.srl"),
            Some(&base("sparql-rl/eval/eval-filter-01.srl")),
        )
        .expect("the rule set parses");
        let data = parse_turtle_document(
            include_str!("../../../vectors/shacl12/tests/sparql-rl/eval/eval-filter-01-data.ttl"),
            Some(&base("sparql-rl/eval/eval-filter-01-data.ttl")),
        )
        .expect("the data parses");
        let inference = srl::infer(&document, &data.dataset, &InferOptions::default())
            .expect("the rule set runs on this target");
        let results = parse_turtle_document(
            include_str!(
                "../../../vectors/shacl12/tests/sparql-rl/eval/eval-filter-01-results.ttl"
            ),
            Some(&base("sparql-rl/eval/eval-filter-01-results.ttl")),
        )
        .expect("the results parse");
        let mut expected: Vec<String> = native_quads(
            results.dataset.as_ref(),
            None,
            None,
            None,
            GraphFilter::AnyGraph,
        )
        .into_iter()
        .map(|(s, p, o)| format!("{s} {p} {o} .\n"))
        .collect();
        expected.sort();
        assert!(!expected.is_empty(), "a non-vacuous expectation");
        assert_eq!(inference.inferred_ntriples(), expected.concat());
    }

    /// A shapes graph that merges all three vendored SHACL 1.2 vocabularies — every
    /// built-in function and component DECLARED, none with a body — loads on this
    /// target, and `sh:sparqlExpr` with `sh:prefixes` still evaluates natively: exactly
    /// the instance whose `ex:size` is not above 2 violates.
    pub(super) fn merged_vocabularies_load_and_validate_on_this_target() {
        let shapes_text = [
            include_str!("../spec/shacl.ttl"),
            include_str!("../spec/shnex.ttl"),
            include_str!("../spec/shnex-sparql.ttl"),
            r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] .
ex:Big a sh:NodeShape ;
  sh:targetClass ex:Item ;
  sh:expression [ sh:sparqlExpr "EXISTS { $this ex:size ?s FILTER(?s > 2) }" ;
                  sh:prefixes ex:Prefixes ] .
"#,
        ]
        .join("\n");
        let document = parse_turtle_document(&shapes_text, Some(&base("merged.ttl")))
            .expect("the merged vocabularies parse");
        let shapes =
            from_dataset_with_config_and_graph(&document.dataset, &document.prefixes, None, None)
                .unwrap_or_else(|e| panic!("the merged vocabularies load on this target: {e}"));
        let data = parse_turtle_document(
            "@prefix ex: <http://example.org/ns#> .\n\
             ex:small a ex:Item ; ex:size 1 .\nex:large a ex:Item ; ex:size 3 .\n",
            None,
        )
        .expect("data parses");
        let report = validate_dataset_with_shapes_graph(&data.dataset, &shapes, None)
            .expect("validation runs on this target");
        let focus: Vec<String> = report
            .results
            .iter()
            .map(|result| result.focus_node.to_string())
            .collect();
        assert_eq!(focus, vec!["<http://example.org/ns#small>".to_owned()]);
    }
}

use shacl12_subset::{
    closed_by_types_003_on_this_target, member_shape_001_on_this_target,
    merged_vocabularies_load_and_validate_on_this_target, node_expr_concat_on_this_target,
    srl_eval_filter_01_on_this_target, unique_values_for_001_on_this_target,
};

purrdf_testkit::harness_main!(
    encode_open_admit_validate_on_this_target,
    encoding_matches_the_committed_bytes_on_this_target,
    the_committed_golden_restores_on_this_target,
    member_shape_001_on_this_target,
    closed_by_types_003_on_this_target,
    unique_values_for_001_on_this_target,
    node_expr_concat_on_this_target,
    srl_eval_filter_01_on_this_target,
    merged_vocabularies_load_and_validate_on_this_target,
);
