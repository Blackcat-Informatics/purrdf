// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The harness side of the W3C SHACL implementation reports: the EARL subject and
//! assertor, the environment that selects where reports are written, the Rust port
//! of the Working Group reports page's `testKey`, the mapping from a harness grade
//! to a [`Record`], file writing, and the read-back used to check a written report.
//!
//! The kit renders EARL from a [`Report`]; everything specific to the W3C suites
//! (the DOAP and FOAF descriptions, the `urn:x-shacl-test:` keys, the two output
//! files) is here, in test code, because PurRDF mints and ships no vocabulary of its
//! own. Included by both W3C harnesses; the 2017 harness uses a subset.

#![allow(dead_code, unreachable_pub)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use purrdf_conformance_kit::outcome::{
    AssertorDescription, DescriptionTerm, Observation, OutcomeKind, Record, Report, ReportRecord,
    SubjectDescription, to_earl,
};
use purrdf_iri::vocab::{earl, rdf};

// ── The environment ───────────────────────────────────────────────────────────

/// The directory the reports are written under.
pub const DIR_VAR: &str = "PURRDF_W3C_EARL_DIR";
/// The 40-hex PurRDF commit the reports are generated for.
pub const COMMIT_VAR: &str = "PURRDF_W3C_EARL_COMMIT";

/// A commit stand-in for in-memory generation, which writes nothing.
pub const SAMPLE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// Where to write the reports, and for which PurRDF commit: both variables,
/// parsed once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlConfig {
    pub dir: PathBuf,
    pub commit: String,
}

/// Whether `commit` is a full lowercase hexadecimal git object name.
pub fn is_full_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Parse the two settings. Neither set means no reports are written; either one
/// alone, an empty directory or a commit that is not 40 hex digits is an error.
pub fn parse_config(
    dir: Option<OsString>,
    commit: Option<OsString>,
) -> Result<Option<EarlConfig>, String> {
    match (dir, commit) {
        (None, None) => Ok(None),
        (Some(_), None) => Err(format!("{DIR_VAR} is set but {COMMIT_VAR} is not")),
        (None, Some(_)) => Err(format!("{COMMIT_VAR} is set but {DIR_VAR} is not")),
        (Some(dir), Some(commit)) => {
            let commit = commit
                .into_string()
                .map_err(|_| format!("{COMMIT_VAR} is not UTF-8"))?;
            if !is_full_commit(&commit) {
                return Err(format!(
                    "{COMMIT_VAR} must be 40 lowercase hex digits, got {commit:?}"
                ));
            }
            if dir.is_empty() {
                return Err(format!("{DIR_VAR} is empty"));
            }
            Ok(Some(EarlConfig {
                dir: PathBuf::from(dir),
                commit,
            }))
        }
    }
}

/// The environment's configuration, read and parsed once per harness binary.
pub fn config() -> &'static Result<Option<EarlConfig>, String> {
    static CONFIG: std::sync::OnceLock<Result<Option<EarlConfig>, String>> =
        std::sync::OnceLock::new();
    CONFIG.get_or_init(|| parse_config(std::env::var_os(DIR_VAR), std::env::var_os(COMMIT_VAR)))
}

// ── The subject and the assertor ──────────────────────────────────────────────

pub const SUBJECT_IRI: &str = "https://github.com/Blackcat-Informatics/purrdf";
pub const BUG_DATABASE_IRI: &str = "https://github.com/Blackcat-Informatics/purrdf/issues";
pub const DEVELOPER_IRI: &str = "https://github.com/paudley";
pub const ASSERTOR_IRI: &str = DEVELOPER_IRI;
pub const ASSERTOR_NAME: &str = "Patrick Audley";

const DOAP_NS: &str = "http://usefulinc.com/ns/doap#";
const FOAF_NS: &str = "http://xmlns.com/foaf/0.1/";

fn doap(local: &str) -> String {
    format!("{DOAP_NS}{local}")
}

/// One of the two suites a report covers.
#[derive(Debug, Clone, Copy)]
pub struct Suite {
    /// The report's file name under the output directory.
    pub file_name: &'static str,
    /// The full commit of `w3c/data-shapes` the vendored suite was taken from.
    pub commit: &'static str,
    /// The profile name written on each record.
    pub profile: &'static str,
    /// The SHACL semantic-law identifier the suite is graded under
    /// (`ShaclProfile::id`), named in the subject's description.
    pub law: &'static str,
    /// Whether the report also covers node-expression and SPARQL 1.2 RL records, which no
    /// SHACL law selects: the description then says they are law-independent.
    pub law_independent_records: bool,
}

