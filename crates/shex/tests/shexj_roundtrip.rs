// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ShExJ round-trip conformance: for the ground-truth `schemas/*.json`
//! corpus, `parse_shexj(json) → to_shexj → parse_shexj` must be the identity
//! on the AST.
//!
//! Every corpus document that parses is round-tripped (not just a sample);
//! a named list of diverse must-pass documents guards against silent
//! coverage loss if the corpus or the xfail ledger drifts.

use std::fs;
use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use purrdf_shex::{parse_shexj, to_shexj};

/// Diverse documents that MUST parse and round-trip (one per feature
/// family): plain shapes, refs, EachOf/OneOf, node kinds, datatypes, all
/// facet families, value sets with every stem/range/exclusion flavor,
/// language tags, semantic actions, annotations, imports, start, EXTERNAL,
/// CLOSED/EXTRA, inverse, cardinalities, bnode labels, and the two
/// kitchen-sink schemas.
const MUST_ROUND_TRIP: &[&str] = &[
    "0",                                   // empty shape
    "1dot",                                // bare triple constraint
    "1dotIMPORT1dot",                      // imports
    "1dotExtra1",                          // EXTRA
    "1dotClosed",                          // CLOSED
    "1dotAnnot3",                          // annotations
    "1dotNoCode1",                         // no-code semantic action
    "1card2Star",                          // {m,*} cardinality
    "1cardOpt",                            // ? cardinality
    "1inversedot",                         // ^ inverse
    "1iriRef1",                            // node kind AND ref
    "1literalPatterni",                    // pattern + flags
    "1literalPattern_with_REGEXP_escapes", // regex escape corners
    "1decimalMininclusiveDECIMAL",         // fractional numeric facet
    "1integerMininclusiveDECIMALint",      // integral numeric facet
    "1val1IRIREF",                         // value set: IRI
    "1val1INTEGER",                        // value set: numeric literal
    "1val1language",                       // value set: Language
    "1val1emptylanguageStem",              // value set: empty LanguageStem
    "1val1dotMinusiri3",                   // value set: wildcard IriStemRange
    "1val1dotMinusliteral3",               // value set: wildcard LiteralStemRange
    "1val1dotMinuslanguage3",              // value set: wildcard LanguageStemRange
    "1NOTNOTdot",                          // nested ShapeNot
    "1Include1",                           // tripleExpr label + inclusion
    "3circRefPlus1",                       // shape references
    "shapeExtern",                         // ShapeExternal
    "startCode1startRef",                  // start + startActs
    "kitchenSink",                         // most features at once
    "_all",                                // every feature at once
];

/// Corpus documents excluded from the round-trip (must match the ShExJ xfail
/// ledger in `syntax_conformance.rs`), each with a reason.
const XFAIL_ROUND_TRIP: &[(&str, &str)] = &[];

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/shexTest/schemas")
}

fn round_trip(name: &str, source: &str) -> Result<(), String> {
    let first = parse_shexj(source).map_err(|e| format!("{name}: initial parse: {e}"))?;
    let serialized = to_shexj(&first);
    let second =
        parse_shexj(&serialized).map_err(|e| format!("{name}: reparse of to_shexj output: {e}"))?;
    if first == second {
        Ok(())
    } else {
        Err(format!("{name}: AST changed across round-trip"))
    }
}

#[test]
fn ground_truth_corpus_round_trips() {
    let dir = corpus();
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("read schemas dir")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let is_json = path.extension().is_some_and(|x| x == "json");
            // Ground truth = .json with a .shex sibling (excludes the two
            // non-schema JSON files in the corpus).
            (is_json && path.with_extension("shex").exists())
                .then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    names.sort();

    let mut failures = Vec::new();
    let mut stale_xfails = Vec::new();
    let mut passed = 0usize;
    for name in &names {
        let source = fs::read_to_string(dir.join(format!("{name}.json"))).expect("read .json");
        let xfail = XFAIL_ROUND_TRIP.iter().any(|(n, _)| n == name);
        match round_trip(name, &source) {
            Ok(()) if xfail => stale_xfails.push(name.clone()),
            Ok(()) => passed += 1,
            Err(_) if xfail => {}
            Err(e) => failures.push(e),
        }
    }
    assert!(
        failures.is_empty(),
        "{} round-trip failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        stale_xfails.is_empty(),
        "XFAIL_ROUND_TRIP entries now pass (remove them): {stale_xfails:?}"
    );
    assert_eq!(
        passed + XFAIL_ROUND_TRIP.len(),
        names.len(),
        "round-trip coverage drifted"
    );
}

#[test]
fn diverse_documents_round_trip() {
    assert!(
        MUST_ROUND_TRIP.len() >= 20,
        "the diverse must-pass list must stay at 20+ documents"
    );
    let dir = corpus();
    for name in MUST_ROUND_TRIP {
        let path = dir.join(format!("{name}.json"));
        let source =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let first = parse_shexj(&source).unwrap_or_else(|e| panic!("{name}: parse: {e}"));
        let second =
            parse_shexj(&to_shexj(&first)).unwrap_or_else(|e| panic!("{name}: reparse: {e}"));
        assert_eq!(first, second, "{name}: AST changed across round-trip");
    }
}
