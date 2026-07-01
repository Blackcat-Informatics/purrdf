// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Media-type → native RDF text format routing (#909 / EPIC #906 S3).
//!
//! [`NativeRdfFormat`] is the single chokepoint the eventual `oxigraph::io::RdfFormat`
//! removal (S14) retargets: every codec consumer names a format by media type at the
//! contract boundary and [`classify`] resolves it once. Unknown media types HARD-fail
//! (`native-codec-unsupported-format`) rather than degrading — no optional fallback
//! codec (`.goals` no-optionality).

use crate::RdfDiagnostic;

/// The RDF text serializations the native codec backend parses and serializes via the
/// `purrdf-gts` codecs. This is the codec-selector enum that replaces
/// `oxigraph::io::RdfFormat`'s *use as a router* across the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRdfFormat {
    /// Turtle (`text/turtle`).
    Turtle,
    /// TriG (`application/trig`).
    TriG,
    /// N-Triples (`application/n-triples`).
    NTriples,
    /// N-Quads (`application/n-quads`).
    NQuads,
    /// RDF/XML (`application/rdf+xml`).
    RdfXml,
}

impl NativeRdfFormat {
    /// The canonical IANA media type for this format. The inverse of the canonical
    /// rows in [`classify`].
    pub fn media_type(self) -> &'static str {
        match self {
            Self::Turtle => "text/turtle",
            Self::TriG => "application/trig",
            Self::NTriples => "application/n-triples",
            Self::NQuads => "application/n-quads",
            Self::RdfXml => "application/rdf+xml",
        }
    }

    /// Whether this format can carry named graphs (TriG / N-Quads). Turtle, N-Triples,
    /// and RDF/XML are single-graph syntaxes, so a `SerializeGraph::Dataset` request
    /// against them falls back to the default graph (see `serialize.rs`).
    pub fn supports_datasets(self) -> bool {
        matches!(self, Self::TriG | Self::NQuads)
    }
}

/// Resolve a media type or local format id to a [`NativeRdfFormat`].
///
/// The input is lowercased and any `;charset=…` parameter is stripped before
/// matching, so `text/turtle; charset=utf-8` and `Turtle` both resolve to
/// [`NativeRdfFormat::Turtle`]. An unrecognized media type is a HARD error
/// (`native-codec-unsupported-format`) — there is no degraded default codec.
pub fn classify(media_type: &str) -> Result<NativeRdfFormat, RdfDiagnostic> {
    let normalized = media_type
        .split(';')
        .next()
        .unwrap_or(media_type)
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "text/turtle" | "application/turtle" | "turtle" | "ttl" => Ok(NativeRdfFormat::Turtle),
        "application/trig" | "trig" => Ok(NativeRdfFormat::TriG),
        "application/n-triples" | "n-triples" | "ntriples" | "nt" => Ok(NativeRdfFormat::NTriples),
        "application/n-quads" | "n-quads" | "nquads" | "nq" => Ok(NativeRdfFormat::NQuads),
        "application/rdf+xml" | "rdf+xml" | "rdf" | "owl" | "xml" => Ok(NativeRdfFormat::RdfXml),
        other => Err(RdfDiagnostic::error(
            "native-codec-unsupported-format",
            format!("unsupported RDF media type or format id `{other}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_resolves_canonical_media_types() {
        assert_eq!(classify("text/turtle").unwrap(), NativeRdfFormat::Turtle);
        assert_eq!(classify("application/trig").unwrap(), NativeRdfFormat::TriG);
        assert_eq!(
            classify("application/n-triples").unwrap(),
            NativeRdfFormat::NTriples
        );
        assert_eq!(
            classify("application/n-quads").unwrap(),
            NativeRdfFormat::NQuads
        );
        assert_eq!(
            classify("application/rdf+xml").unwrap(),
            NativeRdfFormat::RdfXml
        );
    }

    #[test]
    fn classify_strips_charset_and_lowercases() {
        assert_eq!(
            classify("Text/Turtle; charset=utf-8").unwrap(),
            NativeRdfFormat::Turtle
        );
        assert_eq!(classify("  NQ  ").unwrap(), NativeRdfFormat::NQuads);
    }

    #[test]
    fn classify_accepts_short_format_ids() {
        assert_eq!(classify("ttl").unwrap(), NativeRdfFormat::Turtle);
        assert_eq!(classify("nt").unwrap(), NativeRdfFormat::NTriples);
        assert_eq!(classify("rdf").unwrap(), NativeRdfFormat::RdfXml);
        assert_eq!(classify("owl").unwrap(), NativeRdfFormat::RdfXml);
    }

    #[test]
    fn classify_hard_fails_unknown_format() {
        let err = classify("application/json").expect_err("unknown format must fail");
        assert_eq!(err.code, "native-codec-unsupported-format");
    }

    #[test]
    fn media_type_round_trips_through_classify() {
        for format in [
            NativeRdfFormat::Turtle,
            NativeRdfFormat::TriG,
            NativeRdfFormat::NTriples,
            NativeRdfFormat::NQuads,
            NativeRdfFormat::RdfXml,
        ] {
            assert_eq!(classify(format.media_type()).unwrap(), format);
        }
    }
}
