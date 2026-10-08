// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native workload admission, graph acceptance, namespace projection and result checks.
use purrdf_bench::lubm::{Spec, check};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;
fn artifact(bytes: &[u8], name: &str) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut executable = None;
    for line in text.lines() {
        let record = purrdf_lex::json::read(line).map_err(|e| e.to_string())?;
        let string = |key| record.get(key).and_then(purrdf_lex::json::Value::as_str);
        if string("reason") == Some("compiler-artifact")
            && record
                .get("target")
                .and_then(|target| target.get("name"))
                .and_then(purrdf_lex::json::Value::as_str)
                == Some(name)
            && let Some(path) = string("executable")
            && executable.replace(path.to_string()).is_some()
        {
            return Err(format!("duplicate Cargo executable artifact for {name}"));
        }
    }
    executable.ok_or_else(|| format!("Cargo did not report an executable artifact for {name}"))
}
const USAGE: &str = "Usage:\n  lubm-check config SEED INDEX UNIVERSITIES ONTOLOGY DOCUMENT_BASE\n  lubm-check verify DIRECTORY CONVERTED_DIRECTORY AGGREGATE SEED INDEX UNIVERSITIES ONTOLOGY DOCUMENT_BASE ACCEPTANCE\n  lubm-check recheck DIRECTORY CONVERTED_DIRECTORY AGGREGATE SEED INDEX UNIVERSITIES ONTOLOGY DOCUMENT_BASE ACCEPTANCE\n  lubm-check output DIRECTORY\n  lubm-check executable\n  lubm-check artifact TARGET < cargo-build.jsonl\n  lubm-check project ONTOLOGY < external-ontology.nq > projected-ontology.nq\n  lubm-check results [ACCEPTANCE Q1|Q14] < results.json\n";
fn spec(args: &[String]) -> Result<Spec, String> {
    let [seed, index, count, ontology, base] = args else {
        return Err(USAGE.into());
    };
    Spec::from_decimal(seed, index, count, ontology.clone(), base.clone())
}
fn stdin() -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}
fn run(args: &[String]) -> Result<(), String> {
    let Some((command, args)) = args.split_first() else {
        return Err(USAGE.into());
    };
    match (command.as_str(), args) {
        ("config", args) => {
            let admitted = spec(args)?;
            println!("{}\t{}", admitted.seed, admitted.index);
        }
        (
            "verify" | "recheck",
            [
                directory,
                converted,
                aggregate,
                seed,
                index,
                count,
                ontology,
                base,
                output,
            ],
        ) => {
            let spec = spec(&[
                seed.clone(),
                index.clone(),
                count.clone(),
                ontology.clone(),
                base.clone(),
            ])?;
            let acceptance = check::verify(
                Path::new(directory),
                Path::new(converted),
                Path::new(aggregate),
                &spec,
            )?;
            if command == "verify" {
                check::write_acceptance(Path::new(output), &acceptance)?;
            } else {
                let retained: check::Acceptance = purrdf_lex::json::record::from_slice(
                    &std::fs::read(output).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if retained != acceptance {
                    return Err("native graph acceptance changed during the run".into());
                }
            }
        }
        ("executable", []) => println!(
            "{}",
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .display()
        ),
        ("artifact", [name]) => println!("{}", artifact(&stdin()?, name)?),
        ("output", [directory]) => check::admit_output(Path::new(directory))?,
        ("project", [ontology]) => {
            std::io::stdout()
                .write_all(&check::project(&stdin()?, ontology)?)
                .map_err(|e| e.to_string())?;
        }
        ("results", []) => println!("{}", check::results(&stdin()?, None)?),
        ("results", [path, id]) => {
            let acceptance: check::Acceptance = purrdf_lex::json::record::from_slice(
                &std::fs::read(path).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            println!("{}", check::results(&stdin()?, Some((&acceptance, id)))?);
        }
        ("--help" | "-h", []) => print!("{USAGE}"),
        _ => return Err(USAGE.into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
