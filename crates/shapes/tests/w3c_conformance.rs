// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! W3C SHACL conformance harness over the vendored `vectors/shacl` corpus
//! (w3c/data-shapes test suite, `core/` + `sparql/`).
//!
//! Case discovery lives in [`shacl_corpora`], the one reader for this crate's two
//! SHACL corpora: it walks the `mf:` (test-manifest) tree starting at
//! `vectors/shacl/manifest.ttl`, where manifests `mf:include` sub-manifests and
//! list `mf:entries` of type `sht:Validate`, whose `mf:action` names a
//! `sht:shapesGraph` / `sht:dataGraph` (usually `<>` — the test file itself,
//! which then contains data, shapes, manifest entry, AND the expected report in
//! one graph, exactly as the upstream suite intends). This file is the GRADING
//! half: it runs each discovered case through the engine and decides whether the
//! produced report agrees with the manifest's.
//!
//! ## Comparison contract (caveats)
//!
//! Each expected `sh:ValidationReport` is compared, as a MULTISET, on the tuple
//!
//!   `(focusNode, resultPath, value, sourceConstraintComponent, severity,
//!   sourceShape)`
//!
//! with these normalizations:
//!
//! - **blank nodes** (focus/value/path/source shape) normalize to the
//!   placeholder `_:` — expected reports use their own bnode labels which cannot
//!   match the engine's; complex `sh:resultPath` structures
//!   (inverse/sequence/alternative path bnodes) normalize the same way, so only a
//!   *simple* (IRI) result path is compared by identity, and a blank-node source
//!   shape is compared as "a blank node";
//! - **`sh:resultMessage` is compared only where the expected report states
//!   one** — the suite asks a harness "to preserve all sh:resultMessage triples
//!   that are mentioned in the 'expected' results graph", so a produced result
//!   with the same tuple must carry EXACTLY that message set (language tags,
//!   directions and datatypes included); an engine-generated message on a result
//!   the suite states none for is the engine's own;
//! - **nested `sh:detail` is compared where the expected report states it** —
//!   such a result's produced details must be exactly the stated ones, as a
//!   multiset of the same tuple, recursively; a result the expected report
//!   states no details for is not graded on details, since SHACL 1.2 Core makes
//!   them optional and processor-dependent (quoted in `report_grading`);
//! - the expected report's `sh:conformanceDisallows` values are the validation
//!   parameter the case runs under, and the report must echo them;
//! - `mf:result sht:Failure` means the validator must REJECT the test input
//!   (any engine `Err` passes; a successful validation fails);
//! - every case loads with an empty import table. `validator-001` writes
//!   `owl:imports <http://datashapes.org/dash>` on a node that is neither the
//!   document's own IRI nor an `owl:Ontology`, so under OWL 2's mapping to RDF
//!   (§3.1.2) and SHACL 1.2 Core the triple is data, not an import; it is graded
//!   like every other case.
//!
//! ## Xfail ledger
//!
//! Every currently-failing test is listed in [`XFAIL`] with a precise reason;
//! the ledger doubles as the SHACL completion roadmap. The harness asserts
//! that every non-xfail test passes, that every xfail test still fails (an
//! unexpectedly-passing xfail is an error — remove it from the ledger), and
//! the EXACT discovered-test/xfail counts so silent corpus drift fails fast.
//!
//! Run with `--nocapture` for the per-manifest-section scoreboard:
//! `cargo test -p purrdf-shapes --test w3c_conformance -- --nocapture`

#[path = "support/w3c_earl.rs"]
mod w3c_earl;

use purrdf_shapes::shacl_corpora;

use std::collections::BTreeMap;

use shacl_corpora::report_grading::run_validate_case;
use shacl_corpora::{
    W3C_PROPOSED_UNINCLUDED, W3C_SUBSET_CASES, W3C_SUITE_COMMIT, W3C_TOTAL_CASES, w3c_cases,
    w3c_proposed_cases,
};

// ── Xfail ledger ──────────────────────────────────────────────────────────────

