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
//!
//! ## `--base` has two legs, and is refused when neither can take it
//!
//! A base is spent in exactly two places, and each is a column of the format registry: on
//! PARSE a relative IRI reference resolves against it (`admits_relative_iri`), and on
//! SERIALIZE it is written as the output document's base directive and relativized against
//! (`emits_base`). Both are live — Turtle, TriG, RDF/XML, JSON-LD and YAML-LD carry a base
//! either way; N-Triples, N-Quads, TriX, HexTuples and both native containers carry it
//! neither way.
//!
//! [`refuse_unconsumable_base`] is the one decision that reads those two columns, over the
//! legs a subcommand actually has. A base ANY leg can spend is honoured (so `--base X --to
//! ntriples` still resolves the input); a base NO leg can spend is a usage error naming
//! each leg and why it cannot take the value, rather than a parameter accepted and never
//! read.

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

/// Which LEG of the pipeline a format sits on, for the `--base` consumption test.
///
/// The two are independent registry columns, not one fact spelled twice — a syntax can
/// read a base without being able to write one — so the leg has to be named rather than
/// inferred from the format alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaseLeg {
    /// INGRESS: the base a relative IRI reference in the document resolves against, read
    /// off `NativeRdfFormat::admits_relative_iri`.
    Parse,
    /// EGRESS: the base the output document declares and relativizes against, read off
    /// `NativeRdfFormat::emits_base`.
    Serialize,
}

/// One place a `--base` COULD be consumed: a resolved format, on a leg, named by the
/// argument the operator wrote.
///
/// `role` is that argument (e.g. `"the --from source"`, `"the --to target"`, `"the
/// --premise document"`), so a refusal points at the exact flag rather than at "the
/// pipeline".
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaseUse<'a> {
    /// The format resolved for this leg.
    format: SourceFormat,
    /// Which direction the format is used in here.
    leg: BaseLeg,
    /// The argument the operator wrote for it.
    role: &'a str,
}

impl<'a> BaseUse<'a> {
    /// A leg that PARSES `format`.
    pub(crate) const fn parse(format: SourceFormat, role: &'a str) -> Self {
        Self {
            format,
            leg: BaseLeg::Parse,
            role,
        }
    }

    /// A leg that SERIALIZES `format`.
    pub(crate) const fn serialize(format: SourceFormat, role: &'a str) -> Self {
        Self {
            format,
            leg: BaseLeg::Serialize,
            role,
        }
    }

    /// Whether this leg can consume a base at all.
    ///
    /// Decided ENTIRELY by the two format-registry columns, so a newly registered syntax
    /// is classified by its own row rather than by a list kept here that someone would
    /// have to remember to extend.
    fn consumes_base(self) -> bool {
        match self.format {
            SourceFormat::Native(native) => match self.leg {
                BaseLeg::Parse => native.admits_relative_iri(),
                BaseLeg::Serialize => native.emits_base(),
            },
            // Both native containers store fully-resolved terms and have no relative-IRI
            // syntax, in either direction.
            SourceFormat::Pack | SourceFormat::Gts => false,
        }
    }

    /// Why this leg cannot consume one, in the operator's vocabulary.
    fn why_not(self) -> String {
        let token = self.format.token();
        if self.format.is_container() {
            return format!(
                "the native {token} container stores fully-resolved terms and has no \
                 relative-IRI syntax to resolve or relativize against"
            );
        }
        match self.leg {
            BaseLeg::Parse => format!(
                "{token}'s grammar admits no relative IRI reference, so nothing in the \
                 document resolves against a base"
            ),
            BaseLeg::Serialize => format!(
                "{token} can express no base directive, so nothing is written under one or \
                 relativized against it"
            ),
        }
    }
}

/// Refuse `--base` when NEITHER leg of this run can consume it.
///
/// `--base` resolves a relative IRI on PARSE and is written as the document base — and
/// relativized against — on SERIALIZE. A run whose parse leg admits no relative IRI AND
/// whose serialize leg can express no base has nowhere to spend it, so accepting one would
/// be an accepted-and-ignored parameter on the user-facing surface: `convert --from
/// ntriples --to ntriples --base http://example.org/` exited 0, changed nothing, and said
/// nothing. That is the shape this pipeline refuses by name everywhere else.
///
/// The decision is driven entirely off the format registry's `admits_relative_iri` /
/// `emits_base` columns (see [`BaseUse::consumes_base`]), so a format cannot slip through
/// by being added later. `legs` is every place this subcommand could spend the base — a
/// base consumed by ANY ONE of them is honoured, which is what keeps `--base X --to
/// ntriples` working from a relative-admitting source.
///
/// A subcommand whose base ALSO reaches a non-RDF consumer (a SPARQL query or update text,
/// a ShEx schema, a shape map) has a leg this list cannot name and does not call this: for
/// those the base is never inert.
pub(crate) fn refuse_unconsumable_base(
    base: Option<&str>,
    legs: &[BaseUse<'_>],
) -> Result<(), CliError> {
    if base.is_none() || legs.iter().copied().any(BaseUse::consumes_base) {
        return Ok(());
    }
    let reasons = legs
        .iter()
        .map(|leg| format!("on {}, {}", leg.role, leg.why_not()))
        .collect::<Vec<_>>()
        .join("; and ");
    Err(CliError::Usage(format!(
        "--base has no effect on this run: {reasons}. Drop --base, or name a syntax that \
         carries one ({})",
        base_carrying_syntaxes()
    )))
}

/// The `--from`/`--to` tokens whose syntax can carry a base on either leg, read off the
/// format registry rather than listed here.
fn base_carrying_syntaxes() -> String {
    purrdf_rdf::NativeRdfFormat::all()
        .filter(|format| format.admits_relative_iri() || format.emits_base())
        .map(purrdf_rdf::NativeRdfFormat::id)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Refuse `--base` when it is paired with a CONTAINER format.
///
/// The container case of [`refuse_unconsumable_base`], sharing its predicate and its
/// message, for the one lane whose base has a consumer no [`BaseUse`] can name: `shex`
/// resolves relative IRIs in the SCHEMA and the SHAPE MAP as well as in the data graph, so
/// the data source's syntax alone never makes the base inert. A container source still
/// does — it stores fully-resolved terms — and that is exactly what this refuses.
pub(crate) fn refuse_base_with_container(
    format: SourceFormat,
    base: Option<&str>,
    role: &str,
) -> Result<(), CliError> {
    if format.is_container() {
        return refuse_unconsumable_base(base, &[BaseUse::parse(format, role)]);
    }
    Ok(())
}
