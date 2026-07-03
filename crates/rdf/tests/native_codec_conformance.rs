// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C RDF 1.2 syntax-suite **round-trip** conformance gate for the native
//! `purrdf` text codecs (acceptance: "W3C syntax test suites round-trip").
//!
//! This harness is deliberately **oxigraph-free**: a green run proves the native Turtle /
//! TriG / N-Triples / N-Quads / RDF-XML codecs parse and round-trip the official W3C
//! suites with no Store dependency ( end-state).
//!
//! ## What it does (per the W3C RDF test semantics)
//!
//! For every test enumerated from the vendored `manifest.ttl` files (which are
//! themselves parsed with the native Turtle parser — we dogfood `parse_dataset`):
//!
//! - **Positive syntax / Eval:** `parse_dataset(action) -> ds1`, then
//!   `serialize_dataset(ds1) -> text2`, then `parse_dataset(text2) -> ds2`, and assert
//!   `datasets_isomorphic(ds1, ds2)` (the round-trip is lossless). For eval tests with
//!   an `mf:result`, also assert the action dataset is isomorphic to the parsed result.
//! - **Negative syntax:** `parse_dataset(action)` MUST return `Err` (the codec rejects
//!   malformed input; no round-trip is attempted).
//!
//! ## Known gaps (no silent skips)
//!
//! [`KNOWN_GAPS`] is an explicit allowlist keyed by test name. A case on the allowlist
//! that fails *as expected* is tolerated and reported as an allowlisted gap; a case on
//! the allowlist that *unexpectedly passes* is reported as a STALE entry (so the
//! allowlist can be pruned). Every other failure PANICS the test with a full report.
//! The harness always prints a per-format and overall summary — nothing is skipped
//! silently.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf_rdf::{
    datasets_isomorphic, parse_dataset, serialize_dataset, RdfDataset, SerializeGraph, TermRef,
    TermValue,
};

/// Allowlist of W3C tests the native `purrdf-gts` codecs do not handle the same way the
/// strict W3C manifest expects, keyed by `mf:name` → reason. A failing allowlisted case
/// is tolerated; a passing allowlisted case is flagged STALE (prune it).
///
/// As of purrdf-gts 0.9.7 the native codecs pass the **entire** W3C RDF 1.2 syntax
/// suite — Turtle, TriG, N-Triples, N-Quads, and RDF/XML — with NO gaps:
///
/// - The former **G2** serializer gap (a triple term in quad-object position tripping a
///   self-reifier "cycle while declaring term N") is fixed: the self-referential
///   triple-term entry is no longer rendered as a reifier statement.
/// - The former **G3** lenient-lexical trade-off is gone: the native language-tag
///   validator accepts PurRDF's long private-use subtags (`x-purrdf-norwegiannynorsk`)
///   while still REJECTING the genuinely-malformed W3C negative cases — strictly better
///   than the old oxttl `.lenient()` path.
///
/// Keep this empty; add an entry only with a documented, approved codec-level reason.
const KNOWN_GAPS: &[(&str, &str)] = &[];

/// One enumerated W3C test case.
#[derive(Debug, Clone)]
struct Case {
    /// `mf:name` (the allowlist key and human label).
    name: String,
    kind: Kind,
    /// Absolute path to the `mf:action` file on disk.
    action: PathBuf,
    /// Base IRI for the action file (`assumedTestBase` + filename).
    action_base: String,
    /// Media type the action file is parsed/serialized as.
    media_type: &'static str,
    /// Absolute path to the `mf:result` file (eval tests only).
    result: Option<PathBuf>,
    /// Base IRI + media type for the result file (eval tests only).
    result_base: Option<String>,
    result_media_type: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    PositiveSyntax,
    NegativeSyntax,
    Eval,
}

/// A vendored format suite: directory under `tests/corpus/w3c/`, its action media type,
/// and the list of `(submanifest, kind-of-eval-result-mediatype)` sub-suites.
struct Suite {
    /// Display name and corpus subdir (e.g. `"turtle"`).
    dir: &'static str,
    /// Media type for action files in this suite.
    media_type: &'static str,
    /// Sub-manifests to read (`syntax`, `eval`); RDF-XML has only `eval`.
    submanifests: &'static [&'static str],
}

