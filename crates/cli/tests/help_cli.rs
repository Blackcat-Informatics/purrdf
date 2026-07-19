// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `--help` surface: the built `purrdf` binary lists every subcommand, and each
//! subcommand's `--help` lists its options and the `ValueEnum` choices. Drives the
//! real binary via `CARGO_BIN_EXE_purrdf`.

use std::process::Command;

/// The path to the built `purrdf` binary this integration test target links against.
const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");

/// Run `purrdf` with `args`, returning (exit-code, stdout, stderr).
fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(PURRDF)
        .args(args)
        .output()
        .expect("spawn purrdf");
    (
        output.status.code().expect("exit code"),
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

#[test]
fn top_level_help_lists_all_subcommands() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0, "`--help` exits 0");
    for subcommand in ["convert", "query", "reason", "project", "lift"] {
        assert!(
            stdout.contains(subcommand),
            "top-level help must list `{subcommand}`; got:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("--loss-ledger"),
        "top-level help must mention the global --loss-ledger flag"
    );
}

#[test]
fn project_and_lift_help_enumerate_truthful_profiles() {
    let (code, project, _) = run(&["project", "--help"]);
    assert_eq!(code, 0, "`project --help` exits 0");
    for value in [
        "lpg-csv",
        "neo4j-csv",
        "open-cypher",
        "graphml",
        "csvw-exact",
        "csvw-terms",
        "obo-graphs",
        "skos",
        "croissant-1.1",
        "ro-crate-1.3",
        "datacite-4.6",
        "dcat-3",
        "frictionless-data-package-1",
    ] {
        assert!(
            project.contains(value),
            "project help must enumerate `{value}`; got:\n{project}"
        );
    }
    for field in ["--profile", "--config", "--from", "IN", "OUT"] {
        assert!(project.contains(field), "project help missing `{field}`");
    }

    let (code, lift, _) = run(&["lift", "--help"]);
    assert_eq!(code, 0, "`lift --help` exits 0");
    for value in [
        "lpg-csv",
        "neo4j-csv",
        "open-cypher",
        "graphml",
        "csvw-exact",
        "croissant-1.1",
        "ro-crate-1.3",
        "datacite-4.6",
        "dcat-3",
        "frictionless-data-package-1",
    ] {
        assert!(
            lift.contains(value),
            "lift help must enumerate `{value}`; got:\n{lift}"
        );
    }
    assert!(!lift.contains("obo-graphs"));
    assert!(!lift.contains("skos"));
    assert!(!lift.contains("csvw-terms"));
    for field in ["--profile", "--config", "--to", "IN", "OUT"] {
        assert!(lift.contains(field), "lift help missing `{field}`");
    }
}

#[test]
fn convert_help_lists_options_and_format_choices() {
    let (code, stdout, _) = run(&["convert", "--help"]);
    assert_eq!(code, 0, "`convert --help` exits 0");
    for option in ["--from", "--to", "IN", "OUT"] {
        assert!(
            stdout.contains(option),
            "convert help must list `{option}` (IN/OUT are positional); got:\n{stdout}"
        );
    }
    for choice in [
        "turtle", "ntriples", "nquads", "rdfxml", "jsonld", "yamlld", "pack",
    ] {
        assert!(
            stdout.contains(choice),
            "convert help must list the `{choice}` format choice; got:\n{stdout}"
        );
    }
}

#[test]
fn query_help_lists_options_and_results_choices() {
    let (code, stdout, _) = run(&["query", "--help"]);
    assert_eq!(code, 0, "`query --help` exits 0");
    for option in ["--data", "--results-format"] {
        assert!(
            stdout.contains(option),
            "query help must list `{option}`; got:\n{stdout}"
        );
    }
    for choice in ["json", "xml", "csv", "tsv"] {
        assert!(
            stdout.contains(choice),
            "query help must list the `{choice}` results format; got:\n{stdout}"
        );
    }
}

#[test]
fn reason_help_lists_options_and_regime_choices() {
    let (code, stdout, _) = run(&["reason", "--help"]);
    assert_eq!(code, 0, "`reason --help` exits 0");
    for option in ["--regime", "IN", "OUT"] {
        assert!(
            stdout.contains(option),
            "reason help must list `{option}` (IN/OUT are positional); got:\n{stdout}"
        );
    }
    for choice in ["simple", "rdfs", "owl-rl", "owl-direct"] {
        assert!(
            stdout.contains(choice),
            "reason help must list the `{choice}` regime; got:\n{stdout}"
        );
    }
}

#[test]
fn unknown_flag_is_a_usage_error_exit_2() {
    let (code, _, stderr) = run(&["convert", "--nonexistent-flag"]);
    assert_eq!(code, 2, "clap rejects an unknown flag with exit 2");
    assert!(!stderr.is_empty(), "clap prints a diagnostic to stderr");
}
