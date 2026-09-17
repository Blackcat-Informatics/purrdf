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

use purrdf_iri::langtag::{
    Extension, LanguageTagBuf, LanguageTagError, Profile, TagForm, canonical_case,
    canonical_case_with, is_well_formed, is_well_formed_with, parse, parse_with,
};

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

/// The `extlang` gate: which alternative of `language` a following 3ALPHA
/// subtag may attach to.
///
/// `language = 2*3ALPHA ["-" extlang] / 4ALPHA / 5*8ALPHA`. The optional
/// `["-" extlang]` hangs off the **first** alternative only, so a three-letter
/// subtag is an `extlang` after a 2–3 letter primary language and is *nothing*
/// after a 4-letter (reserved) or 5–8 letter (registered) one. The parser
/// decides this with a single length test on the primary subtag, which makes the
/// boundary one comparison wide: flip it and the grammar silently changes shape.
///
/// The four cases below are chosen to pin that one comparison from both sides
/// and at both ends of the range it guards:
///
/// * `abcd-Latn` — a 4ALPHA language really does continue, just into `script`
///   rather than `extlang`; without this the "refuse after 4ALPHA" case could be
///   satisfied by refusing everything after a 4ALPHA language.
/// * `AaBbCcDd-x-y-any-x` — the 5*8ALPHA end of the range, at its maximum
///   length, still carrying a full `privateuse` section whose last subtag is the
///   one-character `x` that only `privateuse` admits. Mixed case throughout,
///   because case is insignificant to the judgement.
/// * `abcd-abc` — the refusal: three letters after a 4ALPHA language satisfy no
///   production (`script` is 4ALPHA, `region` is 2ALPHA or 3DIGIT, `variant` is
///   5*8alphanum or DIGIT 3alphanum), so the subtag is left over.
/// * `ab-abc` — the neighbour that proves the refusal above is about the
///   *primary language's length* and not about three-letter subtags in general.
#[test]
fn extlang_attaches_only_to_the_two_to_three_letter_language_alternative() {
    // 4ALPHA language: no `extlang`, but `script` still follows.
    let reserved = parse("abcd-Latn").expect("`4ALPHA` language plus `script`");
    assert_eq!(reserved.primary_language(), Some("abcd"));
    assert_eq!(reserved.extended_language(), None);
    assert_eq!(reserved.script(), Some("Latn"));

    // 5*8ALPHA language at its ceiling: no `extlang`, but `privateuse` follows,
    // and its final subtag is the one-character `x` no other production admits.
    let registered = parse("AaBbCcDd-x-y-any-x").expect("`5*8ALPHA` language plus `privateuse`");
    assert_eq!(registered.primary_language(), Some("AaBbCcDd"));
    assert_eq!(registered.extended_language(), None);
    assert_eq!(registered.private_use(), Some("x-y-any-x"));
    assert_eq!(
        registered.private_use_subtags().collect::<Vec<_>>(),
        ["y", "any", "x"]
    );

    // The refusal: 3ALPHA after a 4ALPHA language is not an `extlang`.
    assert_eq!(
        parse("abcd-abc"),
        Err(LanguageTagError::UnconsumedSubtag),
        "`[\"-\" extlang]` hangs off `2*3ALPHA` only"
    );

    // The neighbour: the same 3ALPHA subtag after a 2ALPHA language IS one.
    let extended = parse("ab-abc").expect("`2*3ALPHA` language plus `extlang`");
    assert_eq!(extended.primary_language(), Some("ab"));
    assert_eq!(extended.extended_language(), Some("abc"));
}

