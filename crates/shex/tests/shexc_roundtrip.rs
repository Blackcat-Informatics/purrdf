// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ShExC round-trip conformance: for every `schemas/*.shex` document that
//! parses, `parse_shexc(shex) → to_shexc → parse_shexc` must be the identity
//! on the AST.
//!
//! This is the inverse of `tests/shexj_roundtrip.rs` (which pins the JSON wire
//! format). Because the serializer emits absolute `<…>` IRIs and declares no
//! prefixes, the re-parse takes `base = None` and no `@prefix`/`BASE` context.
//!
//! A named list of diverse must-pass documents guards against silent coverage
//! loss, and the whole corpus is round-tripped (not just a sample); the
//! XFAIL ledger is empty and any entry must actually fail.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use purrdf_shex::{parse_shexc, to_shexc};

/// Diverse documents that MUST parse and round-trip through ShExC (one per
/// feature family): plain shapes, refs, EachOf/OneOf, node kinds, datatypes,
/// every facet family, value sets with each stem/range/exclusion flavor,
/// language tags, semantic actions, annotations, imports, start, EXTERNAL,
/// CLOSED/EXTRA, inverse, cardinalities, bnode labels, and the kitchen sinks.
const MUST_ROUND_TRIP: &[&str] = &[
    "0",                                   // empty shape
    "1dot",                                // bare triple constraint
    "1dotIMPORT1dot",                      // imports
    "1dotExtra1",                          // EXTRA
    "1dotClosed",                          // CLOSED
    "1dotAnnot3",                          // annotations
    "1dotNoCode1",                         // no-code semantic action
    "1dotCode1",                           // semantic action with code
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

/// Corpus documents excluded from the round-trip, each with a reason. Empty:
/// every schema that parses round-trips exactly.
const XFAIL_ROUND_TRIP: &[(&str, &str)] = &[];

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/shexTest/schemas")
}

fn shex_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().is_some_and(|x| x == "shex")).then_some(path)
        })
        .collect();
    files.sort();
    files
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned()
}

/// `parse → to_shexc → parse` must be the identity on every parseable schema.
#[test]
fn schemas_shexc_round_trip() {
    let dir = corpus();
    let files = shex_files(&dir);
    let xfail: BTreeSet<&str> = XFAIL_ROUND_TRIP.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        xfail.len(),
        XFAIL_ROUND_TRIP.len(),
        "duplicate entries in XFAIL_ROUND_TRIP"
    );

    let mut mismatches = Vec::new();
    let mut stale_xfails = Vec::new();
    let mut round_tripped = 0usize;
    for path in &files {
        let name = stem(path);
        let source = fs::read_to_string(path).expect("read .shex");
        // Parse failures are owned by `syntax_conformance.rs`; skip here.
        let Ok(original) = parse_shexc(&source, None) else {
            continue;
        };
        let reparsed = parse_shexc(&to_shexc(&original), None);
        let matches = reparsed.as_ref() == Ok(&original);
        if xfail.contains(name.as_str()) {
            if matches {
                stale_xfails.push(name);
            }
        } else if matches {
            round_tripped += 1;
        } else {
            mismatches.push(name);
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} schemas did not round-trip through ShExC: {mismatches:?}",
        mismatches.len()
    );
    assert!(
        stale_xfails.is_empty(),
        "XFAIL_ROUND_TRIP entries now pass (remove them): {stale_xfails:?}"
    );
    // Sanity floor: the corpus is 425 schemas, effectively all parse.
    assert!(
        round_tripped >= 400,
        "unexpectedly few schemas round-tripped: {round_tripped}"
    );
}

/// The diverse must-pass set round-trips (guards against silent coverage loss).
#[test]
fn diverse_documents_round_trip() {
    let dir = corpus();
    let mut missing = Vec::new();
    let mut failed = Vec::new();
    for name in MUST_ROUND_TRIP {
        let path = dir.join(format!("{name}.shex"));
        let Ok(source) = fs::read_to_string(&path) else {
            missing.push((*name).to_owned());
            continue;
        };
        let original = parse_shexc(&source, None)
            .unwrap_or_else(|e| panic!("must-pass {name} failed to parse: {e}"));
        let serialized = to_shexc(&original);
        match parse_shexc(&serialized, None) {
            Ok(reparsed) if reparsed == original => {}
            Ok(_) => failed.push((*name).to_owned()),
            Err(e) => failed.push(format!("{name}: re-parse error: {e}")),
        }
    }
    assert!(
        missing.is_empty(),
        "must-pass corpus files missing: {missing:?}"
    );
    assert!(
        failed.is_empty(),
        "must-pass documents did not round-trip: {failed:?}"
    );
}
