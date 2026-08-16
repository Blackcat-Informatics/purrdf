// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format resolution: the pipeline's input/output format decision.
//!
//! [`CliFormat`] is the resolved kind flowing through the pipeline — either one of
//! the native RDF syntaxes ([`CliFormat::Rdf`]) or the native pack container
//! ([`CliFormat::Pack`]). [`resolve`] turns an optional explicit `--from`/`--to`
//! choice plus a path into a [`CliFormat`]: an explicit choice always wins;
//! otherwise the path's extension is classified (with `purrpck`/`pack` recognized
//! as the pack container, and every other extension routed through the native
//! codec [`classify`]). A `-` (stdin/stdout) path has no
//! extension, so it REQUIRES an explicit format.

use std::path::Path;

use purrdf_rdf::{NativeRdfFormat, classify};

use crate::cli::CliRdfFormat;
use crate::error::CliError;

/// A resolved input/output format: a native RDF syntax or the pack container.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CliFormat {
    /// One of the nine native RDF text/graph syntaxes.
    Rdf(NativeRdfFormat),
    /// The native PurRDF pack container.
    Pack,
}

impl CliFormat {
    /// The loss-ledger codec name for this format, or `None` when it carries no
    /// codec identity (TriX / HexTuples, or a pack).
    pub(crate) fn loss_codec_name(self) -> Option<&'static str> {
        match self {
            Self::Rdf(format) => format.loss_codec_name(),
            Self::Pack => None,
        }
    }
}

/// The two extensions that name the native pack container.
const PACK_EXTENSIONS: [&str; 2] = ["purrpck", "pack"];

/// Resolve a `--from`/`--to` choice plus a path into a [`CliFormat`].
///
/// Precedence: an explicit choice always wins. Otherwise the path's extension is
/// classified — `purrpck`/`pack` → [`CliFormat::Pack`], any other extension is
/// handed (dot-prefixed) to the native codec [`classify`].
/// A `-` (stdin/stdout) path, or any path without an extension, has nothing to
/// infer from and is a usage error unless an explicit format was supplied.
pub(crate) fn resolve(explicit: Option<CliRdfFormat>, path: &str) -> Result<CliFormat, CliError> {
    if let Some(choice) = explicit {
        return Ok(choice.to_cli_format());
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

    if PACK_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(CliFormat::Pack);
    }

    let format = classify(&format!(".{extension}")).map_err(|diagnostic| {
        CliError::Usage(format!(
            "cannot infer a format for `{path}`: {diagnostic}; \
             pass an explicit --from/--to format"
        ))
    })?;
    Ok(CliFormat::Rdf(format))
}

/// Refuse `--base` when it is paired with [`CliFormat::Pack`] and would therefore have no
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
    format: CliFormat,
    base: Option<&str>,
    role: &str,
) -> Result<(), CliError> {
    if base.is_some() && matches!(format, CliFormat::Pack) {
        return Err(CliError::Usage(format!(
            "--base has no effect on {role}: the native pack container stores fully-resolved \
             terms and has no relative-IRI syntax to resolve or relativize against"
        )));
    }
    Ok(())
}
