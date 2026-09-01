// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The analysis pipeline's observable contract.
//!
//! Every assertion here is an **exact** token vector. Nothing checks that a
//! result "contains" a token or has "at least" so many, because a tokenizer's
//! bugs are almost entirely bugs of surplus and shortfall: a stray empty token,
//! a combining mark split off its base, a bigram emitted one too few times. A
//! containment assertion cannot see any of those, so it would pass through
//! exactly the changes this suite exists to stop.

use std::borrow::Cow;

use pretty_assertions::assert_eq;
use purrdf_text::{Analyzer, Token, unicode_versions};

/// The token texts of `input`, in order.
fn tokens(input: &str) -> Vec<String> {
    let mut out: Vec<Token<'_>> = Vec::new();
    Analyzer::new().analyze(input, &mut out);
    out.into_iter().map(|t| t.text.into_owned()).collect()
}

/// The `(text, position)` pairs of `input`, in order.
fn positioned(input: &str) -> Vec<(String, u32)> {
    let mut out: Vec<Token<'_>> = Vec::new();
    Analyzer::new().analyze(input, &mut out);
    out.into_iter()
        .map(|t| (t.text.into_owned(), t.position))
        .collect()
}

/// A precomposed spelling and a canonically decomposed spelling of the same
/// text are the same text, and must reach the dictionary as one term.
///
/// Nothing in a literal's RDF lexical form records which spelling its author
/// used, so an index that told them apart would answer a query for `café` with
/// only the half of the corpus that happened to be typed the same way.
#[test]
fn nfkc_composed_and_decomposed_forms_fold_identically() {
    let precomposed = "caf\u{00E9}";
    let decomposed = "cafe\u{0301}";
    assert_ne!(
        precomposed, decomposed,
        "the two spellings must differ as byte strings, or this proves nothing"
    );

    assert_eq!(tokens(precomposed), vec!["café".to_owned()]);
    assert_eq!(tokens(decomposed), vec!["café".to_owned()]);
    assert_eq!(tokens(precomposed), tokens(decomposed));
}

/// The test that fails under `str::to_lowercase`, and the reason this crate
/// takes a case-folding dependency at all.
///
/// Lowercasing leaves `ß` alone, so it would produce `strasse` from one of
/// these and `straße` from the other: two terms, and a search for either that
/// silently misses the other. Full case folding maps `ß` to `ss`, so both
/// spellings — and the uppercase spelling German itself uses — agree.
#[test]
fn full_case_fold_matches_sharp_s() {
    assert_eq!(tokens("STRASSE"), vec!["strasse".to_owned()]);
    assert_eq!(tokens("Straße"), vec!["strasse".to_owned()]);
    assert_eq!(tokens("strasse"), vec!["strasse".to_owned()]);

    assert_eq!(tokens("STRASSE"), tokens("Straße"));
    assert_ne!(
        "Straße".to_lowercase(),
        "STRASSE".to_lowercase(),
        "lowercasing must still disagree here, or the fold is not what fixed it"
    );
}

/// Compatibility normalization is what collapses a character's presentation
/// variants onto the character itself.
///
/// A fullwidth `ｒ` and a Latin `r` are the same letter differing only in how a
/// legacy encoding drew it, and `ﬁ` is a typographic ligature of two letters,
/// not a letter. Canonical normalization preserves all of these distinctions by
/// design; only compatibility normalization removes them, and a search index
/// that kept them would fail to retrieve text pasted out of a CJK-locale
/// document or a PDF.
#[test]
fn compatibility_fold_matches_fullwidth_and_ligatures() {
    assert_eq!(tokens("ｒｕｓｔ"), vec!["rust".to_owned()]);
    assert_eq!(tokens("ｒｕｓｔ"), tokens("rust"));
    assert_eq!(tokens("ＲＵＳＴ"), tokens("rust"));

    assert_eq!(
        tokens("ﬁle ﬂow"),
        vec!["file".to_owned(), "flow".to_owned()]
    );
    assert_eq!(tokens("ﬁ"), tokens("fi"));

    // Compatibility folding also reaches the presentation forms of numbers:
    // a Roman numeral and a circled digit are spellings, not characters of
    // their own.
    assert_eq!(tokens("Ⅻ"), vec!["xii".to_owned()]);
    assert_eq!(tokens("①②③"), vec!["123".to_owned()]);
}

