// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `bench-corpus` — stream a shard of the deterministic scale corpus as
//! N-Quads. See [`USAGE`] for the full CLI contract; `--help`/`-h` prints the
//! same text.

use std::io::{BufWriter, Write as _};
use std::process::ExitCode;

use purrdf_bench::{CorpusSpec, manifest, write_row};

const FLUSH_EVERY_BYTES: usize = 1 << 20;

/// The single source of truth for the CLI contract: printed by `--help`/`-h`
/// (to stdout) and echoed to stderr alongside every parse error.
const USAGE: &str = "\
Usage: bench-corpus --quads N --iris N [--seed S] [--shard K --shards M] [--out PATH] [--manifest]
       bench-corpus --help | -h

Streams a shard of the deterministic scale corpus as N-Quads.

  --seed S     splitmix64 seed (default 0x5EED_CAFE)
  --quads N    total quads across all shards (positive)
  --iris N     distinct-IRI target (positive)
  --shard K    this shard's zero-based index (default 0)
  --shards M   total shard count (default 1)
  --out PATH   write output to PATH instead of stdout
  --manifest   print the JSON manifest instead of rows
  --help, -h   print this usage text and exit

`--manifest` prints the JSON manifest (profile id, parameters, shard row
range, class mix) instead of rows; a capture records the manifest beside the
output digest. Output goes to stdout unless `--out` is given; `--manifest`
honours `--out` too.

Every flag may be given at most once.
";

/// Every flag token this CLI recognizes, including both spellings of help.
/// Used to detect an unrecognized argument and to reject a flag-shaped
/// `--out` operand (a real path may legitimately start with `-`, so only
/// these exact tokens are rejected there).
const RECOGNIZED_FLAGS: [&str; 9] = [
    "--seed",
    "--quads",
    "--iris",
    "--shard",
    "--shards",
    "--out",
    "--manifest",
    "--help",
    "-h",
];

/// The outcome of a successful parse.
#[derive(Debug)]
enum ParsedArgs {
    /// `--help`/`-h` was requested: print [`USAGE`] and exit successfully.
    Help,
    /// A corpus to generate, where to send it, and in what shape.
    Run {
        /// The validated corpus specification.
        spec: CorpusSpec,
        /// `--out PATH`, or `None` for stdout.
        out: Option<String>,
        /// Whether to emit the JSON manifest instead of rows.
        want_manifest: bool,
    },
}

/// Marks `*seen` and errors if `name` was already given once before.
fn reject_duplicate(seen: &mut bool, name: &str) -> Result<(), String> {
    if *seen {
        return Err(format!("{name} may not be given more than once"));
    }
    *seen = true;
    Ok(())
}

/// Consumes and parses the next argument as an unsigned integer operand for
/// `name`, erroring if it is absent or not a valid `u64`.
fn take_u64(arguments: &mut impl Iterator<Item = String>, name: &str) -> Result<u64, String> {
    arguments
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| format!("{name} requires an unsigned integer"))
}

/// Consumes and validates the path operand for `--out`: it must be present
/// and must not be one of the recognized flag tokens. A path is otherwise
/// unconstrained — in particular a leading `-` is legal in a path and must
/// not be rejected.
fn take_out_path(arguments: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let operand = arguments
        .next()
        .ok_or_else(|| "--out requires a path".to_string())?;
    if RECOGNIZED_FLAGS.contains(&operand.as_str()) {
        return Err(format!(
            "--out requires a path, not the recognized flag {operand:?}"
        ));
    }
    Ok(operand)
}

