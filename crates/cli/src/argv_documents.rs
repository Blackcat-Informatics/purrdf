// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The command line's document arguments, decided once for every subcommand: the
//! `--import IRI=FILE` parser and the one-standard-input rule.
//!
//! Both used to be written out per command (five refusals of a second stdin, four
//! `--import` parsers), and the copies drifted: one accepted a duplicate ontology IRI only
//! to notice it after reading, one skipped the empty-half check, one described a relative
//! IRI half in terms of a document `@base` the operator cannot edit. Each function here is
//! the strongest of the copies, so every command refuses the same things in the same
//! words. The helpers ledger (`helpers-ledger.toml`, job `cli-argv-documents`) fails the
//! gate if a command spells the refusal again.
//!
//! Everything here is decided from the ARGUMENTS alone, with no I/O, so a fault is a usage
//! error (exit 2) reported before the first document is opened.

use purrdf_iri::{BaseScope, IriError};

use crate::cli::CliRdfFormat;
use crate::error::{CliError, argv_iri_refusal};
use crate::format;
use purrdf_rdf::SourceFormat;

/// Whether a path names standard input.
pub(crate) fn is_stdin(path: &str) -> bool {
    path == "-"
}

/// The documents an `--import IRI=FILE` argument list names, each with its resolved source
/// format and the role a diagnostic names it by (`the --import IRI document`).
///
/// A spec with no `=`, or with an empty half, names no document here: the readers of
/// [`parse_import_pairs`] refuse it with the operator-facing message, so this is only the
/// format-resolution leg over the well-formed specs.
pub(crate) fn import_document_legs(
    specs: &[String],
    from: Option<CliRdfFormat>,
) -> Result<Vec<(&str, SourceFormat, String)>, CliError> {
    let mut legs = Vec::new();
    for spec in specs {
        if let Some((iri, path)) = crate::premise_imports::split_import(spec)
            && !iri.is_empty()
            && !path.is_empty()
        {
            legs.push((
                path,
                format::resolve(from, path)?,
                format!("the --import {iri} document"),
            ));
        }
    }
    Ok(legs)
}

/// One decided `--import IRI=FILE` argument.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ImportPair<'a> {
    /// The argument exactly as the operator wrote it, so a diagnostic can quote it back.
    pub(crate) spec: &'a str,
    /// The IRI half, an absolute IRI carried verbatim.
    pub(crate) iri: &'a str,
    /// The document-path half.
    pub(crate) path: &'a str,
}

/// What an `--import` names for one command: the words a refusal uses, and whether the
/// imported document may be read from standard input.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ImportRole {
    /// What imports: `the premise`, `the shapes graph`, ...
    importer: &'static str,
    /// What the IRI half is matched against, e.g. ``the `owl:imports` objects``.
    matched: &'static str,
    /// `None` when the imported document may be `-`; otherwise the remedy for `-`, which
    /// the command cannot read because it infers the document's syntax from the path.
    stdin_remedy: Option<&'static str>,
}

impl ImportRole {
    /// A premise (`reason`, `query`, `convert`, `entails`, `consistency`).
    pub(crate) const PREMISE: Self = Self {
        importer: "premise",
        matched: "`owl:imports` objects",
        stdin_remedy: None,
    };
    /// A SHACL shapes graph.
    pub(crate) const SHAPES_GRAPH: Self = Self {
        importer: "shapes graph",
        matched: "`owl:imports` objects",
        stdin_remedy: Some(
            "Write the document to a file, or name it with a recognized RDF extension",
        ),
    };
    /// A ShEx schema.
    pub(crate) const SHEX_SCHEMA: Self = Self {
        importer: "schema",
        matched: "`IMPORT` IRIs",
        stdin_remedy: Some("Write the document to a `.shex`/`.shexj` path"),
    };
    /// A SPARQL 1.2 RL rule set.
    pub(crate) const RULE_SET: Self = Self {
        importer: "rule set",
        matched: "`IMPORTS` IRIs",
        stdin_remedy: None,
    };
}

