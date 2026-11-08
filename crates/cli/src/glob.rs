// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The compiled matcher command; all decisions use the public facade.
use crate::cli::{GlobMode, GlobSurface};
use crate::error::CliError;
use purrdf::glob::{CompiledGlob, CompiledGlobSet, IriComponent, Mode, SetCompileError};
use std::io::Write as _;

pub(crate) fn run(
    pattern: &str,
    value: &str,
    mode: GlobMode,
    surface: GlobSurface,
    max_work: u64,
) -> Result<(), CliError> {
    let glob = CompiledGlob::compile(pattern, glob_mode(mode)).map_err(CliError::GlobCompile)?;
    let report = if let Some(component) = iri_component(surface) {
        let mut storage = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut storage);
        let iri = purrdf_iri::parse_with_memory(value, &mut memory).map_err(CliError::GlobIri)?;
        glob.match_iri(&iri, component, max_work)
    } else {
        glob.match_literal(value, max_work)
    }
    .map_err(CliError::GlobMatch)?;
    let mut out = std::io::stdout().lock();
    writeln!(out, "matched {}\nwork {}", report.matched, report.work)?;
    out.flush()?;
    Ok(())
}

pub(crate) fn run_set(
    patterns: &[String],
    value: &str,
    mode: GlobMode,
    surface: GlobSurface,
    max_work: u64,
) -> Result<(), CliError> {
    let set =
        CompiledGlobSet::compile(patterns, glob_mode(mode)).map_err(CliError::GlobSetCompile)?;
    let mut storage = purrdf_lex::allocation::Resident;
    let mut workspace = Vec::new();
    let mut memory = purrdf_lex::allocation::Memory::new(&mut storage);
    memory
        .reserve(&mut workspace, patterns.len())
        .map_err(|cause| CliError::GlobSetCompile(SetCompileError::Storage(cause)))?;
    workspace.resize(patterns.len(), 0);
    let report = if let Some(component) = iri_component(surface) {
        let iri = purrdf_iri::parse_with_memory(value, &mut memory).map_err(CliError::GlobIri)?;
        set.match_iri(&iri, component, max_work, &mut workspace)
    } else {
        set.match_literal(value, max_work, &mut workspace)
    }
    .map_err(CliError::GlobSetMatch)?;
    let mut out = std::io::stdout().lock();
    writeln!(out, "matches {}", report.indices.len())?;
    for index in report.indices {
        writeln!(out, "index {index}")?;
    }
    writeln!(out, "work {}", report.work)?;
    out.flush()?;
    Ok(())
}

fn glob_mode(mode: GlobMode) -> Mode {
    match mode {
        GlobMode::Text => Mode::Text,
        GlobMode::Path => Mode::Path,
    }
}

fn iri_component(surface: GlobSurface) -> Option<IriComponent> {
    match surface {
        GlobSurface::Literal => None,
        GlobSurface::Iri => Some(IriComponent::Lexical),
        GlobSurface::IriScheme => Some(IriComponent::Scheme),
        GlobSurface::IriAuthority => Some(IriComponent::Authority),
        GlobSurface::IriPath => Some(IriComponent::Path),
        GlobSurface::IriQuery => Some(IriComponent::Query),
        GlobSurface::IriFragment => Some(IriComponent::Fragment),
    }
}