const SUITES: &[Suite] = &[
    Suite {
        dir: "turtle",
        media_type: "text/turtle",
        submanifests: &["syntax", "eval"],
    },
    Suite {
        dir: "trig",
        media_type: "application/trig",
        submanifests: &["syntax", "eval"],
    },
    Suite {
        dir: "ntriples",
        media_type: "application/n-triples",
        submanifests: &["syntax"],
    },
    Suite {
        dir: "nquads",
        media_type: "application/n-quads",
        submanifests: &["syntax"],
    },
    Suite {
        dir: "rdfxml",
        media_type: "application/rdf+xml",
        submanifests: &["eval"],
    },
];

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
const RDFT: &str = "http://www.w3.org/ns/rdftest#";

/// Corpus root, relative to the crate dir (where `cargo test` runs).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/w3c")
}

/// Map a result-file extension to the media type its OWN parser uses (eval results are
/// N-Triples for triple formats, N-Quads for quad formats).
fn media_type_for_ext(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("nt") => "application/n-triples",
        Some("nq") => "application/n-quads",
        Some("ttl") => "text/turtle",
        Some("trig") => "application/trig",
        Some("rdf") => "application/rdf+xml",
        other => panic!("unknown result extension {other:?} for {}", path.display()),
    }
}

/// Read a manifest's `mf:assumedTestBase` IRI.
fn assumed_test_base(ds: &RdfDataset) -> String {
    for q in ds.quad_refs() {
        if let TermRef::Iri(p) = q.p {
            if p == format!("{MF}assumedTestBase") {
                if let TermRef::Iri(base) = q.o {
                    return base.to_owned();
                }
            }
        }
    }
    panic!("manifest has no mf:assumedTestBase");
}