/// Decide every `--import IRI=FILE` argument, with no I/O.
///
/// A malformed pair is a usage error naming the argument, never a skipped import: a
/// document folded without a file the operator supplied is a different document. Refused:
/// a pair with no `=`, an empty half, a `-` path where the role cannot read one, an IRI half
/// that is not an absolute IRI, and one IRI named twice.
///
/// # The IRI half must be absolute
///
/// It is compared with the importing document's own import IRIs, which are absolute by the
/// time the parser is done with them, and it is the base the imported document parses
/// under. Resolving it against a base this command guessed would hide the fault rather than
/// fix it, so it is checked through [`BaseScope`] with NO base in scope: an absolute value is
/// carried lexical-verbatim, and anything else is refused against the command line by
/// [`argv_iri_refusal`], naming the flag, the pair and the offending half.
///
/// # Errors
///
/// [`CliError::Usage`] for the first pair that breaks a rule above.
pub(crate) fn parse_import_pairs<'a>(
    specs: &'a [String],
    role: ImportRole,
) -> Result<Vec<ImportPair<'a>>, CliError> {
    let ImportRole {
        importer,
        matched,
        stdin_remedy,
    } = role;
    let scope = BaseScope::empty();
    // The code a relative reference earns with no base in scope, read off the error kind
    // rather than respelled here.
    let relative_code = IriError::NoBase {
        reference: String::new(),
    }
    .diagnostic_code();
    let mut pairs: Vec<ImportPair<'a>> = Vec::with_capacity(specs.len());
    for spec in specs {
        let Some((iri, path)) = spec.split_once('=') else {
            return Err(CliError::Usage(format!(
                "--import {spec}: an import pair is `IRI=FILE` — the IRI the {importer} \
                 imports, then the local document that resolves it — and this one has no `=`"
            )));
        };
        if iri.is_empty() || path.is_empty() {
            return Err(CliError::Usage(format!(
                "--import {spec}: both halves of `IRI=FILE` are required — the IRI names what \
                 the {importer} imports, and the path names the document that is it"
            )));
        }
        if let Some(remedy) = stdin_remedy
            && is_stdin(path)
        {
            return Err(CliError::Usage(format!(
                "--import {spec}: an imported document's syntax is inferred from its own path \
                 extension, and `-` has none. {remedy}"
            )));
        }
        scope.resolve(iri).map_err(|error| {
            argv_iri_refusal(
                &format!("--import {spec}"),
                &error,
                relative_code,
                &format!(
                    "the ontology-IRI half `{iri}` is a relative IRI reference. It is matched against \
                     the {importer}'s {matched}, which are absolute, and it is the base the \
                     imported document parses under, so a relative reference can do neither. \
                     This is a command-line value, so no base in any document reaches it and \
                     none is guessed for it: write the half as the absolute IRI the \
                     {importer}'s {matched} name"
                ),
                &format!("the ontology-IRI half `{iri}` is not a usable IRI: "),
            )
        })?;
        if pairs.iter().any(|seen| seen.iri == iri) {
            return Err(CliError::Usage(format!(
                "--import {iri}=…: the IRI is named twice, and one IRI resolves to one \
                 document; keeping either would be a choice the command line did not make"
            )));
        }
        pairs.push(ImportPair { spec, iri, path });
    }
    Ok(pairs)
}

/// Refuse a command line in which two documents read standard input.
///
/// `readers` are `(what, path)` for every document the command may read: `what` is the
/// operator's name for it (`IN`, `--shapes`, `--import IRI=-`). A process has ONE standard
/// input, so two readers would each get part of one stream: refused, naming every reader,
/// rather than mis-read. One reader, or none, is fine.
///
/// # Errors
///
/// [`CliError::Usage`] naming the readers when more than one path is `-`.
pub(crate) fn refuse_shared_stdin(readers: &[(String, &str)]) -> Result<(), CliError> {
    let named: Vec<&str> = readers
        .iter()
        .filter(|(_, path)| is_stdin(path))
        .map(|(what, _)| what.as_str())
        .collect();
    if named.len() < 2 {
        return Ok(());
    }
    Err(CliError::Usage(format!(
        "{} each read standard input, and there is only one: a process has a single stdin \
         stream, so those documents would each get part of one byte stream. Give all but one \
         of them a path",
        list_with_and(&named)
    )))
}

/// `A and B`, `A, B and C` — grammatical at every length, because a refusal an operator has
/// to re-read is a refusal that reads as a bug.
fn list_with_and(items: &[&str]) -> String {
    match items.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => items.join(", "),
    }
}

/// The readers among `flags` that the command line supplied, for [`refuse_shared_stdin`].
pub(crate) fn supplied_readers<'a>(flags: &[(&str, Option<&'a str>)]) -> Vec<(String, &'a str)> {
    flags
        .iter()
        .filter_map(|(what, path)| path.map(|path| ((*what).to_owned(), path)))
        .collect()
}

/// The `--import` readers among `pairs`, for [`refuse_shared_stdin`].
pub(crate) fn import_readers<'a>(pairs: &[ImportPair<'a>]) -> Vec<(String, &'a str)> {
    pairs
        .iter()
        .map(|pair| (format!("--import {}=-", pair.iri), pair.path))
        .collect()
}