/// The `earl:TestSubject`: PurRDF, as a DOAP project, naming the build.
pub fn subject(purrdf_commit: &str, suite: &Suite) -> SubjectDescription {
    let text = |value: String| DescriptionTerm::Text(value);
    let iri = |value: &str| DescriptionTerm::Iri(value.to_owned());
    SubjectDescription::new(
        SUBJECT_IRI,
        vec![
            earl::TEST_SUBJECT.to_owned(),
            earl::SOFTWARE.to_owned(),
            doap("Project"),
        ],
        vec![
            (
                doap("name"),
                text(format!("PurRDF {}", env!("CARGO_PKG_VERSION"))),
            ),
            (doap("homepage"), iri(SUBJECT_IRI)),
            (doap("repository"), iri(env!("CARGO_PKG_REPOSITORY"))),
            (doap("programming-language"), text("Rust".to_owned())),
            (doap("developer"), iri(DEVELOPER_IRI)),
            (doap("bug-database"), iri(BUG_DATABASE_IRI)),
            (doap("description"), text(description(purrdf_commit, suite))),
        ],
    )
}

/// The subject's `doap:description`: the build, the suite commit, the SHACL law the suite is
/// graded under and that every host selects the same law, so a Working Group reader can
/// reproduce the result; for a suite with node-expression and SPARQL 1.2 RL records, that
/// those records are law-independent.
fn description(purrdf_commit: &str, suite: &Suite) -> String {
    let mut text = format!(
        "PurRDF built from source commit {purrdf_commit}, run against the W3C data-shapes \
         test suite at commit {}, graded under the {} profile. That law is selectable on \
         every host: `purrdf validate --shacl-profile {}` on the command line, and the \
         `shacl_profile` keyword (Python), the `shaclProfile` argument (WebAssembly) and the \
         `shacl_profile` argument of `purrdf_shacl_validate_to_sarif_law` (C)",
        suite.commit, suite.law, suite.law
    );
    if suite.law_independent_records {
        text.push_str(
            ". Node-expression and SPARQL 1.2 RL records are law-independent: no SHACL law \
             selects them",
        );
    }
    text
}

/// The `earl:Assertor`, named through `foaf:name`.
pub fn assertor() -> AssertorDescription {
    AssertorDescription::new(
        ASSERTOR_IRI,
        vec![earl::ASSERTOR.to_owned()],
        vec![(
            format!("{FOAF_NS}name"),
            DescriptionTerm::Text(ASSERTOR_NAME.to_owned()),
        )],
    )
}

// ── Keys ──────────────────────────────────────────────────────────────────────

const KEY_SCHEME: &str = "urn:x-shacl-test:";

/// The test IRI an EARL assertion names for the entry `id`, an entry path relative
/// to the suite's `tests/` root, in the Working Group reports page's key scheme.
pub fn test_iri(id: &str) -> String {
    format!("{KEY_SCHEME}/{id}")
}

/// The Working Group reports page's `testKey` (`reports.js`), ported: maps the IRI
/// of a test case, in the `urn:x-shacl-test:` form or as a URL under the tests
/// folder `tests_base`, to its path relative to that folder. Any other IRI maps to
/// the path after its last `/tests/`, else to itself.
pub fn test_key(iri: &str, tests_base: &str) -> String {
    if let Some(rest) = iri.strip_prefix(KEY_SCHEME) {
        return rest.trim_start_matches('/').to_owned();
    }
    if let Some(rest) = iri.strip_prefix(tests_base) {
        return rest.to_owned();
    }
    match iri.rfind("/tests/") {
        Some(index) => iri[index + "/tests/".len()..].to_owned(),
        None => iri.to_owned(),
    }
}

// ── Grades, classes and the ledgers ───────────────────────────────────────────

/// How one in-scope entry counts on the scoreboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Passed,
    SpecContradicted,
    Xfailed,
}

