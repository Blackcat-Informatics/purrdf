// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Container-aware source/target routing identity (S3).
//!
//! [`SourceFormat`] is the ONE shared routing identity `purrdf-rdf` hands every
//! caller (starting with the CLI) that needs to resolve "what is this path/media-type
//! spec" beyond the text-only [`classify`]: it is a [`NativeRdfFormat`] text syntax,
//! the native pack container, or the GTS transport container. Both containers are
//! *containers* — a serialized bundle of zero or more graphs plus non-RDF sidecar
//! artifacts — deliberately NOT folded into [`NativeRdfFormat`], which documents text
//! syntaxes only.
//!
//! The pack and GTS extension/id literals live in [`PACK_EXTENSIONS`] /
//! [`GTS_EXTENSIONS`] and NOWHERE ELSE in the workspace; every consumer (the CLI's
//! `--from`/`--to` extension inference, the `pack`/`purrpck` CLI subcommand aliasing,
//! …) routes through [`classify_source`] rather than re-deciding a container literal
//! itself.
//!
//! # Pack is not GTS
//!
//! They are two different containers and neither is a mode of the other. A pack is the
//! native digest-verified snapshot container `purrdf_core::PackView` reads zero-copy; a
//! GTS file is a CBOR sequence of BLAKE3-chained segments read through
//! [`import_gts_events`](crate::import_gts_events), the authoritative importer that
//! preserves per-segment blank-node scope. Routing them to one variant would make one
//! of the two silently read by the other's reader.

use crate::RdfDiagnostic;
use crate::native_codecs::media_type::{NativeRdfFormat, classify};

/// The extensions/ids that name the native PurRDF pack container. The single authority
/// for the pack literal — no other module in the workspace may spell `"purrpck"` or
/// `"pack"` as a format literal; every consumer routes through [`classify_source`].
pub const PACK_EXTENSIONS: &[&str] = &["purrpck", "pack"];

/// The extensions/ids that name the GTS transport container, on the same single-authority
/// rule as [`PACK_EXTENSIONS`]: no other module spells `"gts"` as a format literal.
pub const GTS_EXTENSIONS: &[&str] = &["gts"];

/// A resolved source/target routing identity: a native RDF text syntax, the native
/// pack container, or the GTS transport container.
///
/// This is the container-aware routing identity that subsumes the text-only
/// [`classify`]/[`NativeRdfFormat`] pair: every caller that must decide between "one of
/// the RDF text syntaxes" and "a container" — not just "which text syntax" — resolves
/// through [`classify_source`] to this type rather than hand-rolling a second
/// container/extension decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// One of the native RDF text syntaxes (see [`NativeRdfFormat`]).
    Native(NativeRdfFormat),
    /// The native PurRDF pack container.
    Pack,
    /// The GTS transport container (a CBOR sequence of BLAKE3-chained segments), read
    /// through [`import_gts_events`](crate::import_gts_events).
    Gts,
}

impl SourceFormat {
    /// Whether this is the pack container.
    pub fn is_pack(self) -> bool {
        matches!(self, Self::Pack)
    }

    /// Whether this is the GTS container.
    pub fn is_gts(self) -> bool {
        matches!(self, Self::Gts)
    }

    /// Whether this is a CONTAINER rather than an RDF text syntax.
    ///
    /// The two containers share every property a caller reasons about at this level:
    /// they store fully-resolved terms (so there is no relative-IRI base to apply) and
    /// they carry non-triple sidecar material a text syntax has no place for.
    pub fn is_container(self) -> bool {
        matches!(self, Self::Pack | Self::Gts)
    }

    /// The canonical lowercase token naming this format, for diagnostics.
    pub fn token(self) -> &'static str {
        match self {
            Self::Native(format) => format.id(),
            Self::Pack => PACK_EXTENSIONS[0],
            Self::Gts => GTS_EXTENSIONS[0],
        }
    }

    /// The `crates/rdf-core/src/loss.rs` canonical codec name, or `None` when this
    /// format carries no loss-ledger codec identity (a native format that itself has
    /// none — TriX / HexTuples — or the pack container).
    ///
    /// GTS resolves to the `"gts"` codec the RDF↔GTS loss matrix is keyed on.
    pub fn loss_codec_name(self) -> Option<&'static str> {
        match self {
            Self::Native(format) => format.loss_codec_name(),
            Self::Pack => None,
            Self::Gts => Some("gts"),
        }
    }
}

