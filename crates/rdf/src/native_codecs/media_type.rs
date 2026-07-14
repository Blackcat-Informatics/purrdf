// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Media-type → native RDF text format routing (S3).
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
    /// TriX — "Triples in XML", a quads/named-graph XML serialization
    /// (`application/trix`).
    TriX,
    /// HexTuples — a line-oriented NDJSON quads serialization
    /// (`application/x-hextuples`).
    HexTuples,
    /// JSON-LD-star — the first-party JSON-LD 1.1 + RDF-1.2-star serialization
    /// (`application/ld+json`). Star-capable (reifier form AND object-position triple
    /// terms) and dataset-capable (named graphs).
    JsonLd,
    /// YAML-LD-star — the deterministic YAML derivative of [`Self::JsonLd`]
    /// (`application/ld+yaml`).
    YamlLd,
}

/// The single source of truth for one format's routing + capability metadata.
///
/// Every per-format DATA decision (canonical media type, the alias spellings
/// [`classify`] accepts, star / dataset / span capability, and the
/// `crates/rdf-core/src/loss.rs` codec name) lives in ONE [`FORMATS`] row rather than
/// scattered `match NativeRdfFormat` arms. The behavior seam (parse / serialize) stays
/// the `RdfCodec` vtable in [`codec`](super::codec); this table is purely the data half.
///
/// The independent per-format capability flags (star / direction / datasets / spans) are
/// orthogonal boolean facts, not a state machine — collapsing them into an enum would
/// obscure the one-fact-per-column table, so the `struct_excessive_bools` lint is
/// intentionally allowed here.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct FormatDescriptor {
    /// The variant this row describes.
    pub format: NativeRdfFormat,
    /// The canonical IANA media type — the value [`NativeRdfFormat::media_type`] returns.
    pub media_type: &'static str,
    /// Every additional spelling [`classify`] accepts: alternate media types, bare
    /// format ids, and `.`-prefixed file extensions (all matched after lowercasing +
    /// charset stripping). The canonical `media_type` is matched separately, so it need
    /// not be repeated here.
    pub aliases: &'static [&'static str],
    /// Whether this format carries the RDF-1.2 statement layer (see
    /// [`NativeRdfFormat::carries_star`]).
    pub carries_star: bool,
    /// Whether this format can carry an RDF-1.2 literal base direction (see
    /// [`NativeRdfFormat::carries_direction`]). `false` only for TriX and HexTuples,
    /// which have a language slot but no direction surface.
    pub carries_direction: bool,
    /// Whether this format can carry named graphs (see
    /// [`NativeRdfFormat::supports_datasets`]).
    pub supports_datasets: bool,
    /// Whether this format's parser records per-statement source spans (see
    /// [`NativeRdfFormat::tokenizer_carries_spans`]).
    pub tokenizer_carries_spans: bool,
    /// The `crates/rdf-core/src/loss.rs` canonical codec name, or `None` for formats
    /// that carry no loss-ledger codec identity (TriX / HexTuples).
    pub loss_codec_name: Option<&'static str>,
}

/// The format registry — one row per [`NativeRdfFormat`] variant. The single place a
/// new syntax's routing + capability data is declared.
pub(crate) const FORMATS: &[FormatDescriptor] = &[
    FormatDescriptor {
        format: NativeRdfFormat::Turtle,
        media_type: "text/turtle",
        aliases: &["application/turtle", "turtle", "ttl", ".ttl"],
        carries_star: true,
        carries_direction: true,
        supports_datasets: false,
        tokenizer_carries_spans: true,
        loss_codec_name: Some("turtle"),
    },
    FormatDescriptor {
        format: NativeRdfFormat::TriG,
        media_type: "application/trig",
        aliases: &["trig", ".trig"],
        carries_star: true,
        carries_direction: true,
        supports_datasets: true,
        tokenizer_carries_spans: true,
        loss_codec_name: Some("trig"),
    },
    FormatDescriptor {
        format: NativeRdfFormat::NTriples,
        media_type: "application/n-triples",
        aliases: &["n-triples", "ntriples", "nt", ".nt"],
        carries_star: true,
        carries_direction: true,
        supports_datasets: false,
        tokenizer_carries_spans: true,
        loss_codec_name: Some("ntriples"),
    },
    FormatDescriptor {
        format: NativeRdfFormat::NQuads,
        media_type: "application/n-quads",
        aliases: &["n-quads", "nquads", "nq", ".nq"],
        carries_star: true,
        carries_direction: true,
        supports_datasets: true,
        tokenizer_carries_spans: true,
        loss_codec_name: Some("nquads"),
    },
    FormatDescriptor {
        format: NativeRdfFormat::RdfXml,
        media_type: "application/rdf+xml",
        // `rdf/xml` + `rdfxml` are absorbed from the wasm resolver so `classify` is a
        // strict superset of every spelling any first-party surface accepts.
        aliases: &[
            "rdf+xml", "rdf", "owl", "xml", "rdf/xml", "rdfxml", ".rdf", ".owl",
        ],
        carries_star: false,
        carries_direction: true,
        supports_datasets: false,
        tokenizer_carries_spans: false,
        loss_codec_name: Some("rdfxml"),
    },
    FormatDescriptor {
        format: NativeRdfFormat::TriX,
        media_type: "application/trix",
        aliases: &["trix", ".trix"],
        carries_star: false,
        carries_direction: false,
        supports_datasets: true,
        tokenizer_carries_spans: false,
        loss_codec_name: None,
    },
    FormatDescriptor {
        format: NativeRdfFormat::HexTuples,
        media_type: "application/x-hextuples",
        aliases: &["application/hex+x-ndjson", "hext", "hextuples", ".hext"],
        carries_star: false,
        carries_direction: false,
        supports_datasets: true,
        tokenizer_carries_spans: false,
        loss_codec_name: None,
    },
    FormatDescriptor {
        format: NativeRdfFormat::JsonLd,
        media_type: "application/ld+json",
        aliases: &["ld+json", "jsonld", "json-ld", ".jsonld"],
        carries_star: true,
        carries_direction: true,
        supports_datasets: true,
        tokenizer_carries_spans: false,
        loss_codec_name: Some("jsonld-star"),
    },
    FormatDescriptor {
        format: NativeRdfFormat::YamlLd,
        media_type: "application/ld+yaml",
        aliases: &["ld+yaml", "yamlld", "yaml-ld", ".yamlld"],
        carries_star: true,
        carries_direction: true,
        supports_datasets: true,
        tokenizer_carries_spans: false,
        loss_codec_name: Some("yaml-ld-star"),
    },
];