/// Classify one graded entry against the ledgers, by match arm. A
/// `SPEC_CONTRADICTED` entry is graded against the unamended expectation and must
/// fail; an entry that passes (XPASS) is refused, as is a failure that no ledger
/// names, an entry in both ledgers and a ledgered entry that passes.
pub fn classify(
    id: &str,
    result: &Result<(), String>,
    xfail: Option<&str>,
    spec_contradicted: Option<&str>,
) -> Result<Class, String> {
    match (result, xfail, spec_contradicted) {
        (_, Some(_), Some(_)) => Err(format!(
            "{id} is in both the XFAIL and the SPEC_CONTRADICTED tables"
        )),
        (Ok(()), None, None) => Ok(Class::Passed),
        (Err(_), Some(_), None) => Ok(Class::Xfailed),
        (Err(_), None, Some(_)) => Ok(Class::SpecContradicted),
        (Ok(()), Some(reason), None) => Err(format!(
            "XPASS [{id}]: now passes — remove it from the XFAIL ledger (reason was: {reason})"
        )),
        (Ok(()), None, Some(rationale)) => Err(format!(
            "XPASS [{id}]: now passes — remove it from SPEC_CONTRADICTED (rationale was: \
             {rationale})"
        )),
        (Err(error), None, None) => Err(format!("FAIL [{id}]: {error}")),
    }
}

/// Check a `SPEC_CONTRADICTED` table: its length is the pinned count, each id occurs
/// once and names a discovered entry (`known`), and each rationale is non-empty.
pub fn check_spec_table(
    table: &[(&str, &str)],
    pinned: usize,
    known: impl Fn(&str) -> bool,
) -> Result<(), String> {
    if table.len() != pinned {
        return Err(format!(
            "SPEC_CONTRADICTED has {} entries but the pin is {pinned}",
            table.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for (id, rationale) in table {
        if !seen.insert(*id) {
            return Err(format!("SPEC_CONTRADICTED names {id} twice"));
        }
        if !known(id) {
            return Err(format!("SPEC_CONTRADICTED names unknown test {id}"));
        }
        if rationale.trim().is_empty() {
            return Err(format!("SPEC_CONTRADICTED gives {id} no rationale"));
        }
    }
    Ok(())
}

// ── Mapping a grade to a record ───────────────────────────────────────────────

/// One in-scope entry as the report sees it.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    /// The entry path relative to the suite's `tests/` root.
    pub id: &'a str,
    /// The harness's grade against the unamended expectation.
    pub result: &'a Result<(), String>,
    /// The `SPEC_CONTRADICTED` rationale, if the entry is in the table.
    pub rationale: Option<&'a str>,
}

/// The record of one graded entry: expected `Compared`; observed `Compared` when
/// the harness's grade passed and `Mismatch`, carrying the error text, when it did
/// not. An entry that expects `sht:Failure` takes the same route: the harness
/// grade already says whether the engine refused as required, and the suites name
/// no stable rejection reason to grade it by.
pub fn record(profile: &str, id: &str, result: &Result<(), String>) -> Record {
    let (kind, message) = match result {
        Ok(()) => (OutcomeKind::Compared, String::new()),
        Err(error) => (OutcomeKind::Mismatch, error.clone()),
    };
    Record {
        case: test_iri(id),
        profile: profile.to_owned(),
        surface: "native".to_owned(),
        expected: Observation {
            kind: OutcomeKind::Compared,
            reason: None,
            message: String::new(),
        },
        observed: Observation {
            kind,
            reason: None,
            message,
        },
        passed: result.is_ok(),
        artifacts: Vec::new(),
    }
}

/// Build the report for `entries`. A harness record can only be passed or failed,
/// so a record with any other category is a bug here, and a `SPEC_CONTRADICTED`
/// rationale on a passing entry is refused. `earl:info` carries the rationale text
/// only, never a machine path or an engine error.
pub fn report(suite: &Suite, purrdf_commit: &str, entries: &[Entry<'_>]) -> Report {
    let records = entries
        .iter()
        .map(|entry| {
            assert!(
                entry.rationale.is_none() || entry.result.is_err(),
                "{} is in SPEC_CONTRADICTED but passes",
                entry.id
            );
            let record = record(suite.profile, entry.id, entry.result);
            assert!(
                matches!(
                    record.observed.kind,
                    OutcomeKind::Compared | OutcomeKind::Mismatch
                ),
                "{}: a harness record is compared or mismatched",
                entry.id
            );
            ReportRecord {
                record,
                info: entry.rationale.map(str::to_owned),
            }
        })
        .collect();
    Report::new(subject(purrdf_commit, suite), assertor(), records)
}

// ── Serializing and writing ───────────────────────────────────────────────────

/// The report of `entries` for the sample commit, serialized: what a harness
/// compares against, without writing a file.
pub fn sample_bytes(suite: &Suite, entries: &[Entry<'_>]) -> Vec<u8> {
    earl_bytes(&report(suite, SAMPLE_COMMIT, entries)).expect("the report renders")
}

/// The report as sorted N-Triples, one statement per line. N-Triples is valid
/// Turtle, and the reports page lists `*.ttl`.
pub fn earl_bytes(report: &Report) -> Result<Vec<u8>, String> {
    let dataset = to_earl(report).map_err(|error| error.to_string())?;
    let bytes = purrdf_rdf::serialize_dataset(
        dataset.as_ref(),
        "application/n-triples",
        purrdf_rdf::SerializeGraph::Dataset,
    )
    .map_err(|error| error.to_string())?;
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    let lines: BTreeSet<&str> = text.lines().collect();
    let mut sorted = String::with_capacity(text.len());
    for line in lines {
        sorted.push_str(line);
        sorted.push('\n');
    }
    Ok(sorted.into_bytes())
}

/// Write `bytes` as `file_name` under `dir`, creating `dir`; any failure is an error.
pub fn write_report(dir: &Path, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let path = dir.join(file_name);
    std::fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(path)
}

// ── Reading a report back ─────────────────────────────────────────────────────

/// One statement of a written report, each term in its N-Triples spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub s: String,
    pub p: String,
    pub o: String,
}

/// An N-Triples term in angle brackets, as written.
pub fn angle(iri: &str) -> String {
    format!("<{iri}>")
}

/// A plain N-Triples string literal, as written for ordinary ASCII text.
pub fn literal(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Parse a written report, one statement per line.
pub fn statements(bytes: &[u8]) -> Result<Vec<Statement>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    text.lines()
        .map(|line| {
            let body = line
                .strip_suffix(" .")
                .ok_or_else(|| format!("not a statement: {line:?}"))?;
            let mut parts = body.splitn(3, ' ');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(s), Some(p), Some(o)) => Ok(Statement {
                    s: s.to_owned(),
                    p: p.to_owned(),
                    o: o.to_owned(),
                }),
                _ => Err(format!("not a statement: {line:?}")),
            }
        })
        .collect()
}

/// One assertion of a written report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertion {
    /// The outcome IRI.
    pub outcome: String,
    pub mode: String,
    pub assertor: String,
    pub subject: String,
    /// The `earl:info` object of the assertion's result as written, when present.
    pub info: Option<String>,
}

