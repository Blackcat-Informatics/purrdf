// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Run the community SHACL corpus and retain its evidence.
//!
//! Usage: `community-conformance CORPUS NEW_OUTPUT_DIRECTORY`
//!
//! The output directory must not exist. It receives `records.json` (one graded
//! record per public route), `earl.nt` (the same verdicts as EARL assertions)
//! and `scoreboard.txt`. The exit status is nonzero unless every applicable
//! execution passed and none was unsupported.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use purrdf_lex::json::{self, Value, record::ToJson};
use purrdf_sparql_conformance::community::Corpus;

const ASSERTED_BY: &str = "http://example.org/community/runner/community-conformance";
const SUBJECT: &str = "http://example.org/community/subject/purrdf-native";

fn run(corpus: &Path, output: &Path) -> Result<bool, String> {
    let corpus = Corpus::acquire(corpus)?;
    std::fs::create_dir(output).map_err(|error| format!("{}: {error}", output.display()))?;
    let (totals, executions) = corpus.run()?;
    let records: Vec<_> = executions
        .iter()
        .flat_map(|execution| execution.records.iter().cloned())
        .collect();
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(output.join(name), bytes).map_err(|error| format!("{name}: {error}"))
    };
    write(
        "records.json",
        json::write_pretty(&Value::array(records.iter().map(ToJson::to_json))).as_bytes(),
    )?;
    let earl = purrdf_conformance_kit::outcome::to_earl(&records, ASSERTED_BY, SUBJECT)
        .map_err(|error| error.to_string())?;
    write(
        "earl.nt",
        &purrdf::serialize_dataset(
            earl.as_ref(),
            "application/n-triples",
            purrdf::SerializeGraph::Dataset,
        )
        .map_err(|error| error.to_string())?,
    )?;
    let scoreboard = totals.scoreboard();
    write("scoreboard.txt", scoreboard.as_bytes())?;
    print!("{scoreboard}");
    Ok(totals.failed == 0 && totals.unsupported == 0 && totals.passed == totals.executions)
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [corpus, output] = arguments.as_slice() else {
        eprintln!("usage: community-conformance CORPUS NEW_OUTPUT_DIRECTORY");
        return ExitCode::from(2);
    };
    match run(corpus, output) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("community-conformance: {error}");
            ExitCode::from(2)
        }
    }
}