/// The [`FormatDescriptor`] row for a variant. Total over the enum — every variant has a
/// [`FORMATS`] row, so the lookup never fails (a missing row is a construction bug the
/// unit tests catch).
pub(crate) fn descriptor(format: NativeRdfFormat) -> &'static FormatDescriptor {
    FORMATS
        .iter()
        .find(|d| d.format == format)
        .expect("every NativeRdfFormat variant has a FORMATS row")
}

impl NativeRdfFormat {
    /// The canonical IANA media type for this format. The inverse of the canonical
    /// rows in [`classify`].
    pub fn media_type(self) -> &'static str {
        descriptor(self).media_type
    }

    /// Whether this format can carry named graphs (TriG / N-Quads / TriX / HexTuples /
    /// JSON-LD / YAML-LD). Turtle, N-Triples, and RDF/XML are single-graph syntaxes, so a
    /// `SerializeGraph::Dataset` request against them falls back to the default graph
    /// (see `serialize.rs`).
    pub fn supports_datasets(self) -> bool {
        descriptor(self).supports_datasets
    }

    /// Whether this format carries the RDF-1.2 statement layer (quoted-triple reifiers +
    /// annotations) under the transcode loss contract. Star-capable formats emit it;
    /// star-incapable formats drop it as declared loss. Kept aligned with the loss ledger
    /// (`crates/rdf-core/src/loss.rs`) — see the drift-guard test in `native_codecs`.
    pub fn carries_star(self) -> bool {
        descriptor(self).carries_star
    }

    /// Whether this format can carry an RDF-1.2 literal base direction. Every format
    /// except TriX and HexTuples emits the direction losslessly; those two have a
    /// language slot but no direction surface, so they drop it as declared loss
    /// (recorded on the loss ledger — never a silent drop; CONSTITUTION P7).
    pub fn carries_direction(self) -> bool {
        descriptor(self).carries_direction
    }

    /// Whether this format's parser can record per-statement source spans. Only the
    /// line/Turtle-family text tokenizer does; the others return an empty span table
    /// (physical-location fallback by design).
    pub fn tokenizer_carries_spans(self) -> bool {
        descriptor(self).tokenizer_carries_spans
    }

    /// The `crates/rdf-core/src/loss.rs` canonical codec name, or `None` when this format
    /// carries no loss-ledger codec identity (TriX / HexTuples).
    pub fn loss_codec_name(self) -> Option<&'static str> {
        descriptor(self).loss_codec_name
    }
}

