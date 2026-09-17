// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two ShEx syntaxes must admit the same language tags, or the crate refuses
//! its own output.
//!
//! `crate::lexer` holds a ShExC `@tag` to `LANGTAG_PROFILE`. ShExJ has no lexer
//! at all: an `ObjectLiteral`'s `"language"`, a `Language` member's
//! `"languageTag"` and a string exclusion of a `LanguageStemRange` are bare
//! `String`s lifted out of an untrusted JSON document — and `to_shexc` writes
//! every one of them straight back out after an `@`. So an ungated ShExJ ingress
//! is a live round-trip break, not a theoretical one: `{"language": "en us"}`
//! parses, serializes to `"v"@en us`, and `parse_shexc` then rejects the bytes
//! this crate itself just wrote.
//!
//! Both halves are driven here. A gate that refused `x-purrdf-afrikaans`,
//! `en-fr-jura` or `fr-be-fbcl` — tags this workspace's artifacts and the
//! approved W3C corpora actually carry — would be a worse bug than the escape it
//! closes, so every accepted tag is proven to still survive the full
//! `parse_shexj → to_shexc → parse_shexc` trip.

use purrdf_shex::{Schema, parse_shexc, parse_shexj, to_shexc};

/// Tags real schemas and real data carry, which every position must keep taking.
///
/// `fr-be` is the exact family the vendored shexTest `LanguageStemRange`
/// exclusions use, and `en-uk`/`en-fr` are the ones its `ObjectLiteral`s use.
const ACCEPTED: &[&str] = &[
    "en",
    "en-US",
    "en-uk",
    "en-fr",
    "fr-be",
    "zh-Hans-CN",
    "de-CH-x-phonebk",
    "i-enochian",
    "x-purrdf-afrikaans",
    "x-gmeow-english",
    "en-fr-jura",
    "fr-be-fbcl",
    "abcdefgh",
    "en-x-cantbethislong",
];

/// Strings no ShExC lexer would have produced. Carried through `to_shexc`, each
/// writes bytes `parse_shexc` cannot read back.
const REFUSED: &[&str] = &[
    "en us",
    "1",
    "9-9",
    "123-456",
    "en-",
    "-",
    "!!!",
    "abcdefghi",
    "",
];

/// A named ShExJ position, as the smallest document that puts a language tag in
/// it — a human label for the failure message, and the builder that reaches it.
type Position = (&'static str, fn(&str) -> String);

/// The three ShExJ members that hold a WHOLE language tag, each wrapped in the
/// smallest schema that reaches it, and each with an absolute-IRI skeleton so no
/// base is needed and nothing but the tag is under test.
const POSITIONS: &[Position] = &[
    ("ObjectLiteral \"language\"", object_literal_doc),
    ("Language \"languageTag\"", language_value_doc),
    ("LanguageStemRange exclusion", stem_range_exclusion_doc),
];

fn wrap(values: &str) -> String {
    format!(
        concat!(
            r#"{{"type":"Schema","shapes":[{{"type":"Shape","#,
            r#""id":"https://example.org/S","#,
            r#""expression":{{"type":"TripleConstraint","#,
            r#""predicate":"https://example.org/p","#,
            r#""valueExpr":{{"type":"NodeConstraint","values":[{values}]}}}}}}]}}"#
        ),
        values = values
    )
}

fn object_literal_doc(tag: &str) -> String {
    wrap(&format!(
        r#"{{"value":"v","language":{tag}}}"#,
        tag = quote(tag)
    ))
}

fn language_value_doc(tag: &str) -> String {
    wrap(&format!(
        r#"{{"type":"Language","languageTag":{tag}}}"#,
        tag = quote(tag)
    ))
}

fn stem_range_exclusion_doc(tag: &str) -> String {
    wrap(&format!(
        r#"{{"type":"LanguageStemRange","stem":"","exclusions":[{tag}]}}"#,
        tag = quote(tag)
    ))
}

/// JSON-quote a tag. Every tag under test is ASCII with no `"` or `\`, so this
/// is exact; a `serde_json` dependency here would only hide what is being fed in.
fn quote(tag: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii() && c != '"' && c != '\\'),
        "the fixture tags are plain ASCII: {tag:?}"
    );
    format!("\"{tag}\"")
}

/// The whole point: a tag ShExJ admits is a tag ShExC can write AND read back.
///
/// Two claims, and the first is the one the gate exists for — `parse_shexc` must
/// accept the bytes `to_shexc` just wrote, which is exactly what fails for
/// `"v"@en us`. The AST-identity claim is then made on the tags that are already
/// ASCII-lowercase, because the ShExC parser's own `@tag` case handling is not
/// uniform across these three positions (an `ObjectLiteral`'s tag is folded, a
/// `Language` member's is kept) — a normalization asymmetry that long predates
/// this gate, is invisible to RDF language-tag equality, and is not what is
/// under test here.
#[test]
fn every_accepted_tag_survives_shexj_to_shexc_to_shexc() {
    for (what, doc) in POSITIONS {
        for tag in ACCEPTED {
            let json = doc(tag);
            let from_json: Schema = parse_shexj(&json, None)
                .unwrap_or_else(|e| panic!("{what} @{tag} must parse from ShExJ: {e}"));

            let shexc = to_shexc(&from_json);
            let from_shexc = parse_shexc(&shexc, None).unwrap_or_else(|e| {
                panic!("{what} @{tag}: this crate wrote ShExC it cannot read: {e}\n{shexc}")
            });

            if tag.chars().all(|c| !c.is_ascii_uppercase()) {
                assert_eq!(
                    from_shexc, from_json,
                    "{what} @{tag} must round-trip identically through ShExC\n{shexc}"
                );
            }
        }
    }
}

/// And the mirror: the refusal lands at ingress, where the document that named
/// the tag is still in hand, not later at a serializer that can only write it.
#[test]
fn every_refused_tag_is_rejected_at_shexj_ingress() {
    for (what, doc) in POSITIONS {
        for tag in REFUSED {
            let json = doc(tag);
            let error = parse_shexj(&json, None).err().unwrap_or_else(|| {
                panic!("{what} {tag:?} must not enter the AST — ShExC could never write it")
            });
            let reason = error.to_string();
            assert!(
                reason.contains("langtag-"),
                "{what} {tag:?} must quote the grammar's own diagnostic code, got: {reason}"
            );
            assert!(
                reason.contains(tag) || tag.is_empty(),
                "{what} {tag:?} must quote the offending tag, got: {reason}"
            );
        }
    }
}

/// A *stem* is deliberately NOT held to the complete-tag grammar: it is an
/// RFC 4647 basic-filtering prefix, and the empty stem is the spec's "any
/// language" wildcard that the vendored corpus uses. Refusing it would be the
/// over-refusal mirror of the escape above.
#[test]
fn a_language_stem_is_not_judged_as_a_complete_tag() {
    for stem in ["", "en", "fr", "zh-Hans"] {
        let json = wrap(&format!(
            r#"{{"type":"LanguageStem","stem":{stem}}}"#,
            stem = quote(stem)
        ));
        parse_shexj(&json, None)
            .unwrap_or_else(|e| panic!("LanguageStem {stem:?} is a prefix, not a tag: {e}"));
    }
}
