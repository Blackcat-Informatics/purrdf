// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format resolution: the pipeline's input/output format decision.
//!
//! [`purrdf_rdf::SourceFormat`] is the resolved kind flowing through the pipeline —
//! one of the native RDF syntaxes ([`SourceFormat::Native`]), the native pack container
//! ([`SourceFormat::Pack`]), or the GTS transport container ([`SourceFormat::Gts`]). It
//! is `purrdf-rdf`'s shared routing identity, NOT a CLI-local type — the CLI never
//! re-decides a container's extension/id itself. [`resolve`] turns an optional explicit
//! `--from`/`--to` choice plus a path into a [`SourceFormat`]: an explicit choice always
//! wins; otherwise the path's extension is classified by `purrdf_rdf::classify_source`
//! (which recognizes the containers' own extensions — declared once in
//! `purrdf_rdf::PACK_EXTENSIONS` / `purrdf_rdf::GTS_EXTENSIONS`, never re-spelled here —
//! and routes every other extension through the native codec `classify`). A `-`
//! (stdin/stdout) path has no extension, so it REQUIRES an explicit format.
//!
//! ## Transport encoding is stripped before the format is inferred
//!
//! A gzip/zstd transport wrapper is not a format: `data.nt.gz` is an N-Triples document
//! that happens to arrive gzipped. `Path::extension` sees only `gz`, which
//! `classify_source` rightly refuses, so [`resolve`] strips a recognized transport
//! suffix (through `purrdf_rdf::strip_transport_suffix`, the workspace's one transport
//! authority) BEFORE inferring, and `source::read_bytes` decodes the bytes. The two
//! halves must agree, which is why neither re-derives the suffix table.
//!
//! On the OUTPUT side there is no such stripping, because this pipeline does not
//! COMPRESS: a `--to` target named `out.nt.gz` would be inferred as N-Triples and then
//! written as plain N-Triples under a name promising gzip. [`refuse_transport_target`]
//! refuses that by name rather than emitting a file whose name lies about its bytes.

use std::path::Path;

use purrdf_rdf::{SourceFormat, classify_source, strip_transport_suffix};

use crate::cli::CliRdfFormat;
use crate::error::CliError;

/// Resolve a `--from`/`--to` choice plus a path into a [`SourceFormat`].
///
/// Precedence: an explicit choice always wins. Otherwise a recognized gzip/zstd
/// transport suffix is stripped and the remaining path's extension is handed
/// (dot-prefixed) to `purrdf_rdf::classify_source`, which recognizes the pack and GTS
/// containers' own extensions and routes every other extension through the native codec
/// classifier. A `-` (stdin/stdout) path, or any path without an extension, has nothing
/// to infer from and is a usage error unless an explicit format was supplied.
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

    // `a.nt.gz` is an N-Triples document under a gzip wrapper: classify the payload
    // name, never the wrapper. `source::read_bytes` decodes the matching bytes.
    let payload = strip_transport_suffix(path).map_or(path, |(stem, _)| stem);

    let extension = Path::new(payload)
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

/// Resolve a `--to` choice plus an output path, refusing what this pipeline cannot write.
///
/// Identical to [`resolve`] plus the two target-side refusals: a transport-suffixed
/// output name ([`refuse_transport_target`]) and a GTS target ([`refuse_gts_target`]).
/// `role` names the argument the operator wrote (e.g. `"--to"`), so a refusal points at
/// the exact flag.
pub(crate) fn resolve_target(
    explicit: Option<CliRdfFormat>,
    path: &str,
    role: &str,
) -> Result<SourceFormat, CliError> {
    refuse_transport_target(path, role)?;
    let format = resolve(explicit, path)?;
    refuse_gts_target(format, role)?;
    Ok(format)
}

/// Refuse an output path whose name claims a gzip/zstd transport wrapper.
///
/// `convert` decodes transport on the way IN and never applies one on the way OUT, so a
/// `out.nt.gz` target would be written as plain N-Triples under a name promising gzip —
/// a file whose name lies about its bytes, which is worse than the missing capability it
/// papers over. Refuse it by name, the same shape every other inapplicable flag in this
/// CLI is refused for.
pub(crate) fn refuse_transport_target(path: &str, role: &str) -> Result<(), CliError> {
    if path == "-" {
        return Ok(());
    }
    if let Some((_, encoding)) = strip_transport_suffix(path) {
        return Err(CliError::Usage(format!(
            "{role} names `{path}`, whose suffix claims {encoding} transport: this pipeline \
             decodes gzip/zstd on input and never applies it on output, so the file would be \
             written uncompressed under a name promising {encoding}. Name an uncompressed \
             output and compress it downstream (e.g. `purrdf convert … - | {encoding} > \
             {path}`)"
        )));
    }
    Ok(())
}

/// Refuse [`SourceFormat::Gts`] as an output target.
///
/// GTS INPUT is admitted through `purrdf_rdf::import_gts_events`, the authoritative
/// importer. There is no matching one-shot authoring surface: emitting a GTS file
/// requires segment boundaries, a profile, and (optionally) a signing identity — choices
/// a `convert` command line does not carry and this pipeline must not invent, because a
/// fabricated segmentation is a different transport than the one a caller meant. The
/// refusal names the flag rather than letting clap accept `--to gts` and the sink write
/// something else.
pub(crate) fn refuse_gts_target(format: SourceFormat, role: &str) -> Result<(), CliError> {
    if format.is_gts() {
        return Err(CliError::Usage(format!(
            "{role} names the GTS container, which this pipeline READS but does not write: \
             authoring a GTS file requires segment boundaries, a profile, and a signing \
             identity that a convert command line does not carry, and inventing them would \
             emit a transport nobody asked for. Convert to a native syntax or the pack \
             container instead"
        )));
    }
    Ok(())
}

/// Refuse `--base` when it is paired with a CONTAINER format and would therefore have no
/// effect.
///
/// `--base` resolves a relative IRI on parse and relativizes one on serialize; neither
/// native container carries either role (both store fully-resolved terms and have no
/// relative-IRI syntax), so `source::run_over_input`/`load_dataset`'s and
/// `sink::write_rdf`'s container arms never read the base they are handed. Without this
/// check a `--base` supplied alongside a pack or GTS source, or a pack target, would be
/// accepted by clap and silently do nothing — the same no-op shape every other
/// inapplicable flag in this CLI is refused by name for, rather than accepted and
/// ignored.
///
/// `role` names which side of the pipeline `format` resolved for (e.g. `"the --from
/// source"`, `"the --to target"`, `"the --premise document"`), so the refusal names the
/// exact flag/argument the operator wrote.
pub(crate) fn refuse_base_with_container(
    format: SourceFormat,
    base: Option<&str>,
    role: &str,
) -> Result<(), CliError> {
    if base.is_some() && format.is_container() {
        return Err(CliError::Usage(format!(
            "--base has no effect on {role}: the native {} container stores fully-resolved \
             terms and has no relative-IRI syntax to resolve or relativize against",
            format.token()
        )));
    }
    Ok(())
}
