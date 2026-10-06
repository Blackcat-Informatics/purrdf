// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Host qualification tool: execute the public law corpora on native and wasm.
//! No build-only or skipped result establishes completed-output parity.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process::{Command, Stdio},
    };

    type Result<T> = std::result::Result<T, String>;

    #[derive(Clone, Copy, Debug)]
    enum Lane {
        Native,
        PortableWasm,
        SimdWasm,
    }

    impl Lane {
        const ALL: [Self; 3] = [Self::Native, Self::PortableWasm, Self::SimdWasm];

        fn name(self) -> &'static str {
            match self {
                Self::Native => "native",
                Self::PortableWasm => "wasm portable",
                Self::SimdWasm => "wasm simd128",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Record {
        case: String,
        digest: u64,
        count: usize,
    }

    struct Expected {
        case: &'static str,
        constant: &'static str,
        minimum_count: usize,
    }

    struct Corpus {
        package: &'static str,
        target: &'static str,
        source: &'static str,
        records: &'static [Expected],
    }

    const CORPORA: [Corpus; 4] = [
        Corpus {
            package: "purrdf-geo",
            target: "determinism",
            source: "crates/geo/tests/determinism.rs",
            records: &[
                Expected {
                    case: "the_digest_is_the_pinned_golden",
                    constant: "GOLDEN_DIGEST",
                    minimum_count: 20,
                },
                Expected {
                    case: "the_digest_does_not_move_between_runs",
                    constant: "GOLDEN_DIGEST",
                    minimum_count: 20,
                },
            ],
        },
        Corpus {
            package: "purrdf-geo-kernel",
            target: "cells_determinism",
            source: "crates/geo-kernel/tests/cells_determinism.rs",
            records: &[Expected {
                case: "public_grid_replay",
                constant: "GOLDEN_DIGEST",
                minimum_count: 20,
            }],
        },
        Corpus {
            package: "purrdf-geo-kernel",
            target: "geo_determinism",
            source: "crates/geo-kernel/tests/geo_determinism.rs",
            records: &[Expected {
                case: "public_geo_capability_replay",
                constant: "GOLDEN_DIGEST",
                minimum_count: 20,
            }],
        },
        Corpus {
            package: "purrdf-xsd",
            target: "certified_numerical",
            source: "crates/xsd/tests/certified_numerical.rs",
            records: &[
                Expected {
                    case: "certified_elementary_replay",
                    constant: "ELEMENTARY_DIGEST",
                    minimum_count: 7,
                },
                Expected {
                    case: "directed_endpoint_product_replay",
                    constant: "ENDPOINT_DIGEST",
                    minimum_count: 8,
                },
            ],
        },
    ];

    fn command_output(command: &mut Command, label: &str) -> Result<String> {
        let output = command
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| format!("{label}: {error}"))?;
        let text = String::from_utf8(output.stdout)
            .map_err(|error| format!("{label}: non-UTF-8 output: {error}"))?;
        if !output.status.success() {
            eprint!("{text}");
            return Err(format!("{label}: {}", output.status));
        }
        Ok(text)
    }

    fn preflight(root: &Path) -> Result<()> {
        for executable in ["node", "wasm-bindgen"] {
            command_output(
                Command::new(executable).arg("--version"),
                &format!("required executable {executable}"),
            )?;
        }
        let targets = command_output(
            Command::new("rustup").args(["target", "list", "--installed"]),
            "installed Rust targets",
        )?;
        if !targets.lines().any(|line| line == "wasm32-unknown-unknown") {
            return Err("wasm32-unknown-unknown standard library is required".into());
        }
        if !root.join("scripts/wasm-test-runner.sh").is_file() {
            return Err("the shared wasm test runner is required".into());
        }
        Ok(())
    }

    fn golden(source: &str, name: &str) -> Result<u64> {
        let prefix = format!("const {name}: u64 = 0x");
        let mut values = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix(&prefix)?.strip_suffix(';'));
        let value = values
            .next()
            .ok_or_else(|| format!("missing frozen {name}"))?;
        if values.next().is_some() {
            return Err(format!("duplicate frozen {name}"));
        }
        u64::from_str_radix(&value.replace('_', ""), 16)
            .map_err(|error| format!("invalid frozen {name}: {error}"))
    }

    fn registered_tests(source: &str) -> Result<usize> {
        let (_, tail) = source
            .split_once("purrdf_testkit::harness_main!(")
            .ok_or("missing shared test runner registration")?;
        let (names, _) = tail.split_once(')').ok_or("unclosed test registration")?;
        let count = names
            .split(',')
            .filter(|name| !name.trim().is_empty())
            .count();
        if count == 0 {
            return Err("empty shared test runner registration".into());
        }
        Ok(count)
    }

    fn records(text: &str, expected_tests: usize) -> Result<Vec<Record>> {
        let summaries = text
            .lines()
            .filter_map(|line| line.strip_prefix("test result: "))
            .collect::<Vec<_>>();
        let expected = format!(
            "ok. {expected_tests} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
        );
        if summaries.len() != 1 || !summaries[0].starts_with(&expected) {
            return Err(format!(
                "expected all {expected_tests} registered tests with no ignored or filtered cases; got {summaries:?}"
            ));
        }
        let mut records = Vec::new();
        for line in text.lines() {
            let Some((_, tail)) = line.split_once("determinism-digest ") else {
                continue;
            };
            let mut fields = tail.split_whitespace();
            let case = fields.next().and_then(|s| s.strip_prefix("case="));
            let digest = fields.next().and_then(|s| s.strip_prefix("digest="));
            let count = fields.next().and_then(|s| s.strip_prefix("corpus_len="));
            let (Some(case), Some(digest), Some(count)) = (case, digest, count) else {
                return Err(format!("invalid digest record: {line}"));
            };
            if digest.len() != 16 {
                return Err(format!("invalid digest width: {line}"));
            }
            records.push(Record {
                case: case.into(),
                digest: u64::from_str_radix(digest, 16)
                    .map_err(|error| format!("invalid digest: {error}"))?,
                count: count
                    .parse()
                    .map_err(|error| format!("invalid count: {error}"))?,
            });
        }
        records.sort_unstable();
        Ok(records)
    }

    fn execute(root: &Path, corpus: &Corpus, lane: Lane) -> Result<String> {
        let mut command = Command::new("cargo");
        command.current_dir(root).args([
            "test",
            "--locked",
            "-p",
            corpus.package,
            "--test",
            corpus.target,
        ]);
        if !matches!(lane, Lane::Native) {
            command.args(["--target", "wasm32-unknown-unknown"]);
            command.env(
                "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
                root.join("scripts/wasm-test-runner.sh"),
            );
            // Cargo gives encoded flags priority. Preserve the effective caller
            // flags in that representation and select the two wasm paths explicitly.
            let mut flags = env::var("CARGO_ENCODED_RUSTFLAGS").map_or_else(
                |_| {
                    env::var("RUSTFLAGS")
                        .unwrap_or_default()
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                },
                |text| {
                    text.split('\u{1f}')
                        .filter(|flag| !flag.is_empty())
                        .map(str::to_owned)
                        .collect()
                },
            );
            flags.push("-C".into());
            flags.push(match lane {
                Lane::PortableWasm => "target-feature=-simd128".into(),
                Lane::SimdWasm => "target-feature=+simd128".into(),
                Lane::Native => unreachable!(),
            });
            command.env("CARGO_ENCODED_RUSTFLAGS", flags.join("\u{1f}"));
            command.env(
                "PURRDF_REQUIRE_SIMD_PATHS",
                match lane {
                    Lane::SimdWasm => "numeric:portable,numeric:simd128",
                    _ => "numeric:portable",
                },
            );
        }
        let label = format!("{}/{} {}", corpus.package, corpus.target, lane.name());
        println!("Executing {label}");
        command_output(&mut command, &label)
    }

    fn compare(root: &Path, corpus: &Corpus) -> Result<()> {
        let source = fs::read_to_string(root.join(corpus.source))
            .map_err(|error| format!("{}: {error}", corpus.source))?;
        let test_count = registered_tests(&source)?;
        let mut baseline = None;
        for lane in Lane::ALL {
            let output = execute(root, corpus, lane)?;
            print!("{output}");
            let observed = records(&output, test_count)?;
            if observed.len() != corpus.records.len() {
                return Err(format!("unexpected digest records: {observed:?}"));
            }
            for expected in corpus.records {
                let matches = observed
                    .iter()
                    .filter(|record| record.case == expected.case)
                    .collect::<Vec<_>>();
                if matches.len() != 1
                    || matches[0].count < expected.minimum_count
                    || matches[0].digest != golden(&source, expected.constant)?
                {
                    return Err(format!(
                        "missing, duplicate, vacuous or changed record {}: {matches:?}",
                        expected.case
                    ));
                }
            }
            if let Some(expected) = &baseline {
                if &observed != expected {
                    return Err(format!(
                        "completed records differ on {}: {observed:?} != {expected:?}",
                        lane.name()
                    ));
                }
            } else {
                baseline = Some(observed);
            }
        }
        println!(
            "OK: {}/{} executed on all three paths with identical frozen records",
            corpus.package, corpus.target
        );
        Ok(())
    }

    pub(super) fn run() -> Result<()> {
        let root: PathBuf = purrdf_testkit::paths::workspace_root();
        preflight(&root)?;
        for corpus in CORPORA {
            compare(&root, &corpus)?;
        }
        println!("OK: all twelve geographic and numerical target executions passed");
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn refuses_skipped_cases_and_missing_or_malformed_records() {
            let valid = "test sample ... determinism-digest case=sample digest=8273649182736491 corpus_len=20\nok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0s\n";
            assert_eq!(records(valid, 1).unwrap().len(), 1);
            for changed in [
                valid.replace("1 passed;", "0 passed;"),
                valid.replace("0 ignored;", "1 ignored;"),
                valid.replace("0 filtered out;", "1 filtered out;"),
                valid.replace("8273649182736491", "0123"),
            ] {
                assert!(records(&changed, 1).is_err());
            }
            assert_eq!(
                golden("const PIN: u64 = 0x8273_6491_8273_6491;", "PIN").unwrap(),
                0x8273_6491_8273_6491
            );
            assert!(golden("const OTHER: u64 = 1;", "PIN").is_err());
            assert!(registered_tests("purrdf_testkit::harness_main!();").is_err());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(error) = native::run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("qualification orchestration requires a native host");
}