fn unangle(term: &str) -> Result<&str, String> {
    term.strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .ok_or_else(|| format!("not an IRI: {term}"))
}

/// The assertions of a written report by test IRI. Every assertion must name exactly
/// one test, subject, assertor, mode and result, the result exactly one outcome; two
/// assertions about one test are refused.
pub fn assertions(statements: &[Statement]) -> Result<BTreeMap<String, Assertion>, String> {
    let ty = angle(rdf::TYPE);
    let one = |subject: &str, predicate: &str| -> Result<String, String> {
        let mut values = statements
            .iter()
            .filter(|t| t.s == subject && t.p == angle(predicate))
            .map(|t| t.o.clone());
        match (values.next(), values.next()) {
            (Some(value), None) => Ok(value),
            _ => Err(format!("{subject} needs exactly one {predicate}")),
        }
    };
    let mut out = BTreeMap::new();
    for assertion in statements
        .iter()
        .filter(|t| t.p == ty && t.o == angle(earl::ASSERTION))
    {
        let a = assertion.s.as_str();
        if statements
            .iter()
            .any(|t| t.s == a && t.p == angle(earl::INFO))
        {
            return Err(format!("{a} is an earl:Assertion carrying earl:info"));
        }
        let test = unangle(&one(a, earl::TEST)?)?.to_owned();
        let result = one(a, earl::RESULT)?;
        let parsed = Assertion {
            outcome: unangle(&one(&result, earl::OUTCOME)?)?.to_owned(),
            mode: unangle(&one(a, earl::MODE)?)?.to_owned(),
            assertor: unangle(&one(a, earl::ASSERTED_BY)?)?.to_owned(),
            subject: unangle(&one(a, earl::SUBJECT)?)?.to_owned(),
            info: statements
                .iter()
                .find(|t| t.s == result && t.p == angle(earl::INFO))
                .map(|t| t.o.clone()),
        };
        if out.insert(test.clone(), parsed).is_some() {
            return Err(format!("two assertions about {test}"));
        }
    }
    Ok(out)
}

/// The assertions of a written report by entry key (see [`test_key`]).
pub fn assertions_by_key(bytes: &[u8]) -> Result<BTreeMap<String, Assertion>, String> {
    let by_iri = assertions(&statements(bytes)?)?;
    let mut out = BTreeMap::new();
    for (iri, assertion) in by_iri {
        let key = test_key(&iri, "");
        if out.insert(key.clone(), assertion).is_some() {
            return Err(format!("two assertions about key {key}"));
        }
    }
    Ok(out)
}

