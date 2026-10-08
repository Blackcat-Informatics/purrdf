// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Qualification-only CLI delegation with one exact-query fault.
//!
//! Set `LUBM_FAULT_REAL_BIN`, `LUBM_FAULT_QUERY_FILE` and `LUBM_FAULT_MODE`
//! (`malformed`, `exit` or `wrong-uri`), then name this executable as `LUBM_BIN`.
//! Version, conversion, regime probes and every other query execute the real CLI
//! with their original arguments and inherited streams. The selected published
//! query alone fails; this tool never supplies a passing benchmark answer.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn same_executable(real: &Path, own: &Path) -> Result<bool, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let real = real.metadata().map_err(|error| error.to_string())?;
        let own = own.metadata().map_err(|error| error.to_string())?;
        Ok(real.dev() == own.dev() && real.ino() == own.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = (real, own);
        Err("fault qualification requires Unix executable file identity".into())
    }
}

fn run(args: &[OsString]) -> Result<ExitCode, String> {
    let real = PathBuf::from(
        std::env::var_os("LUBM_FAULT_REAL_BIN")
            .ok_or("LUBM_FAULT_REAL_BIN must name the actual CLI")?,
    );
    let real = real
        .canonicalize()
        .map_err(|error| format!("resolve real CLI {}: {error}", real.display()))?;
    let own = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("resolve fault driver: {error}"))?;
    if same_executable(&real, &own)? || !real.is_file() {
        return Err("LUBM_FAULT_REAL_BIN must be a separate regular executable".into());
    }
    let query_file = std::env::var_os("LUBM_FAULT_QUERY_FILE")
        .ok_or("LUBM_FAULT_QUERY_FILE must name the exact selected query")?;
    let query = std::fs::read_to_string(&query_file)
        .map_err(|error| format!("read selected query {query_file:?}: {error}"))?;
    // Bash command substitution in the production lane removes trailing LF
    // bytes; retain all other bytes, including CR and significant whitespace.
    let query = query.trim_end_matches('\n');
    if query.is_empty() {
        return Err("selected fault query is empty".into());
    }
    let mode = std::env::var("LUBM_FAULT_MODE")
        .map_err(|error| format!("read LUBM_FAULT_MODE: {error}"))?;
    if !matches!(mode.as_str(), "malformed" | "exit" | "wrong-uri") {
        return Err(format!("unknown LUBM_FAULT_MODE {mode:?}"));
    }
    if args.first().is_some_and(|arg| arg == "query") && args.last().is_some_and(|arg| arg == query)
    {
        eprintln!("qualification fault: {mode} for exact query {query_file:?}");
        match mode.as_str() {
            "exit" => return Ok(ExitCode::from(83)),
            "malformed" => println!("deliberately malformed SPARQL result"),
            "wrong-uri" => println!(
                "{{\"head\":{{\"vars\":[\"X\"]}},\"results\":{{\"bindings\":[{{\"X\":{{\"type\":\"uri\",\"value\":\"https://example.org/qualification-deliberately-wrong\"}}}}]}}}}"
            ),
            _ => unreachable!("mode was admitted above"),
        }
        return Ok(ExitCode::SUCCESS);
    }
    let status = Command::new(&real)
        .args(args)
        .status()
        .map_err(|error| format!("execute real CLI {}: {error}", real.display()))?;
    let code = status
        .code()
        .ok_or_else(|| format!("real CLI terminated without an exit code: {status}"))?;
    let code = u8::try_from(code).map_err(|error| format!("real CLI exit {code}: {error}"))?;
    Ok(ExitCode::from(code))
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("LUBM fault driver: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::same_executable;

    #[test]
    fn executable_identity_refuses_links_but_distinguishes_separate_files() {
        let owned = purrdf_testkit::TempDir::for_unit_test().unwrap();
        let original = owned.path().join("original");
        let linked = owned.path().join("linked");
        let symbolic = owned.path().join("symbolic");
        let separate = owned.path().join("separate");
        std::fs::write(&original, b"identical bytes").unwrap();
        std::fs::write(&separate, b"identical bytes").unwrap();
        std::fs::hard_link(&original, &linked).unwrap();
        std::os::unix::fs::symlink(&original, &symbolic).unwrap();
        for alias in [&original, &linked, &symbolic] {
            assert!(same_executable(alias, &original).unwrap());
        }
        assert!(!same_executable(&separate, &original).unwrap());
    }
}