/// Parses an already-tokenized argument list (everything after `argv[0]`).
fn parse_args_from(mut arguments: impl Iterator<Item = String>) -> Result<ParsedArgs, String> {
    let mut seed = 0x5EED_CAFEu64;
    let mut quads = 0u64;
    let mut iris = 0u64;
    let mut shard = 0u64;
    let mut shards = 1u64;
    let mut out = None;
    let mut want_manifest = false;
    let mut want_help = false;

    let mut seen_seed = false;
    let mut seen_quads = false;
    let mut seen_iris = false;
    let mut seen_shard = false;
    let mut seen_shards = false;
    let mut seen_out = false;
    let mut seen_manifest = false;
    let mut seen_help = false;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--seed" => {
                reject_duplicate(&mut seen_seed, "--seed")?;
                seed = take_u64(&mut arguments, "--seed")?;
            }
            "--quads" => {
                reject_duplicate(&mut seen_quads, "--quads")?;
                quads = take_u64(&mut arguments, "--quads")?;
            }
            "--iris" => {
                reject_duplicate(&mut seen_iris, "--iris")?;
                iris = take_u64(&mut arguments, "--iris")?;
            }
            "--shard" => {
                reject_duplicate(&mut seen_shard, "--shard")?;
                shard = take_u64(&mut arguments, "--shard")?;
            }
            "--shards" => {
                reject_duplicate(&mut seen_shards, "--shards")?;
                shards = take_u64(&mut arguments, "--shards")?;
            }
            "--out" => {
                reject_duplicate(&mut seen_out, "--out")?;
                out = Some(take_out_path(&mut arguments)?);
            }
            "--manifest" => {
                reject_duplicate(&mut seen_manifest, "--manifest")?;
                want_manifest = true;
            }
            "--help" | "-h" => {
                reject_duplicate(&mut seen_help, "--help")?;
                want_help = true;
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }

    if want_help {
        return Ok(ParsedArgs::Help);
    }
    let spec =
        CorpusSpec::new(seed, quads, iris, shard, shards).map_err(|error| error.to_string())?;
    Ok(ParsedArgs::Run {
        spec,
        out,
        want_manifest,
    })
}

/// Parses the real process arguments.
fn parse_args() -> Result<ParsedArgs, String> {
    parse_args_from(std::env::args().skip(1))
}

