// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format resolution: the pipeline's input/output format decision.
//!
//! [`purrdf_rdf::SourceFormat`] is the resolved kind flowing through the pipeline —
//! either one of the native RDF syntaxes ([`SourceFormat::Native`]) or the native pack
//! container ([`SourceFormat::Pack`]). It is `purrdf-rdf`'s shared routing identity, NOT
//! a CLI-local type — the CLI never re-decides the pack extension/id itself. [`resolve`]
//! turns an optional explicit `--from`/`--to` choice plus a path into a
//! [`SourceFormat`]: an explicit choice always wins; otherwise the path's extension is
//! classified by `purrdf_rdf::classify_source` (which recognizes the pack container's
//! own extensions — declared once in `purrdf_rdf::PACK_EXTENSIONS`, never re-spelled
//! here — and routes every other extension through the native codec `classify`). A
//! `-` (stdin/stdout) path has no extension, so it REQUIRES an explicit format.

use std::path::Path;

use purrdf_rdf::{SourceFormat, classify_source};

use crate::cli::CliRdfFormat;
use crate::error::CliError;

/// Resolve a `--from`/`--to` choice plus a path into a [`SourceFormat`].
///
/// Precedence: an explicit choice always wins. Otherwise the path's extension is
/// handed (dot-prefixed) to `purrdf_rdf::classify_source`, which recognizes the pack
/// container's own extensions as [`SourceFormat::Pack`] and routes every other
/// extension through the native codec classifier.
/// A `-` (stdin/stdout) path, or any path without an extension, has nothing to
/// infer from and is a usage error unless an explicit format was supplied.
pub(crate) fn resolve(
    explicit: Option<CliRdfFormat>,
    path: &str,
) -> Result<SourceFormat, CliError> {
    if let Some(choice) = explicit {
        return Ok(choice.to_source_format());
    }

    if path == "-" {
        return Err(CliError::Usage(
            "reading from / writing to stdin/stdout (`-`) requires an explicit --from/--to format"
                .to_string(),
        ));
    }

    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            CliError::Usage(format!(
                "cannot infer a format for `{path}`: it has no file extension; \
                 pass an explicit --from/--to format"
            ))
        })?
        .to_ascii_lowercase();

    classify_source(&format!(".{extension}")).map_err(|diagnostic| {
        CliError::Usage(format!(
            "cannot infer a format for `{path}`: {diagnostic}; \
             pass an explicit --from/--to format"
        ))
    })
}

/// Refuse `--base` when it is paired with [`SourceFormat::Pack`] and would therefore have no
/// effect.
///
/// `--base` resolves a relative IRI on parse and relativizes one on serialize; the native
/// pack container carries neither role (it stores fully-resolved terms and has no
/// relative-IRI syntax), so `source::run_over_input`/`load_dataset`'s and `sink::write_rdf`'s
/// pack arms never read the base they are handed. Without this check a `--base` supplied
/// alongside a pack source or target would be accepted by clap and silently do nothing — the
/// same no-op shape every other inapplicable flag in this CLI is refused by name for, rather
/// than accepted and ignored.
///
/// `role` names which side of the pipeline `format` resolved for (e.g. `"--from"`,
/// `"--to"`, `"--premise"`), so the refusal names the exact flag/argument the operator wrote.
pub(crate) fn refuse_base_with_pack(
    format: SourceFormat,
    base: Option<&str>,
    role: &str,
) -> Result<(), CliError> {
    if base.is_some() && format.is_pack() {
        return Err(CliError::Usage(format!(
            "--base has no effect on {role}: the native pack container stores fully-resolved \
             terms and has no relative-IRI syntax to resolve or relativize against"
        )));
    }
    Ok(())
}
