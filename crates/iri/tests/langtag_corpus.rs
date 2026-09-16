// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RFC 5646 `Language-Tag` well-formedness corpus.
//!
//! Sources (see `tests/PROVENANCE.md`): the worked examples of **RFC 5646
//! Appendix A** transcribed verbatim, the closed grandfathered list of
//! **§2.2.8** in the RFC's own order, and boundary cases derived subtag-by-
//! subtag from the **§2.1 ABNF** — each one names the production and the
//! repetition or length bound it sits on. Every refused input is paired with an
//! accepted neighbor (repo refusal discipline).

use purrdf_iri::langtag::{LanguageTagError, TagForm, is_well_formed, parse};

/// RFC 5646 Appendix A — every well-formed example, transcribed verbatim.
const APPENDIX_A_WELL_FORMED: &[&str] = &[
    // Simple language subtag
    "de",
    "fr",
    "ja",
    "i-enochian", // grandfathered
    // Language subtag plus script subtag
    "zh-Hant",
    "zh-Hans",
    "sr-Cyrl",
    "sr-Latn",
    // Extended language subtags (with their §4.5 preferred forms)
    "zh-cmn-Hans-CN",
    "cmn-Hans-CN",
    "zh-yue-HK",
    "yue-HK",
    // Language-script-region
    "zh-Hans-CN",
    "sr-Latn-RS",
    // Language-variant
    "sl-rozaj",
    "sl-rozaj-biske",
    "sl-nedis",
    // Language-region-variant
    "de-CH-1901",
    "sl-IT-nedis",
    // Language-script-region-variant
    "hy-Latn-IT-arevela",
    // Language-region
    "de-DE",
    "en-US",
    "es-419",
    // Private-use subtags
    "de-CH-x-phonebk",
    "az-Arab-x-AZE-derbend",
    // Private-use registry values
    "x-whatever",
    "qaa-Qaaa-QM-x-southern",
    "de-Qaaa",
    "sr-Latn-QM",
    "sr-Qaaa-RS",
    // Tags that use extensions
    "en-US-u-islamcal",
    "zh-CN-a-myext-x-private",
    "en-a-myext-b-another",
];

