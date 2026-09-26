// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The runner, observed from outside: the fixture binary (three passing
//! cases, one panicking, one ignored) runs as a child process, and its exit
//! status and console output are compared with libtest's. The command-line
//! parser and the in-process runner are checked directly as well.

use std::process::{Command, Output};

use purrdf_testkit::harness::{
    self, Action, Arguments, ColorChoice, Conclusion, ERROR_EXIT_CODE, Failed, Format, RunIgnored,
    Trial,
};

const FIXTURE: &str = env!("CARGO_BIN_EXE_testkit-harness-fixture");

fn run_fixture(args: &[&str]) -> Output {
    Command::new(FIXTURE)
        .args(args)
        .env_remove("RUST_TEST_THREADS")
        .env_remove("RUST_TEST_NOCAPTURE")
        .output()
        .expect("run the fixture harness")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

/// The output with the run's duration replaced by `S.SS`.
fn without_duration(text: &str) -> String {
    let marker = "; finished in ";
    let Some(start) = text.find(marker) else {
        return text.to_owned();
    };
    let tail = &text[start + marker.len()..];
    let end = tail.find('s').expect("a duration ends in `s`");
    let duration = &tail[..end];
    let (whole, fraction) = duration.split_once('.').expect("two decimals");
    assert!(
        whole.bytes().all(|b| b.is_ascii_digit()) && !whole.is_empty(),
        "{duration}"
    );
    assert!(
        fraction.len() == 2 && fraction.bytes().all(|b| b.is_ascii_digit()),
        "{duration}"
    );
    format!("{}{marker}S.SS{}", &text[..start], &tail[end..])
}

#[test]
fn the_fixture_reports_libtest_output_serially_and_exits_non_zero() {
    let output = run_fixture(&["--test-threads=1"]);
    assert_eq!(output.status.code(), Some(101), "a failed case exits 101");
    let stdout = without_duration(&stdout_of(&output));
    let expected_head = "\nrunning 5 tests\n\
        test alpha_passes ... ok\n\
        test beta_passes ... ok\n\
        test delta_panics ... FAILED\n\
        test epsilon_is_ignored ... ignored\n\
        test gamma_passes ... ok\n\
        \nfailures:\n\n\
        ---- delta_panics stdout ----\n\
        thread 'delta_panics' panicked at ";
    assert!(
        stdout.starts_with(expected_head),
        "unexpected output:\n{stdout}"
    );
    let expected_tail = ":\nthe planted failure\n\n\
        \nfailures:\n    delta_panics\n\
        \ntest result: FAILED. 3 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in S.SSs\n\n";
    assert!(
        stdout.ends_with(expected_tail),
        "unexpected output:\n{stdout}"
    );
    assert!(
        stdout.contains("src/bin/harness_fixture.rs:"),
        "the panic location is reported:\n{stdout}"
    );
}

#[test]
fn the_parallel_run_has_the_same_lines_and_tally() {
    let output = run_fixture(&["--test-threads=4"]);
    assert_eq!(output.status.code(), Some(101));
    let stdout = without_duration(&stdout_of(&output));
    let mut case_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("test ") && line.contains(" ... "))
        .collect();
    case_lines.sort_unstable();
    assert_eq!(
        case_lines,
        [
            "test alpha_passes ... ok",
            "test beta_passes ... ok",
            "test delta_panics ... FAILED",
            "test epsilon_is_ignored ... ignored",
            "test gamma_passes ... ok",
        ]
    );
    assert!(stdout.contains(
        "\ntest result: FAILED. 3 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in S.SSs\n"
    ));
}

#[test]
fn the_tally_line_is_the_one_the_conformance_scraper_reads() {
    // scripts/conformance-matrix.py sums
    // `test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored`.
    let stdout = stdout_of(&run_fixture(&["--test-threads=1"]));
    let line = stdout
        .lines()
        .find(|line| line.starts_with("test result: "))
        .expect("a tally line");
    let rest = line
        .strip_prefix("test result: FAILED. ")
        .expect("status word then `. `");
    let fields: Vec<&str> = rest.split("; ").collect();
    assert_eq!(
        &fields[..5],
        [
            "3 passed",
            "1 failed",
            "1 ignored",
            "0 measured",
            "0 filtered out"
        ]
    );
}