/// Tests the engine currently fails, with the reason. `(test id, reason)` where
/// the id is the entry IRI relative to the corpus root (no `.ttl`).
///
/// A test listed here MUST fail; when engine work fixes it, the harness errors
/// with `XPASS` and the entry must be removed. This is the SHACL completion
/// roadmap — keep reasons precise.
const XFAIL: &[(&str, &str)] = &[
    // All ledgered AF tests now pass; keep this ledger empty unless new
    // validation-only gaps are discovered.
];

// ── Running one case ──────────────────────────────────────────────────────────
//
// Grading lives in `shacl_corpora::report_grading`, shared with the SHACL 1.2
// harness so both suites mean the same thing by "agrees with the manifest".

// ── Spec-contradicted entries ─────────────────────────────────────────────────

/// Entries the engine fails against their UNAMENDED expectation because the
/// specification text contradicts that expectation: `(test id, the clause and the
/// observable difference)`. Graded, reported `earl:failed` with the rationale as
/// `earl:info`, and counted under their own label; an entry that passes (`XPASS`)
/// or an id no discovered case has is refused. Empty today.
const SPEC_CONTRADICTED: &[(&str, &str)] = &[];

/// [`SPEC_CONTRADICTED`] pinned by count.
const SPEC_CONTRADICTED_COUNT: usize = 0;

/// The law the vendored W3C entries are graded under: the Recommendation they test.
/// The first-party `af/` seam exercises SHACL-AF, which the Recommendation does not
/// define, and keeps the compatibility law.
const LAW: purrdf_shapes::ShaclProfile = purrdf_shapes::ShaclProfile::REC_20170720;

/// This suite's report: its file, the full commit of the vendored suite, and the
/// profile each record carries.
const SUITE: w3c_earl::Suite = w3c_earl::Suite {
    file_name: "purrdf-shacl-earl.ttl",
    commit: W3C_SUITE_COMMIT,
    profile: "shacl-20170720",
    law: LAW.id(),
    law_independent_records: false,
};

/// The EARL-scope scoreboard line's label and the name its failed count carries.
const SCOPE_LABEL: &str = "W3C 2017 subset";
const SCOPE_FAILED_LABEL: &str = "spec-contradicted";

/// One discovered case and its grade against the unamended expectation.
struct Graded {
    id: String,
    section: String,
    first_party: bool,
    result: Result<(), String>,
}

/// Every discovered case, graded exactly once per harness binary.
fn graded() -> &'static [Graded] {
    static GRADED: std::sync::OnceLock<Vec<Graded>> = std::sync::OnceLock::new();
    GRADED.get_or_init(|| {
        let mut tests = w3c_cases();
        for tc in tests.iter_mut().filter(|tc| !tc.first_party) {
            tc.profile = LAW;
        }
        // Engine panics are caught by `report_grading::no_panic` and reported as
        // ledgered failures, so their backtraces would only drown the scoreboard.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let graded = tests
            .iter()
            .map(|tc| Graded {
                id: tc.id.clone(),
                section: tc.section.clone(),
                first_party: tc.first_party,
                result: run_validate_case(tc),
            })
            .collect();
        std::panic::set_hook(default_hook);
        graded
    })
}

/// The entries a SHACL 2017 EARL report covers: the vendored W3C entries, not the
/// first-party `af/` seam.
fn scope_entries(graded: &[Graded]) -> Vec<w3c_earl::Entry<'_>> {
    let spec: BTreeMap<&str, &str> = SPEC_CONTRADICTED.iter().copied().collect();
    graded
        .iter()
        .filter(|g| !g.first_party)
        .map(|g| w3c_earl::Entry {
            id: &g.id,
            result: &g.result,
            rationale: spec.get(g.id.as_str()).copied(),
        })
        .collect()
}

/// The scoreboard's own tally, computed from the ledger match arms alone.
#[derive(Default)]
struct Scoreboard {
    errors: Vec<String>,
    /// (section, passed, xfailed) in discovery order.
    sections: Vec<(String, usize, usize)>,
    passed: usize,
    spec_contradicted: usize,
    xfailed: usize,
    /// Each in-scope case id, and whether it passed.
    scope: BTreeMap<String, bool>,
}