/// Check the subject and the assertor of a written report against exact values,
/// spelled out here independently of [`subject`] and [`assertor`]: every
/// `earl:subject` is the PurRDF project resource and every `earl:assertedBy` is the
/// assertor, each described once.
pub fn check_descriptions(
    bytes: &[u8],
    purrdf_commit: &str,
    suite_commit: &str,
    law: &str,
    law_independent_records: bool,
) -> Result<(), String> {
    let statements = statements(bytes)?;
    let about = |subject: &str| -> BTreeSet<(String, String)> {
        statements
            .iter()
            .filter(|t| t.s == angle(subject))
            .map(|t| (t.p.clone(), t.o.clone()))
            .collect()
    };
    let ty = angle(rdf::TYPE);
    let doap = |local: &str| angle(&format!("http://usefulinc.com/ns/doap#{local}"));
    let project = about("https://github.com/Blackcat-Informatics/purrdf");
    let description = project
        .iter()
        .find(|(p, _)| *p == doap("description"))
        .map(|(_, o)| o.clone())
        .ok_or("the subject has no doap:description")?;
    // Spelled out here independently of `description`, so a change to either is a failure.
    let mut spelled = format!(
        "PurRDF built from source commit {purrdf_commit}, run against the W3C data-shapes test \
         suite at commit {suite_commit}, graded under the {law} profile. That law is selectable \
         on every host: `purrdf validate --shacl-profile {law}` on the command line, and the \
         `shacl_profile` keyword (Python), the `shaclProfile` argument (WebAssembly) and the \
         `shacl_profile` argument of `purrdf_shacl_validate_to_sarif_law` (C)"
    );
    if law_independent_records {
        spelled.push_str(
            ". Node-expression and SPARQL 1.2 RL records are law-independent: no SHACL law \
             selects them",
        );
    }
    if description != literal(&spelled) {
        return Err(format!(
            "doap:description {description} is not exactly {spelled:?}"
        ));
    }
    let expected: BTreeSet<(String, String)> = [
        (ty.clone(), angle(earl::TEST_SUBJECT)),
        (ty.clone(), angle(earl::SOFTWARE)),
        (ty.clone(), doap("Project")),
        (
            doap("name"),
            literal(&format!("PurRDF {}", env!("CARGO_PKG_VERSION"))),
        ),
        (
            doap("homepage"),
            angle("https://github.com/Blackcat-Informatics/purrdf"),
        ),
        (
            doap("repository"),
            angle("https://github.com/Blackcat-Informatics/purrdf"),
        ),
        (doap("programming-language"), literal("Rust")),
        (doap("developer"), angle("https://github.com/paudley")),
        (
            doap("bug-database"),
            angle("https://github.com/Blackcat-Informatics/purrdf/issues"),
        ),
        (doap("description"), description),
    ]
    .into_iter()
    .collect();
    if project != expected {
        return Err(format!("the subject is {project:?}, not {expected:?}"));
    }
    let assertor = about("https://github.com/paudley");
    let expected: BTreeSet<(String, String)> = [
        (ty, angle(earl::ASSERTOR)),
        (
            angle("http://xmlns.com/foaf/0.1/name"),
            literal("Patrick Audley"),
        ),
    ]
    .into_iter()
    .collect();
    if assertor != expected {
        return Err(format!("the assertor is {assertor:?}, not {expected:?}"));
    }
    for assertion in assertions(&statements)?.values() {
        if assertion.subject != "https://github.com/Blackcat-Informatics/purrdf"
            || assertion.assertor != "https://github.com/paudley"
        {
            return Err(format!("an assertion names {assertion:?}"));
        }
    }
    Ok(())
}

/// Every predicate a written report uses, which must be a known EARL, RDF, DOAP or
/// FOAF term: in particular no date or time predicate.
pub fn predicates(bytes: &[u8]) -> Result<BTreeSet<String>, String> {
    Ok(statements(bytes)?.into_iter().map(|t| t.p).collect())
}

// ── Agreement with the scoreboard ─────────────────────────────────────────────

/// The counts of an EARL-scope scoreboard line, `LABEL: passed P, FAILED_LABEL F of N`.
pub fn parse_scope_line(
    line: &str,
    label: &str,
    failed_label: &str,
) -> Result<(usize, usize, usize), String> {
    let rest = line
        .trim()
        .strip_prefix(label)
        .and_then(|rest| rest.strip_prefix(": passed "))
        .ok_or_else(|| format!("{line:?} is not a {label} line"))?;
    let (passed, rest) = rest
        .split_once(&format!(", {failed_label} "))
        .ok_or_else(|| format!("{line:?} has no {failed_label} count"))?;
    let (failed, total) = rest
        .split_once(" of ")
        .ok_or_else(|| format!("{line:?} has no total"))?;
    let number = |text: &str| {
        text.parse::<usize>()
            .map_err(|error| format!("{line:?}: {text:?}: {error}"))
    };
    Ok((number(passed)?, number(failed)?, number(total)?))
}

