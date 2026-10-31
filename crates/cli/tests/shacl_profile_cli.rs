// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf validate --shacl-profile ID` over the BUILT binary: the exact dated SHACL law is
//! selectable on the CLI, on the whole-graph route, the `--changes` route and the
//! `--shapes-product` restore.
//!
//! Every refusal here is paired with its valid neighbour, so a refusal is shown to be the
//! law's and not the flag's:
//!
//! * `sparql/pre-binding/pre-binding-009` (a SPARQL constraint with `VALUES` over a pre-bound
//!   variable): the compatibility law refuses the shapes graph at load; the editor's-draft law
//!   (`shacl-ed-2026-10-16`) validates it.
//! * `sparql/node/prefixes-003` (a prefix declared through the version-IRI path, which the
//!   2026-09-18 working draft does not define): `shacl-wd-2026-09-18` refuses it at load; the
//!   editor's-draft law validates it.
//! * `shacl-ed-2026-10-17` names no law: the refusal is `complete_profile`'s
//!   `unsupported-profile` status, and the id is not mapped to a neighbouring edition.
//! * a dated law requires its own XPath bundle: `--xpath-regex` naming another law is a
//!   usage error, and naming the same law is accepted.
//!
//! A prepared product stores no SHACL law, so the `--shapes-product` restore takes it from the
//! request. `shacl pack` parses under the compatibility law, so a product cannot be made from
//! `pre-binding-009` (which that law refuses); `prefixes-003` is the packable entry whose
//! answer depends on the law.

use std::path::PathBuf;

use purrdf_shapes::shacl_corpora::shacl12::shacl12_root;

mod support;
use support::{code, run, stderr, write_file};

const ED: &str = "shacl-ed-2026-10-16";
const WD: &str = "shacl-wd-2026-09-18";
const XPATH_20: &str = "xpath-2.0-2010-12-14";
const XPATH_31: &str = "xpath-3.1-2017-03-21";

/// A vendored SHACL 1.2 suite file, as the string path the binary takes.
fn suite_file(relative: &str) -> String {
    let path: PathBuf = shacl12_root().join(relative);
    path.to_str().expect("utf8 corpus path").to_owned()
}

/// `purrdf validate --shapes FILE [--shacl-profile ID] FILE -`, the entry validating its own
/// file as shapes and data, as the suite's manifests do.
fn validate_entry(file: &str, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["validate", "--shapes", file];
    args.extend_from_slice(extra);
    args.extend_from_slice(&[file, "-"]);
    run(&args)
}

#[test]
fn pre_binding_009_is_refused_at_load_by_the_default_law_and_validated_by_the_editors_draft() {
    let file = suite_file("sparql/pre-binding/pre-binding-009.ttl");
    let default = validate_entry(&file, &[]);
    assert_eq!(code(&default), 1, "{}", stderr(&default));
    assert!(
        stderr(&default).contains("VALUES is not allowed"),
        "{}",
        stderr(&default)
    );
    let legacy_spelled = validate_entry(&file, &["--shacl-profile", "legacy"]);
    assert_eq!(code(&legacy_spelled), 1, "{}", stderr(&legacy_spelled));

    let ed = validate_entry(&file, &["--shacl-profile", ED]);
    assert_eq!(code(&ed), 0, "{}", stderr(&ed));
    assert!(stderr(&ed).contains("shacl results 1"), "{}", stderr(&ed));
}

#[test]
fn prefixes_003_is_refused_at_load_by_the_working_draft_and_validated_by_the_editors_draft() {
    let file = suite_file("sparql/node/prefixes-003.ttl");
    let wd = validate_entry(&file, &["--shacl-profile", WD]);
    assert_eq!(code(&wd), 1, "{}", stderr(&wd));
    assert!(stderr(&wd).contains("undeclared prefix"), "{}", stderr(&wd));

    let ed = validate_entry(&file, &["--shacl-profile", ED]);
    assert_eq!(code(&ed), 0, "{}", stderr(&ed));
    assert!(stderr(&ed).contains("shacl results 1"), "{}", stderr(&ed));
}

#[test]
fn an_unknown_profile_id_is_the_unsupported_profile_status_and_no_edition_is_guessed() {
    let file = suite_file("sparql/node/prefixes-003.ttl");
    for id in ["shacl-ed-2026-10-17", "SHACL-ED-2026-10-16", "shacl-ed", ""] {
        let refused = validate_entry(&file, &["--shacl-profile", id]);
        assert_eq!(code(&refused), 2, "{id:?}: {}", stderr(&refused));
        let text = stderr(&refused);
        assert!(text.contains("unsupported-profile"), "{id:?}: {text}");
        assert!(text.contains(&format!("{id:?}")), "{id:?}: {text}");
    }
    let accepted = validate_entry(&file, &["--shacl-profile", ED]);
    assert_eq!(code(&accepted), 0, "{}", stderr(&accepted));
}

