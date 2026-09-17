// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Language tags must survive **crossing** codecs, not merely round-tripping
//! within one.
//!
//! # Why this file exists
//!
//! Every serializer in this crate writes a dataset's language tag out verbatim,
//! so a tag one codec accepts is a tag another codec is handed. If two codecs
//! draw their acceptance language differently, the workspace writes files it
//! cannot read back — and nothing catches it, because a per-codec round-trip
//! test never hands one codec's output to another.
//!
//! That is not hypothetical. Held to RFC 5646 §2.1 on the line path and to the
//! bare `LANGTAG` terminal on the document path, this crate would parse
//! `"a"@fr-be-fbcl` from Turtle, serialize it to N-Quads with an exit status of
//! zero, and then refuse its own output. So the assertion here is deliberately
//! shaped as a crossing: **parse from one syntax, serialize to a different one,
//! parse THAT back**, for every pair that matters.
//!
//! # The two obligations that must hold at once
//!
//! The tags below are not a wish list; each names a corpus that already carries
//! it, and the accept/refuse split is fixed by those corpora:
//!
//! * `en-fr-jura` and `fr-be-fbcl` appear in the vendored shexTest vectors,
//!   read as `text/turtle`. Neither is well-formed RFC 5646 — `jura` and `fbcl`
//!   are four alphabetic characters, and `variant` admits four only when the
//!   first is a DIGIT — so only the `LANGTAG` terminal takes them.
//! * `x-purrdf-afrikaans` appears in this workspace's own `.nt` fixtures and
//!   `x-gmeow-norwegiannynorsk` in a downstream project's literals; both run
//!   past the §2.1 eight-character private-use cap.
//! * `cantbethislong` is the W3C N-Triples NEGATIVE syntax test
//!   `ntriples-langdir-bad-4`, and must stay refused.
//!
//! An over-refusal here is the mirror of the silent-drop bug: it looks like
//! correct strictness, every other test stays green, and the damage only shows
//! up in a user's data. So every refusal below is paired with the neighbour
//! that must still be accepted.

use purrdf_rdf::{NativeRdfFormat, SerializeGraph, parse_dataset, serialize_dataset};

/// The subject and predicate every fixture in this file uses.
const SUBJECT: &str = "http://example.org/s";
const PREDICATE: &str = "http://example.org/p";

/// The literal's lexical form, kept to one character so that the `@tag` token
/// is the only thing under test.
const VALUE: &str = "a";

/// Tags that MUST survive every codec crossing, each with the corpus that
/// carries it.
const MUST_SURVIVE: &[(&str, &str)] = &[
    ("en-fr-jura", "shexTest vector, `text/turtle`"),
    ("fr-be-fbcl", "shexTest vector, `text/turtle`"),
    ("x-purrdf-afrikaans", "workspace `.nt` fixture"),
    ("x-gmeow-norwegiannynorsk", "downstream project literals"),
    ("en-US", "ordinary RFC 5646"),
    ("zh-Hans-CN", "ordinary RFC 5646"),
    ("de-CH-x-phonebk", "ordinary RFC 5646 with private use"),
];

/// A one-quad document carrying `tag` on its object literal.
///
/// The same bytes are a valid Turtle document, a valid N-Triples line and a
/// valid N-Quads line — absolute IRIs, no prefixes, one statement — which is
/// what lets the tests below feed one fixture to every text codec and compare
/// the answers. A fixture that differed per syntax could not prove they agree.
fn document_source(tag: &str) -> String {
    format!("<{SUBJECT}> <{PREDICATE}> \"{VALUE}\"@{tag} .\n")
}

/// The exact token a text serializer must emit for `tag`: the lexical form
/// quoted, then `@` and the tag, lowercased.
///
/// The lowercasing is the IR's term-identity rule (C0.1) and applies at intern
/// time, before any codec is involved, so `@EN` and `@en` are one term. It is a
/// **case** normalization and nothing else: no subtag is added, dropped,
/// reordered or re-spelled, which is precisely what these tests are watching
/// for. Asserting the lowercased form rather than the input's own case is
/// therefore the tighter assertion, not the looser one — an equality on the
/// exact bytes a conformant reader must see.
fn expected_token(tag: &str) -> String {
    format!("\"{VALUE}\"@{}", tag.to_lowercase())
}

