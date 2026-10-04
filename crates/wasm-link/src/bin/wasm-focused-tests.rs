// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Run named WASM obligations after proving every exact filter selects a case.
//!
//! Build this host binary once, then invoke it directly. It launches Cargo for
//! the selected WASM tests and preserves the caller's environment and runner.

use std::io::{self, Write};
use std::process::{Command, ExitCode, Stdio};

const USAGE: &str = "usage: wasm-focused-tests PACKAGE TARGET CASE [CASE ...]\n       wasm-focused-tests --self-test\n       wasm-focused-tests --print-self-path";

#[derive(Debug)]
struct Selection<'a> {
    package: &'a str,
    target: &'a str,
    names: Vec<&'a str>,
}

#[derive(Debug)]
struct ChildResult {
    code: u8,
    stdout: Vec<u8>,
}

fn require_names(names: &[&str]) -> Result<(), String> {
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    if sorted.is_empty()
        || sorted.iter().any(|name| name.is_empty())
        || sorted.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err("a selection must contain distinct, nonempty case names".to_owned());
    }
    Ok(())
}

fn parse_args(argv: &[String]) -> Result<Selection<'_>, String> {
    if argv.len() < 3 {
        return Err(USAGE.to_owned());
    }
    let names: Vec<&str> = argv[2..].iter().map(String::as_str).collect();
    require_names(&names)?;
    Ok(Selection {
        package: &argv[0],
        target: &argv[1],
        names,
    })
}

fn require_selection(output: &str, names: &[&str]) -> Result<(), String> {
    require_names(names)?;
    let mut found: Vec<&str> = output
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .filter(|name| !name.is_empty())
        .collect();
    let summaries: Vec<&str> = output
        .lines()
        .filter(|line| {
            line.split_once(' ').is_some_and(|(count, suffix)| {
                !count.is_empty()
                    && count.bytes().all(|byte| byte.is_ascii_digit())
                    && matches!(suffix, "test, 0 benchmarks" | "tests, 0 benchmarks")
            })
        })
        .collect();
    let expected_summary = format!(
        "{} {}, 0 benchmarks",
        names.len(),
        if names.len() == 1 { "test" } else { "tests" }
    );
    let mut expected = names.to_vec();
    expected.sort_unstable();
    found.sort_unstable();
    if found != expected || summaries != [expected_summary.as_str()] {
        return Err(format!(
            "expected {names:?}; listed {found:?}; summaries {summaries:?}"
        ));
    }
    Ok(())
}