/// Three refusals this corpus asserts elsewhere without the accepted neighbour
/// the repo's refusal discipline requires, each paired here with the *smallest*
/// edit that turns it back into a well-formed tag.
///
/// Each pair isolates exactly one rule, so an over-refusal that widened any of
/// them by a single character shows up as a failure on the right-hand column
/// rather than as continued green:
///
/// * `a1` / `aa` — every alternative of `language` is ALPHA, so a two-character
///   primary containing a DIGIT matches none of them. The neighbour differs by
///   one character and is the shortest possible tag.
/// * `en-a-b-cc` / `en-a-bb-b-cc` — `extension = singleton 1*("-" (2*8alphanum))`
///   gives each extension at least one subtag of two or more characters, so a
///   second singleton may not appear until the first has one. The neighbour
///   gives the first singleton its subtag and keeps the second extension intact,
///   proving that back-to-back extensions themselves are fine.
/// * `a-DE` / `ab-DE` — a one-character primary language is the RFC's own
///   Appendix A invalid example; the neighbour shows the region after it was
///   never the problem.
#[test]
fn the_unpaired_refusals_each_have_an_accepted_neighbour() {
    // (refused, the error it names, accepted neighbour, the rule at stake)
    let pairs: &[(&str, LanguageTagError, &str, &str)] = &[
        (
            "a1",
            LanguageTagError::LanguageProductionUnmatched,
            "aa",
            "a two-character primary language is 2ALPHA, not 2alphanum",
        ),
        (
            "en-a-b-cc",
            LanguageTagError::SingletonWithoutSubtag,
            "en-a-bb-b-cc",
            "a singleton needs a 2*8alphanum subtag before the next singleton",
        ),
        (
            "a-DE",
            LanguageTagError::LanguageProductionUnmatched,
            "ab-DE",
            "the primary language needs at least two characters",
        ),
    ];
    for (refused, expected, accepted, why) in pairs {
        assert_eq!(parse(refused), Err(*expected), "{refused:?} ({why})");
        assert!(is_well_formed(accepted), "{accepted:?} must accept ({why})");
    }

    // The accepted halves are not merely accepted: they decompose the way the
    // rule at stake says they do.
    assert_eq!(
        parse("aa").expect("well-formed").primary_language(),
        Some("aa")
    );
    assert_eq!(
        parse("en-a-bb-b-cc")
            .expect("well-formed")
            .extensions_by_singleton()
            .map(Extension::as_str)
            .collect::<Vec<_>>(),
        ["a-bb", "b-cc"],
        "both extensions survive, each with its own subtag"
    );
    assert_eq!(parse("ab-DE").expect("well-formed").region(), Some("DE"));
}

#[test]
fn case_is_insignificant_for_well_formedness() {
    for tag in ["en-US", "EN-us", "eN-Us", "zh-hant", "ZH-HANT", "X-FOO"] {
        assert!(is_well_formed(tag), "{tag:?}");
    }
}

/// The private-use tags PurRDF's own artifacts carry, and the bound that makes
/// [`Profile::Rfc5646PrivateUseRelaxed`] necessary rather than defensive.
///
/// Each of these appears verbatim in a workspace fixture or corpus, so the
/// pairing below is a live contract: the left column is what §2.1 says, the
/// right column is what a codec reading those artifacts must be able to take.
#[test]
fn purrdf_private_use_tags_need_the_relaxed_profile() {
    // Over the §2.1 ceiling — the relaxation is load-bearing for these.
    for over_ceiling in ["x-purrdf-afrikaans", "x-purrdf-norwegiannynorsk"] {
        assert_eq!(
            parse(over_ceiling),
            Err(LanguageTagError::SubtagLengthOverEight),
            "{over_ceiling:?} exceeds `privateuse = \"x\" 1*(\"-\" (1*8alphanum))`"
        );
        assert!(
            is_well_formed_with(over_ceiling, Profile::Rfc5646PrivateUseRelaxed),
            "{over_ceiling:?} must be accepted by the relaxed profile"
        );
    }
    // Within it — accepted by BOTH profiles, so this tag alone would never
    // have justified the widening.
    assert!(is_well_formed("x-purrdf-english"));
    assert!(is_well_formed_with(
        "x-purrdf-english",
        Profile::Rfc5646PrivateUseRelaxed
    ));
}

/// The relaxed profile is a widening and nothing else: it must never accept a
/// tag the strict profile rejects for a reason other than the private-use
/// ceiling, and must never reject one the strict profile accepts.
#[test]
fn the_relaxed_profile_is_a_strict_superset() {
    let accepted_everywhere = APPENDIX_A_WELL_FORMED
        .iter()
        .chain(GRANDFATHERED.iter())
        .chain(["en-US", "x-purrdf-english", "und"].iter());
    for tag in accepted_everywhere {
        assert!(is_well_formed(tag), "{tag:?} under RFC 5646");
        assert!(
            is_well_formed_with(tag, Profile::Rfc5646PrivateUseRelaxed),
            "{tag:?} must stay accepted under the relaxed profile"
        );
    }
    // Refusals that have nothing to do with private use survive the widening,
    // each with the neighbour that must still be taken.
    let pairs: &[(&str, &str)] = &[
        ("", "en"),
        ("e", "en"),
        ("a-DE", "ab-DE"),
        ("de-419-DE", "de-DE"),
        ("en-Lat1", "en-Latn"),
        ("en-ü", "en-u-uu"),
        ("not a tag", "und"),
        ("abcdefghi", "abcdefgh"),
    ];
    for (refused, accepted) in pairs {
        assert_eq!(
            parse(refused).err(),
            parse_with(refused, Profile::Rfc5646PrivateUseRelaxed).err(),
            "{refused:?} must refuse identically under both profiles"
        );
        assert!(
            is_well_formed_with(accepted, Profile::Rfc5646PrivateUseRelaxed),
            "{accepted:?} must stay accepted"
        );
    }
}