/// Compare the scoreboard's own tally (`tally`: entry id to whether it passed,
/// computed from the match arms before any report existed) with the PRINTED
/// EARL-scope line and the SERIALIZED report: the same entries, the same outcome for
/// each, and the same totals in all three.
pub fn agree(
    serialized: &[u8],
    printed: &str,
    label: &str,
    failed_label: &str,
    tally: &BTreeMap<String, bool>,
) -> Result<(), String> {
    let (line_passed, line_failed, line_total) = parse_scope_line(printed, label, failed_label)?;
    let written = assertions_by_key(serialized)?;
    let outcome = |passed: bool| if passed { earl::PASSED } else { earl::FAILED };
    for (id, passed) in tally {
        match written.get(id) {
            None => return Err(format!("{id} is on the scoreboard but not in the EARL")),
            Some(assertion) if assertion.outcome != outcome(*passed) => {
                return Err(format!(
                    "{id}: the scoreboard says {}, the EARL says {}",
                    outcome(*passed),
                    assertion.outcome
                ));
            }
            Some(_) => {}
        }
    }
    if let Some(id) = written.keys().find(|id| !tally.contains_key(*id)) {
        return Err(format!("{id} is in the EARL but not on the scoreboard"));
    }
    let tally_passed = tally.values().filter(|passed| **passed).count();
    let earl_passed = written
        .values()
        .filter(|a| a.outcome == earl::PASSED)
        .count();
    let earl_failed = written
        .values()
        .filter(|a| a.outcome == earl::FAILED)
        .count();
    let counts = [
        ("passed", line_passed, tally_passed, earl_passed),
        (
            "failed",
            line_failed,
            tally.len() - tally_passed,
            earl_failed,
        ),
        ("total", line_total, tally.len(), written.len()),
    ];
    for (what, line, tallied, written) in counts {
        if line != tallied || tallied != written {
            return Err(format!(
                "{what}: printed {line}, scoreboard {tallied}, EARL {written}"
            ));
        }
    }
    Ok(())
}

// ── Mutations of a written report ─────────────────────────────────────────────

/// The report with the first `earl:passed` outcome turned into `earl:failed`.
pub fn mutate_flip(bytes: &[u8]) -> Vec<u8> {
    let from = format!("{} {} .", angle(earl::OUTCOME), angle(earl::PASSED));
    let to = format!("{} {} .", angle(earl::OUTCOME), angle(earl::FAILED));
    let text = String::from_utf8(bytes.to_vec()).expect("a report is UTF-8");
    assert!(text.contains(&from), "the report has a passed outcome");
    text.replacen(&from, &to, 1).into_bytes()
}

/// The report without its first assertion and that assertion's result.
pub fn mutate_remove(bytes: &[u8]) -> Vec<u8> {
    let all = statements(bytes).expect("a report parses");
    let first = all
        .iter()
        .find(|t| t.p == angle(earl::TEST))
        .expect("the report has an assertion");
    let assertion = first.s.clone();
    let result = all
        .iter()
        .find(|t| t.s == assertion && t.p == angle(earl::RESULT))
        .expect("the assertion has a result")
        .o
        .clone();
    let mut text = String::new();
    for line in String::from_utf8(bytes.to_vec())
        .expect("a report is UTF-8")
        .lines()
    {
        let t = &statements(format!("{line}\n").as_bytes()).expect("a line parses")[0];
        if t.s != assertion && t.s != result {
            text.push_str(line);
            text.push('\n');
        }
    }
    text.into_bytes()
}

/// The report with one more passing assertion, about a test it does not name.
pub fn mutate_add(bytes: &[u8]) -> Vec<u8> {
    let mut text = String::from_utf8(bytes.to_vec()).expect("a report is UTF-8");
    let (a, r) = ("_:extraAssertion", "_:extraResult");
    let ty = angle(rdf::TYPE);
    let added = [
        (a, ty.clone(), angle(earl::ASSERTION)),
        (a, angle(earl::ASSERTED_BY), angle(ASSERTOR_IRI)),
        (a, angle(earl::SUBJECT), angle(SUBJECT_IRI)),
        (a, angle(earl::TEST), angle(&test_iri("extra/added-001"))),
        (a, angle(earl::RESULT), r.to_owned()),
        (a, angle(earl::MODE), angle(earl::AUTOMATIC)),
        (r, ty, angle(earl::TEST_RESULT)),
        (r, angle(earl::OUTCOME), angle(earl::PASSED)),
    ]
    .map(|(s, p, o)| format!("{s} {p} {o} .\n"));
    text.push_str(&added.concat());
    text.into_bytes()
}