fn scoreboard() -> Scoreboard {
    let graded = graded();
    let xfail: BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    assert_eq!(
        xfail.len(),
        XFAIL.len(),
        "duplicate entries in the XFAIL ledger"
    );
    for (id, _) in XFAIL {
        assert!(
            graded.iter().any(|g| g.id == *id),
            "XFAIL ledger names unknown test {id} — stale entry?"
        );
    }
    w3c_earl::check_spec_table(SPEC_CONTRADICTED, SPEC_CONTRADICTED_COUNT, |id| {
        graded.iter().any(|g| g.id == id && !g.first_party)
    })
    .unwrap_or_else(|error| panic!("{error}"));
    let spec: BTreeMap<&str, &str> = SPEC_CONTRADICTED.iter().copied().collect();

    let mut board = Scoreboard::default();
    for g in graded {
        if board.sections.last().is_none_or(|(s, ..)| s != &g.section) {
            board.sections.push((g.section.clone(), 0, 0));
        }
        let class = match w3c_earl::classify(
            &g.id,
            &g.result,
            xfail.get(g.id.as_str()).copied(),
            spec.get(g.id.as_str()).copied(),
        ) {
            Ok(class) => class,
            Err(error) => {
                board.errors.push(error);
                continue;
            }
        };
        let slot = board.sections.last_mut().expect("section pushed above");
        match class {
            w3c_earl::Class::Passed => {
                slot.1 += 1;
                board.passed += 1;
            }
            w3c_earl::Class::Xfailed => {
                slot.2 += 1;
                board.xfailed += 1;
            }
            w3c_earl::Class::SpecContradicted => board.spec_contradicted += 1,
        }
        if !g.first_party {
            if class == w3c_earl::Class::Xfailed {
                // The EARL-scope line labels every failure spec-contradicted.
                board.errors.push(format!(
                    "{}: an XFAIL entry is in the EARL scope; the report would label it \
                     spec-contradicted",
                    g.id
                ));
            }
            board
                .scope
                .insert(g.id.clone(), class == w3c_earl::Class::Passed);
        }
    }
    board
}

/// The EARL-scope line, as printed: a projection of the report.
fn scope_line() -> String {
    w3c_earl::report(&SUITE, w3c_earl::SAMPLE_COMMIT, &scope_entries(graded()))
        .scoreboard_line(SCOPE_LABEL, SCOPE_FAILED_LABEL)
}

// ── The harness ───────────────────────────────────────────────────────────────

#[test]
fn w3c_shacl_conformance() {
    let tests = graded();
    let board = scoreboard();

    // Scoreboard: one line per manifest section.
    println!("W3C SHACL conformance scoreboard ({} tests):", tests.len());
    for (section, passed, xfailed) in &board.sections {
        println!("  {section:<28} passed {passed:>3}  xfailed {xfailed:>3}");
    }
    println!(
        "  TOTAL: passed {}, spec-contradicted {}, xfailed {}, ledger {}",
        board.passed,
        board.spec_contradicted,
        board.xfailed,
        XFAIL.len()
    );
    println!("  {}", scope_line());

    assert!(
        board.errors.is_empty(),
        "w3c_shacl_conformance: {} error(s):\n{}",
        board.errors.len(),
        board.errors.join("\n\n")
    );

    // Exact-count gates: every test is a pass, a spec-contradicted entry or a
    // ledgered xfail.
    assert_eq!(
        board.xfailed,
        XFAIL.len(),
        "xfail count must match the ledger exactly"
    );
    assert_eq!(
        board.spec_contradicted, SPEC_CONTRADICTED_COUNT,
        "spec-contradicted count must match the table exactly"
    );
    assert_eq!(
        board.passed + board.spec_contradicted + board.xfailed,
        W3C_TOTAL_CASES,
        "every discovered test must be a pass, a spec-contradicted entry or a ledgered xfail"
    );
}