/// The tags the [`Profile::ConcreteSyntaxLangtag`] terminal exists to keep
/// readable, sourced the same way as the rest of this corpus: from artifacts
/// that are actually published rather than from imagination.
///
/// The first group is W3C's — two tags carried by approved ShEx validation
/// vectors which the conformance harness reads as `text/turtle`. The second is
/// a private-use family a downstream project publishes across a large body of
/// literals, whose language-name subtags routinely run past the eight-character
/// private-use cap. Neither group is well-formed RFC 5646, and refusing either
/// would be an over-refusal of input the ecosystem really does write.
#[test]
fn the_terminal_profile_keeps_published_tags_readable() {
    // (tag, is it well-formed RFC 5646?)
    let published: &[(&str, bool)] = &[
        // W3C ShEx validation vectors, read as `text/turtle`. `jura` and `fbcl`
        // are four ALPHA, and `variant` admits four characters only when the
        // first is a DIGIT — so §2.1 refuses both.
        ("en-fr-jura", false),
        ("fr-be-fbcl", false),
        // A downstream project's `x-<project>-<language name>` family. The
        // short names fit §2.1; the long ones do not, and they are the common
        // case rather than the exotic one.
        ("x-gmeow-english", true),
        ("x-gmeow-chinese-latn", true),
        ("x-gmeow-norwegiannynorsk", false),
        ("x-gmeow-westernfrisian", false),
        // This workspace's own artifacts, for the same reason.
        ("x-purrdf-english", true),
        ("x-purrdf-afrikaans", false),
        ("x-purrdf-norwegiannynorsk", false),
    ];
    for (tag, rfc5646_well_formed) in published {
        assert_eq!(
            is_well_formed(tag),
            *rfc5646_well_formed,
            "{tag:?} under RFC 5646"
        );
        assert!(
            is_well_formed_with(tag, Profile::ConcreteSyntaxLangtag),
            "{tag:?} is published and must stay readable"
        );
    }

    // The widening is not a rubber stamp: the terminal still has a grammar, and
    // each refusal below is paired with the neighbour that must survive it.
    let refused: &[(&str, &str)] = &[
        ("1", "en"),
        ("-", "en"),
        ("9-9", "en-9"),
        ("en-", "en"),
        ("123-456", "abc-456"),
        ("en-ü", "en-u-uu"),
    ];
    for (bad, neighbour) in refused {
        assert!(
            !is_well_formed_with(bad, Profile::ConcreteSyntaxLangtag),
            "{bad:?} is not a `LANGTAG` terminal"
        );
        assert!(
            is_well_formed_with(neighbour, Profile::ConcreteSyntaxLangtag),
            "{neighbour:?} must stay accepted"
        );
    }
}

#[test]
fn consumer_contracts_hold() {
    // The two workspace consumers this module replaces a dependency for:
    //
    // 1. Embedding metadata accepts only lowercase, well-formed tags (that
    //    lowercase check is the RDF term-identity form, is separate from
    //    §2.1.1 canonical case, and stays separate — they disagree on purpose).
    assert!(is_well_formed("en-us"));
    assert!(is_well_formed("zh-hans-cn"));
    // 2. CSVW accepts any well-formed tag and stores it in canonical case.
    assert!(is_well_formed("en-US"));
    assert_eq!(canonical_case("en-us").as_deref(), Ok("en-US"));
    assert_eq!(canonical_case("zh-hans-cn").as_deref(), Ok("zh-Hans-CN"));
    assert!(!is_well_formed("not a tag"));
    assert!(!is_well_formed("und-"));
    assert!(is_well_formed("und"));
    assert_eq!(canonical_case("und").as_deref(), Ok("und"));
}