/// Writes `bytes` to `writer` and flushes, reporting any failure through the
/// one error discipline shared by both the manifest and the row output
/// paths: a message on stderr and [`ExitCode::FAILURE`] — never a panic.
fn write_checked(writer: &mut impl std::io::Write, bytes: &[u8]) -> ExitCode {
    match writer.write_all(bytes).and_then(|()| writer.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bench-corpus: write failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let (spec, out_path, want_manifest) = match parse_args() {
        Ok(ParsedArgs::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(ParsedArgs::Run {
            spec,
            out,
            want_manifest,
        }) => (spec, out, want_manifest),
        Err(message) => {
            eprintln!("bench-corpus: {message}");
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    // The sink is built once, before the manifest/rows branch, so both
    // shapes of output share the same destination and the same checked
    // write path below.
    let sink: Box<dyn std::io::Write> = match &out_path {
        Some(path) => match std::fs::File::create(path) {
            Ok(file) => Box::new(file),
            Err(error) => {
                eprintln!("bench-corpus: cannot create {path:?}: {error}");
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(std::io::stdout().lock()),
    };
    let mut writer = BufWriter::with_capacity(FLUSH_EVERY_BYTES * 2, sink);

    if want_manifest {
        return write_checked(&mut writer, manifest(&spec).as_bytes());
    }

    let (start, end) = spec.shard_range();
    let mut buffer = String::with_capacity(FLUSH_EVERY_BYTES + 4096);
    for slot in start..end {
        write_row(&mut buffer, &spec, slot);
        if buffer.len() >= FLUSH_EVERY_BYTES {
            if let Err(error) = writer.write_all(buffer.as_bytes()) {
                eprintln!("bench-corpus: write failed: {error}");
                return ExitCode::FAILURE;
            }
            buffer.clear();
        }
    }
    write_checked(&mut writer, buffer.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{ParsedArgs, parse_args_from};

    fn args(values: &[&str]) -> std::vec::IntoIter<String> {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn run_spec(values: &[&str]) -> (u64, u64, u64, u64, u64, Option<String>, bool) {
        match parse_args_from(args(values)).expect("expected a successful parse") {
            ParsedArgs::Help => panic!("expected Run, got Help"),
            ParsedArgs::Run {
                spec,
                out,
                want_manifest,
            } => (
                spec.seed(),
                spec.quads(),
                spec.iris(),
                spec.shard(),
                spec.shards(),
                out,
                want_manifest,
            ),
        }
    }

    #[test]
    fn out_requires_an_operand() {
        let error = parse_args_from(args(&["--quads", "10", "--iris", "10", "--out"])).unwrap_err();
        assert!(error.contains("--out"), "unexpected message: {error}");
        // Neighbouring valid input: --out followed by a real path succeeds.
        let (.., out, _) = run_spec(&["--quads", "10", "--iris", "10", "--out", "/tmp/ok.nq"]);
        assert_eq!(out.as_deref(), Some("/tmp/ok.nq"));
    }

    #[test]
    fn out_rejects_a_recognized_flag_as_its_operand() {
        for flag_shaped in ["--manifest", "--seed", "--help", "-h"] {
            let error = parse_args_from(args(&[
                "--quads",
                "10",
                "--iris",
                "10",
                "--out",
                flag_shaped,
            ]))
            .unwrap_err();
            assert!(error.contains("--out"), "unexpected message: {error}");
        }
    }

    #[test]
    fn out_accepts_a_path_that_merely_starts_with_a_dash() {
        // A leading '-' is legal in a path; only the exact recognized flag
        // tokens are rejected as an --out operand.
        let (.., out, _) = run_spec(&["--quads", "10", "--iris", "10", "--out", "-weird.nq"]);
        assert_eq!(out.as_deref(), Some("-weird.nq"));
    }

    #[test]
    fn manifest_honours_out() {
        let (.., out, want_manifest) = run_spec(&[
            "--quads",
            "10",
            "--iris",
            "10",
            "--out",
            "/tmp/m.json",
            "--manifest",
        ]);
        assert_eq!(out.as_deref(), Some("/tmp/m.json"));
        assert!(want_manifest);
    }

    #[test]
    fn duplicate_flags_are_rejected() {
        let error =
            parse_args_from(args(&["--quads", "10", "--quads", "20", "--iris", "10"])).unwrap_err();
        assert!(error.contains("--quads"), "unexpected message: {error}");
    }

    #[test]
    fn duplicate_help_is_rejected() {
        let error = parse_args_from(args(&["--help", "--help"])).unwrap_err();
        assert!(error.contains("--help"), "unexpected message: {error}");
        let error = parse_args_from(args(&["--help", "-h"])).unwrap_err();
        assert!(error.contains("--help"), "unexpected message: {error}");
    }

    #[test]
    fn non_duplicate_flags_do_not_collide() {
        // Neighbouring valid input: distinct flags, including a shard count
        // that legitimately exceeds the quad count, must not be confused
        // with a duplicate.
        let (.., shard, shards, _, _) = run_spec(&[
            "--quads", "3", "--iris", "10", "--shard", "7", "--shards", "8",
        ]);
        assert_eq!((shard, shards), (7, 8));
    }

    #[test]
    fn help_short_and_long_both_short_circuit() {
        assert!(matches!(
            parse_args_from(args(&["--help"])),
            Ok(ParsedArgs::Help)
        ));
        assert!(matches!(
            parse_args_from(args(&["-h"])),
            Ok(ParsedArgs::Help)
        ));
        // --help must win even without the otherwise-required --quads/--iris.
        assert!(matches!(
            parse_args_from(args(&[])),
            Err(ref message) if message.contains("quads")
        ));
    }

    #[test]
    fn unknown_argument_is_rejected() {
        let error = parse_args_from(args(&["--bogus"])).unwrap_err();
        assert!(error.contains("--bogus"), "unexpected message: {error}");
    }

    #[test]
    fn manifest_flag_defaults_out_to_none() {
        let (.., out, want_manifest) = run_spec(&["--quads", "10", "--iris", "10", "--manifest"]);
        assert_eq!(out, None);
        assert!(want_manifest);
    }
}
