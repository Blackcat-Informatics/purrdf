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

/// Every subcommand the binary carries, in `Command`'s declaration order.
///
/// The list is exhaustive on purpose: a capability the top-level help does not list is a
/// capability no operator will find, which is indistinguishable from not shipping it. This
/// pinned list is what keeps a new verb from being added to the tree and left off the surface
/// an installed consumer actually meets.
const SUBCOMMANDS: [&str; 12] = [
    "convert",
    "query",
    "update",
    "reason",
    "entails",
    "consistency",
    "validate",
    "shex",
    "describe",
    "project",
    "lift",
    "pack",
];

#[test]
fn top_level_help_lists_all_subcommands() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0, "`--help` exits 0");
    for subcommand in SUBCOMMANDS {
        assert!(
            stdout.contains(subcommand),
            "top-level help must list `{subcommand}`; got:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("--loss-ledger"),
        "top-level help must mention the global --loss-ledger flag"
    );
    assert!(
        stdout.contains("--jsonld-options"),
        "top-level help must mention the global --jsonld-options flag"
    );
}

/// Every subcommand answers its OWN `--help` with exit 0.
///
/// A verb in the tree whose help panics or errors is not a shipped surface; this is the cheap
/// total check that each one is at least reachable and self-describing.
#[test]
fn every_subcommand_has_its_own_help() {
    for subcommand in SUBCOMMANDS {
        let (code, stdout, stderr) = run(&[subcommand, "--help"]);
        assert_eq!(code, 0, "`{subcommand} --help` exits 0; stderr:\n{stderr}");
        assert!(
            !stdout.is_empty(),
            "`{subcommand} --help` must describe itself"
        );
    }
}

/// `validate --help` names every input, the output-format choices, and the five governors.
///
/// The `--format` enumeration is the one an operator has to read to learn that SARIF is
/// available at all, and the governor list is what makes the ceilings settable; a flag the
/// help does not name is a ceiling nobody sets.
#[test]
fn validate_help_lists_its_inputs_formats_and_governors() {
    let (code, stdout, _) = run(&["validate", "--help"]);
    assert_eq!(code, 0, "`validate --help` exits 0");
    for option in [
        "--shapes",
        "--shapes-from",
        "--shapes-graph",
        "--from",
        "--base",
        "--format",
        "--fuel",
        "--deadline",
        "--max-intermediate-cells",
        "--max-scratch-bytes",
        "--max-remote-requests",
        "IN",
        "OUT",
    ] {
        assert!(
            stdout.contains(option),
            "validate help must list `{option}` (IN/OUT are positional); got:\n{stdout}"
        );
    }
    for choice in ["ntriples", "turtle", "rdfxml", "jsonld", "sarif"] {
        assert!(
            stdout.contains(choice),
            "validate help must list the `{choice}` output format; got:\n{stdout}"
        );
    }
    assert!(
        !stdout.contains("--max-answers"),
        "a validation has no answer sequence to bound: {stdout}"
    );
    assert!(
        stdout.contains("exits 3") || stdout.contains("exit"),
        "the help must say what a trip does to the exit code; got:\n{stdout}"
    );
}

/// `shex --help` names the schema, the data, the imports, the map and the syntax choices.
#[test]
fn shex_help_lists_its_inputs_and_schema_syntaxes() {
    let (code, stdout, _) = run(&["shex", "--help"]);
    assert_eq!(code, 0, "`shex --help` exits 0");
    for option in [
        "--schema",
        "--schema-from",
        "--import",
        "--data",
        "--from",
        "--base",
        "MAP",
        "OUT",
    ] {
        assert!(
            stdout.contains(option),
            "shex help must list `{option}` (MAP/OUT are positional); got:\n{stdout}"
        );
    }
    for choice in ["shexc", "shexj"] {
        assert!(
            stdout.contains(choice),
            "shex help must list the `{choice}` schema syntax; got:\n{stdout}"
        );
    }
}

/// `describe --help` names the subject flag and the ordinary source/target plumbing.
#[test]
fn describe_help_lists_its_subject_and_format_flags() {
    let (code, stdout, _) = run(&["describe", "--help"]);
    assert_eq!(code, 0, "`describe --help` exits 0");
    for option in ["--iri", "--from", "--to", "--base", "IN", "OUT"] {
        assert!(
            stdout.contains(option),
            "describe help must list `{option}` (IN/OUT are positional); got:\n{stdout}"
        );
    }
    for choice in ["turtle", "ntriples", "jsonld", "pack"] {
        assert!(
            stdout.contains(choice),
            "describe help must list the `{choice}` format choice; got:\n{stdout}"
        );
    }
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
        "okf-terms",
        "obo-graphs",
        "skos",
        "croissant-1.1",
        "ro-crate-1.3",
        "datacite-4.6",
        "dcat-3",
        "dcat-rdf",
        "void",
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
    assert!(!lift.contains("okf-terms"));
    assert!(!lift.contains("dcat-rdf"));
    assert!(!lift.contains("void"));
    for field in ["--profile", "--config", "--to", "IN", "OUT"] {
        assert!(lift.contains(field), "lift help missing `{field}`");
    }
}

