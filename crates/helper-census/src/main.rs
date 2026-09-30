// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `helper-census` — the structural census of the workspace's shipping Rust.
//!
//! It finds functionality written more than once: function bodies that are the
//! same algorithm under different names and constants (clone types 1 and 2), thin
//! forwarders that forward to the same place, integer constants compared by value
//! whatever their spelling, and hex-digit tables. It reads `helpers-ledger.toml`,
//! which names the one home of each job and the sanctioned variants, and reports
//! against it. `scripts/check-shared-helpers.py` is the gate built on it; see
//! [`USAGE`] for the modes.

mod census;
mod layout;
mod ledger;
mod normalize;
mod rules;
mod source;
mod structure;
mod toml;

use std::path::PathBuf;
use std::process::ExitCode;

use crate::source::{Disk, Memory, Tree, Workspace};

/// The CLI contract, printed by `--help` and beside every argument error.
const USAGE: &str = "\
Usage: helper-census [--root DIR] [--ledger FILE] <mode>

Modes:
  --baseline [PATH]  write every isomorphic group, shim group, hex table and
                     cross-package constant as JSON (default PATH:
                     target/helper-census/clone-baseline.json under the root)
  --census JOB       list the units matching JOB's forbidden set
  --count JOB        print copies=<n>: matches outside JOB's home that are not
                     ledger variants
  --check            fail on any unsanctioned isomorphic group and on any
                     forbidden match outside its home
  --index            print the JSON index scripts/check-shared-helpers.py reads
  --dump-ledger      print the parsed ledger as JSON
  --self-test        run the seeded fixtures and exit
  --help, -h         print this text

--root defaults to the workspace this binary was built from; --ledger defaults
to helpers-ledger.toml under the root.
";

/// What the arguments ask for.
#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Baseline(Option<PathBuf>),
    Census(String),
    Count(String),
    Check,
    Index,
    DumpLedger,
    SelfTest,
    Help,
}

/// Parsed arguments.
#[derive(Debug)]
struct Arguments {
    root: PathBuf,
    ledger: Option<PathBuf>,
    mode: Mode,
}