/// Greek writes its lowercase sigma two ways — `σ` mid-word and `ς` word-final
/// — for the same letter, and case folding unifies them on `σ`.
///
/// Asserted as observed, including what the fold does *not* do: it is a case
/// operation, not an accent-stripping one, so `σοφός` keeps its acute and stays
/// a different term from `σοφος`. That is the correct behaviour — Greek accents
/// are lexical — and it is pinned here so that a later change to accent handling
/// cannot arrive unannounced.
#[test]
fn greek_final_sigma_folds_with_medial_sigma() {
    assert_eq!(tokens("ΣΟΦΟΣ"), vec!["σοφοσ".to_owned()]);
    assert_eq!(tokens("σοφος"), vec!["σοφοσ".to_owned()]);
    assert_eq!(
        tokens("ΣΟΦΟΣ"),
        tokens("σοφος"),
        "uppercase and lowercase spellings of the same word must agree"
    );

    // The same word spelled with the word-final sigma, which is the spelling
    // Greek actually uses, folds onto the medial letter as well.
    let with_final_sigma = "\u{03C3}\u{03BF}\u{03C6}\u{03BF}\u{03C2}";
    assert_eq!(tokens(with_final_sigma), vec!["σοφοσ".to_owned()]);
    assert_eq!(tokens(with_final_sigma), tokens("ΣΟΦΟΣ"));

    assert_eq!(tokens("σοφός"), vec!["σοφόσ".to_owned()]);
    assert_ne!(
        tokens("σοφός"),
        tokens("σοφος"),
        "case folding must not strip accents"
    );
}

/// Word boundaries are the standard's, not a run of alphanumerics.
///
/// The difference shows up on exactly the characters an ad-hoc tokenizer gets
/// wrong: an apostrophe inside a contraction joins (`don't` is one word), a
/// decimal point inside a number joins (`3.14` is one number and not `3` then
/// `14`), a group separator inside a number joins, and a hyphen between words
/// does not.
#[test]
fn uax29_word_boundaries_over_apostrophes_and_numerals() {
    assert_eq!(tokens("don't"), vec!["don't".to_owned()]);
    assert_eq!(
        tokens("shelf's don't O'Brien"),
        vec![
            "shelf's".to_owned(),
            "don't".to_owned(),
            "o'brien".to_owned()
        ]
    );

    assert_eq!(tokens("3.14"), vec!["3.14".to_owned()]);
    assert_eq!(
        tokens("3.14 and 2,718"),
        vec!["3.14".to_owned(), "and".to_owned(), "2,718".to_owned()]
    );
    assert_eq!(
        tokens("1st 42 007"),
        vec!["1st".to_owned(), "42".to_owned(), "007".to_owned()]
    );

    assert_eq!(
        tokens("state-of-the-art"),
        vec![
            "state".to_owned(),
            "of".to_owned(),
            "the".to_owned(),
            "art".to_owned()
        ]
    );
}