#[test]
fn convert_help_lists_options_and_format_choices() {
    let (code, stdout, _) = run(&["convert", "--help"]);
    assert_eq!(code, 0, "`convert --help` exits 0");
    // `--input` and `--transport` are the two admission flags: a multi-source list and a
    // gzip/zstd wrapper are capabilities no operator finds if the help does not name them.
    for option in ["--from", "--to", "--input", "--transport", "IN", "OUT"] {
        assert!(
            stdout.contains(option),
            "convert help must list `{option}` (IN/OUT are positional); got:\n{stdout}"
        );
    }
    for choice in [
        "turtle", "ntriples", "nquads", "rdfxml", "jsonld", "yamlld", "pack", "gts",
    ] {
        assert!(
            stdout.contains(choice),
            "convert help must list the `{choice}` format choice; got:\n{stdout}"
        );
    }
    for encoding in ["auto", "none", "gzip", "zstd"] {
        assert!(
            stdout.contains(encoding),
            "convert help must list the `{encoding}` transport choice; got:\n{stdout}"
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

/// `query --help` lists every execution governor, plus `--explain`.
///
/// A governor the help does not name is a ceiling no operator will set, which is the same
/// as not having it. The help must also say what `--deadline` accepts (its value is the one
/// flag whose grammar cannot be guessed from a type name) and what a trip does, since the
/// exit code is the only thing a shell can test.
#[test]
fn query_help_lists_every_execution_governor() {
    let (code, stdout, _) = run(&["query", "--help"]);
    assert_eq!(code, 0, "`query --help` exits 0");
    for option in [
        "--fuel",
        "--deadline",
        "--max-answers",
        "--max-intermediate-cells",
        "--max-scratch-bytes",
        "--max-remote-requests",
        "--explain",
    ] {
        assert!(
            stdout.contains(option),
            "query help must list `{option}`; got:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("1m30s"),
        "the deadline's help must show the accepted spelling; got:\n{stdout}"
    );
    assert!(
        stdout.contains("exits 3"),
        "the help must say what a trip does to the exit code; got:\n{stdout}"
    );
}

#[test]
fn update_help_lists_applicable_governors_and_no_answer_cap() {
    let (code, stdout, _) = run(&["update", "--help"]);
    assert_eq!(code, 0, "`update --help` exits 0");
    for option in [
        "--fuel",
        "--deadline",
        "--max-intermediate-cells",
        "--max-scratch-bytes",
        "--max-remote-requests",
    ] {
        assert!(
            stdout.contains(option),
            "update help must list `{option}`; got:\n{stdout}"
        );
    }
    assert!(
        !stdout.contains("--max-answers"),
        "UPDATE has no answer sequence to bound: {stdout}"
    );
    assert!(stdout.contains("exits 3"), "{stdout}");
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

/// `entails --help` lists the flags that reach the conclusion-directed boundary.
///
/// Every one of them is a boundary PARAMETER or a service SELECTOR: `--regime`,
/// `--premise` and `--import` are the parameter list, and `--conclusion` / `--verify` /
/// `--pattern` pick which of the three services answers. A flag missing here is a
/// capability the binary cannot reach, which is exactly what
/// `scripts/check-entailment-surface.py` gates against from the other side.
#[test]
fn entails_help_lists_the_boundary_flags_and_regime_choices() {
    let (code, stdout, _) = run(&["entails", "--help"]);
    assert_eq!(code, 0, "`entails --help` exits 0");
    for option in [
        "--regime",
        "--premise",
        "--conclusion",
        "--pattern",
        "--verify",
        "--import",
        "--report",
        "--from",
        "OUT",
    ] {
        assert!(
            stdout.contains(option),
            "entails help must list `{option}` (OUT is positional); got:\n{stdout}"
        );
    }
    for choice in ["simple", "rdf", "rdfs", "owl-rl", "d"] {
        assert!(
            stdout.contains(choice),
            "entails help must list the `{choice}` regime; got:\n{stdout}"
        );
    }
}

#[test]
fn unknown_flag_is_a_usage_error_exit_2() {
    let (code, _, stderr) = run(&["convert", "--nonexistent-flag"]);
    assert_eq!(code, 2, "clap rejects an unknown flag with exit 2");
    assert!(!stderr.is_empty(), "clap prints a diagnostic to stderr");
}