fn parse_arguments(mut args: impl Iterator<Item = String>) -> Result<Arguments, String> {
    let mut root = purrdf_testkit::paths::workspace_root();
    let mut ledger = None;
    let mut mode = None;
    let set = |next: Mode, mode: &mut Option<Mode>| {
        if mode.replace(next).is_some() {
            Err("give exactly one mode".to_owned())
        } else {
            Ok(())
        }
    };
    let mut pending: Vec<String> = Vec::new();
    while let Some(arg) = pending.pop().or_else(|| args.next()) {
        match arg.as_str() {
            "--root" => root = args.next().ok_or("--root needs a directory")?.into(),
            "--ledger" => ledger = Some(args.next().ok_or("--ledger needs a file")?.into()),
            "--baseline" => {
                let path = match args.next() {
                    Some(next) if next.starts_with("--") => {
                        pending.push(next);
                        None
                    }
                    other => other.map(PathBuf::from),
                };
                set(Mode::Baseline(path), &mut mode)?;
            }
            "--census" => set(
                Mode::Census(args.next().ok_or("--census needs a job id")?),
                &mut mode,
            )?,
            "--count" => set(
                Mode::Count(args.next().ok_or("--count needs a job id")?),
                &mut mode,
            )?,
            "--check" => set(Mode::Check, &mut mode)?,
            "--index" => set(Mode::Index, &mut mode)?,
            "--dump-ledger" => set(Mode::DumpLedger, &mut mode)?,
            "--self-test" => set(Mode::SelfTest, &mut mode)?,
            "--help" | "-h" => set(Mode::Help, &mut mode)?,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(Arguments {
        root,
        ledger,
        mode: mode.ok_or("give a mode")?,
    })
}

fn main() -> ExitCode {
    let arguments = match parse_arguments(std::env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("helper-census: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&arguments) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("helper-census: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &Arguments) -> Result<ExitCode, String> {
    match &arguments.mode {
        Mode::Help => {
            print!("{USAGE}");
            return Ok(ExitCode::SUCCESS);
        }
        Mode::SelfTest => return Ok(self_test()),
        _ => {}
    }
    let root = arguments
        .root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", arguments.root.display()))?;
    let ledger_path = arguments
        .ledger
        .clone()
        .unwrap_or_else(|| root.join("helpers-ledger.toml"));
    let ledger_text = std::fs::read_to_string(&ledger_path)
        .map_err(|error| format!("{}: {error}", ledger_path.display()))?;
    let ledger = ledger::parse(&ledger_text)?;
    if arguments.mode == Mode::DumpLedger {
        println!("{}", toml::Value::Table(ledger.document).to_json());
        return Ok(ExitCode::SUCCESS);
    }
    let workspace = Workspace::walk(&Disk { root: root.clone() });
    if !workspace.errors.is_empty() && arguments.mode != Mode::Index {
        return Err(format!(
            "the walk failed:\n  {}",
            workspace.errors.join("\n  ")
        ));
    }
    let job = |id: &str| {
        ledger
            .job(id)
            .ok_or_else(|| format!("no job `{id}` in the ledger"))
    };
    match &arguments.mode {
        Mode::Baseline(path) => {
            let path = path
                .clone()
                .unwrap_or_else(|| root.join("target/helper-census/clone-baseline.json"));
            let document = census::baseline(&workspace, &ledger);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("{}: {error}", parent.display()))?;
            }
            let text = purrdf_lex::json::write_pretty(&document);
            std::fs::write(&path, text + "\n")
                .map_err(|error| format!("{}: {error}", path.display()))?;
            println!(
                "{} units; {} isomorphic groups, {} shim groups, {} hex tables, {} shared constants -> {}",
                workspace.units.len(),
                document["isomorphic_groups"].as_array().map_or(0, Vec::len),
                document["shim_groups"].as_array().map_or(0, Vec::len),
                document["hex_tables"].as_array().map_or(0, Vec::len),
                document["shared_constants"].as_array().map_or(0, Vec::len),
                path.display()
            );
        }
        Mode::Census(id) => {
            for line in census::census_lines(job(id)?, &workspace)? {
                println!("{line}");
            }
        }
        Mode::Count(id) => {
            let job = job(id)?;
            let home = census::home(job, &workspace)?;
            println!(
                "copies={}",
                census::copies(&census::matches(job, &workspace, &home.package))
            );
        }
        Mode::Check => {
            let findings = census::check_findings(&workspace, &ledger);
            if !findings.is_empty() {
                eprintln!("helper-census --check: {} finding(s)", findings.len());
                for finding in &findings {
                    eprintln!("  {finding}");
                }
                return Ok(ExitCode::FAILURE);
            }
            println!(
                "OK: no unsanctioned isomorphic group and no forbidden match outside its home"
            );
        }
        Mode::Index => println!("{}", census::index(&workspace, &ledger)),
        Mode::DumpLedger | Mode::SelfTest | Mode::Help => unreachable!("handled above"),
    }
    Ok(ExitCode::SUCCESS)
}

/// The seeded fixture workspace: a home package and a second package that
/// re-derives it, plus the near misses that must not be reported.
fn fixture_tree() -> Memory {
    let files = [
        (
            "crates/alpha/Cargo.toml",
            "[package]\nname = \"fixture-alpha\"\n",
        ),
        (
            "crates/alpha/src/lib.rs",
            "/// The one FNV-1a.\npub fn fnv1a(bytes: &[u8]) -> u64 {\n    let mut hash = 0xcbf2_9ce4_8422_2325_u64;\n    for byte in bytes {\n        hash ^= u64::from(*byte);\n        hash = hash.wrapping_mul(0x0100_0000_01b3);\n    }\n    hash\n}\n",
        ),
        (
            "crates/beta/Cargo.toml",
            "[package]\nname = \"fixture-beta\"\n",
        ),
        (
            "crates/beta/src/lib.rs",
            "mod digits;\n/// A retyped copy under other names.\npub fn fold(input: &[u8]) -> u64 {\n    let mut state = 14695981039346656037_u64;\n    for b in input {\n        state ^= u64::from(*b);\n        state = state.wrapping_mul(1099511628211);\n    }\n    state\n}\n/// The same length, a different algorithm.\npub fn count(input: &[u8]) -> u64 {\n    let mut state = 0_u64;\n    let mut index = 0;\n    while index < input.len() {\n        if input[index] == b'/' { state += 1; }\n        index += 1;\n    }\n    state\n}\n#[cfg(test)]\nmod tests {\n    fn fnv_again(bytes: &[u8]) -> u64 {\n        let mut hash = 0xcbf2_9ce4_8422_2325_u64;\n        for byte in bytes { hash ^= u64::from(*byte); hash = hash.wrapping_mul(0x0100_0000_01b3); }\n        hash\n    }\n}\n",
        ),
        (
            "crates/beta/src/digits.rs",
            // The two tables' digits are joined at run time below, so this
            // file spells no table of its own.
            "pub(crate) const DIGITS: &[u8; 16] = b\"01234567LOWER\";\npub(crate) const NOT_DIGITS: &[u8; 16] = b\"01234567NEAR\";\n",
        ),
        (
            "crates/beta/tests/copy.rs",
            "pub fn fnv1a(bytes: &[u8]) -> u64 { 0xcbf2_9ce4_8422_2325 }\n",
        ),
    ];
    Memory {
        files: files
            .into_iter()
            .map(|(path, text)| {
                let text = text
                    .replace("LOWER", "89abcdef")
                    .replace("NEAR", "89abcdeF");
                (path.to_owned(), text)
            })
            .collect(),
    }
}

const FIXTURE_JOB: &str = "[[job]]\nid = \"fnv\"\nsummary = \"FNV-1a\"\nhome = \"fixture_alpha::fnv1a\"\nentry_points = []\nspec = \"FNV-1a\"\nvectors = []\nbench = []\nsites = []\nreplaces_external = []\nenforced = true\n[job.forbidden]\nconstants = [\"0xcbf29ce484222325\"]\nfingerprints = [\"table:hex-lower\"]\nnames = []\n";

const FIXTURE_VARIANT: &str = "[[job.variant]]\nsymbol = \"fixture_beta::fold\"\nfile = \"crates/beta/src/lib.rs\"\ndetector = \"DETECTOR\"\ncriterion = \"a\"\nanchor = \"fixture_beta::fold\"\nreason = \"fixture\"\n";

const FIXTURE_DISTINCT: &str = "[[distinct]]\nmembers = [MEMBERS]\nkind = \"KIND\"\ncriterion = \"b\"\nanchor = \"ANCHOR\"\nreason = \"fixture\"\n";

/// The `--check` findings over `tree` under a ledger of one `[[distinct]]` row.
fn distinct_findings(tree: &dyn Tree, members: &str, kind: &str, anchor: &str) -> Vec<String> {
    let row = FIXTURE_DISTINCT
        .replace("MEMBERS", members)
        .replace("KIND", kind)
        .replace("ANCHOR", anchor);
    ledger::parse(&row).map_or_else(
        |error| vec![error],
        |ledger| census::check_findings(&Workspace::walk(tree), &ledger),
    )
}

/// The `[[distinct]]` cases: the row sanctions its exact group and nothing
/// else, and a row that no longer matches, or whose anchor is undocumented, fails.
fn distinct_cases() -> Vec<(&'static str, bool)> {
    let tree = fixture_tree();
    let pair = "\"fixture_beta::fold\", \"fixture_alpha::fnv1a\"";
    let mut grown = fixture_tree();
    grown.files.insert(
        "crates/gamma/Cargo.toml".to_owned(),
        "[package]\nname = \"fixture-gamma\"\n".to_owned(),
    );
    grown.files.insert(
        "crates/gamma/src/lib.rs".to_owned(),
        "/// A third copy of the sanctioned shape.\npub fn mix(data: &[u8]) -> u64 {\n    let mut acc = 1_u64;\n    for d in data {\n        acc ^= u64::from(*d);\n        acc = acc.wrapping_mul(3);\n    }\n    acc\n}\n".to_owned(),
    );
    vec![
        (
            "a distinct row naming exactly its group's members sanctions it",
            distinct_findings(&tree, pair, "body", "fixture_alpha::fnv1a").is_empty(),
        ),
        (
            "a distinct row of the other kind does not sanction the group, and is STALE",
            distinct_findings(&tree, pair, "shim", "fixture_alpha::fnv1a").len() == 2,
        ),
        (
            "a group that gains a member its distinct row does not name fails, and the row is STALE",
            distinct_findings(&grown, pair, "body", "fixture_alpha::fnv1a")
                .iter()
                .filter(|finding| {
                    finding.contains("fixture_gamma::mix") || finding.contains("STALE")
                })
                .count()
                == 2,
        ),
        (
            "a distinct row whose members are no group is STALE, and the group still fails",
            {
                let findings = distinct_findings(
                    &tree,
                    "\"fixture_alpha::fnv1a\", \"fixture_beta::count\"",
                    "body",
                    "fixture_alpha::fnv1a",
                );
                findings.len() == 2
                    && findings
                        .iter()
                        .any(|finding| finding.starts_with("isomorphic bodies"))
                    && findings.iter().any(|finding| finding.contains("STALE"))
            },
        ),
        (
            "a distinct row naming a member that no longer exists is STALE",
            distinct_findings(
                &tree,
                "\"fixture_alpha::fnv1a\", \"fixture_beta::gone\"",
                "body",
                "fixture_alpha::fnv1a",
            )
            .iter()
            .any(|finding| finding.contains("STALE") && finding.contains("fixture_beta::gone")),
        ),
        (
            "a distinct row whose anchor carries no documentation fails",
            distinct_findings(&tree, pair, "body", "fixture_beta::digits::DIGITS")
                == [
                    "[[distinct]] body [fixture_alpha::fnv1a, fixture_beta::fold]: anchor `fixture_beta::digits::DIGITS` carries no documentation",
                ],
        ),
        (
            "a distinct row whose anchor does not resolve fails",
            distinct_findings(&tree, pair, "body", "fixture_alpha::missing")
                .iter()
                .any(|finding| finding.contains("does not resolve")),
        ),
    ]
}

/// One seeded case: its name and whether it held.
fn self_test_cases() -> Vec<(&'static str, bool)> {
    let tree: &dyn Tree = &fixture_tree();
    let workspace = Workspace::walk(tree);
    let Ok(plain) = ledger::parse(FIXTURE_JOB) else {
        return vec![("the fixture ledger parses", false)];
    };
    let with_variant = |detector: &str| {
        ledger::parse(&format!(
            "{FIXTURE_JOB}{}",
            FIXTURE_VARIANT.replace("DETECTOR", detector)
        ))
    };
    let (Ok(isomorphic), Ok(forbidden)) = (with_variant("isomorphic"), with_variant("forbidden"))
    else {
        return vec![("the fixture ledger with a variant parses", false)];
    };
    let groups = census::isomorphic_groups(&workspace);
    let group_symbols: Vec<Vec<&str>> = groups
        .iter()
        .map(|group| {
            group
                .members
                .iter()
                .map(|&index| workspace.units[index].symbol.as_str())
                .collect()
        })
        .collect();
    let Some(job) = plain.job("fnv") else {
        return vec![("the fixture job exists", false)];
    };
    let home_package = census::home(job, &workspace).map(|home| home.package);
    let found = home_package
        .as_ref()
        .map_or_default(|package| census::matches(job, &workspace, package));
    let matched: Vec<(&str, bool)> = found
        .iter()
        .map(|found| (found.symbol.as_str(), found.in_home))
        .collect();
    let copies_with_forbidden_variant = forbidden.job("fnv").map_or(usize::MAX, |job| {
        census::copies(&census::matches(job, &workspace, "fixture-alpha"))
    });
    let mut cases = vec![
        (
            "the walk reads every fixture file",
            workspace.errors.is_empty(),
        ),
        (
            "a renamed, re-spelt copy of a body is one isomorphic group with its original",
            group_symbols == [vec!["fixture_alpha::fnv1a", "fixture_beta::fold"]],
        ),
        (
            "a body of the same size with different control flow is not grouped",
            !group_symbols
                .iter()
                .flatten()
                .any(|symbol| *symbol == "fixture_beta::count"),
        ),
        (
            "test-only code and tests/ files are never walked",
            !workspace
                .units
                .iter()
                .any(|unit| unit.name == "fnv_again" || unit.file.contains("/tests/")),
        ),
        (
            "the home resolves to its package",
            home_package.as_deref() == Ok("fixture-alpha"),
        ),
        (
            "a decimal spelling of a forbidden hex constant is a copy outside the home",
            matched.contains(&("fixture_beta::fold", false))
                && matched.contains(&("fixture_alpha::fnv1a", true)),
        ),
        (
            "a hex-digit table is a forbidden fingerprint and a near-miss table is not",
            matched.contains(&("fixture_beta::digits::DIGITS", false))
                && !matched
                    .iter()
                    .any(|(symbol, _)| *symbol == "fixture_beta::digits::NOT_DIGITS"),
        ),
        ("--count counts both copies", census::copies(&found) == 2),
        (
            "--check reports the unsanctioned group and both copies",
            census::check_findings(&workspace, &plain).len() == 3,
        ),
        (
            "an isomorphic variant row sanctions its group",
            census::check_findings(&workspace, &isomorphic).len() == 2,
        ),
        (
            "a forbidden variant row removes its copy from the count",
            copies_with_forbidden_variant == 1,
        ),
    ];
    cases.extend(distinct_cases());
    cases
}

fn self_test() -> ExitCode {
    let mut failed = 0;
    for (name, held) in self_test_cases() {
        if held {
            println!("PASS: {name}");
        } else {
            println!("FAIL: {name}");
            failed += 1;
        }
    }
    if failed == 0 {
        println!("OK: helper-census self-test");
        ExitCode::SUCCESS
    } else {
        eprintln!("helper-census self-test: {failed} case(s) failed");
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, parse_arguments, self_test_cases};

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|arg| (*arg).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn every_seeded_case_holds() {
        for (name, held) in self_test_cases() {
            assert!(held, "{name}");
        }
    }

    #[test]
    fn baseline_takes_an_optional_path() {
        assert_eq!(
            parse_arguments(args(&["--baseline"])).expect("bare").mode,
            Mode::Baseline(None)
        );
        assert_eq!(
            parse_arguments(args(&["--baseline", "out.json"]))
                .expect("with a path")
                .mode,
            Mode::Baseline(Some("out.json".into()))
        );
        let followed =
            parse_arguments(args(&["--baseline", "--root", "."])).expect("followed by a flag");
        assert_eq!(followed.mode, Mode::Baseline(None));
    }

    #[test]
    fn two_modes_or_none_are_refused() {
        assert!(parse_arguments(args(&["--check", "--index"])).is_err());
        assert!(parse_arguments(args(&[])).is_err());
        assert!(parse_arguments(args(&["--count"])).is_err());
        assert_eq!(
            parse_arguments(args(&["--count", "fnv"]))
                .expect("one mode")
                .mode,
            Mode::Count("fnv".to_owned())
        );
    }
}