/// The CJK model this crate documents, asserted rather than assumed.
///
/// Unspaced Han and Hiragana arrive from `UAX #29` as one token per character,
/// so bigrams are formed by rejoining **adjacent** tokens; a space or an
/// intervening Latin word ends a run, and a run of one character is emitted
/// whole so that a single ideograph stays retrievable.
#[test]
fn cjk_tokenization_matches_the_documented_model() {
    // Six adjacent ideographs become five overlapping bigrams.
    assert_eq!(
        tokens("中文全文検索"),
        vec![
            "中文".to_owned(),
            "文全".to_owned(),
            "全文".to_owned(),
            "文検".to_owned(),
            "検索".to_owned()
        ]
    );

    // A phrase query is expressible: the needle's bigram is one of the
    // document's.
    assert_eq!(tokens("全文"), vec!["全文".to_owned()]);

    // A lone ideograph has no bigram to form and survives whole.
    assert_eq!(tokens("中"), vec!["中".to_owned()]);

    // A space ends a run, so two separated ideographs are two whole tokens and
    // NOT the bigram the unspaced spelling produces.
    assert_eq!(tokens("中 文"), vec!["中".to_owned(), "文".to_owned()]);
    assert_ne!(tokens("中 文"), tokens("中文"));

    // A Latin word ends a run in both directions, and is itself untouched by
    // bigram expansion.
    assert_eq!(
        tokens("中文rust混合"),
        vec!["中文".to_owned(), "rust".to_owned(), "混合".to_owned()]
    );

    // Mixed Japanese: Hiragana arrives one character at a time and Katakana
    // arrives as a whole run, but both are CJK and adjacent, so the whole
    // sentence is one run and bigrams cross the script change.
    assert_eq!(
        tokens("私はサンドイッチを食べます"),
        vec![
            "私は".to_owned(),
            "はサ".to_owned(),
            "サン".to_owned(),
            "ンド".to_owned(),
            "ドイ".to_owned(),
            "イッ".to_owned(),
            "ッチ".to_owned(),
            "チを".to_owned(),
            "を食".to_owned(),
            "食べ".to_owned(),
            "べま".to_owned(),
            "ます".to_owned()
        ]
    );

    // Korean is written with spaces, so each space-delimited word is its own
    // run and is bigrammed within itself.
    assert_eq!(
        tokens("한국어 전문 검색"),
        vec![
            "한국".to_owned(),
            "국어".to_owned(),
            "전문".to_owned(),
            "검색".to_owned()
        ]
    );
}

/// Positions are the token's ordinal in the stream: zero-based, consecutive,
/// and with no gap where a bigram or a dropped punctuation segment sits.
///
/// A later stage emits term occurrences at these numbers so that phrase and
/// proximity matching are expressible in SPARQL as `FILTER(?p2 = ?p1 + 1)`, and
/// that predicate is only true of adjacent terms if the numbering has no holes
/// in it.
#[test]
fn positions_are_consecutive_and_zero_based() {
    assert_eq!(
        positioned("the quick brown fox"),
        vec![
            ("the".to_owned(), 0),
            ("quick".to_owned(), 1),
            ("brown".to_owned(), 2),
            ("fox".to_owned(), 3)
        ]
    );

    // Punctuation is dropped without leaving a hole behind it.
    assert_eq!(
        positioned("the, quick! brown -- fox"),
        vec![
            ("the".to_owned(), 0),
            ("quick".to_owned(), 1),
            ("brown".to_owned(), 2),
            ("fox".to_owned(), 3)
        ]
    );

    // A bigram run numbers consecutively and hands the next number back to the
    // word that follows it.
    assert_eq!(
        positioned("hello 中文全文 world"),
        vec![
            ("hello".to_owned(), 0),
            ("中文".to_owned(), 1),
            ("文全".to_owned(), 2),
            ("全文".to_owned(), 3),
            ("world".to_owned(), 4)
        ]
    );

    for input in [
        "a b c",
        "中文全文検索",
        "hello 中文 world 検索",
        "私はサンドイッチを食べます",
        "한국어 전문 검색",
    ] {
        let positions: Vec<u32> = positioned(input).into_iter().map(|(_, p)| p).collect();
        let expected: Vec<u32> =
            (0..u32::try_from(positions.len()).expect("short input")).collect();
        assert_eq!(positions, expected, "positions broke for {input:?}");
    }
}

/// Input with no words yields no tokens at all — not one empty token.
///
/// This is load-bearing beyond tidiness. A later stage divides by the corpus's
/// average document length, and a document that contributed a phantom token
/// would corrupt that average, while a document of genuinely zero length must
/// be counted as zero rather than smuggled in as one. Both directions are
/// wrong in the same silent way: the ranking is still produced, just no longer
/// the ranking the formula defines.
#[test]
fn an_empty_or_punctuation_only_input_yields_no_tokens() {
    for input in ["", " ", "   ", "\t\n", "!!! ... ???", "—— :: ;;", "()[]{}"] {
        assert_eq!(
            tokens(input),
            Vec::<String>::new(),
            "{input:?} must produce no tokens"
        );
    }
}