#[test]
fn an_unknown_flag_is_refused_and_a_known_one_runs_every_case() {
    let refused = run_fixture(&["--frobnicate"]);
    assert_eq!(
        refused.status.code(),
        Some(101),
        "an unknown flag exits 101"
    );
    assert!(refused.stdout.is_empty(), "nothing runs");
    assert_eq!(
        String::from_utf8_lossy(&refused.stderr),
        "error: Unrecognized option: 'frobnicate'\n"
    );

    let accepted = run_fixture(&["--test-threads=1"]);
    let stdout = stdout_of(&accepted);
    assert!(stdout.contains("\nrunning 5 tests\n"), "{stdout}");
    assert!(stdout.contains("3 passed; 1 failed; 1 ignored"), "{stdout}");
}

#[test]
fn filters_skip_and_exact_select_cases_and_count_the_rest_as_filtered_out() {
    let output = run_fixture(&["passes", "--skip", "beta", "--test-threads", "1"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "no failing case was selected"
    );
    let stdout = without_duration(&stdout_of(&output));
    assert_eq!(
        stdout,
        "\nrunning 2 tests\ntest alpha_passes ... ok\ntest gamma_passes ... ok\n\
         \ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in S.SSs\n\n"
    );

    let exact = run_fixture(&["--exact", "alpha", "--test-threads=1"]);
    assert!(stdout_of(&exact).contains("\nrunning 0 tests\n"));
    let exact = run_fixture(&["--exact", "alpha_passes", "--test-threads=1"]);
    assert!(
        stdout_of(&exact).contains("1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out")
    );
}

#[test]
fn ignored_and_include_ignored_run_the_ignored_case() {
    let only = run_fixture(&["--ignored", "--test-threads=1"]);
    assert_eq!(
        only.status.code(),
        Some(101),
        "the ignored case panics when run"
    );
    let stdout = stdout_of(&only);
    assert!(
        stdout.contains("test epsilon_is_ignored ... FAILED\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out"),
        "{stdout}"
    );

    let all = run_fixture(&["--include-ignored", "--test-threads=1"]);
    assert!(stdout_of(&all).contains("3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out"));

    let both = run_fixture(&["--ignored", "--include-ignored"]);
    assert_eq!(
        both.status.code(),
        Some(101),
        "the two are mutually exclusive"
    );
}

#[test]
fn list_prints_names_in_libtest_form() {
    let pretty = run_fixture(&["--list"]);
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(
        stdout_of(&pretty),
        "alpha_passes: test\nbeta_passes: test\ndelta_panics: test\nepsilon_is_ignored: test\n\
         gamma_passes: test\n\n5 tests, 0 benchmarks\n"
    );
    let terse = run_fixture(&["--list", "--format", "terse"]);
    assert_eq!(
        stdout_of(&terse),
        "alpha_passes: test\nbeta_passes: test\ndelta_panics: test\nepsilon_is_ignored: test\n\
         gamma_passes: test\n"
    );
    let ignored = run_fixture(&["--list", "--ignored"]);
    assert_eq!(
        stdout_of(&ignored),
        "epsilon_is_ignored: test\n\n1 test, 0 benchmarks\n"
    );
}

#[test]
fn terse_prints_one_character_per_case() {
    let output = run_fixture(&["-q", "--test-threads=1", "--color=never"]);
    let stdout = without_duration(&stdout_of(&output));
    assert!(
        stdout.starts_with("\nrunning 5 tests\n..Fi.\nfailures:\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with(
        "3 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in S.SSs\n\n"
    ));
}

#[test]
fn nocapture_prints_the_panic_as_it_happens() {
    let output = run_fixture(&["--nocapture", "--test-threads=1", "delta"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("thread 'delta_panics' panicked at "),
        "{stderr}"
    );
    assert!(stderr.contains("the planted failure"), "{stderr}");
    let stdout = stdout_of(&output);
    assert!(
        !stdout.contains("---- delta_panics stdout ----"),
        "{stdout}"
    );
    assert!(
        stdout.contains("\nfailures:\n    delta_panics\n"),
        "{stdout}"
    );
}

#[test]
fn show_output_lists_the_successes() {
    let output = run_fixture(&["--show-output", "--test-threads=1", "passes"]);
    let stdout = stdout_of(&output);
    assert!(
        stdout.contains(
            "\nsuccesses:\n\nsuccesses:\n    alpha_passes\n    beta_passes\n    gamma_passes\n"
        ),
        "{stdout}"
    );
}

#[test]
fn color_always_paints_the_result_words() {
    let output = run_fixture(&["--color", "always", "--test-threads=1", "alpha"]);
    let stdout = stdout_of(&output);
    assert!(
        stdout.contains("test alpha_passes ... \u{1b}[32mok\u{1b}[0m\n"),
        "{stdout:?}"
    );
}

#[test]
fn help_prints_usage_and_succeeds() {
    let output = run_fixture(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), harness::USAGE);
}

#[test]
fn the_parser_accepts_the_libtest_flag_set() {
    let parsed = Arguments::parse([
        "name",
        "--exact",
        "--skip",
        "a",
        "--skip=b",
        "--list",
        "--format=terse",
        "--ignored",
        "--nocapture",
        "--show-output",
        "--color",
        "never",
        "--test-threads",
        "3",
        "-Z",
        "unstable-options",
        "-Zunstable-options",
    ])
    .expect("every libtest flag parses");
    assert_eq!(
        parsed,
        Arguments {
            filters: vec!["name".to_owned()],
            exact: true,
            skip: vec!["a".to_owned(), "b".to_owned()],
            action: Action::List,
            format: Format::Terse,
            run_ignored: RunIgnored::Only,
            nocapture: true,
            show_output: true,
            color: ColorChoice::Never,
            test_threads: Some(3),
        }
    );
    let quiet = Arguments::parse(["-q", "--include-ignored", "--no-capture", "--", "--literal"])
        .expect("parses");
    assert_eq!(quiet.format, Format::Terse);
    assert_eq!(quiet.run_ignored, RunIgnored::Yes);
    assert!(quiet.nocapture);
    assert_eq!(quiet.filters, ["--literal"]);
    assert_eq!(
        Arguments::parse(["--list", "--help"])
            .expect("parses")
            .action,
        Action::Help,
        "help takes precedence over list"
    );
    assert_eq!(
        Arguments::parse(["-h"]).expect("parses").action,
        Action::Help
    );
    assert_eq!(
        Arguments::parse(Vec::<String>::new())
            .expect("parses")
            .action,
        Action::Run
    );
}

#[test]
fn the_parser_refuses_what_libtest_refuses_and_accepts_the_neighbours() {
    let refusals: [(&[&str], &str); 9] = [
        (&["--frobnicate"], "Unrecognized option: 'frobnicate'"),
        (&["-x"], "Unrecognized option: 'x'"),
        (
            &["--test-threads=0"],
            "argument for --test-threads must not be 0",
        ),
        (
            &["--test-threads=many"],
            "argument for --test-threads must be a number > 0",
        ),
        (
            &["--color=sometimes"],
            "argument for --color must be auto, always, or never",
        ),
        (&["--format=json"], "--format json is not implemented"),
        (&["-Z", "nightly-only"], "unknown -Z option: 'nightly-only'"),
        (&["--skip"], "Argument to option 'skip' missing"),
        (&["--list=yes"], "Option 'list' does not take an argument"),
    ];
    for (args, message) in refusals {
        let error = Arguments::parse(args.iter().copied()).expect_err("refused");
        assert!(error.to_string().starts_with(message), "{args:?}: {error}");
    }
    for accepted in [
        &["--test-threads=1"][..],
        &["--color=always"],
        &["--format=pretty"],
        &["-Z", "unstable-options"],
        &["--skip", "x"],
        &["--list"],
    ] {
        Arguments::parse(accepted.iter().copied()).expect("the valid neighbour parses");
    }
}

#[test]
fn a_returned_failure_is_reported_with_its_message() {
    let arguments = Arguments {
        test_threads: Some(1),
        ..Arguments::default()
    };
    let mut out = Vec::new();
    let conclusion = harness::run(
        &arguments,
        vec![
            Trial::test("returns_err", || Err(Failed::from("a returned failure"))),
            Trial::test("returns_ok", || Ok(())),
        ],
        &mut out,
    )
    .expect("the run completes");
    assert_eq!(
        conclusion,
        Conclusion {
            passed: 1,
            failed: 1,
            ignored: 0,
            filtered_out: 0
        }
    );
    let text = without_duration(&String::from_utf8(out).expect("utf-8"));
    assert_eq!(
        text,
        "\nrunning 2 tests\ntest returns_err ... FAILED\ntest returns_ok ... ok\n\
         \nfailures:\n\n---- returns_err stdout ----\na returned failure\n\
         \nfailures:\n    returns_err\n\
         \ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in S.SSs\n\n"
    );
}

#[test]
fn duplicate_case_names_are_refused_and_distinct_names_run() {
    let mut out = Vec::new();
    let error = harness::run(
        &Arguments::default(),
        vec![
            Trial::test("same", || Ok(())),
            Trial::test("same", || Ok(())),
        ],
        &mut out,
    )
    .expect_err("two cases with one name are refused");
    assert!(error.to_string().contains("`same`"), "{error}");
    assert!(out.is_empty(), "nothing runs");

    let conclusion = harness::run(
        &Arguments::default(),
        vec![
            Trial::test("same", || Ok(())),
            Trial::test("other", || Ok(())),
        ],
        &mut Vec::new(),
    )
    .expect("distinct names run");
    assert_eq!(conclusion.passed, 2);
}

#[test]
fn every_case_runs_across_many_threads() {
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let trials: Vec<Trial> = (0..200)
        .map(|index| {
            let counter = std::sync::Arc::clone(&counter);
            Trial::test(format!("case_{index:03}"), move || {
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                assert!(index % 50 != 7, "planted {index}");
                Ok(())
            })
        })
        .collect();
    let arguments = Arguments {
        test_threads: Some(8),
        ..Arguments::default()
    };
    let mut out = Vec::new();
    let conclusion = harness::run(&arguments, trials, &mut out).expect("the run completes");
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 200);
    assert_eq!(conclusion.passed, 196);
    assert_eq!(conclusion.failed, 4);
    let text = String::from_utf8(out).expect("utf-8");
    assert!(
        text.contains("\nfailures:\n    case_007\n    case_057\n    case_107\n    case_157\n"),
        "{text}"
    );
}