#[test]
fn a_dated_law_refuses_an_xpath_override_of_another_law_and_accepts_its_own() {
    let file = suite_file("sparql/node/prefixes-003.ttl");
    let conflict = validate_entry(&file, &["--shacl-profile", ED, "--xpath-regex", XPATH_20]);
    assert_eq!(code(&conflict), 2, "{}", stderr(&conflict));
    let text = stderr(&conflict);
    assert!(text.contains(ED) && text.contains(XPATH_20), "{text}");
    assert!(text.contains("requires"), "{text}");

    let same = validate_entry(&file, &["--shacl-profile", ED, "--xpath-regex", XPATH_31]);
    assert_eq!(code(&same), 0, "{}", stderr(&same));
    // The 2017 Recommendation requires XPath 2.0: the same pair, the other way round.
    let rec = validate_entry(
        &suite_file("sparql/pre-binding/pre-binding-001.ttl"),
        &[
            "--shacl-profile",
            "shacl-rec-2017-07-20",
            "--xpath-regex",
            XPATH_31,
        ],
    );
    assert_eq!(code(&rec), 2, "{}", stderr(&rec));
    // The compatibility law leaves the pattern law to the caller, as it always did.
    let legacy = validate_entry(&file, &["--xpath-regex", XPATH_20]);
    assert_eq!(code(&legacy), 0, "{}", stderr(&legacy));
}

#[test]
fn the_changes_lane_takes_the_law_from_the_request() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let file = suite_file("sparql/pre-binding/pre-binding-009.ttl");
    let change = write_file(
        dir.path(),
        "change.ttl",
        "<http://example.org/extra> <http://example.org/p> 1 .\n",
    );
    let refused = run(&[
        "validate",
        "--shapes",
        &file,
        "--changes",
        &change,
        &file,
        "-",
    ]);
    assert_eq!(code(&refused), 1, "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("VALUES is not allowed"),
        "{}",
        stderr(&refused)
    );

    let ed = run(&[
        "validate",
        "--shacl-profile",
        ED,
        "--shapes",
        &file,
        "--changes",
        &change,
        &file,
        "-",
    ]);
    assert_eq!(code(&ed), 0, "{}", stderr(&ed));
    assert!(
        stderr(&ed).contains("shacl change-expansion"),
        "{}",
        stderr(&ed)
    );

    let unknown = run(&[
        "validate",
        "--shacl-profile",
        "shacl-ed-2026-10-17",
        "--shapes",
        &file,
        "--changes",
        &change,
        &file,
        "-",
    ]);
    assert_eq!(code(&unknown), 2, "{}", stderr(&unknown));
}

#[test]
fn a_governed_run_takes_the_law_from_the_request() {
    let file = suite_file("sparql/pre-binding/pre-binding-009.ttl");
    let governed = validate_entry(&file, &["--shacl-profile", ED, "--fuel", "1000000"]);
    assert_eq!(code(&governed), 0, "{}", stderr(&governed));
    assert!(
        stderr(&governed).contains("shacl results 1"),
        "{}",
        stderr(&governed)
    );
}

#[test]
fn a_product_restore_takes_the_law_from_the_request() {
    let canonical_receipt_dir = purrdf_testkit::temp_dir!().expect("canonical receipt directory");
    let canonical_receipt_1 = canonical_receipt_dir.path().join("request-1.json");

    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let file = suite_file("sparql/node/prefixes-003.ttl");
    let product = dir.path().join("prefixes-003.purrshp");
    let product = product.to_str().expect("utf8 path");
    let packed = run(&[
        "shacl",
        "pack",
        "--shapes",
        &file,
        "--out",
        product,
        "--canonical-search",
        "unlimited",
        "--canonicalization-receipts",
        canonical_receipt_1
            .to_str()
            .expect("UTF-8 canonical receipt path"),
    ]);
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));

    let restore = |extra: &[&str]| {
        let mut args = vec!["validate", "--shapes-product", product];
        args.extend_from_slice(extra);
        args.extend_from_slice(&[&file, "-"]);
        run(&args)
    };
    // One product, three requests: the default is unchanged, the working draft refuses the
    // restored shapes at admission, and the editor's draft validates them.
    let default = restore(&[]);
    assert_eq!(code(&default), 0, "{}", stderr(&default));
    let wd = restore(&["--shacl-profile", WD]);
    assert_eq!(code(&wd), 1, "{}", stderr(&wd));
    assert!(stderr(&wd).contains("undeclared prefix"), "{}", stderr(&wd));
    let ed = restore(&["--shacl-profile", ED]);
    assert_eq!(code(&ed), 0, "{}", stderr(&ed));
    assert!(stderr(&ed).contains("shacl results 1"), "{}", stderr(&ed));
    // The law is not part of the product, so the report is the same under the default law.
    assert_eq!(
        ed.stdout, default.stdout,
        "the restored product answers alike under the compatibility and editor's-draft laws here"
    );

    let unknown = restore(&["--shacl-profile", "shacl-ed-2026-10-17"]);
    assert_eq!(code(&unknown), 2, "{}", stderr(&unknown));
    assert!(
        stderr(&unknown).contains("unsupported-profile"),
        "{}",
        stderr(&unknown)
    );
    let conflict = restore(&["--shacl-profile", ED, "--xpath-regex", XPATH_20]);
    assert_eq!(code(&conflict), 2, "{}", stderr(&conflict));
}
