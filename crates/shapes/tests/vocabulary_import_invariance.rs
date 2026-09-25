// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **Merging the W3C SHACL 1.2 vocabulary into a shapes graph changes nothing.**
//!
//! A shapes graph assembled by resolving `owl:imports` into one stand-alone graph
//! carries the W3C vocabularies — `shacl.ttl`, `shnex.ttl`, `shnex-sparql.ttl` —
//! whole: every built-in component and function DECLARED, with parameters, and
//! none of them implemented in RDF. The spec symbol table binds those
//! declarations to the engine's own implementations, so the merge must be a
//! no-op. This harness proves it over every shapes graph three corpora contain —
//! the first-party corpus, the W3C SHACL 1.0 suite and the `sht:Validate` entries
//! of the W3C SHACL 1.2 suite:
//!
//! * the validation report is byte-identical, as N-Triples, with and without the
//!   merge (and a shapes graph that failed to load fails identically);
//! * the merged vocabulary adds nothing to the custom-function index, claims no
//!   key parameter and registers no custom component — the linker's inspection
//!   surface is compared before and after;
//! * its `sh:Parameter` nodes stay inert: the merge adds no shape;
//! * a prepared product of the MERGED shapes graph, restored, answers exactly what
//!   the fresh parse does.
//!
//! The vocabulary is unioned into the SHAPES graph only, as its default graph and
//! never as a named graph. The DATA graph is untouched, even where a test uses one
//! file for both. The vocabulary text is appended AFTER the shapes text, so the
//! shapes' own blank nodes keep their labels and a byte comparison stays exact;
//! the document prefix map a SPARQL body falls back on is the original
//! document's, because a graph-level merge carries no prefix declarations.
//!
//! [`INVARIANCE_EXCEPTIONS`] ledgers any case whose answer legitimately depends on
//! the vocabulary being present — a SPARQL constraint that reads `$shapesGraph` —
//! with its reason; it is pinned by count and runs both ways.

mod shacl_corpora;