#[test]
fn a_failed_run_and_a_refused_command_line_exit_with_the_error_code() {
    let failed = run_fixture(&["--test-threads=1"]);
    assert_eq!(failed.status.code(), Some(i32::from(ERROR_EXIT_CODE)));
    let refused = run_fixture(&["--frobnicate"]);
    assert_eq!(refused.status.code(), Some(i32::from(ERROR_EXIT_CODE)));
    let passed = run_fixture(&["--test-threads=1", "passes"]);
    assert_eq!(passed.status.code(), Some(0), "the valid neighbour exits 0");
}

#[test]
fn the_environment_supplies_threads_and_nocapture_when_the_flags_are_absent() {
    // `harness::main` reads its configuration through `Arguments::from_env`,
    // so the fixture observes that constructor in a real process.
    let refused = Command::new(FIXTURE)
        .env("RUST_TEST_THREADS", "0")
        .env_remove("RUST_TEST_NOCAPTURE")
        .output()
        .expect("run the fixture harness");
    assert_eq!(refused.status.code(), Some(i32::from(ERROR_EXIT_CODE)));
    assert_eq!(
        String::from_utf8_lossy(&refused.stderr),
        "error: RUST_TEST_THREADS is `0`, should be a positive integer.\n"
    );
    assert!(refused.stdout.is_empty(), "nothing runs");

    let threaded = Command::new(FIXTURE)
        .arg("passes")
        .env("RUST_TEST_THREADS", "2")
        .env_remove("RUST_TEST_NOCAPTURE")
        .output()
        .expect("run the fixture harness");
    assert_eq!(threaded.status.code(), Some(0), "a positive count runs");

    let overridden = Command::new(FIXTURE)
        .args(["--test-threads=1", "passes"])
        .env("RUST_TEST_THREADS", "0")
        .env_remove("RUST_TEST_NOCAPTURE")
        .output()
        .expect("run the fixture harness");
    assert_eq!(
        overridden.status.code(),
        Some(0),
        "the flag takes precedence, so the variable is not read"
    );

    let nocapture = Command::new(FIXTURE)
        .args(["--test-threads=1", "delta"])
        .env("RUST_TEST_NOCAPTURE", "1")
        .env_remove("RUST_TEST_THREADS")
        .output()
        .expect("run the fixture harness");
    let stderr = String::from_utf8_lossy(&nocapture.stderr);
    assert!(stderr.contains("the planted failure"), "{stderr}");
    assert!(
        !stdout_of(&nocapture).contains("---- delta_panics stdout ----"),
        "the panic went to stderr, not the failures section"
    );

    let captured = Command::new(FIXTURE)
        .args(["--test-threads=1", "delta"])
        .env("RUST_TEST_NOCAPTURE", "0")
        .env_remove("RUST_TEST_THREADS")
        .output()
        .expect("run the fixture harness");
    assert!(
        stdout_of(&captured).contains("---- delta_panics stdout ----"),
        "`0` leaves capture on"
    );
}

