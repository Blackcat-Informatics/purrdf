// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The whole prepared-product lifecycle, EXECUTED on
//! `wasm32-unknown-unknown`, against the same bytes the host writes.**
//!
//! `make wasm` proves the crate builds for wasm32. It cannot prove the codec
//! *answers* the same way there, and for a cache format that is the claim that
//! matters: a product is prepared by a build tool on a host and restored months
//! later by a browser. If those two disagree by one byte, the browser's `open`
//! refuses a product that is perfectly valid, or — far worse — the browser writes
//! products the host cannot read, and the divergence surfaces as a cache that
//! mysteriously never hits.
//!
//! Two runs on one target cannot tell a codec that is target-independent from one
//! that merely agrees with whichever target it was last compiled for. So this file
//! asserts against a byte string that was produced *elsewhere*: the golden
//! committed under `tests/fixtures/`, written by a native build.
//!
//! # How it runs on both
//!
//! One test body, two attributes. Natively these are ordinary `#[test]`s picked up
//! by `cargo test -p purrdf-shapes`; on `wasm32-unknown-unknown` they are
//! `#[wasm_bindgen_test]`s compiled to wasm and executed in Node by
//! `make wasm-test`:
//!
//! ```text
//! cargo test -p purrdf-shapes --target wasm32-unknown-unknown --test product_wasm
//! ```
//!
//! Both runs assert the *same* expectations, so the native run is not a weaker
//! version of the wasm one — it is the other half of the comparison.
//!
//! # The W3C SHACL 1.2 subset
//!
//! The second half of this file runs a vendored W3C SHACL 1.2 subset the same
//! way — the list components with `sh:detail`, `sh:closed sh:ByTypes`,
//! `sh:uniqueValuesFor`, a node-expression entry evaluated through the standalone
//! evaluator every host reaches, a SPARQL 1.2 RL evaluation, and a shapes graph
//! that merges all three vendored SHACL 1.2 vocabularies — each graded against the
//! expectation the W3C file itself states, on whichever target runs it. A
//! behaviour that holds natively but not in a browser fails here rather than in a
//! user's page.
//!
//! # No filesystem, on purpose
//!
//! Nothing here opens a file. The shapes graph, the data graph and the golden
//! product are all compile-time constants (`include_bytes!` resolves before the
//! module ever reaches a target), because a wasm32 test that needed a filesystem
//! would be proving something about the runner's shims rather than about the
//! codec.

mod product_fixture;

use purrdf_shapes::product::{ShapesProduct, ShapesProfile};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// The bytes this target writes for the fixture are the bytes the host wrote.
///
/// The golden was produced by a native build and committed; the wasm run encodes
/// the same shapes graph from source and compares. A target whose pointer width,
/// endianness or hash seeding reached the writer renders a different product here
/// rather than shipping a cache two engines disagree about.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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

    use purrdf::RdfDataset;
    use purrdf_shapes::data::{GraphFilter, native_quads};
    use purrdf_shapes::engine::validate_dataset_with_shapes_graph;
    use purrdf_shapes::free_expression::{FreeExpression, evaluate};
    use purrdf_shapes::shapes::from_dataset_with_config_and_graph;
    use purrdf_shapes::srl::{self, InferOptions};
    use purrdf_shapes::term::{NamedNode, Term};
    use purrdf_shapes::text_ingest::parse_turtle_document;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    const SH: &str = "http://www.w3.org/ns/shacl#";
    const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
    const SHT: &str = "http://www.w3.org/ns/shacl-test#";

    /// The base every vendored file parses under: the files name themselves with `<>`
    /// and their entries relatively, and no filesystem path exists on wasm32.
    fn base(name: &str) -> String {
        format!("http://example.org/shacl12/{name}")
    }

    fn iri(value: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(value))
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
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn member_shape_001_on_this_target() {
        grade_validate(
            "core/node/memberShape-001.ttl",
            include_str!("../../../vectors/shacl12/tests/core/node/memberShape-001.ttl"),
            "core/node/memberShape-001",
        );
    }

    /// SHACL 1.2 Core `sh:closed sh:ByTypes`.
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn closed_by_types_003_on_this_target() {
        grade_validate(
            "core/node/closed-003.ttl",
            include_str!("../../../vectors/shacl12/tests/core/node/closed-003.ttl"),
            "core/node/closed-003",
        );
    }

    /// SHACL 1.2 Core `sh:uniqueValuesFor`.
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unique_values_for_001_on_this_target() {
        grade_validate(
            "core/node/uniqueValuesFor-001.ttl",
            include_str!("../../../vectors/shacl12/tests/core/node/uniqueValuesFor-001.ttl"),
            "core/node/uniqueValuesFor-001",
        );
    }

    /// Every `sht:EvalNodeExpr` entry of `shnex/concat.ttl`, through the standalone
    /// evaluator every host reaches, graded term for term and in order against its
    /// `mf:result` list.
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn node_expr_concat_on_this_target() {
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
            })
            .unwrap_or_else(|e| panic!("{entry}: evaluates on this target: {e}"));
            let expected = list(
                dataset,
                &one(dataset, &entry, &format!("{MF}result")).expect("mf:result"),
            );
            assert_eq!(produced, expected, "{entry} on this target");
        }
    }

    /// A SPARQL 1.2 RL evaluation entry: the inference graph equals the results file.
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn srl_eval_filter_01_on_this_target() {
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
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn merged_vocabularies_load_and_validate_on_this_target() {
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
