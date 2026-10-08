// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fresh-directory native university corpus executable.
use purrdf_bench::lubm::{Spec, generate_directory};
use std::collections::BTreeMap;
use std::process::ExitCode;

const USAGE: &str = "Usage: lubm-corpus --universities N --index N --seed N --ontology IRI --document-base IRI/ --out DIRECTORY\nEvery option is required and may occur exactly once. DIRECTORY must not exist.\n";
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        print!("{USAGE}");
        return Ok(());
    }
    let mut values = BTreeMap::new();
    let mut at = 0;
    while at < args.len() {
        let name = &args[at];
        if ![
            "--universities",
            "--index",
            "--seed",
            "--ontology",
            "--document-base",
            "--out",
        ]
        .contains(&name.as_str())
        {
            return Err(format!("unknown option {name}"));
        }
        let value = args
            .get(at + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("missing operand for {name}"))?;
        if values.insert(name.as_str(), value.as_str()).is_some() {
            return Err(format!("duplicate option {name}"));
        }
        at += 2;
    }
    let get = |name| {
        values
            .get(name)
            .copied()
            .ok_or_else(|| format!("missing {name}"))
    };
    let spec = Spec::from_decimal(
        get("--seed")?,
        get("--index")?,
        get("--universities")?,
        get("--ontology")?.into(),
        get("--document-base")?.into(),
    )?;
    generate_directory(&spec, std::path::Path::new(get("--out")?))
        .map_err(|error| format!("generation failed: {error}"))?;
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}