/// RFC 5646 §2.1.1 over the whole Appendix A corpus, and over the published
/// private-use families the terminal profile exists for.
///
/// Normalization rewrites strings, so it needs the same two-sided discipline a
/// refusal does: the tags whose spelling must change, and the tags that must
/// come back byte-identical. The second column is the load-bearing one — the
/// RFC's own examples are written in canonical case, so a normalizer that was
/// wrong about *any* of the three conventions would move one of them.
#[test]
fn canonical_case_is_a_fixed_point_of_the_rfc_examples() {
    for tag in APPENDIX_A_WELL_FORMED.iter().chain(GRANDFATHERED.iter()) {
        // The one Appendix A example not written in canonical case: the RFC
        // gives it as an illustration that private-use subtags may be written
        // in any case, and §2.1.1 folds them down.
        let expected = if *tag == "az-Arab-x-AZE-derbend" {
            "az-Arab-x-aze-derbend"
        } else {
            tag
        };
        assert_eq!(
            canonical_case(tag).as_deref(),
            Ok(expected),
            "{tag:?} under RFC 5646 §2.1.1"
        );
    }

    // The three conventions, each with the case-folded input that exercises it.
    for (written, canonical) in [
        ("de-de", "de-DE"),
        ("EN-us", "en-US"),
        ("zh-hant", "zh-Hant"),
        ("SR-latn-rs", "sr-Latn-RS"),
        ("HY-latn-it-AREVELA", "hy-Latn-IT-arevela"),
        ("EN-us-U-ISLAMCAL", "en-US-u-islamcal"),
        ("I-ENOCHIAN", "i-enochian"),
        ("en-gb-oed", "en-GB-oed"),
        ("SGN-be-fr", "sgn-BE-FR"),
    ] {
        assert_eq!(
            canonical_case(written).as_deref(),
            Ok(canonical),
            "{written:?}"
        );
        assert_eq!(
            canonical_case(canonical).as_deref(),
            Ok(canonical),
            "{canonical:?} must be a fixed point"
        );
    }
}

/// The private-use families downstream projects publish in volume must be
/// neither refused nor mangled by normalization.
///
/// A private-use subtag is case-insensitive and carries no title-case rule at
/// any length, so the four-character `latn` in `x-gmeow-chinese-latn` must NOT
/// become `Latn`: it is not a script subtag and never was. The over-long
/// members need [`Profile::Rfc5646PrivateUseRelaxed`] to be admitted at all —
/// that is the pre-existing §2.1 ceiling, not something normalization decides —
/// and under it they come back byte-identical.
#[test]
fn canonical_case_leaves_the_published_private_use_families_alone() {
    // Within the §2.1 private-use ceiling: unchanged under the default profile.
    for tag in [
        "x-gmeow-english",
        "x-gmeow-chinese-latn",
        "x-purrdf-english",
        "de-CH-x-phonebk",
    ] {
        assert_eq!(
            canonical_case(tag).as_deref(),
            Ok(tag),
            "{tag:?} must survive normalization byte-identical"
        );
    }

    // Over the ceiling: refused by §2.1 exactly as before, and unchanged under
    // the profile that admits them. `x-gmeow-norwegiannynorsk` is the
    // sixteen-character case.
    for tag in [
        "x-gmeow-norwegiannynorsk",
        "x-gmeow-westernfrisian",
        "x-purrdf-afrikaans",
        "x-purrdf-norwegiannynorsk",
    ] {
        assert_eq!(
            canonical_case(tag),
            Err(LanguageTagError::SubtagLengthOverEight),
            "{tag:?} is over the §2.1 private-use ceiling, as it always was"
        );
        for profile in [
            Profile::Rfc5646PrivateUseRelaxed,
            Profile::ConcreteSyntaxLangtag,
        ] {
            assert_eq!(
                canonical_case_with(tag, profile).as_deref(),
                Ok(tag),
                "{tag:?} must survive normalization byte-identical under {profile:?}"
            );
        }
    }

    // Mixed case inside the private-use space folds down, and only down: no
    // subtag there acquires a capital.
    assert_eq!(
        canonical_case("X-GMEOW-CHINESE-LATN").as_deref(),
        Ok("x-gmeow-chinese-latn")
    );
    assert_eq!(
        canonical_case("DE-ch-X-PHONEBK").as_deref(),
        Ok("de-CH-x-phonebk")
    );
}