/// The tripwire.
///
/// Tokenization is a function of four independently versioned Unicode tables —
/// the standard library's, `unicode-normalization`'s, `caseless`'s and
/// `unicode-segmentation`'s — and none of them is under this repository's
/// control. A toolchain bump or a dependency bump can therefore change what a
/// literal tokenizes to, which changes the term dictionary, which changes which
/// documents a query retrieves. Nothing about that failure announces itself:
/// the engine still returns rows, just not the same rows, and a ranking that
/// was reproducible stops being so.
///
/// These vectors turn that into a loud failure at the exact place the change
/// enters. They deliberately span scripts that exercise different parts of the
/// tables — Latin case folding, Greek sigma, Cyrillic, right-to-left Arabic and
/// pointed Hebrew, Devanagari with dependent vowel signs, Han bigrams, mixed
/// Kana, Hangul, numerals, compatibility presentation forms and punctuation —
/// so a change confined to any one of them still lands on an assertion.
///
/// A failure here is not a test to update. It is a report that the term
/// dictionary this crate would build has changed, and the change has to be
/// understood, deliberately accepted, and reflected in the recorded Unicode
/// versions before the vector is rewritten.
#[test]
fn golden_token_vectors_pin_the_unicode_tables() {
    let golden: &[(&str, &[&str])] = &[
        // Latin, with case folding and a sharp s.
        ("The Quick Brown Fox", &["the", "quick", "brown", "fox"]),
        ("Straße Größe", &["strasse", "grösse"]),
        ("Ångström", &["ångström"]),
        // Latin with a canonically decomposed input.
        ("cafe\u{0301} café", &["café", "café"]),
        // Titlecase digraph and Turkish dotted capital I.
        ("ǅungla", &["džungla"]),
        ("İstanbul", &["i\u{0307}stanbul"]),
        // Greek.
        ("Ελληνικά κείμενο", &["ελληνικά", "κείμενο"]),
        ("ΣΟΦΟΣ", &["σοφοσ"]),
        // Cyrillic.
        ("Привет, мир!", &["привет", "мир"]),
        // Arabic (right-to-left).
        ("مرحبا بالعالم", &["مرحبا", "بالعالم"]),
        // Hebrew with points.
        ("שָׁלוֹם עוֹלָם", &["שָׁלוֹם", "עוֹלָם"]),
        // Devanagari, whose dependent vowel signs must stay with their base.
        ("नमस्ते दुनिया", &["नमस्ते", "दुनिया"]),
        // Han, bigrammed.
        ("中文全文検索", &["中文", "文全", "全文", "文検", "検索"]),
        // Hiragana and Katakana in one run.
        (
            "私はサンドイッチを食べます",
            &[
                "私は", "はサ", "サン", "ンド", "ドイ", "イッ", "ッチ", "チを", "を食", "食べ",
                "べま", "ます",
            ],
        ),
        // Hangul, space-delimited.
        ("한국어 전문 검색", &["한국", "국어", "전문", "검색"]),
        // Digits and number-internal punctuation.
        ("3.14 and 2,718", &["3.14", "and", "2,718"]),
        ("1st 42 007", &["1st", "42", "007"]),
        // Punctuation that separates rather than joins.
        ("state-of-the-art", &["state", "of", "the", "art"]),
        // Punctuation only.
        ("!!! ... ???", &[]),
        // Compatibility presentation forms.
        ("ｒｕｓｔ ＡＢＣ１２３", &["rust", "abc123"]),
        ("ﬁle ﬂow", &["file", "flow"]),
        ("Ⅻ ①②③", &["xii", "123"]),
    ];

    for (input, expected) in golden {
        let expected: Vec<String> = expected.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(tokens(input), expected, "golden vector moved for {input:?}");
    }
}

