// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `--import IRI=FILE` for the entailment subcommands that take a premise's import table:
//! `reason`, `convert --entailment`, `query --entailment`, `entails` and `consistency`.
//!
//! OWL 2 defines an ontology's imports closure to BE the ontology, so a premise carrying an
//! `owl:imports` is not closed over itself alone: every entailment lane closes over the
//! kernel's one import closure ([`purrdf_core::imports`]), the rule `entails` and `validate`
//! already apply. PurRDF fetches nothing and mints no vocabulary, so each document of the
//! closure is caller-supplied configuration: one `--import` pair resolves one ontology IRI
//! to one local document. The refusals are the ones `validate` gives a shapes graph:
//!
//! * an `owl:imports` no pair resolves, that does not name the premise document itself
//!   (its `file://` retrieval IRI or `--base`), and whose ontology the premise does not
//!   already hold, is refused BY NAME (exit 1) — never closed as a smaller premise;
//! * a pair the premise's closure never reaches is a USAGE error (exit 2): the document
//!   would be read and never used, and the fault is in the command line;
//! * a malformed pair, or an ontology-IRI half that is not an absolute IRI, is a usage
//!   error naming the argument, decided before any document is opened.
//!
//! # The common case costs nothing
//!
//! A premise that imports nothing, closed with no `--import`, runs exactly as it did: the
//! anchors are read off the zero-copy view ([`purrdf_core::imports::imported_iris`]) and the
//! closure is materialized over the same view, so a pack is still never rebuilt. Only a
//! premise that DOES import something, or a command line that supplies a pair, pays for an
//! owned dataset to resolve the closure against.

use std::sync::Arc;

use purrdf_core::DatasetView;
use purrdf_core::imports::imported_iris;
use purrdf_entail::{ImportMap, Materialization};
use purrdf_rdf::SourceFormat;
use purrdf_validate::regime::MaterializeLimits;

use crate::argv_documents::{
    ImportRole, import_document_legs, import_readers, parse_import_pairs, refuse_shared_stdin,
};
use crate::cli::{CliRdfFormat, ReportTarget};
use crate::error::CliError;
use crate::format;
use crate::report;
use crate::source;

/// The `(ontology-iri, path)` halves of one `--import` argument, or `None` when it has no
/// `=`.
///
/// The IRI is everything before the FIRST `=`, which is the conventional reading of a
/// `KEY=VALUE` argument; a path containing `=` therefore works and an ontology IRI
/// containing one does not, and that trade is stated rather than discovered.
pub(crate) fn split_import(spec: &str) -> Option<(&str, &str)> {
    spec.split_once('=')
}

/// The IRI a premise document was read FROM — the base it parsed under, which is its
/// `file://` retrieval IRI or `--base`. An `owl:imports` of that IRI names the premise
/// itself, so it resolves in place. A container (pack, GTS) stores resolved IRIs and has no
/// document base.
pub(crate) fn premise_iri(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
) -> Result<Option<String>, CliError> {
    match format {
        SourceFormat::Native(native) => source::effective_base(path, native, base),
        SourceFormat::Pack | SourceFormat::Gts => Ok(None),
    }
}

/// A premise's `--import` table, resolved against the command line and read, ready to close
/// a premise over its `owl:imports` closure.
#[derive(Debug, Default)]
pub(crate) struct PremiseImports {
    /// The supplied documents, and the IRIs the premise was read under.
    map: ImportMap,
}

impl PremiseImports {
    /// Decide every `--import` pair (no I/O), refuse a second stdin, read each document
    /// through the CLI's own format resolution (`--from`, else the path's extension) and
    /// `--base`, and declare the premise documents' own IRIs loaded.
    ///
    /// `premises` are the premise sources `(path, format)` — one, or `convert`'s merged
    /// list, each of whose IRIs names part of the premise.
    ///
    /// # Errors
    ///
    /// A usage error for a malformed pair, a relative ontology-IRI half, two stdins or one
    /// ontology IRI named twice; a runtime error for a document that cannot be read.
    pub(crate) fn read(
        specs: &[String],
        from: Option<CliRdfFormat>,
        base: Option<&str>,
        premises: &[(&str, SourceFormat)],
    ) -> Result<Self, CliError> {
        let pairs = parse_import_pairs(specs, ImportRole::PREMISE)?;
        let mut readers: Vec<(String, &str)> = premises
            .iter()
            .map(|(path, _)| ("the premise".to_owned(), *path))
            .collect();
        readers.extend(import_readers(&pairs));
        refuse_shared_stdin(&readers)?;
        let mut map = ImportMap::default();
        for pair in &pairs {
            let document_format = format::resolve(from, pair.path)?;
            let document = source::load_dataset(pair.path, document_format, base)?;
            // `parse_import_pairs` already blamed the argument for a repeated or relative IRI;
            // the table's own key policy is the one every insertion site shares.
            map.try_insert(pair.iri, document)
                .map_err(|error| CliError::Usage(format!("--import {}=…: {error}", pair.iri)))?;
        }
        for (path, premise_format) in premises {
            if let Some(iri) = premise_iri(path, *premise_format, base)? {
                map.declare_loaded(iri);
            }
        }
        Ok(Self { map })
    }

    /// The resolved import table: the supplied documents, and the IRIs the premise was read
    /// under.
    pub(crate) const fn map(&self) -> &ImportMap {
        &self.map
    }

    /// The parse legs the `--import` documents give `--base`, for
    /// [`format::refuse_unconsumable_base`]: each document parses under it.
    pub(crate) fn base_legs(
        specs: &[String],
        from: Option<CliRdfFormat>,
    ) -> Result<Vec<(SourceFormat, String)>, CliError> {
        Ok(import_document_legs(specs, from)?
            .into_iter()
            .map(|(_path, format, role)| (format, role))
            .collect())
    }

    /// Close `view` under `plan` together with its `owl:imports` closure, surfacing the
    /// report to `target`.
    ///
    /// A premise that imports nothing, with no pair supplied, is materialized over `view`
    /// directly — the zero-copy lane is untouched. Otherwise the view becomes an owned
    /// dataset, the closure is resolved against the pairs (refusing an unresolved import and
    /// an unreached pair) and the merge is closed.
    ///
    /// # Errors
    ///
    /// Everything [`report::materialize_reported_with_imports`] refuses.
    pub(crate) fn materialize<D: DatasetView>(
        &self,
        view: &D,
        plan: Materialization<'_>,
        limits: &MaterializeLimits,
        target: &ReportTarget,
    ) -> Result<Arc<purrdf_core::RdfDataset>, CliError> {
        let loaded: Vec<&str> = self.map.loaded().collect();
        if self.map.is_empty() && imported_iris(view, &loaded).is_empty() {
            return report::materialize_reported(view, plan, limits, target);
        }
        let premise = purrdf_core::dataset_from_view(view)?;
        report::materialize_reported_with_imports(&premise, plan, &self.map, limits, target)
    }
}