/// The tags the module pins as must-keep-working end to end, through every
/// surface this crate offers: the judgement, the decomposition, the owning
/// form, and the normalizer.
#[test]
fn the_pinned_tags_survive_every_surface() {
    // (tag, the narrowest profile that accepts it)
    let pinned: &[(&str, Profile)] = &[
        ("en", Profile::Rfc5646),
        ("en-US", Profile::Rfc5646),
        ("zh-Hans-CN", Profile::Rfc5646),
        ("de-CH-x-phonebk", Profile::Rfc5646),
        ("i-enochian", Profile::Rfc5646),
        ("x-gmeow-english", Profile::Rfc5646),
        ("x-gmeow-chinese-latn", Profile::Rfc5646),
        ("x-purrdf-afrikaans", Profile::Rfc5646PrivateUseRelaxed),
        (
            "x-gmeow-norwegiannynorsk",
            Profile::Rfc5646PrivateUseRelaxed,
        ),
    ];
    for (tag, profile) in pinned {
        assert!(is_well_formed_with(tag, *profile), "{tag:?}");
        let parsed = parse_with(tag, *profile).expect("accepted");
        assert_eq!(parsed.as_str(), *tag, "the input is kept verbatim");
        assert!(parsed.is_canonical_case(), "{tag:?} is already canonical");
        assert_eq!(parsed.canonical_case(), **tag, "{tag:?}");

        let owned = LanguageTagBuf::parse_with(tag, *profile).expect("accepted");
        assert_eq!(owned.as_language_tag(), parsed, "{tag:?}");
        assert_eq!(owned.as_str(), *tag);
        assert_eq!(owned.into_canonical_case().as_str(), *tag, "{tag:?}");
    }
}

/// The two grouped sections a caller previously had to re-split by hand.
#[test]
fn grouped_sections_are_reported_rather_than_left_to_the_caller() {
    let tag = parse("de-DE-u-co-phonebk-t-en-a-myext-x-priv-ate").expect("well-formed");
    assert_eq!(
        tag.extensions_by_singleton()
            .map(|extension| (extension.singleton(), extension.as_str()))
            .collect::<Vec<_>>(),
        [('u', "u-co-phonebk"), ('t', "t-en"), ('a', "a-myext"),]
    );
    assert_eq!(
        tag.extension('u')
            .expect("a `u` extension")
            .subtags()
            .collect::<Vec<_>>(),
        ["co", "phonebk"]
    );
    assert_eq!(
        tag.private_use_subtags().collect::<Vec<_>>(),
        ["priv", "ate"]
    );

    // A tag with neither section reports neither, rather than an empty string.
    let plain = parse("en-US").expect("well-formed");
    assert_eq!(plain.extensions_by_singleton().count(), 0);
    assert_eq!(plain.private_use_subtags().count(), 0);
}

/// Normalization must not invent structure the grammar did not find.
///
/// §2.1.1 assigns case per production: lower for the language, upper for a
/// region, title for a script. A `ConcreteSyntaxOnly` tag matched the concrete
/// syntaxes' `LANGTAG` terminal and no RFC 5646 alternative, so it has no
/// region and no script — casing it by subtag *position* would rewrite
/// `en-fr-jura` to `en-FR-Jura`, asserting a decomposition the parser
/// explicitly failed to find. These tags appear in approved W3C corpora, so
/// that rewrite would corrupt real data.
///
/// Both directions are asserted: the terminal-only tags come back untouched,
/// and tags the same profile *can* decompose still normalize.
#[test]
fn canonical_case_leaves_terminal_only_tags_alone() {
    for tag in ["en-fr-jura", "fr-be-fbcl", "EN-FR-JURA", "cantbethislong"] {
        let parsed = parse_with(tag, Profile::ConcreteSyntaxLangtag).expect("terminal accepts it");
        assert_eq!(
            parsed.form(),
            TagForm::ConcreteSyntaxOnly,
            "{tag:?} should have no RFC 5646 reading"
        );
        assert_eq!(
            canonical_case_with(tag, Profile::ConcreteSyntaxLangtag).as_deref(),
            Ok(tag),
            "{tag:?} has no productions to case by and must come back unchanged"
        );
        assert!(
            parsed.is_canonical_case(),
            "{tag:?} is its own fixed point, so it is already canonical"
        );
    }

    // The neighbour that must still normalize: same profile, but this one has a
    // §2.1 reading, so the region and script rules do apply.
    for (input, expected) in [
        ("EN-us", "en-US"),
        ("ZH-hant-cn", "zh-Hant-CN"),
        ("X-GMEOW-CHINESE-LATN", "x-gmeow-chinese-latn"),
    ] {
        assert_eq!(
            canonical_case_with(input, Profile::ConcreteSyntaxLangtag).as_deref(),
            Ok(expected),
            "{input:?} decomposes, so §2.1.1 still applies"
        );
    }
}