/// The vendored `sht:proposed` entries no manifest includes
/// ([`W3C_PROPOSED_UNINCLUDED`]), graded by the approved suite's grader and reported
/// under their own category, "proposed, graded" — never among the approved passes
/// [`w3c_shacl_conformance`] counts. Every one must pass: a proposed test this engine
/// fails is a defect to fix, or an expectation to dispute against the specification.
#[test]
fn w3c_shacl_proposed_unincluded_files() {
    let cases = w3c_proposed_cases();
    assert_eq!(cases.len(), W3C_PROPOSED_UNINCLUDED.len());
    let mut errors: Vec<String> = Vec::new();
    let mut passed = 0usize;
    for tc in &cases {
        match run_validate_case(tc) {
            Ok(()) => passed += 1,
            Err(e) => errors.push(format!("FAIL [{id}] (proposed): {e}", id = tc.id)),
        }
    }
    println!(
        "W3C SHACL proposed, graded (not counted among approved tests): passed {passed} of {}",
        cases.len()
    );
    assert!(errors.is_empty(), "{}", errors.join("\n\n"));
}

// ── EARL ──────────────────────────────────────────────────────────────────────

/// The vendored commit is the one the suite's README pins, in full.
#[test]
fn the_2017_suite_commit_is_the_vendored_one() {
    assert!(w3c_earl::is_full_commit(W3C_SUITE_COMMIT));
    let readme = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/shacl/README.md"
    ))
    .expect("vectors/shacl/README.md is vendored");
    assert!(
        readme.contains(&format!("Commit: `{W3C_SUITE_COMMIT}`")),
        "README.md does not pin {W3C_SUITE_COMMIT}"
    );
}

/// `SPEC_CONTRADICTED` is pinned, well formed and graded unamended; the ledger
/// classification refuses an `XPASS` and an unknown id beside the accepted cases.
#[test]
fn spec_contradicted_entries_fail_unamended() {
    let graded = graded();
    w3c_earl::check_spec_table(SPEC_CONTRADICTED, SPEC_CONTRADICTED_COUNT, |id| {
        graded.iter().any(|g| g.id == id && !g.first_party)
    })
    .expect("the table is well formed");
    for (id, _) in SPEC_CONTRADICTED {
        let g = graded.iter().find(|g| g.id == *id).expect("discovered");
        assert!(
            g.result.is_err(),
            "{id} passes: remove it from SPEC_CONTRADICTED"
        );
    }
    w3c_earl::check_classification();
}

/// The mapping, the environment parsing, the key port and the write path.
#[test]
fn w3c_2017_earl_support_behaviour() {
    w3c_earl::check_record_mapping(&SUITE);
    w3c_earl::check_config_parsing();
    w3c_earl::check_test_key();
    w3c_earl::check_write_errors();
}

/// The 2017 subset is the 120 vendored `core/` and `sparql/` entries, found by the
/// discovery field and not by a path: the first-party `af/` seam is the rest.
#[test]
fn w3c_2017_subset_is_the_vendored_entries() {
    let graded = graded();
    let subset: Vec<&Graded> = graded.iter().filter(|g| !g.first_party).collect();
    assert_eq!(subset.len(), W3C_SUBSET_CASES);
    assert_eq!(
        graded.len() - subset.len(),
        W3C_TOTAL_CASES - W3C_SUBSET_CASES
    );
    assert!(
        subset
            .iter()
            .all(|g| g.id.starts_with("core/") || g.id.starts_with("sparql/")),
        "the subset is core/ and sparql/"
    );
    assert!(
        graded
            .iter()
            .filter(|g| g.first_party)
            .all(|g| g.id.starts_with("af/")),
        "the first-party entries are af/"
    );
}

