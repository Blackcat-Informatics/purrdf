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

// ───────────────────────────────────────────────────────────────────────────────
// The TREE codecs: JSON-LD, YAML-LD, HexTuples and TriX
// ───────────────────────────────────────────────────────────────────────────────

/// The four tree codecs read language tags from a structure, not from a
/// `LANGTAG` token, so they need their own fixtures — and they needed their own
/// tests, because for a long time they had NO language-tag contract at all.
///
/// Every one of them interned `@language` / field 5 / `xml:lang` unexamined,
/// which made the claim "genuine garbage is refused on every path" false on four
/// of the six readers. The demonstration was on the shipped CLI: a JSON-LD
/// document carrying `"@language": "en us"` converted to N-Quads with exit 0, an
/// empty loss ledger, and the line `"hello"@en us .` — bytes that no RDF parser
/// anywhere can read back. Writing `9-9` was the same bug one step less
/// visible: exit 0 on the way out, exit 1 reading the same file back in.
///
/// So these tests are shaped as the asymmetry they exist to forbid: what a tree
/// codec ACCEPTS must be what a text codec accepts, in both directions.
const TREE_FORMATS: &[NativeRdfFormat] = &[
    NativeRdfFormat::JsonLd,
    NativeRdfFormat::YamlLd,
    NativeRdfFormat::HexTuples,
    NativeRdfFormat::TriX,
];

/// A one-quad document carrying `tag` on its object literal, in `format`.
///
/// Each is the smallest document in that syntax that reaches its codec's
/// language-tag site: JSON-LD's and YAML-LD's value object, HexTuples' field 5
/// under the `rdf:langString` field-4 sentinel, and TriX's `xml:lang` attribute
/// on `<plainLiteral>`. The subject, predicate and lexical form match
/// [`document_source`] so the tree and text arms compare like with like.
fn tree_document_source(format: NativeRdfFormat, tag: &str) -> String {
    match format {
        NativeRdfFormat::JsonLd => format!(
            "{{\"@id\":\"{SUBJECT}\",\"{PREDICATE}\":\
             [{{\"@value\":\"{VALUE}\",\"@language\":\"{tag}\"}}]}}"
        ),
        NativeRdfFormat::YamlLd => format!(
            "\"@id\": \"{SUBJECT}\"\n\
             \"{PREDICATE}\":\n  \
             - \"@value\": \"{VALUE}\"\n    \
             \"@language\": \"{tag}\"\n"
        ),
        NativeRdfFormat::HexTuples => format!(
            "[\"{SUBJECT}\",\"{PREDICATE}\",\"{VALUE}\",\
             \"http://www.w3.org/1999/02/22-rdf-syntax-ns#langString\",\"{tag}\",\"\"]\n"
        ),
        NativeRdfFormat::TriX => format!(
            "<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"><graph><triple>\
             <uri>{SUBJECT}</uri><uri>{PREDICATE}</uri>\
             <plainLiteral xml:lang=\"{tag}\">{VALUE}</plainLiteral>\
             </triple></graph></TriX>"
        ),
        other => panic!("{other:?} is not one of the four tree codecs"),
    }
}

/// JSON-LD reaches a language tag from THREE places, not one, and a gate on
/// only the first would leave the other two open. These are the other two —
/// plus the term-scoped `@language` mapping, which shares the context path with
/// the document default but resolves through a different branch.
///
/// YAML-LD is deliberately not repeated here: its reader is the YAML→JSON
/// bridge followed by the very same expander, so a second copy of these
/// fixtures would re-test the same code and prove nothing new. The bridge
/// itself is proven by [`tree_document_source`]'s YAML-LD arm.
fn jsonld_context_sources(tag: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "@context default @language applied to a bare string",
            format!(
                "{{\"@context\":{{\"@language\":\"{tag}\",\
                 \"p\":{{\"@id\":\"{PREDICATE}\"}}}},\
                 \"@id\":\"{SUBJECT}\",\"p\":\"{VALUE}\"}}"
            ),
        ),
        (
            "term-scoped @language mapping applied to a bare string",
            format!(
                "{{\"@context\":{{\"p\":{{\"@id\":\"{PREDICATE}\",\
                 \"@language\":\"{tag}\"}}}},\
                 \"@id\":\"{SUBJECT}\",\"p\":\"{VALUE}\"}}"
            ),
        ),
        (
            "@container: @language map key",
            format!(
                "{{\"@context\":{{\"p\":{{\"@id\":\"{PREDICATE}\",\
                 \"@container\":\"@language\"}}}},\
                 \"@id\":\"{SUBJECT}\",\"p\":{{\"{tag}\":\"{VALUE}\"}}}}"
            ),
        ),
    ]
}

/// The tags the four tree codecs must keep taking. Over-refusal is the mirror
/// of the bug being fixed: a gate that refuses `x-purrdf-afrikaans` would look
/// like correct strictness and quietly reject a downstream project's data.
///
/// `i-enochian` earns its place by being the awkward one — a one-character
/// primary subtag, which RFC 5646's `langtag` production does not admit at all
/// and only the `LANGTAG` terminal takes.
const TREE_MUST_SURVIVE: &[&str] = &[
    "en",
    "en-US",
    "zh-Hans-CN",
    "de-CH-x-phonebk",
    "i-enochian",
    "x-purrdf-afrikaans",
    "x-gmeow-english",
];

/// The tags the four tree codecs must now REFUSE, each paired with the
/// neighbour one edit away that must still be accepted.
///
/// `en us` is first because it is the one that produced syntactically invalid
/// output rather than merely unreadable-by-us output: a space cannot appear in
/// an N-Triples `LANGTAG` under any profile, so the written file was not
/// N-Quads at all. Its neighbour is the same two subtags spelled the way the
/// user meant.
const TREE_MUST_REFUSE: &[(&str, &str)] = &[
    ("en us", "en-US"),
    ("1", "en"),
    ("-", "en"),
    ("en-", "en"),
    ("9-9", "en-9"),
    ("123-456", "abc-456"),
    ("!!!", "en"),
];

