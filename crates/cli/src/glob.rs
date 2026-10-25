// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The compiled matcher command; all decisions use the public facade.
use crate::cli::{GlobMode, GlobSurface};
use crate::error::CliError;
use purrdf::glob::{CompiledGlob, IriComponent, Mode};
use std::io::Write as _;

pub(crate) fn run(
    pattern: &str,
    value: &str,
    mode: GlobMode,
    surface: GlobSurface,
    max_work: u64,
) -> Result<(), CliError> {
    let mode = match mode {
        GlobMode::Text => Mode::Text,
        GlobMode::Path => Mode::Path,
    };
    let glob = CompiledGlob::compile(pattern, mode).map_err(CliError::GlobCompile)?;
    let component = match surface {
        GlobSurface::Literal => None,
        GlobSurface::Iri => Some(IriComponent::Lexical),
        GlobSurface::IriScheme => Some(IriComponent::Scheme),
        GlobSurface::IriAuthority => Some(IriComponent::Authority),
        GlobSurface::IriPath => Some(IriComponent::Path),
        GlobSurface::IriQuery => Some(IriComponent::Query),
        GlobSurface::IriFragment => Some(IriComponent::Fragment),
    };
    let report = if let Some(component) = component {
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