// ── Checks both harnesses run ─────────────────────────────────────────────────

/// The environment parsing: neither setting writes nothing, both valid settings
/// configure, and each way of getting it wrong is refused (each beside the valid
/// neighbour it differs from).
pub fn check_config_parsing() {
    let os = |text: &str| Some(OsString::from(text));
    assert_eq!(parse_config(None, None), Ok(None));
    assert_eq!(
        parse_config(os("/tmp/earl"), os(SAMPLE_COMMIT)),
        Ok(Some(EarlConfig {
            dir: PathBuf::from("/tmp/earl"),
            commit: SAMPLE_COMMIT.to_owned(),
        }))
    );
    let dir_alone = parse_config(os("/tmp/earl"), None).expect_err("a directory without a commit");
    assert!(dir_alone.contains(COMMIT_VAR), "{dir_alone}");
    let commit_alone = parse_config(None, os(SAMPLE_COMMIT)).expect_err("a commit without a dir");
    assert!(commit_alone.contains(DIR_VAR), "{commit_alone}");
    for bad in [
        "",
        "abc123",
        &SAMPLE_COMMIT[..39],
        &format!("{SAMPLE_COMMIT}0"),
        &SAMPLE_COMMIT.to_uppercase(),
        "0123456789abcdef0123456789abcdef0123456g",
    ] {
        let error = parse_config(os("/tmp/earl"), os(bad)).expect_err(bad);
        assert!(error.contains(COMMIT_VAR), "{bad}: {error}");
    }
    assert!(parse_config(os(""), os(SAMPLE_COMMIT)).is_err());
}

/// Writing: a report lands under the directory it is given, and a failure to
/// write is an error, not a skipped file.
pub fn check_write_errors() {
    let scratch = purrdf_testkit::temp_dir!().expect("a scratch directory");
    let written = write_report(&scratch.path().join("nested"), "report.ttl", b"x\n")
        .expect("a writable directory is written to");
    assert_eq!(std::fs::read(&written).expect("the file exists"), b"x\n");
    // A plain file where the directory should be: the directory cannot be made.
    let blocker = scratch.path().join("blocker");
    std::fs::write(&blocker, b"").expect("a blocking file");
    let error = write_report(&blocker, "report.ttl", b"x\n").expect_err("no directory");
    assert!(error.contains("blocker"), "{error}");
    // A directory where the file should be: the file cannot be written.
    let occupied = scratch.path().join("occupied");
    std::fs::create_dir_all(occupied.join("report.ttl")).expect("a blocking directory");
    let error = write_report(&occupied, "report.ttl", b"x\n").expect_err("no file");
    assert!(error.contains("report.ttl"), "{error}");
}

/// The key port, branch by branch: the page's own URN form, a URL under the tests
/// folder, a foreign URL with a `/tests/` segment, and an IRI it leaves alone.
pub fn check_test_key() {
    let base = "file:///x/vectors/shacl12/tests/";
    assert_eq!(
        test_key("urn:x-shacl-test:/core/node/in-003", base),
        "core/node/in-003"
    );
    assert_eq!(
        test_key("urn:x-shacl-test:///core/node/in-003", base),
        "core/node/in-003"
    );
    assert_eq!(
        test_key(&format!("{base}core/node/in-003"), base),
        "core/node/in-003"
    );
    assert_eq!(
        test_key("https://example.org/suite/tests/core/node/in-003", base),
        "core/node/in-003"
    );
    assert_eq!(
        test_key("https://example.org/elsewhere", base),
        "https://example.org/elsewhere"
    );
    assert_eq!(test_key(&test_iri("a/b"), base), "a/b");
}

