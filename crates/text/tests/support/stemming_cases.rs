// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Full English Snowball reference-corpus conformance and input-domain laws.

use purrdf_hash::{blake3, hex};
use purrdf_text::stem::{PROFILE_ID, english, english_in_place};

pub(crate) fn every_official_snowball_english_vector() {
    let vocabulary = include_str!("../stemming_vectors/voc.txt");
    let expected = include_str!("../stemming_vectors/output.txt");
    assert_eq!(
        hex::Lower(blake3::hash(vocabulary.as_bytes()).as_bytes()).to_string(),
        "85ef8ec647dc072a6d2ffe5f9d2fc9ef65f562a88433b5070cc4eed226b582b1"
    );
    assert_eq!(
        hex::Lower(blake3::hash(expected.as_bytes()).as_bytes()).to_string(),
        "f47390102f95053e207fb15b154d6a64297d926f7e0d7fb90ced5e1de1535378"
    );
    assert_eq!(vocabulary.lines().count(), 42_649);
    assert_eq!(expected.lines().count(), 42_649);
    for (line, (input, output)) in vocabulary.lines().zip(expected.lines()).enumerate() {
        // One punctuation-only corpus word changes under raw Snowball. The
        // declared Latin-domain law instead preserves it; the fixture is intact.
        let expected = if input == "'''" {
            assert_eq!(output, "'");
            input
        } else {
            output
        };
        assert_eq!(english(input), expected, "vector {}: {input:?}", line + 1);
        let mut in_place = input.to_owned();
        let capacity = in_place.capacity();
        english_in_place(&mut in_place);
        assert_eq!(
            in_place,
            expected,
            "in-place vector {}: {input:?}",
            line + 1
        );
        assert_eq!(
            capacity,
            in_place.capacity(),
            "allocation grew for {input:?}"
        );
    }
}

pub(crate) fn version_and_new_snowball_exceptions_are_pinned() {
    assert_eq!(PROFILE_ID, "snowball-english-3.1.0-latin-scalars-v2");
    for (input, output) in [
        ("skis", "ski"),
        ("skies", "sky"),
        ("added", "add"),
        ("egging", "egg"),
        ("offing", "off"),
        ("hopping", "hop"),
        ("hoping", "hope"),
        ("evening", "evening"),
        ("evenings", "evening"),
        ("pasted", "paste"),
        ("pasting", "paste"),
        ("past", "past"),
        ("universe", "univers"),
        ("universal", "universal"),
        ("university", "universiti"),
        ("lateral", "lateral"),
        ("later", "later"),
        ("emerge", "emerg"),
        ("emergency", "emergenc"),
        ("organ", "organ"),
        ("organic", "organic"),
        ("organize", "organiz"),
        ("geologist", "geolog"),
        ("geology", "geolog"),
        ("internal", "internal"),
        ("international", "internat"),
        ("internment", "internment"),
        ("succeed", "succeed"),
        ("proceeded", "proceed"),
        ("exceedingly", "exceed"),
        ("dyings", "die"),
        ("lying", "lie"),
        ("tying", "tie"),
        ("vying", "vie"),
        ("herring", "herring"),
        ("'running", "run"),
        ("running's", "run"),
        ("running's'", "run"),
    ] {
        assert_eq!(english(input), output, "{input:?}");
    }
}

pub(crate) fn non_latin_words_and_short_inputs_are_preserved() {
    for input in [
        "",
        "a",
        "as",
        "by",
        "y",
        "'s",
        "''",
        "123",
        "😀",
        "日本running",
        "runnіng",
        "รันนิ่ง",
        "東京大学",
        "中文ing",
        "αrunning",
        "🙂🇨🇦",
    ] {
        assert_eq!(english(input), input, "{input:?}");
        let mut in_place = input.to_owned();
        english_in_place(&mut in_place);
        assert_eq!(in_place, input, "in-place {input:?}");
    }
}

pub(crate) fn latin_domain_uses_scalar_regions_and_preserves_non_ascii_consonants() {
    for (input, expected) in [
        ("cafés", "café"),
        ("naïvely", "naïv"),
        ("caféing", "café"),
        ("øing", "øing"),
        ("cøning", "cøning"),
        ("aérunning", "aérun"),
        ("ésses", "éss"),
        ("éies", "éie"),
        ("aéies", "aéi"),
        ("cafe\u{301}s", "cafe\u{301}"),
        ("123running", "123run"),
        ("ßrunning", "ßrun"),
        ("𐞀running", "𐞀run"),
    ] {
        assert_eq!(english(input), expected, "{input:?}");
    }
}

pub(crate) fn y_is_marked_as_a_consonant_only_in_the_defined_positions() {
    for (input, output) in [
        ("youthful", "youth"),
        ("boy", "boy"),
        ("boys", "boy"),
        ("boyish", "boyish"),
        ("yodeling", "yodel"),
        ("crying", "cri"),
        ("say", "say"),
        ("cry", "cri"),
        ("playing", "play"),
    ] {
        assert_eq!(english(input), output, "{input:?}");
    }
}

pub(crate) fn arbitrarily_long_admitted_tokens_do_not_recurse_or_grow() {
    let prefix = "ay".repeat(65_536);
    let mut input = format!("{prefix}running");
    let capacity = input.capacity();
    english_in_place(&mut input);
    assert_eq!(input, format!("{prefix}run"));
    assert_eq!(input.capacity(), capacity);
}
