// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native, report-only corpus expansion and forward-parser scaling measurements.

use std::{
    error::Error,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::Instant,
};

use purrdf_json::{SourceDocument, analyze, decode_document, project};
use purrdf_rdf::{NativeRdfFormat, parse_dataset, serialize_dataset_to_format};

#[path = "support/fixture.rs"]
mod fixture;

fn scaling() {
    let profile = fixture::profile();
    for rows in fixture::SIZES.into_iter().chain([10_000]) {
        let text = fixture::document(rows);
        let source = SourceDocument {
            id: fixture::SOURCE,
            bytes: text.as_bytes(),
        };
        let started = Instant::now();
        let repeats = 100;
        for _ in 0..repeats {
            black_box(analyze(source, &profile).unwrap());
        }
        println!(
            "scaling rows={rows} bytes={} iterations={repeats} total_us={}",
            text.len(),
            started.elapsed().as_micros()
        );
    }
}

fn files(
    root: &Path,
    output: &mut Vec<PathBuf>,
    symlinks: &mut usize,
) -> Result<(), Box<dyn Error>> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.is_symlink() {
        *symlinks += 1;
    } else if metadata.is_dir() {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if !["target", ".git", "node_modules"]
                .iter()
                .any(|name| entry.file_name() == *name)
            {
                files(&entry.path(), output, symlinks)?;
            }
        }
    } else if root
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        output.push(root.to_owned());
    }
    Ok(())
}

#[derive(Default)]
struct Totals {
    accepted: usize,
    refused: usize,
    source_bytes: usize,
    rdf_bytes: usize,
    relative_rdf_bytes: usize,
    statements: usize,
    values: usize,
    runs: usize,
    parse_us: u128,
    project_us: u128,
    decode_us: u128,
}

fn corpus(roots: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    let mut paths = Vec::new();
    let mut symlinks = 0;
    for root in roots {
        files(root, &mut paths, &mut symlinks)?;
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Err("corpus contains no JSON files".into());
    }
    let profile = fixture::profile();
    let mut totals = Totals::default();
    let mut identities = Vec::new();
    for path in &paths {
        let bytes = fs::read(path)?;
        let started = Instant::now();
        let result = analyze(
            SourceDocument {
                id: "urn:x:1",
                bytes: &bytes,
            },
            &profile,
        );
        let parse_us = started.elapsed().as_micros();
        let model = match result {
            Ok(model) => model,
            Err(error) => {
                totals.refused += 1;
                eprintln!("refused {}: {error}", path.display());
                continue;
            }
        };
        let started = Instant::now();
        let dataset = project(&model, &profile)?;
        let project_us = started.elapsed().as_micros();
        let rdf = serialize_dataset_to_format(&*dataset, NativeRdfFormat::Turtle, None)?;
        let relative =
            serialize_dataset_to_format(&*dataset, NativeRdfFormat::Turtle, Some(model.id()))?;
        let reparsed = parse_dataset(&relative.bytes, "text/turtle", None)?;
        let started = Instant::now();
        let decoded = decode_document(&reparsed, model.id(), &profile)?;
        let decode_us = started.elapsed().as_micros();
        if decoded.as_bytes() != bytes {
            return Err(format!("byte mismatch: {}", path.display()).into());
        }
        identities.push((*model.digest().as_bytes(), bytes.len() as u64));
        totals.accepted += 1;
        totals.source_bytes += bytes.len();
        totals.rdf_bytes += rdf.bytes.len();
        totals.relative_rdf_bytes += relative.bytes.len();
        totals.statements += dataset.quads().count();
        totals.values += model.values().len();
        totals.runs += model.structure().len();
        totals.parse_us += parse_us;
        totals.project_us += project_us;
        totals.decode_us += decode_us;
    }
    if totals.accepted == 0 {
        return Err("corpus contains no accepted JSON documents".into());
    }
    identities.sort_unstable();
    let mut identity_bytes = Vec::with_capacity(identities.len() * 40);
    for (digest, length) in identities {
        identity_bytes.extend_from_slice(&digest);
        identity_bytes.extend_from_slice(&length.to_le_bytes());
    }
    println!(
        "accepted_corpus_sha256={}",
        purrdf_core::ContentDigest::of(&identity_bytes).to_hex()
    );
    println!(
        "corpus files={} accepted={} refused={} skipped_symlinks={symlinks} source_bytes={} turtle_bytes={} base_relative_turtle_bytes={} statements={} values={} structure_runs={} parse_us={} project_us={} decode_us={}",
        paths.len(),
        totals.accepted,
        totals.refused,
        totals.source_bytes,
        totals.rdf_bytes,
        totals.relative_rdf_bytes,
        totals.statements,
        totals.values,
        totals.runs,
        totals.parse_us,
        totals.project_us,
        totals.decode_us
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let roots: Vec<_> = std::env::args_os()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .map(PathBuf::from)
        .collect();
    if roots.is_empty() {
        scaling();
        Ok(())
    } else {
        corpus(&roots)
    }
}
