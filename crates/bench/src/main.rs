// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `bench-corpus` — stream a shard of the deterministic scale corpus as
//! N-Quads.
//!
//! Usage:
//! `bench-corpus --quads N --iris N [--seed S] [--shard K --shards M]
//!  [--out PATH] [--manifest]`
//!
//! `--manifest` prints the JSON manifest (profile id, parameters, shard row
//! range, class mix) instead of rows; a capture records the manifest beside
//! the output digest. Output goes to stdout unless `--out` is given.

use std::io::{BufWriter, Write as _};
use std::process::ExitCode;

use purrdf_bench::{CorpusSpec, manifest, write_row};

const FLUSH_EVERY_BYTES: usize = 1 << 20;

fn parse_args() -> Result<(CorpusSpec, Option<String>, bool), String> {
    let mut seed = 0x5EED_CAFEu64;
    let mut quads = 0u64;
    let mut iris = 0u64;
    let mut shard = 0u64;
    let mut shards = 1u64;
    let mut out = None;
    let mut want_manifest = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut take = |name: &str| -> Result<u64, String> {
            arguments
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| format!("{name} requires an unsigned integer"))
        };
        match argument.as_str() {
            "--seed" => seed = take("--seed")?,
            "--quads" => quads = take("--quads")?,
            "--iris" => iris = take("--iris")?,
            "--shard" => shard = take("--shard")?,
            "--shards" => shards = take("--shards")?,
            "--out" => out = arguments.next(),
            "--manifest" => want_manifest = true,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let spec =
        CorpusSpec::new(seed, quads, iris, shard, shards).map_err(|error| error.to_string())?;
    Ok((spec, out, want_manifest))
}

fn main() -> ExitCode {
    let (spec, out_path, want_manifest) = match parse_args() {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("bench-corpus: {message}");
            return ExitCode::from(2);
        }
    };
    if want_manifest {
        print!("{}", manifest(&spec));
        return ExitCode::SUCCESS;
    }
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
    if let Err(error) = writer
        .write_all(buffer.as_bytes())
        .and_then(|()| writer.flush())
    {
        eprintln!("bench-corpus: write failed: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