/// Serialize `dataset` to `format`, parse the result back with the SAME format,
/// and return the bytes that were written.
///
/// Returning the bytes is what lets the caller assert on what was *written* as
/// well as on what came back, which is the difference between "the tag
/// survived" and "the tag survived and was written the way it was read".
fn cross(
    dataset: &std::sync::Arc<purrdf_rdf::RdfDataset>,
    format: NativeRdfFormat,
) -> (Vec<u8>, std::sync::Arc<purrdf_rdf::RdfDataset>) {
    let bytes = serialize_dataset(dataset, format.media_type(), SerializeGraph::Dataset)
        .unwrap_or_else(|error| panic!("{format:?}: serialize must succeed, got {error}"));
    let back = parse_dataset(&bytes, format.media_type(), None).unwrap_or_else(|error| {
        panic!(
            "{format:?}: this crate must be able to re-read what it just wrote, got {error}\n\
             --- written ---\n{}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (bytes, back)
}

/// Re-serialize `dataset` as N-Triples and assert the tag rode through verbatim.
///
/// N-Triples is used as the common observation point on purpose: it is the one
/// text form with no prefixes, no abbreviation and one statement per line, so
/// the `@tag` token appears literally or it does not appear at all.
fn assert_tag_present(dataset: &std::sync::Arc<purrdf_rdf::RdfDataset>, tag: &str, leg: &str) {
    let bytes = serialize_dataset(
        dataset,
        NativeRdfFormat::NTriples.media_type(),
        SerializeGraph::Dataset,
    )
    .expect("N-Triples serialize must succeed");
    let text = String::from_utf8(bytes).expect("serialized RDF must be UTF-8");
    let token = expected_token(tag);
    assert!(
        text.contains(&token),
        "{leg}: the language tag must survive verbatim as {token:?}, got:\n{text}"
    );
}

/// The crossing itself: Turtle in, every other codec out and back again.
///
/// The legs are the four this crate's codecs actually take between them —
/// N-Triples, N-Quads, RDF/XML and TriG — plus Turtle, so that the syntax the
/// tag was read from is also proven to re-read it.
#[test]
fn language_tags_survive_crossing_every_codec() {
    for (tag, corpus) in MUST_SURVIVE {
        let source = document_source(tag);
        let parsed = parse_dataset(
            source.as_bytes(),
            NativeRdfFormat::Turtle.media_type(),
            None,
        )
        .unwrap_or_else(|error| panic!("Turtle must accept {tag:?} ({corpus}), got {error}"));
        assert_tag_present(&parsed, tag, "Turtle parse");

        for format in [
            NativeRdfFormat::NTriples,
            NativeRdfFormat::NQuads,
            NativeRdfFormat::RdfXml,
            NativeRdfFormat::TriG,
            NativeRdfFormat::Turtle,
        ] {
            let (written, back) = cross(&parsed, format);
            if matches!(
                format,
                NativeRdfFormat::NTriples | NativeRdfFormat::NQuads | NativeRdfFormat::TriG
            ) {
                let token = expected_token(tag);
                let text = String::from_utf8(written).expect("serialized RDF must be UTF-8");
                assert!(
                    text.contains(&token),
                    "{format:?}: must WRITE {token:?} for {tag:?} ({corpus}), got:\n{text}"
                );
            }
            assert_tag_present(&back, tag, &format!("Turtle -> {format:?} -> back"));
        }
    }
}

/// The other direction, because the bug was found writing N-Quads and reading
/// Turtle: a tag read from the LINE path must be readable by the DOCUMENT path
/// and by RDF/XML, and back again.
#[test]
fn language_tags_survive_crossing_from_the_line_codecs() {
    for (tag, corpus) in MUST_SURVIVE {
        let source = document_source(tag);
        for entry in [NativeRdfFormat::NTriples, NativeRdfFormat::NQuads] {
            let parsed = parse_dataset(source.as_bytes(), entry.media_type(), None).unwrap_or_else(
                |error| panic!("{entry:?} must accept {tag:?} ({corpus}), got {error}"),
            );
            for format in [
                NativeRdfFormat::Turtle,
                NativeRdfFormat::TriG,
                NativeRdfFormat::RdfXml,
            ] {
                let (_, back) = cross(&parsed, format);
                assert_tag_present(&back, tag, &format!("{entry:?} -> {format:?} -> back"));
            }
        }
    }
}

/// The negative arm. A shared acceptance language is only worth having if it is
/// still a grammar, so this pins what every codec must REFUSE — and pins it on
/// every codec, because the defect this file exists for was two codecs
/// disagreeing about exactly this.
///
/// `cantbethislong` is the W3C negative syntax test `ntriples-langdir-bad-4`, a
/// bare fourteen-character primary subtag that the §2.1 length ceiling refuses.
/// The rest are not `LANGTAG` terminals at all.
#[test]
fn the_refused_tags_are_refused_on_every_text_codec() {
    // (refused tag, the neighbour one edit away that must still be accepted)
    let pairs: &[(&str, &str)] = &[
        // The length bound. `cantbethis` is not the neighbour — it is ten
        // characters and also over — so the pairing steps the bound itself.
        ("cantbethislong", "abcdefgh"),
        ("abcdefghi", "abcdefgh"),
        // The terminal's own shape refusals.
        ("1", "en"),
        ("-", "en"),
        ("en-", "en"),
        ("9-9", "en-9"),
        ("123-456", "abc-456"),
    ];

    for format in [
        NativeRdfFormat::NTriples,
        NativeRdfFormat::NQuads,
        NativeRdfFormat::Turtle,
        NativeRdfFormat::TriG,
    ] {
        for (refused, neighbour) in pairs {
            let bad = document_source(refused);
            assert!(
                parse_dataset(bad.as_bytes(), format.media_type(), None).is_err(),
                "{format:?}: {refused:?} is not an acceptable language tag"
            );

            let good = document_source(neighbour);
            let parsed =
                parse_dataset(good.as_bytes(), format.media_type(), None).unwrap_or_else(|error| {
                    panic!("{format:?}: {neighbour:?} must stay accepted, got {error}")
                });
            assert_tag_present(&parsed, neighbour, &format!("{format:?} neighbour"));
        }
    }
}

/// The private-use relaxation is a LENGTH relaxation inside `x-`, not a licence
/// to run past the ceiling anywhere else — the one refusal above and the one
/// acceptance here are a single character apart in position, not in length.
#[test]
fn the_length_ceiling_stops_at_the_private_use_marker() {
    for format in [NativeRdfFormat::NTriples, NativeRdfFormat::Turtle] {
        let inside = document_source("en-x-cantbethislong");
        let parsed = parse_dataset(inside.as_bytes(), format.media_type(), None)
            .unwrap_or_else(|error| panic!("{format:?}: private use is uncapped, got {error}"));
        assert_tag_present(&parsed, "en-x-cantbethislong", &format!("{format:?}"));

        let outside = document_source("en-cantbethislong");
        assert!(
            parse_dataset(outside.as_bytes(), format.media_type(), None).is_err(),
            "{format:?}: the same subtag OUTSIDE private use is over the §2.1 ceiling"
        );
    }
}