/// Enumerate cases from one sub-manifest (parsed with the native Turtle codec).
fn enumerate(suite: &Suite, submanifest: &str) -> Vec<Case> {
    let dir = corpus_root().join(suite.dir).join(submanifest);
    let manifest_path = dir.join("manifest.ttl");
    let bytes = std::fs::read(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    // Dogfood the native Turtle parser on the manifest itself.
    let manifest_iri = format!("file://{}", manifest_path.display());
    let ds = parse_dataset(&bytes, "text/turtle", Some(&manifest_iri))
        .unwrap_or_else(|e| panic!("parse manifest {}: {e:?}", manifest_path.display()));

    let base = assumed_test_base(&ds);

    // Group quads by subject IRI, collecting type/name/action/result.
    #[derive(Default)]
    struct Row {
        types: Vec<String>,
        name: Option<String>,
        action: Option<String>,
        result: Option<String>,
    }
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for q in ds.quad_refs() {
        let TermRef::Iri(subj) = q.s else { continue };
        let TermRef::Iri(pred) = q.p else { continue };
        let row = rows.entry(subj.to_owned()).or_default();
        if pred == RDF_TYPE {
            if let TermRef::Iri(t) = q.o {
                row.types.push(t.to_owned());
            }
        } else if pred == format!("{MF}name") {
            if let TermRef::Literal { lexical, .. } = q.o {
                row.name = Some(lexical.to_owned());
            }
        } else if pred == format!("{MF}action") {
            if let TermRef::Iri(a) = q.o {
                row.action = Some(a.to_owned());
            }
        } else if pred == format!("{MF}result") {
            if let TermRef::Iri(r) = q.o {
                row.result = Some(r.to_owned());
            }
        }
    }

    let mut cases = Vec::new();
    for (subj, row) in rows {
        // Determine the test kind from the rdft:* type (suffix match so this works
        // across all five format-specific type names).
        let kind = row.types.iter().find_map(|t| {
            let local = t.strip_prefix(RDFT)?;
            if local.ends_with("PositiveSyntax") {
                Some(Kind::PositiveSyntax)
            } else if local.ends_with("NegativeSyntax") {
                Some(Kind::NegativeSyntax)
            } else if local.ends_with("Eval") {
                Some(Kind::Eval)
            } else {
                None
            }
        });
        let Some(kind) = kind else { continue };
        let action_iri = row
            .action
            .unwrap_or_else(|| panic!("{subj} has no mf:action"));
        // The action IRI is resolved against the manifest's @base (the file:// IRI),
        // so it is already absolute. Recover the bare filename to locate it on disk
        // and to build the spec base IRI (assumedTestBase + filename).
        let action_file = action_iri.rsplit('/').next().unwrap().to_owned();
        let action_path = dir.join(&action_file);
        let action_base = format!("{base}{action_file}");

        let name = row.name.unwrap_or_else(|| subj.clone());

        let (result_path, result_base, result_mt) = match (kind, row.result) {
            (Kind::Eval, Some(result_iri)) => {
                let result_file = result_iri.rsplit('/').next().unwrap().to_owned();
                let result_path = dir.join(&result_file);
                let result_mt = media_type_for_ext(&result_path);
                let result_base = format!("{base}{result_file}");
                (Some(result_path), Some(result_base), Some(result_mt))
            }
            _ => (None, None, None),
        };

        cases.push(Case {
            name,
            kind,
            action: action_path,
            action_base,
            media_type: suite.media_type,
            result: result_path,
            result_base,
            result_media_type: result_mt,
        });
    }
    cases
}

/// Round-trip a parsed dataset: serialize → re-parse, assert isomorphic.
fn round_trip(ds: &RdfDataset, media_type: &str, base: &str) -> Result<bool, String> {
    let text = serialize_dataset(ds, media_type, SerializeGraph::Dataset)
        .map_err(|e| format!("serialize: {e:?}"))?;
    let reparsed =
        parse_dataset(&text, media_type, Some(base)).map_err(|e| format!("re-parse: {e:?}"))?;
    Ok(datasets_isomorphic(ds, &reparsed))
}

/// Outcome of running one case.
enum Outcome {
    /// Behaved exactly as the W3C semantics require.
    Pass,
    /// Did not; carries a human reason (becomes either an allowlisted gap or a hard
    /// failure depending on whether the name is in [`KNOWN_GAPS`]).
    Fail(String),
}

fn run_case(case: &Case) -> Outcome {
    match case.kind {
        Kind::NegativeSyntax => {
            let bytes = std::fs::read(&case.action).expect("read action");
            match parse_dataset(&bytes, case.media_type, Some(&case.action_base)) {
                Err(_) => Outcome::Pass,
                Ok(_) => Outcome::Fail("negative-syntax test parsed without error".to_owned()),
            }
        }
        Kind::PositiveSyntax | Kind::Eval => {
            let bytes = std::fs::read(&case.action).expect("read action");
            let ds = match parse_dataset(&bytes, case.media_type, Some(&case.action_base)) {
                Ok(ds) => ds,
                Err(e) => return Outcome::Fail(format!("parse action: {e:?}")),
            };
            // Round-trip the action dataset.
            match round_trip(&ds, case.media_type, &case.action_base) {
                Ok(true) => {}
                Ok(false) => return Outcome::Fail("action round-trip not isomorphic".to_owned()),
                Err(e) => return Outcome::Fail(e),
            }
            // Eval: also compare the action dataset to the parsed result file.
            if let (Some(result), Some(rbase), Some(rmt)) =
                (&case.result, &case.result_base, case.result_media_type)
            {
                let rbytes = std::fs::read(result).expect("read result");
                let rds = match parse_dataset(&rbytes, rmt, Some(rbase)) {
                    Ok(rds) => rds,
                    Err(e) => return Outcome::Fail(format!("parse result: {e:?}")),
                };
                if !datasets_isomorphic(&ds, &rds) {
                    return Outcome::Fail("action not isomorphic to mf:result".to_owned());
                }
            }
            Outcome::Pass
        }
    }
}

#[test]
fn w3c_rdf12_syntax_suites_round_trip() {
    let gaps: BTreeMap<&str, &str> = KNOWN_GAPS.iter().copied().collect();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut allowlisted: Vec<(String, String, String)> = Vec::new(); // (name, reason, observed)
    let mut stale: Vec<String> = Vec::new(); // allowlisted but passed
    let mut hard_failures: Vec<(String, String, String)> = Vec::new(); // (format, name, reason)
    let mut per_format: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new(); // (total, pass, gap)

    for suite in SUITES {
        for sub in suite.submanifests {
            for case in enumerate(suite, sub) {
                total += 1;
                let entry = per_format.entry(suite.dir).or_default();
                entry.0 += 1;
                let on_allowlist = gaps.get(case.name.as_str()).copied();
                match (run_case(&case), on_allowlist) {
                    (Outcome::Pass, None) => {
                        passed += 1;
                        entry.1 += 1;
                    }
                    (Outcome::Pass, Some(_)) => {
                        // Allowlisted but now passes → stale allowlist entry.
                        passed += 1;
                        entry.1 += 1;
                        stale.push(case.name.clone());
                    }
                    (Outcome::Fail(reason), Some(why)) => {
                        // Expected gap.
                        entry.2 += 1;
                        allowlisted.push((case.name.clone(), why.to_owned(), reason));
                    }
                    (Outcome::Fail(reason), None) => {
                        hard_failures.push((suite.dir.to_owned(), case.name.clone(), reason));
                    }
                }
            }
        }
    }

    // Always print a full summary — never a silent skip.
    eprintln!("\n=== W3C RDF 1.2 native-codec round-trip conformance ===");
    eprintln!("vendored corpus: crates/rdf/tests/corpus/w3c (w3c/rdf-tests @ 8519110)");
    for (fmt, (t, p, g)) in &per_format {
        eprintln!("  {fmt:>9}: total {t:>3}  passed {p:>3}  allowlisted-gap {g:>2}");
    }
    eprintln!(
        "  {:>9}: total {total:>3}  passed {passed:>3}  allowlisted-gap {:>2}",
        "TOTAL",
        allowlisted.len()
    );
    if !allowlisted.is_empty() {
        eprintln!("\nallowlisted known gaps (tolerated):");
        for (name, why, observed) in &allowlisted {
            eprintln!("  - {name}\n      reason : {why}\n      observed: {observed}");
        }
    }
    if !stale.is_empty() {
        eprintln!("\nSTALE allowlist entries (now PASS — prune them):");
        for name in &stale {
            eprintln!("  - {name}");
        }
    }

    // Inline lexical-form preservation cases (B2 fidelity): a full parse → serialize →
    // re-parse must preserve the literal lexical form byte-for-byte. The native path
    // must NOT canonicalize the way the oxigraph Store does.
    lexical_form_preserved_verbatim();

    let mut problems = Vec::new();
    if !hard_failures.is_empty() {
        problems.push(format!(
            "{} non-allowlisted W3C case(s) failed:",
            hard_failures.len()
        ));
        for (fmt, name, reason) in &hard_failures {
            problems.push(format!("  [{fmt}] {name}: {reason}"));
        }
    }
    if !stale.is_empty() {
        problems.push(format!(
            "{} stale KNOWN_GAPS entry/entries now pass (prune the allowlist): {}",
            stale.len(),
            stale.join(", ")
        ));
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));

    eprintln!("\nall non-allowlisted cases behaved as the W3C semantics require.\n");
}

