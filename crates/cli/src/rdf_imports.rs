// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent RDF import configuration admitted before source acquisition.

use crate::argv_documents::{ImportPair, ImportRole, parse_import_pairs};
use crate::cli::CliRdfFormat;
use crate::error::CliError;
use crate::{format, source};
use purrdf_rdf::{NativeRdfFormat, SourceFormat};

/// Resolved RDF import pairs and their independently admitted input controls.
/// No RDF source is acquired while preparing this plan.
pub(crate) struct RdfImportPlan<'a> {
    pairs: Vec<RdfImportPair<'a>>,
}

impl<'a> RdfImportPlan<'a> {
    /// Borrow the resolved formats and each import's own admitted bytes.
    pub(crate) fn pairs(&self) -> &[RdfImportPair<'a>] {
        &self.pairs
    }

    /// The original arguments, for the shared stdin ownership check.
    pub(crate) fn arguments(&self) -> impl Iterator<Item = ImportPair<'a>> + '_ {
        self.pairs.iter().map(|pair| ImportPair {
            spec: pair.spec,
            iri: pair.iri,
            path: pair.path,
        })
    }

    /// Validate every mapping and target before opening any options file, then decode all.
    pub(crate) fn prepare(
        imports: &'a [String],
        controls: &[String],
        role: ImportRole,
        from: Option<CliRdfFormat>,
    ) -> Result<Self, CliError> {
        let mut pairs = resolve_import_pairs(imports, role, from)?;
        let controls = parse_import_pairs(controls, ImportRole::INPUT_OPTIONS)?;
        let targets = controls
            .iter()
            .map(|control| {
                let index = pairs
                    .iter()
                    .position(|pair| pair.iri == control.iri)
                    .ok_or_else(|| {
                        CliError::Usage(format!(
                            "--import-yamlld-input-options {}: no --import pair names this IRI",
                            control.spec
                        ))
                    })?;
                let pair = &pairs[index];
                if pair.format != SourceFormat::Native(NativeRdfFormat::YamlLd) {
                    return Err(CliError::Usage(format!(
                        "--import-yamlld-input-options {}: --import {} must be YAML-LD",
                        control.spec, pair.spec
                    )));
                }
                Ok(index)
            })
            .collect::<Result<Vec<_>, CliError>>()?;
        for (control, index) in controls.iter().zip(targets) {
            pairs[index].input_options = Some(source::read_yamlld_input_options(
                std::path::Path::new(control.path),
                &[pairs[index].format],
            )?);
        }
        Ok(Self { pairs })
    }
}

/// One RDF import, fully admitted but not yet read.
pub(crate) struct RdfImportPair<'a> {
    /// The pair exactly as the operator wrote it, so a diagnostic can quote it back.
    pub(crate) spec: &'a str,
    /// The ontology-IRI half, checked absolute.
    pub(crate) iri: &'a str,
    /// The document-path half.
    pub(crate) path: &'a str,
    /// The resolved syntax, with the caller's format override applied when supplied.
    pub(crate) format: SourceFormat,
    /// This import's own admitted controls; absent means native defaults.
    pub(crate) input_options: Option<Vec<u8>>,
}

/// Decide every `--import IRI=FILE` ARGUMENT, with no I/O at all: the shared
/// [`parse_import_pairs`] rules, so the FIRST malformed pair is reported before the FIRST file
/// is opened, then the syntax selected by the override or each path's extension.
fn resolve_import_pairs(
    specs: &[String],
    role: ImportRole,
    from: Option<CliRdfFormat>,
) -> Result<Vec<RdfImportPair<'_>>, CliError> {
    parse_import_pairs(specs, role)?
        .into_iter()
        .map(|pair| {
            Ok(RdfImportPair {
                spec: pair.spec,
                iri: pair.iri,
                path: pair.path,
                format: format::resolve(from, pair.path)?,
                input_options: None,
            })
        })
        .collect()
}
