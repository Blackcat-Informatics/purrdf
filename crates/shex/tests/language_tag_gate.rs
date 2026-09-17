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
//! The same is true one field over, of a `LanguageStem`/`LanguageStemRange`
//! *stem*, which `to_shexc` writes verbatim between `@` and `~`. A stem is only a
//! prefix, so it is held to "empty, or a whole tag" rather than to the whole-tag
//! grammar alone — and that is not a weakening, because `{""} ∪ {valid tags}` is
//! exactly the set ShExC can produce there.
//!
//! Both halves are driven here, at all six positions. A gate that refused
//! `x-purrdf-afrikaans`,
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

/// A stem position: the label, the builder, and whether ShExC has a production
/// for an **empty** stem there. See [`STEM_POSITIONS`] for why the third does not.
type StemPosition = (&'static str, fn(&str) -> String, bool);

/// The three ShExJ positions that hold a language **stem**, each wrapped in the
/// smallest schema that reaches it.
///
/// A stem is an RFC 4647 basic-filtering prefix, which is why these were once
/// left ungated — but `to_shexc` writes a stem verbatim between `@` and `~` just
/// as it writes a whole tag verbatim after `@`, so the round-trip break is
/// identical in kind. The set ShExC can *produce* here is `{""} ∪ {valid tags}`
/// and nothing else (`crate::parser` builds a language stem from a `Token::LangTag`
/// or from the empty stem `@~`), so holding a stem to "empty, or a whole tag"
/// refuses nothing a ShExC schema could have written.
///
/// The third entry's `false` is the exception, and it is not about the tag
/// grammar: ShExC's `languageExclusion ::= '-' LANGTAG '~'?` has no empty form at
/// all — see `every_accepted_stem_survives_shexj_to_shexc_to_shexc`.
const STEM_POSITIONS: &[StemPosition] = &[
    ("LanguageStem \"stem\"", language_stem_doc, true),
    ("LanguageStemRange \"stem\"", stem_range_stem_doc, true),
    (
        "LanguageStemRange exclusion stem",
        exclusion_stem_doc,
        false,
    ),
];

fn language_stem_doc(stem: &str) -> String {
    wrap(&format!(
        r#"{{"type":"LanguageStem","stem":{stem}}}"#,
        stem = quote(stem)
    ))
}

fn stem_range_stem_doc(stem: &str) -> String {
    wrap(&format!(
        r#"{{"type":"LanguageStemRange","stem":{stem},"exclusions":["fr"]}}"#,
        stem = quote(stem)
    ))
}

/// The object form of an exclusion — `{"type":"LanguageStem","stem":…}` inside a
/// `LanguageStemRange` — which is a third ingress, distinct from the string form
/// `stem_range_exclusion_doc` drives.
fn exclusion_stem_doc(stem: &str) -> String {
    wrap(&format!(
        concat!(
            r#"{{"type":"LanguageStemRange","stem":"","exclusions":"#,
            r#"[{{"type":"LanguageStem","stem":{stem}}}]}}"#
        ),
        stem = quote(stem)
    ))
}

/// Every stem a ShExC schema can actually contain: the empty "any language"
/// wildcard the vendored corpus uses, plus every whole tag.
///
/// The empty stem is the over-refusal case that matters most here — it is the one
/// string that is a legal stem and NOT a legal tag, so it is the single place
/// where gating a stem could have broken a real schema.
fn accepted_stems() -> impl Iterator<Item = &'static str> {
    core::iter::once("").chain(ACCEPTED.iter().copied())
}

/// [`REFUSED`] minus the empty string, which is a legal stem even though it is
/// not a legal tag. Everything left is a string `to_shexc` would write and
/// `parse_shexc` would then refuse.
fn refused_stems() -> impl Iterator<Item = &'static str> {
    REFUSED.iter().copied().filter(|s| !s.is_empty())
}

/// A stem this crate admits must be a stem it can write and read back — the same
/// claim the whole-tag test makes, made at the three stem positions.
///
/// Identity is asserted unconditionally here, unlike the whole-tag case: no stem
/// production folds case on either side, so `@zh-Hans-CN~` comes back exactly as
/// it went in.
///
/// # The empty stem is skipped at the exclusion position, and that is a finding
///
/// ShExC's `languageExclusion ::= '-' LANGTAG '~'?` has no empty form — an
/// exclusion must start from a `LANGTAG`, and `@~` lexes as `'@' '~'`, which
/// `parse_language_exclusions` does not accept. So
/// `{"type":"LanguageStemRange","stem":"","exclusions":[{"type":"LanguageStem","stem":""}]}`
/// is a ShExJ schema with **no** ShExC spelling: `to_shexc` writes `[@~ - @~]` and
/// `parse_shexc` refuses it with "expected language tag exclusion".
///
/// That is a real round-trip break, and it is **not** the language-tag gate's:
/// it lives entirely in `shexc.rs` and `parser.rs`, which the gate does not
/// reach, the string involved is not an ungrammatical language tag, and the gap
/// is in the ShEx spec's own ShExC grammar rather than in any profile. Refusing an empty
/// exclusion stem at ingress would close it, but an empty stem is legal ShExJ, so
/// that refusal would be the over-refusal mirror — it is the maintainer's call,
/// not this test's. What this test does is refuse to launder it: the skip is
/// explicit, and the vector is written down above.
#[test]
fn every_accepted_stem_survives_shexj_to_shexc_to_shexc() {
    for (what, doc, empty_is_writable) in STEM_POSITIONS {
        for stem in accepted_stems() {
            if stem.is_empty() && !empty_is_writable {
                // Still prove the INGRESS half: the gate must admit it.
                parse_shexj(&doc(stem), None)
                    .unwrap_or_else(|e| panic!("{what} \"\" is a legal ShExJ stem: {e}"));
                continue;
            }
            let json = doc(stem);
            let from_json: Schema = parse_shexj(&json, None)
                .unwrap_or_else(|e| panic!("{what} {stem:?} must parse from ShExJ: {e}"));

            let shexc = to_shexc(&from_json);
            let from_shexc = parse_shexc(&shexc, None).unwrap_or_else(|e| {
                panic!("{what} {stem:?}: this crate wrote ShExC it cannot read: {e}\n{shexc}")
            });
            assert_eq!(
                from_shexc, from_json,
                "{what} {stem:?} must round-trip identically through ShExC\n{shexc}"
            );
        }
    }
}

/// And the mirror, at the three stem positions: a stem ShExC could never have
/// written is refused where the document that named it is still in hand.
///
/// This is the test that was missing. Its predecessor stopped at `parse_shexj`
/// and fed it only stems that were already valid tags, so it asserted the gate's
/// absence without ever taking the `to_shexc → parse_shexc` leg that exposes it:
/// `{"type":"LanguageStem","stem":"en-"}` became `[@en-~]`, which this crate's own
/// lexer rejects as `langtag-subtag-length-zero`.
#[test]
fn every_refused_stem_is_rejected_at_shexj_ingress() {
    for (what, doc, _) in STEM_POSITIONS {
        for stem in refused_stems() {
            let json = doc(stem);
            let error = parse_shexj(&json, None).err().unwrap_or_else(|| {
                panic!("{what} {stem:?} must not enter the AST — ShExC could never write it")
            });
            let reason = error.to_string();
            assert!(
                reason.contains("langtag-"),
                "{what} {stem:?} must quote the grammar's own diagnostic code, got: {reason}"
            );
            assert!(
                reason.contains(stem),
                "{what} {stem:?} must quote the offending stem, got: {reason}"
            );
        }
    }
}
