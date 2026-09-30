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
use purrdf_iri::BaseScope;
use purrdf_rdf::SourceFormat;
use purrdf_validate::regime::MaterializeLimits;

use crate::cli::{CliRdfFormat, ReportTarget};
use crate::error::{CliError, argv_iri_refusal};
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

/// Decide every `--import IRI=FILE` ARGUMENT, with no I/O: the pair's shape, and the
/// ontology-IRI half as an ABSOLUTE IRI.
///
/// A malformed pair is a usage error naming the argument, never a skipped import: a premise
/// answered without a document the operator supplied is answered over a different premise.
///
/// # Why the half must be absolute, rather than resolved against something
///
/// The half is compared with the premise's `owl:imports` OBJECTS, which are absolute by the
/// time the parser is done with them. So `foo` matched nothing, and the only thing the
/// operator saw was a refusal naming the premise's `owl:imports` — a typo in an ARGUMENT
/// reported as a defect in their DATA.
///
/// Resolving the half against a base this command guessed would not fix that; it would hide
/// it. Which base an `owl:imports` object resolved under is the PREMISE's business — it may
/// declare its own `@base`, and one document may rebind it several times — so a base picked
/// here would turn `foo` into some absolute IRI that still matches nothing. The half is
/// therefore required to be absolute, through the shared [`BaseScope`] with NO base in
/// scope: an absolute value is carried lexical-verbatim, and anything else is refused
/// against the command line by [`argv_iri_refusal`](crate::error::argv_iri_refusal), naming the flag, the pair and the
/// offending half.
pub(crate) fn parse_pairs(specs: &[String]) -> Result<Vec<(String, &str)>, CliError> {
    // No base, deliberately: see the section above. `BaseScope` is still the seam, so the
    // codes an operator sees here are the workspace's shared `purrdf_iri` spellings rather
    // than a private one this module invented.
    let scope = BaseScope::empty();
    let mut resolved: Vec<(String, &str)> = Vec::with_capacity(specs.len());
    for spec in specs {
        let Some((iri, path)) = split_import(spec) else {
            return Err(CliError::Usage(format!(
                "--import {spec}: an import pair is `IRI=FILE` — the ontology IRI the premise \
                 declares, then the local document that resolves it — and this one has no `=`"
            )));
        };
        if iri.is_empty() || path.is_empty() {
            return Err(CliError::Usage(format!(
                "--import {spec}: both halves of `IRI=FILE` are required — the ontology IRI \
                 names what the premise imports, and the path names the document that is it"
            )));
        }
        let absolute = scope.resolve(iri).map_err(|error| {
            argv_iri_refusal(
                &format!("--import {spec}"),
                &error,
                "iri-relative-no-base",
                &format!(
                    "the ontology-IRI half `{iri}` is a relative IRI reference, and it is \
                         matched against the premise's `owl:imports` objects, which are \
                         absolute. It can therefore resolve no import at all. This is a \
                         command-line value, so no `@base` in any document reaches it and none \
                         is guessed for it: write the half as the absolute IRI the premise's \
                         `owl:imports` names"
                ),
                &format!("the ontology-IRI half `{iri}` is not a usable IRI: "),
            )
        })?;
        resolved.push((absolute.as_str().to_owned(), path));
    }
    Ok(resolved)
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

/// Refuse a command line that reads standard input twice: the premise and an `--import`
/// document, or two `--import` documents. A process has ONE standard input, so two readers
/// would each get part of one stream.
pub(crate) fn refuse_two_stdins(
    premises: &[&str],
    pairs: &[(String, &str)],
) -> Result<(), CliError> {
    let mut named: Vec<String> = premises
        .iter()
        .filter(|path| **path == "-")
        .map(|_| "the premise".to_owned())
        .collect();
    named.extend(
        pairs
            .iter()
            .filter(|(_, path)| *path == "-")
            .map(|(iri, _)| format!("--import {iri}=-")),
    );
    if named.len() > 1 {
        return Err(CliError::Usage(format!(
            "{} each read standard input, and there is only one: a process has a single stdin \
             stream, so two documents reading it would each get part of one. Give all but one \
             of them a path",
            named.join(" and ")
        )));
    }
    Ok(())
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
        let pairs = parse_pairs(specs)?;
        let paths: Vec<&str> = premises.iter().map(|(path, _)| *path).collect();
        refuse_two_stdins(&paths, &pairs)?;
        let mut map = ImportMap::default();
        for (iri, path) in &pairs {
            let document_format = format::resolve(from, path)?;
            let document = source::load_dataset(path, document_format, base)?;
            if map.insert(iri.clone(), document).is_some() {
                return Err(CliError::Usage(format!(
                    "--import {iri}: the ontology IRI is named by two pairs; keeping either \
                     document would be a choice the command line did not make"
                )));
            }
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
        let mut legs = Vec::new();
        for spec in specs {
            if let Some((iri, path)) = split_import(spec)
                && !iri.is_empty()
                && !path.is_empty()
            {
                legs.push((
                    format::resolve(from, path)?,
                    format!("the --import {iri} document"),
                ));
            }
        }
        Ok(legs)
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