/// One token vector serves a whole corpus: the analyzer clears it, so the
/// second call's contents are the second input's tokens and nothing else.
///
/// This is what makes indexing allocate a token vector once rather than once
/// per literal, and it only works if the clearing is the analyzer's job. If it
/// were the caller's, one forgotten `clear()` would append every document's
/// terms to the previous document's — an index that is wrong rather than one
/// that fails.
#[test]
fn analyze_reuses_the_caller_buffer() {
    let analyzer = Analyzer::new();
    let mut out: Vec<Token<'_>> = Vec::new();

    analyzer.analyze("alpha beta gamma delta", &mut out);
    assert_eq!(
        out.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
        vec!["alpha", "beta", "gamma", "delta"]
    );

    analyzer.analyze("epsilon", &mut out);
    assert_eq!(
        out.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
        vec!["epsilon"],
        "the second call must leave only the second input's tokens"
    );
    assert_eq!(out[0].position, 0, "positions restart with the input");

    analyzer.analyze("", &mut out);
    assert!(out.is_empty(), "an empty input must empty the buffer");
}

/// Text that survives analysis unchanged is handed back borrowed, and text that
/// does not is not — which is the whole reason the token text is a [`Cow`].
#[test]
fn unchanged_text_is_borrowed_rather_than_copied() {
    let analyzer = Analyzer::new();
    let mut out: Vec<Token<'_>> = Vec::new();

    analyzer.analyze("already folded text", &mut out);
    assert!(
        out.iter().all(|t| matches!(t.text, Cow::Borrowed(_))),
        "text needing no change must not be copied"
    );

    analyzer.analyze("Needs Folding", &mut out);
    assert!(
        out.iter().all(|t| matches!(t.text, Cow::Owned(_))),
        "text the fold rewrote cannot borrow the caller's string"
    );
}

/// The scratch-buffer form produces exactly the same tokens as the owning form
/// while borrowing every one of them, and its buffer survives a whole corpus.
///
/// That reuse is the point: one `String` for the run, no `String` per token and
/// none per literal. It is asserted by driving the loop with a single buffer,
/// because "the buffer is reusable" is precisely the property the token vector
/// shape could not offer — a vector of borrowed tokens pins the borrow for as
/// long as its type exists, so the loop would not compile at all.
#[test]
fn the_scratch_form_agrees_and_borrows_every_token() {
    let analyzer = Analyzer::new();
    let mut scratch = String::new();

    for input in [
        "The Quick Brown Fox",
        "Straße",
        "中文全文検索",
        "私はサンドイッチを食べます",
        "!!! ... ???",
        "",
    ] {
        let expected = positioned(input);
        let mut actual: Vec<(String, u32)> = Vec::new();
        analyzer.analyze_each(input, &mut scratch, |token| {
            assert!(
                matches!(token.text, Cow::Borrowed(_)),
                "every token must borrow the scratch buffer, for {input:?}"
            );
            actual.push((token.text.into_owned(), token.position));
        });
        assert_eq!(actual, expected, "the two forms disagreed on {input:?}");
    }
}

/// The reported versions are read from the tables actually linked in, and all
/// four are reported because they are not obliged to agree.
///
/// A single summary number would have to pick one of them, and picking would
/// hide exactly the case this crate has to survive: a fold table on one Unicode
/// release while the normalization and segmentation tables are on the next.
#[test]
fn the_reported_unicode_versions_are_the_linked_tables() {
    let versions = unicode_versions();
    assert_eq!(
        versions,
        unicode_versions(),
        "the answer must be a constant"
    );

    for (name, version) in [
        ("core", versions.core),
        ("normalization", versions.normalization),
        ("case folding", versions.case_folding),
        ("segmentation", versions.segmentation),
    ] {
        assert!(
            version.major > 0,
            "the {name} table reported no major version"
        );
        assert_eq!(
            version.to_string(),
            format!("{}.{}.{}", version.major, version.minor, version.patch),
            "the {name} version renders as major.minor.patch"
        );
    }
}