/// Resolve a media type, format id, or (optionally dot-prefixed) file extension to a
/// [`SourceFormat`].
///
/// The input is normalized exactly like [`classify`] (lowercased, `;charset=…`
/// stripped, a leading `.` tolerated). If the normalized spelling names a container —
/// an entry of [`PACK_EXTENSIONS`] or [`GTS_EXTENSIONS`], with or without a leading dot
/// — this resolves to [`SourceFormat::Pack`] / [`SourceFormat::Gts`]. Otherwise the
/// ORIGINAL spec is delegated to [`classify`] and the result wrapped in
/// [`SourceFormat::Native`]. An unrecognized spec is the same hard error [`classify`]
/// returns (`native-codec-unsupported-format`) — there is no degraded default.
pub fn classify_source(spec: &str) -> Result<SourceFormat, RdfDiagnostic> {
    let normalized = spec
        .split(';')
        .next()
        .unwrap_or(spec)
        .trim()
        .to_ascii_lowercase();
    let bare = normalized.strip_prefix('.').unwrap_or(&normalized);
    if PACK_EXTENSIONS.contains(&bare) {
        return Ok(SourceFormat::Pack);
    }
    if GTS_EXTENSIONS.contains(&bare) {
        return Ok(SourceFormat::Gts);
    }
    classify(spec).map(SourceFormat::Native)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_source_resolves_pack_extensions_with_and_without_dot() {
        for spec in [".purrpck", "purrpck", ".pack", "pack", "PACK", ".PURRPCK"] {
            assert_eq!(
                classify_source(spec).unwrap(),
                SourceFormat::Pack,
                "spec {spec}"
            );
        }
    }

    #[test]
    fn classify_source_delegates_native_syntaxes() {
        assert_eq!(
            classify_source("ttl").unwrap(),
            SourceFormat::Native(NativeRdfFormat::Turtle)
        );
        assert_eq!(
            classify_source(".ttl").unwrap(),
            SourceFormat::Native(NativeRdfFormat::Turtle)
        );
        assert_eq!(
            classify_source("text/turtle").unwrap(),
            SourceFormat::Native(NativeRdfFormat::Turtle)
        );
    }

    #[test]
    fn classify_source_hard_fails_unknown_spec() {
        let err = classify_source("application/json").expect_err("unknown spec must fail");
        assert_eq!(err.code, "native-codec-unsupported-format");
    }

    #[test]
    fn classify_source_resolves_gts_with_and_without_dot() {
        for spec in [".gts", "gts", "GTS", ".GTS"] {
            assert_eq!(
                classify_source(spec).unwrap(),
                SourceFormat::Gts,
                "spec {spec}"
            );
        }
    }

    #[test]
    fn pack_and_gts_are_distinct_containers() {
        assert!(SourceFormat::Gts.is_gts());
        assert!(!SourceFormat::Gts.is_pack());
        assert!(!SourceFormat::Pack.is_gts());
        assert!(SourceFormat::Gts.is_container());
        assert!(SourceFormat::Pack.is_container());
        assert!(!SourceFormat::Native(NativeRdfFormat::Turtle).is_container());
        // GTS carries a loss-ledger codec identity (the RDF<->GTS matrix is keyed on
        // it); the pack container carries none, being lossless by construction.
        assert_eq!(SourceFormat::Gts.loss_codec_name(), Some("gts"));
        assert_eq!(SourceFormat::Gts.token(), "gts");
        assert_eq!(SourceFormat::Pack.token(), "purrpck");
        assert_eq!(
            SourceFormat::Native(NativeRdfFormat::Turtle).token(),
            NativeRdfFormat::Turtle.id()
        );
    }

    #[test]
    fn is_pack_and_loss_codec_name_are_consistent() {
        assert!(SourceFormat::Pack.is_pack());
        assert_eq!(SourceFormat::Pack.loss_codec_name(), None);
        assert!(!SourceFormat::Native(NativeRdfFormat::Turtle).is_pack());
        assert_eq!(
            SourceFormat::Native(NativeRdfFormat::Turtle).loss_codec_name(),
            Some("turtle")
        );
        assert_eq!(
            SourceFormat::Native(NativeRdfFormat::TriX).loss_codec_name(),
            None
        );
    }

    #[test]
    fn pack_literals_are_declared_only_in_this_module() {
        // Regression guard for the "single authority" contract: PACK_EXTENSIONS is the
        // only place the pack literal is declared in `purrdf-rdf`. This asserts its
        // shape rather than scanning the crate (a workspace-wide grep is the CLI-side
        // contract check), but it pins the exact spellings every consumer must route
        // through `classify_source` for.
        assert_eq!(PACK_EXTENSIONS, &["purrpck", "pack"]);
        assert_eq!(GTS_EXTENSIONS, &["gts"]);
    }
}