use std::fs;
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{PreparedShapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::shapes::{__linked_declarations, Shapes, from_dataset_with_config_and_graph};
use purrdf_shapes::text_ingest::{
    extract_prefixes, parse_ntriples_to_dataset, parse_turtle_to_dataset,
};

use shacl_corpora::shacl12::{Body, shacl12_cases};
use shacl_corpora::{
    file_iri, first_party_box_role_vocab, first_party_cases, parse_turtle_file, w3c_cases,
};

/// The three vocabulary files, as vendored.
const VOCABULARY: [&str; 3] = [
    include_str!("../spec/shacl.ttl"),
    include_str!("../spec/shnex.ttl"),
    include_str!("../spec/shnex-sparql.ttl"),
];

/// The built-in list-parameter functions the vocabulary declares:
/// `shnex:conformsToShape` and 77 `sparql:` functions.
const VOCABULARY_LIST_FUNCTIONS: usize = 78;

/// Cases whose answer legitimately changes when the vocabulary is merged, as
/// `(case id, reason)`. Empty: no case in the three corpora reads the shapes graph
/// in a way the vocabulary's declarations can reach.
const INVARIANCE_EXCEPTIONS: &[(&str, &str)] = &[];

/// Every case the three corpora contribute (129 + 72 + 174).
const TOTAL_CASES: usize = 375;

/// The cases whose shapes graph loads and whose two reports — and restored
/// product — were compared, rather than two identical load errors: every case of
/// the three corpora except the 12 declared `sht:Failure` inputs and the 7 SHACL
/// 1.2 entries this engine refuses at load (the same partition the product
/// equivalence harness pins).
///
/// Moved from 335 to 338 when `sh:singleLine`, `sh:rootClass` and `sh:someValue`
/// became evaluated: `singleLine-001`, `rootClass-001` and `someValue-001` now
/// load, so their reports are compared too, and the refused-at-load entries went
/// from 28 to 25.
///
/// Moved from 338 to 344 when the property-pair components took any SHACL
/// property path and `sh:subsetOf` became evaluated: `equals-002`,
/// `disjoint-002`, `lessThan-003`, `lessThanOrEquals-002`, `subsetOf-001` and
/// `subsetOf-002` now load, so their reports are compared too, and the
/// refused-at-load entries went from 25 to 19.
///
/// Moved from 344 to 349 when `sh:uniqueValuesFor` became evaluated:
/// `uniqueValuesFor-001` to `-005` now load, so their reports are compared too,
/// and the refused-at-load entries went from 19 to 14.
///
/// Moved from 349 to 351 when `sh:closed sh:ByTypes` became evaluated:
/// `closed-003` and `closed-004` now load, so their reports are compared too,
/// and the refused-at-load entries went from 14 to 12.
///
/// Moved from 351 to 356 when per-constraint reifier annotations and the
/// `sh:Debug` / `sh:Trace` severities became evaluated: `deactivated-003`,
/// `severity-003`, `severity-004`, `severity-005` and `message-002` now load, so
/// their reports are compared too, and the refused-at-load entries went from 12
/// to 7.
const COMPARED_ON_REPORT: usize = 356;

/// One case, reduced to what both parses need.
struct Input {
    id: String,
    shapes_text: String,
    base: Option<String>,
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
    /// The data graph, parsed once and shared by both runs.
    data: Result<Arc<RdfDataset>, String>,
}

/// The shapes graph of `input`, parsed from `text` — the original text, or the
/// original text with the vocabulary appended.
fn parse(input: &Input, text: &str) -> Result<(Arc<RdfDataset>, Shapes), String> {
    let dataset = parse_turtle_to_dataset(text, input.base.as_deref())
        .map_err(|errors| format!("shapes graph parse error: {}", errors.join("; ")))?;
    let prefixes = extract_prefixes(&input.shapes_text);
    let shapes = from_dataset_with_config_and_graph(
        &dataset,
        &prefixes,
        input.box_role_vocab.clone(),
        input.shapes_graph.clone(),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;
    Ok((dataset, shapes))
}

/// The N-Triples report `shapes` produces over the case's data, or the error.
fn report(input: &Input, shapes: &Shapes) -> Result<String, String> {
    let data = input.data.as_ref().map_err(Clone::clone)?;
    validate_dataset_with_shapes_graph(data, shapes, None).map(|report| report.to_ntriples())
}

/// Every case of the three corpora.
fn inputs() -> Vec<Input> {
    let mut out = Vec::new();
    for case in first_party_cases() {
        let shapes_text = fs::read_to_string(&case.shapes_path).expect("shapes.ttl reads");
        let data_nt = fs::read_to_string(&case.data_path).expect("data.nt reads");
        out.push(Input {
            id: format!("corpus/{}", case.name),
            shapes_text,
            base: None,
            box_role_vocab: Some(first_party_box_role_vocab()),
            shapes_graph: None,
            data: parse_ntriples_to_dataset(&data_nt).map_err(|errors| errors.join("; ")),
        });
    }
    let w3c = w3c_cases()
        .into_iter()
        .map(|case| (format!("w3c/{}", case.id), case));
    let w3c12 = shacl12_cases()
        .into_iter()
        .filter_map(|case| match case.body {
            Body::Validate(validate) => Some((format!("w3c12/{}", validate.id), validate)),
            _ => None,
        });
    for (id, case) in w3c.chain(w3c12) {
        let shapes_text = fs::read_to_string(&case.shapes_path).expect("shapes file reads");
        // The data graph is parsed from its own file even when that file is the
        // shapes file: the merge never reaches it.
        let data = parse_turtle_file(&case.data_path);
        out.push(Input {
            id,
            shapes_text,
            base: Some(file_iri(&case.shapes_path)),
            box_role_vocab: None,
            shapes_graph: case.shapes_graph_iri.clone(),
            data,
        });
    }
    out
}

/// What went wrong with one case, if anything; `Ok(true)` when the two parses
/// were compared on an actual REPORT (and a restored product), not on an
/// identical load error.
fn check(input: &Input, vocabulary: &str) -> Result<bool, String> {
    let merged_text = format!("{}\n{vocabulary}", input.shapes_text);
    let original = parse(input, &input.shapes_text);
    let merged = parse(input, &merged_text);

    // The linker's view of the declarations, before and after.
    let linked = |result: &Result<(Arc<RdfDataset>, Shapes), String>, text: &str| {
        let dataset = match result {
            Ok((dataset, _)) => Arc::clone(dataset),
            Err(_) => parse_turtle_to_dataset(text, input.base.as_deref())
                .map_err(|errors| errors.join("; "))?,
        };
        __linked_declarations(&dataset)
    };
    let before = linked(&original, &input.shapes_text);
    let after = linked(&merged, &merged_text);
    match (&before, &after) {
        (Ok(before), Ok(after)) => {
            if before.custom_functions != after.custom_functions
                || before.custom_key_parameters != after.custom_key_parameters
                || before.registered_components != after.registered_components
            {
                return Err(format!(
                    "the vocabulary changed the custom index or the component registry:\n  \
                     before {before:?}\n  after  {after:?}"
                ));
            }
            if after.native_list_functions.len()
                < before
                    .native_list_functions
                    .len()
                    .max(VOCABULARY_LIST_FUNCTIONS)
            {
                return Err(format!(
                    "the vocabulary's built-in list functions did not all bind: {:?}",
                    after.native_list_functions
                ));
            }
        }
        (Err(before), Err(after)) if before == after => {}
        (before, after) => {
            return Err(format!(
                "the linker answered differently:\n  before {before:?}\n  after  {after:?}"
            ));
        }
    }

    let (original_report, merged_report) = match (&original, &merged) {
        (Ok((_, original)), Ok((_, merged))) => {
            if original.node_shapes.len() != merged.node_shapes.len() {
                return Err(format!(
                    "the vocabulary added shapes: {} before, {} after",
                    original.node_shapes.len(),
                    merged.node_shapes.len()
                ));
            }
            (report(input, original), report(input, merged))
        }
        (Err(before), Err(after)) => (Err(before.clone()), Err(after.clone())),
        (Ok(_), Err(after)) => {
            return Err(format!(
                "the vocabulary made the shapes graph unloadable: {after}"
            ));
        }
        (Err(before), Ok(_)) => {
            return Err(format!(
                "the vocabulary made an unloadable shapes graph load: {before}"
            ));
        }
    };
    if original_report != merged_report {
        return Err(format!(
            "the report changed:\n  without the vocabulary: {}\n  with it:                {}",
            summary(&original_report),
            summary(&merged_report)
        ));
    }

    // A product of the MERGED graph restores to the fresh parse's answer.
    let compared_on_report = merged_report.is_ok();
    if let Ok((_, merged_shapes)) = merged {
        let merged_shapes = Arc::new(merged_shapes);
        let bytes = PreparedShapes::new(Arc::clone(&merged_shapes))
            .to_product(&ShapesProfile::CORE)
            .map_err(|e| format!("the merged shapes graph does not pack: {e}"))?;
        let host = HostBindings::empty();
        for lane in ["admit", "rebuild"] {
            let view = ShapesProduct::open(&bytes).map_err(|e| format!("{lane}: open: {e}"))?;
            let restored = if lane == "admit" {
                view.admit(&ShapesProfile::CORE, &host)
            } else {
                view.rebuild(&ShapesProfile::CORE, &host)
            }
            .map_err(|e| format!("{lane}: {e}"))?;
            let restored_report = report(input, restored.shapes());
            if restored_report != merged_report {
                return Err(format!(
                    "the {lane} lane disagrees with the fresh parse of the merged graph:\n  \
                     fresh: {}\n  {lane}: {}",
                    summary(&merged_report),
                    summary(&restored_report)
                ));
            }
        }
    }
    Ok(compared_on_report)
}

fn summary(report: &Result<String, String>) -> String {
    match report {
        Ok(nt) => format!("report of {} lines", nt.lines().count()),
        Err(e) => format!("error: {e}"),
    }
}

#[test]
fn merging_the_vocabulary_changes_no_answer() {
    // The corpus relation, installed exactly as the other corpus harnesses install
    // it: one first-party case reaches it.
    let (corpus_relations, _opens) = shacl_corpora::corpus_relations();
    let _relations = purrdf_shapes::sparql::enter_property_function_scope(corpus_relations);

    let vocabulary = VOCABULARY.join("\n");
    let inputs = inputs();
    assert_eq!(inputs.len(), TOTAL_CASES, "discovered case count");

    let mut failures: Vec<String> = Vec::new();
    let mut excepted = 0usize;
    let mut compared_on_report = 0usize;
    for input in &inputs {
        let outcome = check(input, &vocabulary);
        let exception = INVARIANCE_EXCEPTIONS.iter().find(|(id, _)| *id == input.id);
        match (outcome, exception) {
            (Ok(on_report), None) => compared_on_report += usize::from(on_report),
            (Err(_), Some(_)) => excepted += 1,
            (Ok(_), Some((_, reason))) => failures.push(format!(
                "[{}] is ledgered in INVARIANCE_EXCEPTIONS ({reason}) but is invariant; remove it",
                input.id
            )),
            (Err(why), None) => failures.push(format!("[{}] {why}", input.id)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} case(s) are not invariant under the vocabulary merge:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    assert_eq!(
        excepted,
        INVARIANCE_EXCEPTIONS.len(),
        "the exception ledger is exact"
    );
    println!(
        "VOCABULARY-INVARIANCE: {compared_on_report} compared on a report of {}",
        inputs.len()
    );
    assert_eq!(
        compared_on_report, COMPARED_ON_REPORT,
        "the number of cases compared on an actual report (rather than an identical load \
         error) moved; a harness comparing only errors would be vacuous"
    );
}

/// The vocabulary ALONE, as a shapes graph: no shape, nothing indexed, nothing
/// registered — and every built-in list-parameter function bound natively.
#[test]
fn the_vocabulary_alone_indexes_and_registers_nothing() {
    let dataset =
        parse_turtle_to_dataset(&VOCABULARY.join("\n"), None).expect("the vocabulary parses");
    let linked = __linked_declarations(&dataset).expect("the vocabulary links");
    assert_eq!(linked.custom_functions, Vec::<String>::new());
    assert_eq!(linked.custom_key_parameters, Vec::<(String, String)>::new());
    assert_eq!(linked.registered_components, Vec::<String>::new());
    assert_eq!(
        linked.native_list_functions.len(),
        VOCABULARY_LIST_FUNCTIONS
    );
    let shapes = purrdf_shapes::shapes::from_dataset(&dataset).expect("the vocabulary loads");
    assert!(
        shapes.node_shapes.is_empty(),
        "the vocabulary's sh:Parameter nodes are not shapes"
    );
}