/// RFC 5646 §2.2.8 — the closed grandfathered list, in the order the RFC
/// presents it: the 17 `irregular` tags, then the 9 `regular` ones.
const GRANDFATHERED: &[&str] = &[
    // irregular
    "en-GB-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    // regular
    "art-lojban",
    "cel-gaulish",
    "no-bok",
    "no-nyn",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

#[test]
fn appendix_a_examples_are_well_formed() {
    for tag in APPENDIX_A_WELL_FORMED {
        assert!(is_well_formed(tag), "{tag:?} must be well-formed");
    }
}

#[test]
fn appendix_a_invalid_examples() {
    // Appendix A "Some Invalid Tags", split along the well-formed/valid line
    // RFC 5646 §2.2.9 draws. The first two fail the ABNF and must refuse:
    assert_eq!(
        parse("de-419-DE"),
        Err(LanguageTagError::UnconsumedSubtag),
        "two region subtags"
    );
    assert_eq!(
        parse("a-DE"),
        Err(LanguageTagError::LanguageProductionUnmatched),
        "single-character primary language"
    );
    // The third is *invalid* (duplicate singleton) but still *well-formed*;
    // a well-formedness checker must accept it.
    assert!(
        is_well_formed("ar-a-aaa-b-bbb-a-ccc"),
        "duplicate singletons are well-formed (validity is a registry concern)"
    );
}

#[test]
fn grandfathered_tags_match_as_whole_case_insensitive_units() {
    for tag in GRANDFATHERED {
        let parsed = parse(tag).expect("grandfathered tag must parse");
        assert_eq!(parsed.form(), TagForm::Grandfathered, "{tag:?}");
        let upper = tag.to_ascii_uppercase();
        assert_eq!(
            parse(&upper).expect("case-insensitive").form(),
            TagForm::Grandfathered,
            "{upper:?}"
        );
    }
    // A grandfathered tag extended by one subtag is no longer on the closed
    // list and must re-enter the ordinary grammar. `en-GB-oed-x` fails there
    // (`oed` fits no production after a region), while `zh-min-nan-nan`
    // happens to re-parse as an ordinary three-extlang langtag.
    assert!(parse("en-GB-oed-x").is_err());
    assert_eq!(
        parse("zh-min-nan").expect("on the closed list").form(),
        TagForm::Grandfathered
    );
    assert_eq!(
        parse("zh-min-nan-nan").expect("ordinary grammar").form(),
        TagForm::Langtag
    );
}

#[test]
fn abnf_boundaries_with_accepted_neighbors() {
    // (refused, accepted-neighbor, why)
    let pairs: &[(&str, &str, &str)] = &[
        ("", "en", "empty input"),
        ("e", "en", "primary language needs 2+ letters"),
        ("abcdefghi", "abcdefgh", "8-character subtag ceiling"),
        ("en--US", "en-US", "empty interior subtag"),
        ("en-", "en", "trailing hyphen"),
        ("-en", "en", "leading hyphen"),
        ("en-a", "en-a-bb", "extension singleton needs a subtag"),
        ("en-x", "en-x-a", "private-use marker needs a subtag"),
        ("x-", "x-a", "whole-tag private use needs a subtag"),
        ("x-abcdefghi", "x-abcdefgh", "private-use subtag ceiling"),
        (
            // `extlang = 3ALPHA *2("-" 3ALPHA)`: one subtag plus at most two
            // repetitions, so three in total.
            "zh-cmn-yue-nan-hak",
            "zh-cmn-yue-nan",
            "the extlang repetition is bounded at two",
        ),
        (
            // `["-" extlang]` hangs off the `2*3ALPHA` alternative of
            // `language` only, not off `4ALPHA` or `5*8ALPHA`.
            "abcd-efg",
            "abcd-Latn",
            "a 4ALPHA primary language admits no extlang",
        ),
        ("en-Lat1", "en-Latn", "script is exactly 4 letters"),
        (
            "en-a234",
            "en-1234",
            "4-char variant must start with a digit",
        ),
        ("en-ü", "en-u-uu", "ASCII only"),
        ("de-419-DE", "de-DE", "one region subtag"),
    ];
    for (refused, accepted, why) in pairs {
        assert!(parse(refused).is_err(), "{refused:?} must refuse ({why})");
        assert!(is_well_formed(accepted), "{accepted:?} must accept ({why})");
    }

    // Spending the extlang repetition must not refuse the sections that may
    // legitimately follow it: `script`, `region`, `variant`, `extension` and
    // `privateuse` all still apply after a third extlang subtag.
    for accepted in [
        "zh-cmn-yue-nan-Hant",
        "zh-cmn-yue-nan-CN",
        "zh-cmn-yue-nan-Hant-CN",
        "zh-cmn-yue-nan-Hant-CN-1901",
        "zh-cmn-yue-nan-u-islamcal",
        "zh-cmn-yue-nan-x-priv",
    ] {
        assert!(
            is_well_formed(accepted),
            "{accepted:?} must accept: only a *fourth extlang* is barred"
        );
    }
}

#[test]
fn case_is_insignificant_for_well_formedness() {
    for tag in ["en-US", "EN-us", "eN-Us", "zh-hant", "ZH-HANT", "X-FOO"] {
        assert!(is_well_formed(tag), "{tag:?}");
    }
}

#[test]
fn consumer_contracts_hold() {
    // The two workspace consumers this module replaces a dependency for:
    //
    // 1. Embedding metadata accepts only canonical-lowercase, well-formed
    //    tags (its lowercase check is separate and stays separate).
    assert!(is_well_formed("en-us"));
    assert!(is_well_formed("zh-hans-cn"));
    // 2. CSVW accepts any well-formed tag and lowercases afterwards.
    assert!(is_well_formed("en-US"));
    assert!(!is_well_formed("not a tag"));
    assert!(!is_well_formed("und-"));
    assert!(is_well_formed("und"));
}