/// Inline lexical-form preservation assertions — typed literals must survive a full
/// N-Triples round-trip with their lexical form UNCHANGED (no canonicalization).
fn lexical_form_preserved_verbatim() {
    use purrdf_rdf::{RdfDatasetBuilder, RdfLiteral};
    let cases = [
        ("0.70", "http://www.w3.org/2001/XMLSchema#decimal"),
        (
            "2024-01-01T00:00:00+00:00",
            "http://www.w3.org/2001/XMLSchema#dateTime",
        ),
        ("1.0E0", "http://www.w3.org/2001/XMLSchema#double"),
    ];
    for (lexical, datatype) in cases {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let lit = b.intern_literal(RdfLiteral::typed(lexical, datatype));
        b.push_quad(s, p, lit, None);
        let ds: Arc<RdfDataset> = b.freeze().expect("freeze");
        let bytes = serialize_dataset(&ds, "application/n-triples", SerializeGraph::Dataset)
            .expect("serialize");
        let reparsed =
            parse_dataset(&bytes, "application/n-triples", None).expect("re-parse lexical case");
        assert!(
            reparsed
                .term_id_by_value(&TermValue::Literal {
                    lexical_form: lexical.to_owned(),
                    datatype: datatype.to_owned(),
                    language: None,
                    direction: None,
                })
                .is_some(),
            "lexical form `{lexical}^^{datatype}` must survive a full round-trip verbatim"
        );
    }
}