#[test]
fn failed_carries_its_message_or_none() {
    assert_eq!(Failed::without_message().message(), None);
    assert_eq!(Failed::from("boom").message(), Some("boom"));
    assert_eq!(Failed::from(42).message(), Some("42"));

    let mut out = Vec::new();
    let conclusion = harness::run(
        &Arguments {
            test_threads: Some(1),
            ..Arguments::default()
        },
        vec![Trial::test("silent", || Err(Failed::without_message()))],
        &mut out,
    )
    .expect("the run completes");
    assert_eq!(conclusion.failed, 1);
    let text = without_duration(&String::from_utf8(out).expect("utf-8"));
    assert!(
        text.contains("test silent ... FAILED\n") && !text.contains("---- silent stdout ----"),
        "a failure without a message has no stdout block:\n{text}"
    );
}

#[test]
fn a_trial_reports_its_name_and_ignored_flag() {
    let trial = Trial::test("named_case", || Ok(()));
    assert_eq!(trial.name(), "named_case");
    assert!(!trial.is_ignored());
    let ignored = Trial::test(String::from("other"), || Ok(())).with_ignored_flag(true);
    assert_eq!(ignored.name(), "other");
    assert!(ignored.is_ignored());
}

#[test]
fn a_conclusion_fails_exactly_when_a_case_failed() {
    let clean = Conclusion {
        passed: 3,
        failed: 0,
        ignored: 2,
        filtered_out: 1,
    };
    assert!(!clean.has_failed());
    assert_eq!(clean.exit_code(), std::process::ExitCode::SUCCESS);
    let failed = Conclusion { failed: 1, ..clean };
    assert!(failed.has_failed());
    assert_eq!(
        failed.exit_code(),
        std::process::ExitCode::from(ERROR_EXIT_CODE)
    );
    assert!(!Conclusion::default().has_failed());
}