/// The mapping from a harness grade to a record, and from records to outcomes:
/// a pass is `Compared` against `Compared`, a failure is a `Mismatch` carrying the
/// error text, and neither is ever `cantTell` or `untested`.
pub fn check_record_mapping(suite: &Suite) {
    let pass = record(suite.profile, "core/node/in-001", &Ok(()));
    assert!(pass.passed);
    assert_eq!(pass.expected.kind, OutcomeKind::Compared);
    assert_eq!(pass.observed.kind, OutcomeKind::Compared);
    assert_eq!(pass.case, "urn:x-shacl-test:/core/node/in-001");
    let fail = record(
        suite.profile,
        "core/node/in-002",
        &Err("it differs".to_owned()),
    );
    assert!(!fail.passed);
    assert_eq!(fail.expected.kind, OutcomeKind::Compared);
    assert_eq!(fail.observed.kind, OutcomeKind::Mismatch);
    assert_eq!(fail.observed.message, "it differs");
    let spec = "SHACL 1.2 Core: a clause";
    let ok = Ok(());
    let err = Err("it differs".to_owned());
    let entries = [
        Entry {
            id: "a/pass",
            result: &ok,
            rationale: None,
        },
        Entry {
            id: "a/fail",
            result: &err,
            rationale: Some(spec),
        },
    ];
    let bytes = earl_bytes(&report(suite, SAMPLE_COMMIT, &entries)).expect("renders");
    let written = assertions_by_key(&bytes).expect("parses");
    assert_eq!(written["a/pass"].outcome, earl::PASSED);
    assert_eq!(written["a/pass"].info, None);
    assert_eq!(written["a/fail"].outcome, earl::FAILED);
    // `earl:info` is the rationale text only: not the engine's error.
    assert_eq!(
        written["a/fail"].info.as_deref(),
        Some(literal(spec).as_str())
    );
    let text = String::from_utf8(bytes).expect("UTF-8");
    assert!(!text.contains("it differs"));
    // `earl:info` on an assertion (not its result) is refused by the reader.
    let misplaced = format!("{text}_:assertion0 {} \"x\" .\n", angle(earl::INFO));
    assert!(assertions_by_key(misplaced.as_bytes()).is_err());
}

/// The ledger classification, each refusal beside the neighbouring accepted case.
pub fn check_classification() {
    let ok: Result<(), String> = Ok(());
    let err: Result<(), String> = Err("boom".to_owned());
    assert_eq!(classify("a", &ok, None, None), Ok(Class::Passed));
    assert_eq!(classify("a", &err, Some("why"), None), Ok(Class::Xfailed));
    assert_eq!(
        classify("a", &err, None, Some("why")),
        Ok(Class::SpecContradicted)
    );
    assert!(
        classify("a", &err, None, None)
            .unwrap_err()
            .starts_with("FAIL [a]")
    );
    assert!(
        classify("a", &ok, Some("why"), None)
            .unwrap_err()
            .starts_with("XPASS [a]")
    );
    let xpass = classify("a", &ok, None, Some("why")).unwrap_err();
    assert!(
        xpass.starts_with("XPASS [a]") && xpass.contains("SPEC_CONTRADICTED"),
        "{xpass}"
    );
    assert!(classify("a", &err, Some("x"), Some("y")).is_err());

    let known = |id: &str| id == "a" || id == "b";
    assert_eq!(check_spec_table(&[], 0, known), Ok(()));
    assert_eq!(
        check_spec_table(&[("a", "why"), ("b", "why")], 2, known),
        Ok(())
    );
    assert!(
        check_spec_table(&[("a", "why")], 0, known).is_err(),
        "the count is pinned"
    );
    assert!(
        check_spec_table(&[("c", "why")], 1, known)
            .unwrap_err()
            .contains("unknown")
    );
    assert!(
        check_spec_table(&[("a", "x"), ("a", "y")], 2, known)
            .unwrap_err()
            .contains("twice")
    );
    assert!(
        check_spec_table(&[("a", " ")], 1, known)
            .unwrap_err()
            .contains("rationale")
    );
}

/// The scoreboard-agreement check passes on the real report and fails on each of
/// three mutations of the serialized report alone: one outcome flipped, one
/// assertion removed, one assertion added.
pub fn check_agreement_and_mutations(
    serialized: &[u8],
    printed: &str,
    label: &str,
    failed_label: &str,
    tally: &BTreeMap<String, bool>,
) {
    agree(serialized, printed, label, failed_label, tally)
        .unwrap_or_else(|error| panic!("the EARL disagrees with the scoreboard: {error}"));
    for (name, mutated) in [
        ("flip", mutate_flip(serialized)),
        ("remove", mutate_remove(serialized)),
        ("add", mutate_add(serialized)),
    ] {
        assert_ne!(mutated, serialized, "{name} changes the report");
        assert!(
            agree(&mutated, printed, label, failed_label, tally).is_err(),
            "the {name} mutation of the EARL must disagree with the scoreboard"
        );
    }
}