fn cargo(args: &[String], capture_stdout: bool) -> io::Result<ChildResult> {
    let output = Command::new("cargo")
        .args(args)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(if capture_stdout {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .output()?;
    // A child terminated by a signal has no exit code and must still fail the gate.
    let code = output
        .status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1);
    Ok(ChildResult {
        code,
        stdout: output.stdout,
    })
}

fn run_selection(
    selection: &Selection<'_>,
    mut execute: impl FnMut(&[String], bool) -> io::Result<ChildResult>,
    mut report: impl FnMut(&str) -> io::Result<()>,
) -> Result<u8, String> {
    require_names(&selection.names)?;
    let mut command: Vec<String> = [
        "test",
        "--locked",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        selection.package,
        "--test",
        selection.target,
        "--",
        "--exact",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    command.extend(selection.names.iter().map(|name| (*name).to_owned()));
    let mut listing_command = command.clone();
    listing_command.push("--list".to_owned());
    let listed = execute(&listing_command, true)
        .map_err(|error| format!("cannot list selected cases: {error}"))?;
    if listed.code != 0 {
        return Ok(listed.code);
    }
    let output = std::str::from_utf8(&listed.stdout)
        .map_err(|error| format!("listing is not UTF-8: {error}"))?;
    require_selection(output, &selection.names)?;
    report(&format!(
        "== {}/{}: {} verified WASM obligations",
        selection.package,
        selection.target,
        selection.names.len()
    ))
    .map_err(|error| format!("cannot report verified selection: {error}"))?;
    execute(&command, false)
        .map(|result| result.code)
        .map_err(|error| format!("cannot run selected cases: {error}"))
}

fn self_test() -> Result<(), String> {
    let valid = "beta: test\nalpha: test\n\n2 tests, 0 benchmarks\n";
    require_selection(valid, &["alpha", "beta"])?;
    require_selection("alpha: test\n\n1 test, 0 benchmarks\n", &["alpha"])?;
    let refused: &[(&str, &[&str])] = &[
        (valid, &[]),
        (valid, &["alpha", "alpha"]),
        (valid, &["alpha", "renamed"]),
        (valid, &["alpha"]),
        ("\n0 tests, 0 benchmarks\n", &["alpha"]),
        (
            "alpha: test\nalpha: test\n\n2 tests, 0 benchmarks\n",
            &["alpha", "beta"],
        ),
        (
            "alpha: test\nbeta: test\n\n1 tests, 0 benchmarks\n",
            &["alpha", "beta"],
        ),
        (
            "alpha: test\nbeta: test\n\n2 test, 0 benchmarks\n",
            &["alpha", "beta"],
        ),
        ("alpha: test\n\n1 tests, 0 benchmarks\n", &["alpha"]),
        ("alpha: test\nbeta: test\n", &["alpha", "beta"]),
        (
            "alpha: test\n\n1 test, 0 benchmarks\n1 test, 0 benchmarks\n",
            &["alpha"],
        ),
        ("alpha: test\n\n1 test, 0 benchmarks\n", &[""]),
    ];
    for (index, (output, names)) in refused.iter().enumerate() {
        if require_selection(output, names).is_ok() {
            return Err(format!("selection refusal {index} accepted invalid input"));
        }
    }
    Ok(())
}

fn path_text(path: std::path::PathBuf) -> Result<String, String> {
    path.into_os_string()
        .into_string()
        .map_err(|_| "executable path is not UTF-8".to_owned())
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv == ["--print-self-path"] {
        return match std::env::current_exe()
            .map_err(|error| format!("cannot locate executable: {error}"))
            .and_then(path_text)
        {
            Ok(path) => {
                println!("{path}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("FAIL: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if argv == ["--self-test"] {
        return match checks::run() {
            Ok(()) => {
                println!("OK: focused WASM selection self-test");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("FAIL: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let selection = match parse_args(&argv) {
        Ok(selection) => selection,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let mut stdout = io::stdout().lock();
    match run_selection(&selection, cargo, |message| {
        writeln!(stdout, "{message}")?;
        stdout.flush()
    }) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("FAIL: {}/{}: {error}", selection.package, selection.target);
            ExitCode::FAILURE
        }
    }
}

// The CLI and the native test harness run the same refusals and fake-child
// checks. The CLI self-test never launches Cargo or acquires another build slot.
mod checks {
    use super::{
        ChildResult, Selection, parse_args, path_text, require_selection, run_selection, self_test,
    };
    use std::io;

    fn selection() -> Selection<'static> {
        Selection {
            package: "demo-package",
            target: "demo-target",
            names: vec!["alpha", "beta"],
        }
    }

    fn listed(code: u8, stdout: &str) -> ChildResult {
        ChildResult {
            code,
            stdout: stdout.as_bytes().to_vec(),
        }
    }

    #[cfg_attr(test, test)]
    fn exact_listing_and_every_refused_neighbour() {
        self_test().expect("all selection refusals fire");
        require_selection(
            "beta: test\r\nalpha: test\r\n\r\n2 tests, 0 benchmarks\r\n",
            &["alpha", "beta"],
        )
        .expect("CRLF child output is accepted");
    }

    #[cfg_attr(test, test)]
    fn malformed_arguments_fail_before_any_child_runs() {
        for args in [vec![], vec!["package"], vec!["package", "target"]] {
            let argv: Vec<String> = args.into_iter().map(str::to_owned).collect();
            assert!(parse_args(&argv).is_err());
        }
        let argv: Vec<String> = ["package", "target", "alpha", "alpha"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert!(parse_args(&argv).is_err());
        for names in [vec![], vec!["alpha", "alpha"], vec![""]] {
            let invalid = Selection {
                names,
                ..selection()
            };
            assert!(
                run_selection(
                    &invalid,
                    |_, _| panic!("invalid selection must not launch Cargo"),
                    |_| panic!("invalid selection must not report verification"),
                )
                .is_err()
            );
        }
    }

    #[cfg_attr(test, test)]
    fn verifies_before_running_the_identical_exact_selection() {
        let mut commands = Vec::new();
        let mut reports = Vec::new();
        let events = std::cell::RefCell::new(Vec::new());
        let code = run_selection(
            &selection(),
            |args, capture| {
                events
                    .borrow_mut()
                    .push(if capture { "list" } else { "execute" });
                commands.push((args.to_vec(), capture));
                Ok(listed(
                    0,
                    "beta: test\nalpha: test\n\n2 tests, 0 benchmarks\n",
                ))
            },
            |message| {
                events.borrow_mut().push("report");
                reports.push(message.to_owned());
                Ok(())
            },
        )
        .expect("verified selection runs");
        assert_eq!(code, 0);
        let expected = [
            "test",
            "--locked",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            "demo-package",
            "--test",
            "demo-target",
            "--",
            "--exact",
            "alpha",
            "beta",
        ];
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[1].0, expected);
        assert!(!commands[1].1);
        assert_eq!(commands[0].0[..expected.len()], expected);
        assert_eq!(commands[0].0[expected.len()..], ["--list"]);
        assert!(commands[0].1);
        assert_eq!(
            reports,
            ["== demo-package/demo-target: 2 verified WASM obligations"]
        );
        assert_eq!(events.into_inner(), ["list", "report", "execute"]);
    }

    #[cfg_attr(test, test)]
    fn listing_failure_keeps_its_status_and_never_executes_cases() {
        let mut calls = 0;
        let code = run_selection(
            &selection(),
            |_, capture| {
                calls += 1;
                assert!(capture);
                Ok(ChildResult {
                    code: 37,
                    stdout: vec![0xff],
                })
            },
            |_| panic!("failed listing must not report verification"),
        )
        .expect("listing exit status is preserved");
        assert_eq!(code, 37);
        assert_eq!(calls, 1);
    }

    #[cfg_attr(test, test)]
    fn mismatched_and_invalid_utf8_listings_never_execute_cases() {
        for stdout in [
            b"\n0 tests, 0 benchmarks\n".as_slice(),
            b"alpha: test\nrenamed: test\n\n2 tests, 0 benchmarks\n".as_slice(),
            &[0xff],
        ] {
            let mut calls = 0;
            assert!(
                run_selection(
                    &selection(),
                    |_, capture| {
                        calls += 1;
                        assert!(capture);
                        Ok(ChildResult {
                            code: 0,
                            stdout: stdout.to_vec(),
                        })
                    },
                    |_| panic!("invalid listing must not report verification"),
                )
                .is_err()
            );
            assert_eq!(calls, 1);
        }
    }

    #[cfg_attr(test, test)]
    fn execution_failure_keeps_its_status() {
        let mut calls = 0;
        let code = run_selection(
            &selection(),
            |_, capture| {
                calls += 1;
                Ok(listed(
                    if capture { 0 } else { 29 },
                    "alpha: test\nbeta: test\n\n2 tests, 0 benchmarks\n",
                ))
            },
            |_| Ok(()),
        )
        .expect("execution exit status is preserved");
        assert_eq!(code, 29);
        assert_eq!(calls, 2);
    }

    #[cfg_attr(test, test)]
    fn launch_failures_are_not_successful_selections() {
        for failing_capture in [true, false] {
            let mut calls = 0;
            let error = run_selection(
                &selection(),
                |_, capture| {
                    calls += 1;
                    if capture == failing_capture {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            "fixture launch failure",
                        ));
                    }
                    Ok(listed(
                        0,
                        "alpha: test\nbeta: test\n\n2 tests, 0 benchmarks\n",
                    ))
                },
                |_| Ok(()),
            )
            .expect_err("launch error fails the gate");
            assert!(error.contains("fixture launch failure"));
            assert_eq!(calls, if failing_capture { 1 } else { 2 });
        }
    }

    #[cfg_attr(test, test)]
    fn report_failure_prevents_execution() {
        let mut calls = 0;
        let error = run_selection(
            &selection(),
            |_, capture| {
                calls += 1;
                assert!(capture);
                Ok(listed(
                    0,
                    "alpha: test\nbeta: test\n\n2 tests, 0 benchmarks\n",
                ))
            },
            |_| {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "fixture report failure",
                ))
            },
        )
        .expect_err("report failure blocks execution");
        assert!(error.contains("fixture report failure"));
        assert_eq!(calls, 1);
    }

    #[cfg_attr(test, test)]
    fn executable_path_is_returned_without_lossy_conversion() {
        assert_eq!(
            path_text(std::path::PathBuf::from("target/host tool")),
            Ok("target/host tool".to_owned())
        );
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let invalid = std::ffi::OsString::from_vec(vec![0xff]);
            assert!(path_text(std::path::PathBuf::from(invalid)).is_err());
        }
    }

    pub(super) fn run() -> Result<(), String> {
        let checks: &[(&str, fn())] = &[
            (
                "listing refusals",
                exact_listing_and_every_refused_neighbour,
            ),
            ("arguments", malformed_arguments_fail_before_any_child_runs),
            (
                "command identity",
                verifies_before_running_the_identical_exact_selection,
            ),
            (
                "listing status",
                listing_failure_keeps_its_status_and_never_executes_cases,
            ),
            (
                "listing evidence",
                mismatched_and_invalid_utf8_listings_never_execute_cases,
            ),
            ("execution status", execution_failure_keeps_its_status),
            (
                "launch failure",
                launch_failures_are_not_successful_selections,
            ),
            ("report failure", report_failure_prevents_execution),
            (
                "executable path",
                executable_path_is_returned_without_lossy_conversion,
            ),
        ];
        for (name, check) in checks {
            if std::panic::catch_unwind(*check).is_err() {
                return Err(format!("focused WASM self-test failed: {name}"));
            }
        }
        Ok(())
    }
}