/// Resolve a media type or local format id to a [`NativeRdfFormat`].
///
/// The input is lowercased and any `;charset=…` parameter is stripped before
/// matching, so `text/turtle; charset=utf-8` and `Turtle` both resolve to
/// [`NativeRdfFormat::Turtle`]. Matching scans the internal format table for a row whose
/// canonical `media_type` OR any `alias` equals the normalized input (aliases include `.`-prefixed
/// file extensions, so `.jsonld` resolves too). An unrecognized media type is a HARD
/// error (`native-codec-unsupported-format`) — there is no degraded default codec.
pub fn classify(media_type: &str) -> Result<NativeRdfFormat, RdfDiagnostic> {
    let normalized = media_type
        .split(';')
        .next()
        .unwrap_or(media_type)
        .trim()
        .to_ascii_lowercase();
    FORMATS
        .iter()
        .find(|d| d.media_type == normalized || d.aliases.contains(&normalized.as_str()))
        .map(|d| d.format)
        .ok_or_else(|| {
            RdfDiagnostic::error(
                "native-codec-unsupported-format",
                format!("unsupported RDF media type or format id `{normalized}`"),
            )
        })
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
        assert_eq!(classify("application/trix").unwrap(), NativeRdfFormat::TriX);
        assert_eq!(
            classify("application/x-hextuples").unwrap(),
            NativeRdfFormat::HexTuples
        );
    }

    #[test]
    fn classify_accepts_trix_and_hextuples_ids() {
        assert_eq!(classify("trix").unwrap(), NativeRdfFormat::TriX);
        assert_eq!(classify("hext").unwrap(), NativeRdfFormat::HexTuples);
        assert_eq!(classify("hextuples").unwrap(), NativeRdfFormat::HexTuples);
    }

    #[test]
    fn trix_and_hextuples_support_datasets() {
        assert!(NativeRdfFormat::TriX.supports_datasets());
        assert!(NativeRdfFormat::HexTuples.supports_datasets());
    }

    #[test]
    fn only_trix_and_hextuples_drop_direction() {
        // TriX / HexTuples have a language slot but no base-direction surface.
        assert!(!NativeRdfFormat::TriX.carries_direction());
        assert!(!NativeRdfFormat::HexTuples.carries_direction());
        // Every other format carries the RDF-1.2 base direction losslessly.
        for format in [
            NativeRdfFormat::Turtle,
            NativeRdfFormat::TriG,
            NativeRdfFormat::NTriples,
            NativeRdfFormat::NQuads,
            NativeRdfFormat::RdfXml,
            NativeRdfFormat::JsonLd,
            NativeRdfFormat::YamlLd,
        ] {
            assert!(
                format.carries_direction(),
                "{format:?} must carry base direction"
            );
        }
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
        // Every variant in the registry — a table-driven loop so a new FORMATS row is
        // covered automatically.
        for descriptor in FORMATS {
            assert_eq!(
                classify(descriptor.format.media_type()).unwrap(),
                descriptor.format
            );
        }
    }

    #[test]
    fn classify_resolves_jsonld_and_yamlld() {
        for id in [
            "application/ld+json",
            "ld+json",
            "jsonld",
            "json-ld",
            ".jsonld",
        ] {
            assert_eq!(classify(id).unwrap(), NativeRdfFormat::JsonLd, "id {id}");
        }
        for id in [
            "application/ld+yaml",
            "ld+yaml",
            "yamlld",
            "yaml-ld",
            ".yamlld",
        ] {
            assert_eq!(classify(id).unwrap(), NativeRdfFormat::YamlLd, "id {id}");
        }
    }

    #[test]
    fn jsonld_and_yamlld_support_datasets() {
        assert!(NativeRdfFormat::JsonLd.supports_datasets());
        assert!(NativeRdfFormat::YamlLd.supports_datasets());
    }

    #[test]
    fn descriptor_alias_regression_every_pre_existing_spelling_resolves() {
        // The FormatDescriptor table refactor must not silently narrow `classify`: every
        // spelling the pre-table `classify` accepted still resolves to its variant.
        let cases: &[(&str, NativeRdfFormat)] = &[
            ("text/turtle", NativeRdfFormat::Turtle),
            ("application/turtle", NativeRdfFormat::Turtle),
            ("turtle", NativeRdfFormat::Turtle),
            ("ttl", NativeRdfFormat::Turtle),
            ("application/trig", NativeRdfFormat::TriG),
            ("trig", NativeRdfFormat::TriG),
            ("application/n-triples", NativeRdfFormat::NTriples),
            ("n-triples", NativeRdfFormat::NTriples),
            ("ntriples", NativeRdfFormat::NTriples),
            ("nt", NativeRdfFormat::NTriples),
            ("application/n-quads", NativeRdfFormat::NQuads),
            ("n-quads", NativeRdfFormat::NQuads),
            ("nquads", NativeRdfFormat::NQuads),
            ("nq", NativeRdfFormat::NQuads),
            ("application/rdf+xml", NativeRdfFormat::RdfXml),
            ("rdf+xml", NativeRdfFormat::RdfXml),
            ("rdf", NativeRdfFormat::RdfXml),
            ("owl", NativeRdfFormat::RdfXml),
            ("xml", NativeRdfFormat::RdfXml),
            // Absorbed from the wasm resolver so delegation drops nothing.
            ("rdf/xml", NativeRdfFormat::RdfXml),
            ("rdfxml", NativeRdfFormat::RdfXml),
            ("application/trix", NativeRdfFormat::TriX),
            ("trix", NativeRdfFormat::TriX),
            ("application/x-hextuples", NativeRdfFormat::HexTuples),
            ("application/hex+x-ndjson", NativeRdfFormat::HexTuples),
            ("hext", NativeRdfFormat::HexTuples),
            ("hextuples", NativeRdfFormat::HexTuples),
        ];
        for &(id, expected) in cases {
            assert_eq!(classify(id).unwrap(), expected, "spelling {id}");
        }
    }
}
