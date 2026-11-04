// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Raw source and N-Quads transport around the shared native YAML boundary.

use crate::{
    canonical,
    error::{CliError, CliOutcome},
    sink, source,
};
use clap::Subcommand;
use purrdf_core::RdfDiagnostic;
use purrdf_validate::yaml_source as boundary;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Export exact YAML presentation as lossless N-Quads metadata.
    Export {
        #[arg(long)]
        source_iri: String,
        #[arg(long)]
        profile: String,
        /// Write the computed root IRI and newline to a distinct named file.
        #[arg(long)]
        root_output: Option<String>,
        input: String,
        output: String,
    },
    /// Reconstruct exact YAML bytes after native cover and reprojection checks.
    Reconstruct {
        #[arg(long)]
        root: String,
        #[arg(long)]
        profile: String,
        /// Raw RDF acquisition limit only; excludes parser/native decode storage.
        #[arg(long, default_value_t = 64 << 20)]
        max_rdf_bytes: u32,
        input: String,
        output: String,
    },
}

/// Original boundary, raw acquisition or N-Quads parser refusal.
#[derive(Debug)]
pub enum Failure {
    /// Exact original configuration or native carrier failure.
    Boundary(boundary::Error),
    /// Original N-Quads diagnostic with its native code and available position.
    Rdf(RdfDiagnostic),
    /// Raw file/stdin acquisition failed before document parsing.
    Input {
        /// The caller's input path, or `-` for stdin.
        path: String,
        /// Original I/O, physical allocation or content-limit refusal.
        cause: source::RawReadError,
    },
}
impl std::fmt::Display for Failure {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boundary(cause) => cause.fmt(out),
            Self::Rdf(cause) => cause.fmt(out),
            Self::Input { path, cause } => write!(out, "{path}: {cause}"),
        }
    }
}
impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Boundary(cause) => Some(cause),
            Self::Rdf(cause) => Some(cause),
            Self::Input { cause, .. } => Some(cause),
        }
    }
}
impl purrdf_hash::error::StructuredError for Failure {
    purrdf_hash::error_visit! {
        Boundary(cause) => "cli-yaml-source-boundary" { "cause" => Cause(cause) },
        Rdf(cause) => "cli-yaml-source-rdf" { "cause" => Cause(cause) },
        Input { path, cause } => "cli-yaml-source-input" { "path" => Text(path), "cause" => Cause(cause) },
    }
}

fn acquire(path: &str, maximum: usize) -> Result<Vec<u8>, CliError> {
    source::read_raw_bytes_bounded(path, maximum).map_err(|cause| {
        failure(Failure::Input {
            path: path.to_owned(),
            cause,
        })
    })
}
fn failure(cause: Failure) -> CliError {
    CliError::YamlSource(Box::new(cause))
}

impl Command {
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        let (profile, input, output, root) = match self {
            Self::Export {
                profile,
                input,
                output,
                root_output,
                ..
            } => (profile, input, output, root_output.as_ref()),
            Self::Reconstruct {
                profile,
                input,
                output,
                ..
            } => (profile, input, output, None),
        };
        [Some(profile), Some(input), Some(output), root]
            .into_iter()
            .flatten()
            .map(PathBuf::from)
            .collect()
    }

    pub(crate) fn run(&self, canonical: &mut canonical::Request) -> Result<CliOutcome, CliError> {
        let (profile_path, input, output) = match self {
            Self::Export {
                profile,
                input,
                output,
                root_output,
                ..
            } => {
                if root_output.as_deref() == Some("-") {
                    return Err(CliError::Usage(
                        "--root-output requires a named file".to_owned(),
                    ));
                }
                (profile, input, output)
            }
            Self::Reconstruct {
                profile,
                input,
                output,
                ..
            } => (profile, input, output),
        };
        if profile_path == "-" {
            return Err(CliError::Usage(
                "--profile requires a named file".to_owned(),
            ));
        }
        let paths = self.paths();
        let named: Vec<_> = paths
            .iter()
            .filter(|path| path.as_path() != Path::new("-"))
            .collect();
        for (index, first) in named.iter().enumerate() {
            for second in &named[index + 1..] {
                if canonical::resolve(first)? == canonical::resolve(second)?
                    || canonical::same_file(first, second).map_err(CliError::FileIdentity)?
                {
                    return Err(CliError::Usage(format!(
                        "YAML source paths {} and {} alias; choose distinct files",
                        first.display(),
                        second.display()
                    )));
                }
            }
        }
        let config = acquire(profile_path, boundary::MAX_PROFILE_BYTES)?;
        let profile =
            boundary::profile(&config).map_err(|cause| failure(Failure::Boundary(cause)))?;
        match self {
            Self::Export {
                source_iri,
                root_output,
                ..
            } => {
                let bytes = acquire(input, profile.bounds().syntax.syntax.max_input_bytes)?;
                let exported = boundary::export(source_iri, &bytes, &profile)
                    .map_err(|cause| failure(Failure::Boundary(cause)))?;
                sink::write_rdf(
                    &exported.dataset,
                    output,
                    purrdf_rdf::SourceFormat::Native(purrdf_rdf::NativeRdfFormat::NQuads),
                    None,
                    None,
                    None,
                    canonical,
                )?;
                if let Some(path) = root_output {
                    let mut root = exported.root;
                    let mut resident = purrdf_lex::allocation::Resident;
                    let mut memory =
                        purrdf_lex::allocation::Memory::resume(&mut resident, root.capacity());
                    memory.push_str(&mut root, "\n").map_err(|cause| {
                        failure(Failure::Boundary(boundary::YamlError::from(cause).into()))
                    })?;
                    sink::write_out(path, root.as_bytes())?;
                }
            }
            Self::Reconstruct {
                root,
                max_rdf_bytes,
                ..
            } => {
                let bytes = acquire(input, *max_rdf_bytes as usize)?;
                let dataset = purrdf_rdf::parse_dataset(&bytes, "application/n-quads", None)
                    .map_err(|cause| failure(Failure::Rdf(cause)))?;
                let text = boundary::reconstruct(&dataset, root, &profile)
                    .map_err(|cause| failure(Failure::Boundary(cause)))?;
                sink::write_out_with_request(output, text.as_bytes(), canonical)?;
            }
        }
        Ok(CliOutcome::Complete)
    }
}