/// Exactly the 120 vendored entries are assertions, once each, every one
/// `earl:automatic`; the first-party entries and the proposed entry no manifest
/// includes are absent; each outcome is the unamended grade.
#[test]
fn w3c_2017_earl_covers_the_subset_and_nothing_else() {
    let written =
        w3c_earl::assertions_by_key(&w3c_earl::sample_bytes(&SUITE, &scope_entries(graded())))
            .expect("parses");
    assert_eq!(written.len(), W3C_SUBSET_CASES);
    assert!(
        written
            .values()
            .all(|a| a.mode == purrdf_iri::vocab::earl::AUTOMATIC),
        "every assertion is earl:automatic"
    );
    let subset: std::collections::BTreeSet<&str> = graded()
        .iter()
        .filter(|g| !g.first_party)
        .map(|g| g.id.as_str())
        .collect();
    assert_eq!(
        written
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        subset
    );
    for g in graded().iter().filter(|g| !g.first_party) {
        assert_eq!(
            written[&g.id].outcome,
            if g.result.is_ok() {
                purrdf_iri::vocab::earl::PASSED
            } else {
                purrdf_iri::vocab::earl::FAILED
            },
            "{}",
            g.id
        );
        assert_eq!(
            written[&g.id].info.is_some(),
            SPEC_CONTRADICTED.iter().any(|(id, _)| *id == g.id),
            "{}: earl:info exactly for SPEC_CONTRADICTED entries",
            g.id
        );
    }
    // The proposed entry (`mf:status sht:proposed`) is not an assertion.
    let proposed = w3c_proposed_cases();
    assert_eq!(proposed.len(), W3C_PROPOSED_UNINCLUDED.len());
    for tc in &proposed {
        assert!(!written.contains_key(&tc.id), "{} is proposed", tc.id);
    }
    assert!(written.keys().all(|k| !k.starts_with("af/")));
}

/// The Working Group's own inventory of the 2017 suite
/// (`vectors/shacl-reports/alltests.ttl`, frozen) is the reference for what the
/// report must cover: its approved `urn:x-shacl-test:` subjects are exactly the
/// report's `earl:test` IRIs, once each; its proposed subject is absent; and the
/// report's passed and failed counts are the ones the scoreboard prints.
#[test]
fn w3c_2017_earl_matches_wg_inventory() {
    use purrdf_iri::vocab::{earl, mf, sht};
    use purrdf_shapes::data::{GraphFilter, native_quads};
    use purrdf_shapes::term::Term;
    use std::collections::BTreeSet;

    let inventory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vectors/shacl-reports/alltests.ttl");
    let graph = shacl_corpora::parse_turtle_file(&inventory).expect("the WG inventory parses");
    let with_status = |status: &str| -> BTreeSet<String> {
        native_quads(
            &*graph,
            None,
            Some(&shacl_corpora::named(mf::STATUS)),
            Some(&shacl_corpora::named(status)),
            GraphFilter::AnyGraph,
        )
        .into_iter()
        .map(|(subject, _, _)| match subject {
            Term::NamedNode(n) => n.as_str().to_owned(),
            other => panic!("inventory subject {other} is not an IRI"),
        })
        .collect()
    };
    let approved = with_status(sht::APPROVED);
    let proposed = with_status(sht::PROPOSED);
    assert_eq!(
        approved.len(),
        W3C_SUBSET_CASES,
        "the WG approves 120 tests"
    );
    assert_eq!(proposed.len(), W3C_PROPOSED_UNINCLUDED.len());
    assert!(approved.is_disjoint(&proposed));

    // The report as the writer builds it, parsed back: any parse error fails. A
    // second assertion about one test is a parse error in `assertions`, so each
    // test occurs exactly once.
    let bytes = w3c_earl::sample_bytes(&SUITE, &scope_entries(graded()));
    let statements = w3c_earl::statements(&bytes).expect("the report parses");
    let assertions = w3c_earl::assertions(&statements).expect("the assertions parse");
    let asserted: BTreeSet<&str> = assertions.keys().map(String::as_str).collect();
    let test_count = statements
        .iter()
        .filter(|t| t.p == w3c_earl::angle(earl::TEST))
        .count();
    assert_eq!(test_count, approved.len(), "one earl:test per assertion");
    assert_eq!(
        asserted,
        approved.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        "the report covers exactly the WG's approved tests"
    );
    for iri in &proposed {
        assert!(!assertions.contains_key(iri), "{iri} is proposed");
    }

    // The counts equal the printed line.
    let line = scope_line();
    let counts = line
        .strip_prefix(&format!("{SCOPE_LABEL}: passed "))
        .and_then(|rest| rest.split_once(&format!(", {SCOPE_FAILED_LABEL} ")))
        .and_then(|(passed, rest)| {
            let (failed, total) = rest.split_once(" of ")?;
            Some((
                passed.parse::<usize>().ok()?,
                failed.parse::<usize>().ok()?,
                total.parse::<usize>().ok()?,
            ))
        })
        .unwrap_or_else(|| panic!("unparseable scoreboard line {line:?}"));
    let outcome = |wanted: &str| assertions.values().filter(|a| a.outcome == wanted).count();
    assert_eq!(
        (
            outcome(earl::PASSED),
            outcome(earl::FAILED),
            assertions.len()
        ),
        counts,
        "{line}"
    );
    assert_eq!(counts.2, 120);
}