/// The accepting half: every valid tag survives being read by a tree codec and
/// written out to N-Triples, and survives the crossing back through every text
/// codec and every other tree codec.
#[test]
fn language_tags_survive_crossing_the_tree_codecs() {
    for tag in TREE_MUST_SURVIVE {
        for &entry in TREE_FORMATS {
            let source = tree_document_source(entry, tag);
            let parsed = parse_dataset(source.as_bytes(), entry.media_type(), None)
                .unwrap_or_else(|error| panic!("{entry:?} must accept {tag:?}, got {error}"));
            assert_tag_present(&parsed, tag, &format!("{entry:?} parse"));

            // Out to every text codec and back, then out to every tree codec
            // and back: the round-trip property the PR claims, executed rather
            // than assumed.
            for format in [
                NativeRdfFormat::NTriples,
                NativeRdfFormat::NQuads,
                NativeRdfFormat::Turtle,
                NativeRdfFormat::TriG,
                NativeRdfFormat::RdfXml,
                NativeRdfFormat::JsonLd,
                NativeRdfFormat::YamlLd,
                NativeRdfFormat::HexTuples,
                NativeRdfFormat::TriX,
            ] {
                let (_, back) = cross(&parsed, format);
                assert_tag_present(&back, tag, &format!("{entry:?} -> {format:?} -> back"));
            }
        }
    }
}

/// The other direction, which is where the asymmetry actually bit: a tag read
/// from a TEXT codec must be writable by every tree codec and readable back.
#[test]
fn language_tags_survive_crossing_from_text_into_the_tree_codecs() {
    for (tag, corpus) in MUST_SURVIVE {
        let source = document_source(tag);
        let parsed = parse_dataset(
            source.as_bytes(),
            NativeRdfFormat::NTriples.media_type(),
            None,
        )
        .unwrap_or_else(|error| panic!("N-Triples must accept {tag:?} ({corpus}), got {error}"));

        for &format in TREE_FORMATS {
            let (_, back) = cross(&parsed, format);
            assert_tag_present(&back, tag, &format!("N-Triples -> {format:?} -> back"));
        }
    }
}

/// The refusing half, on all four tree codecs, with its neighbour.
///
/// The named diagnostic code matters as much as the refusal: a bare "parse
/// error" tells a user their file is wrong, while `langtag-subtag-length-zero`
/// tells them which production refused and therefore what to change.
#[test]
fn the_refused_tags_are_refused_on_every_tree_codec() {
    for &format in TREE_FORMATS {
        for (refused, neighbour) in TREE_MUST_REFUSE {
            let bad = tree_document_source(format, refused);
            let Err(error) = parse_dataset(bad.as_bytes(), format.media_type(), None) else {
                panic!("{format:?}: {refused:?} is not an acceptable language tag");
            };
            assert!(
                error.code.starts_with("langtag-"),
                "{format:?}: {refused:?} must be refused by the langtag grammar and say so, \
                 got code {:?} ({})",
                error.code,
                error.message
            );

            let good = tree_document_source(format, neighbour);
            let parsed =
                parse_dataset(good.as_bytes(), format.media_type(), None).unwrap_or_else(|error| {
                    panic!("{format:?}: {neighbour:?} must stay accepted, got {error}")
                });
            assert_tag_present(&parsed, neighbour, &format!("{format:?} neighbour"));
        }
    }
}

/// JSON-LD's other two language-tag sites, both halves.
///
/// A gate placed on the value object alone would leave a document's `@context`
/// default and its `@container: "@language"` map keys ungated, and both reach
/// the same literal.
#[test]
fn every_jsonld_language_site_is_gated_both_ways() {
    for (site, source) in jsonld_context_sources("zh-Hans-CN") {
        let parsed = parse_dataset(
            source.as_bytes(),
            NativeRdfFormat::JsonLd.media_type(),
            None,
        )
        .unwrap_or_else(|error| panic!("JSON-LD {site} must accept a valid tag, got {error}"));
        assert_tag_present(&parsed, "zh-Hans-CN", site);
    }

    for (refused, _) in TREE_MUST_REFUSE {
        for (site, source) in jsonld_context_sources(refused) {
            let Err(error) = parse_dataset(
                source.as_bytes(),
                NativeRdfFormat::JsonLd.media_type(),
                None,
            ) else {
                panic!("JSON-LD {site}: {refused:?} must be refused");
            };
            assert!(
                error.code.starts_with("langtag-"),
                "JSON-LD {site}: {refused:?} must be refused by the langtag grammar, \
                 got code {:?} ({})",
                error.code,
                error.message
            );
        }
    }
}

/// The corrupt-output case, pinned on its own because it is the only one that
/// produced bytes that are not RDF in any syntax.
///
/// The assertion is not merely "the read is refused" but "the invalid N-Quads
/// can no longer be produced": the write is attempted, and the only way past
/// the refusal would be for it to succeed.
#[test]
fn a_language_tag_with_a_space_can_no_longer_produce_invalid_nquads() {
    for &format in TREE_FORMATS {
        let source = tree_document_source(format, "en us");
        let Err(error) = parse_dataset(source.as_bytes(), format.media_type(), None) else {
            panic!(
                "{format:?}: a language tag containing a space must never reach a dataset — \
                 serializing it writes bytes that are not N-Quads"
            );
        };
        assert!(
            error.code.starts_with("langtag-"),
            "{format:?}: got code {:?} ({})",
            error.code,
            error.message
        );
    }
}
