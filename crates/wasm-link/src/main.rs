// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `wasm-link` — link the PurRDF wasm package's optimized module, or check one.
//!
//! See [`USAGE`]; `--help`/`-h` prints the same text.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

/// The CLI contract, printed by `--help` and beside every argument error.
const USAGE: &str = "\
Usage: wasm-link [--out PATH] [--verbose] <module.wasm>
       wasm-link --check [--verbose] <module.wasm>
       wasm-link --help | -h

Links purrdf_wasm_bg.wasm after wasm-opt: exports the shadow-stack pointer,
routes every call of the suspending import through $suspend, wraps the run
export so it starts on the region top its caller passes, and puts every
exported function behind the poison gate. Validates the result. Rewrites the
module in place unless --out names another path.

  --check      verify an already-linked module instead of linking it
  --out PATH   write the linked module to PATH instead of over the input
  --verbose    list every export behind each gate variant (trapping, inert)
  --help, -h   print this usage text and exit
";

/// What the arguments ask for.
#[derive(Debug, PartialEq, Eq)]
struct Args {
    check: bool,
    verbose: bool,
    out: Option<PathBuf>,
    input: PathBuf,
}

fn parse_args(argv: &[String]) -> Result<Option<Args>, String> {
    let mut check = false;
    let mut verbose = false;
    let mut out = None;
    let mut input = None;
    let mut rest = argv.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--check" => check = true,
            "--verbose" => verbose = true,
            "--out" => {
                let path = rest.next().ok_or("--out needs a path")?;
                out = Some(PathBuf::from(path));
            }
            other if other.starts_with('-') => return Err(format!("unknown option {other}")),
            other => {
                if input.replace(PathBuf::from(other)).is_some() {
                    return Err("exactly one module path is expected".to_owned());
                }
            }
        }
    }
    let input = input.ok_or("a module path is required")?;
    if check && out.is_some() {
        return Err("--check does not write, so --out has no meaning with it".to_owned());
    }
    Ok(Some(Args {
        check,
        verbose,
        out,
        input,
    }))
}

/// Write `bytes` to `path` through a sibling temporary file, so a failure leaves the
/// previous module intact.
fn write_module(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let file_name = path.file_name().map_or_else(
        || "module.wasm".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let temporary = path.with_file_name(format!("{file_name}.wasm-link.tmp"));
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}

fn run(args: &Args) -> Result<String, String> {
    let bytes = fs::read(&args.input)
        .map_err(|error| format!("cannot read {}: {error}", args.input.display()))?;
    let (report, verb) = if args.check {
        (
            wasm_link::check(&bytes).map_err(|error| error.to_string())?,
            "checked",
        )
    } else {
        let (linked, report) = wasm_link::link(&bytes).map_err(|error| error.to_string())?;
        let target = args.out.as_deref().unwrap_or(&args.input);
        write_module(target, &linked)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        (report, "linked")
    };
    let mut text = format!(
        "OK: wasm-link {verb} {}: {}",
        args.input.display(),
        report.describe()
    );
    if args.verbose {
        let trapping: Vec<&str> = report
            .wrapped_exports
            .iter()
            .filter(|name| !report.inert_exports.contains(name))
            .map(String::as_str)
            .collect();
        text.push_str("\n  trapping gate (");
        text.push_str(&trapping.len().to_string());
        text.push_str("): ");
        text.push_str(&trapping.join(", "));
        text.push_str("\n  inert gate, release functions (");
        text.push_str(&report.inert_exports.len().to_string());
        text.push_str("): ");
        text.push_str(&report.inert_exports.join(", "));
    }
    Ok(text)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(Some(args)) => args,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("wasm-link: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("FAIL: wasm-link: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn a_bare_path_links_in_place() {
        let args = parse_args(&argv(&["pkg/a.wasm"]))
            .expect("parses")
            .expect("not help");
        assert_eq!(
            args,
            Args {
                check: false,
                verbose: false,
                out: None,
                input: PathBuf::from("pkg/a.wasm"),
            }
        );
    }

    #[test]
    fn check_and_out_and_verbose_are_read_in_any_order() {
        let args = parse_args(&argv(&["--verbose", "--check", "a.wasm"]))
            .expect("parses")
            .expect("not help");
        assert!(args.check && args.verbose && args.out.is_none());
        let args = parse_args(&argv(&["a.wasm", "--out", "b.wasm"]))
            .expect("parses")
            .expect("not help");
        assert_eq!(args.out, Some(PathBuf::from("b.wasm")));
    }

    #[test]
    fn help_two_paths_unknown_options_and_check_with_out_are_refused_or_help() {
        assert_eq!(parse_args(&argv(&["-h"])), Ok(None));
        assert!(parse_args(&argv(&["a.wasm", "b.wasm"])).is_err());
        assert!(parse_args(&argv(&["--frobnicate", "a.wasm"])).is_err());
        assert!(parse_args(&argv(&["--check", "--out", "b.wasm", "a.wasm"])).is_err());
        assert!(parse_args(&argv(&["--out"])).is_err());
        assert!(parse_args(&argv(&[])).is_err());
    }
}