/// The subject and the assertor, by exact value.
#[test]
fn w3c_2017_earl_subject_and_assertor_are_exact() {
    w3c_earl::check_descriptions(
        &w3c_earl::sample_bytes(&SUITE, &scope_entries(graded())),
        w3c_earl::SAMPLE_COMMIT,
        "08adb3776709a014bc3062ede793c36275b22446",
        "shacl-rec-2017-07-20",
        false,
    )
    .unwrap_or_else(|error| panic!("{error}"));
}

/// Two in-memory generations are byte-identical, sorted, and carry no date or time.
#[test]
fn w3c_2017_earl_is_deterministic_and_carries_no_date() {
    let first = w3c_earl::sample_bytes(&SUITE, &scope_entries(graded()));
    assert_eq!(
        first,
        w3c_earl::sample_bytes(&SUITE, &scope_entries(graded()))
    );
    let text = String::from_utf8(first.clone()).expect("UTF-8");
    assert!(!text.contains("dateTime") && !text.contains("XMLSchema#date"));
    for predicate in w3c_earl::predicates(&first).expect("parses") {
        assert!(
            !predicate.to_ascii_lowercase().contains("date")
                && !predicate.to_ascii_lowercase().contains("time"),
            "{predicate} is a date or time predicate"
        );
    }
    let written: Vec<&str> = text.lines().collect();
    let mut sorted = written.clone();
    sorted.sort_unstable();
    assert_eq!(written, sorted, "the file is sorted");
}

/// The EARL and the scoreboard agree, checked independently of the shared report:
/// the printed subset line and the serialized report are compared, per entry and in
/// total, with the tally the match arms computed before any report existed; and
/// each of three mutations of the serialized report alone breaks the agreement.
#[test]
fn earl_agrees_with_scoreboard() {
    let board = scoreboard();
    assert_eq!(board.scope.len(), W3C_SUBSET_CASES);
    w3c_earl::check_agreement_and_mutations(
        &w3c_earl::sample_bytes(&SUITE, &scope_entries(graded())),
        &scope_line(),
        SCOPE_LABEL,
        SCOPE_FAILED_LABEL,
        &board.scope,
    );
}

/// With both settings the report is written under the directory; with neither,
/// nothing is; with either alone or a malformed commit, the run fails.
#[test]
fn w3c_2017_earl_report_is_written_when_configured() {
    let config = match w3c_earl::config() {
        Err(error) => panic!("{error}"),
        Ok(None) => {
            println!(
                "W3C 2017 EARL: no {} set; nothing written",
                w3c_earl::DIR_VAR
            );
            return;
        }
        Ok(Some(config)) => config,
    };
    let bytes = w3c_earl::earl_bytes(&w3c_earl::report(
        &SUITE,
        &config.commit,
        &scope_entries(graded()),
    ))
    .expect("the report renders");
    w3c_earl::check_descriptions(
        &bytes,
        &config.commit,
        W3C_SUITE_COMMIT,
        SUITE.law,
        SUITE.law_independent_records,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let path = w3c_earl::write_report(&config.dir, SUITE.file_name, &bytes)
        .unwrap_or_else(|error| panic!("{error}"));
    println!(
        "W3C 2017 EARL: wrote {} ({} bytes)",
        path.display(),
        bytes.len()
    );
}
