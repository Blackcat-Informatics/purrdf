// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The analysis pipeline's observable contract.
//!
//! Every assertion here is an **exact** token vector. Nothing checks that a
//! result "contains" a token or has "at least" so many, because a tokenizer's
//! bugs are almost entirely bugs of surplus and shortfall: a stray empty token,
//! a combining mark split off its base, a grapheme emitted one too few times. A
//! containment assertion cannot see any of those, so it would pass through
//! exactly the changes this suite exists to stop.

use std::borrow::Cow;

use purrdf_text::{Analyzer, Token, unicode_versions};

/// The `(text, position)` pairs of `input`, in order.
fn positioned(input: &str) -> Vec<(String, u32)> {
    let mut out: Vec<Token<'_>> = Vec::new();
    Analyzer::empty_lexicon()
        .analyze(input, &mut out)
        .expect("valid text analysis");
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

    assert_eq!(
        Analyzer::empty_lexicon()
            .terms(precomposed)
            .expect("valid text analysis"),
        vec!["cafe".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms(decomposed)
            .expect("valid text analysis"),
        vec!["cafe".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms(precomposed)
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms(decomposed)
            .expect("valid text analysis")
    );
}

/// The test that fails under `str::to_lowercase`, and the reason this crate
/// implements pinned native full case folding.
///
/// Lowercasing leaves `ß` alone, so it would produce `strasse` from one of
/// these and `straße` from the other: two terms, and a search for either that
/// silently misses the other. Full case folding maps `ß` to `ss`, so both
/// spellings — and the uppercase spelling German itself uses — agree.
#[test]
fn full_case_fold_matches_sharp_s() {
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("STRASSE")
            .expect("valid text analysis"),
        vec!["strasse".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("Straße")
            .expect("valid text analysis"),
        vec!["strasse".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("strasse")
            .expect("valid text analysis"),
        vec!["strasse".to_owned()]
    );

    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("STRASSE")
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("Straße")
            .expect("valid text analysis")
    );
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
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ｒｕｓｔ")
            .expect("valid text analysis"),
        vec!["rust".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ｒｕｓｔ")
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("rust")
            .expect("valid text analysis")
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ＲＵＳＴ")
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("rust")
            .expect("valid text analysis")
    );

    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ﬁle ﬂow")
            .expect("valid text analysis"),
        vec!["file".to_owned(), "flow".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ﬁ")
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("fi")
            .expect("valid text analysis")
    );

    // Compatibility folding also reaches the presentation forms of numbers:
    // a Roman numeral and a circled digit are spellings, not characters of
    // their own.
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("Ⅻ")
            .expect("valid text analysis"),
        vec!["xii".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("①②③")
            .expect("valid text analysis"),
        vec!["123".to_owned()]
    );
}

/// Greek writes its lowercase sigma two ways — `σ` mid-word and `ς` word-final
/// — for the same letter, and case folding unifies them on `σ`.
///
/// The standard scoped accent stage also removes Greek accents. A caller
/// selecting accent preservation keeps those marks while sharing case folding.
#[test]
fn greek_final_sigma_folds_with_medial_sigma() {
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ΣΟΦΟΣ")
            .expect("valid text analysis"),
        vec!["σοφοσ".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("σοφος")
            .expect("valid text analysis"),
        vec!["σοφοσ".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("ΣΟΦΟΣ")
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("σοφος")
            .expect("valid text analysis"),
        "uppercase and lowercase spellings of the same word must agree"
    );

    // The same word spelled with the word-final sigma, which is the spelling
    // Greek actually uses, folds onto the medial letter as well.
    let with_final_sigma = "\u{03C3}\u{03BF}\u{03C6}\u{03BF}\u{03C2}";
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms(with_final_sigma)
            .expect("valid text analysis"),
        vec!["σοφοσ".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms(with_final_sigma)
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("ΣΟΦΟΣ")
            .expect("valid text analysis")
    );

    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("σοφός")
            .expect("valid text analysis"),
        vec!["σοφοσ".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("σοφός")
            .expect("valid text analysis"),
        Analyzer::empty_lexicon()
            .terms("σοφος")
            .expect("valid text analysis"),
        "the default scoped Greek accent fold applies after casing"
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
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("don't")
            .expect("valid text analysis"),
        vec!["don't".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("shelf's don't O'Brien")
            .expect("valid text analysis"),
        vec![
            "shelf's".to_owned(),
            "don't".to_owned(),
            "o'brien".to_owned()
        ]
    );

    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("3.14")
            .expect("valid text analysis"),
        vec!["3.14".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("3.14 and 2,718")
            .expect("valid text analysis"),
        vec!["3.14".to_owned(), "and".to_owned(), "2,718".to_owned()]
    );
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("1st 42 007")
            .expect("valid text analysis"),
        vec!["1st".to_owned(), "42".to_owned(), "007".to_owned()]
    );

    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("state-of-the-art")
            .expect("valid text analysis"),
        vec![
            "state".to_owned(),
            "of".to_owned(),
            "the".to_owned(),
            "art".to_owned()
        ]
    );
}

/// Explicit empty-lexicon analysis emits complete graphemes for unspaced
/// scripts. Character bigrams belong to the separate ranked Han producer.
#[test]
fn unspaced_script_fallback_emits_complete_graphemes() {
    for (input, expected) in [
        ("中文全文検索", vec!["中", "文", "全", "文", "検", "索"]),
        ("全文", vec!["全", "文"]),
        ("中", vec!["中"]),
        ("中 文", vec!["中", "文"]),
        ("中文rust混合", vec!["中", "文", "rust", "混", "合"]),
        (
            "私はサンドイッチを食べます",
            vec![
                "私", "は", "サ", "ン", "ド", "イ", "ッ", "チ", "を", "食", "べ", "ま", "す",
            ],
        ),
        ("한국어 전문 검색", vec!["한국어", "전문", "검색"]),
    ] {
        assert_eq!(
            Analyzer::empty_lexicon()
                .terms(input)
                .expect("valid text analysis"),
            expected,
            "{input:?}"
        );
    }
}

/// Positions are the token's ordinal in the stream: zero-based, consecutive,
/// and with no gap where a grapheme or a dropped punctuation segment sits.
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

    // A grapheme run numbers consecutively before the next lexical word.
    assert_eq!(
        positioned("hello 中文全文 world"),
        vec![
            ("hello".to_owned(), 0),
            ("中".to_owned(), 1),
            ("文".to_owned(), 2),
            ("全".to_owned(), 3),
            ("文".to_owned(), 4),
            ("world".to_owned(), 5)
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
            Analyzer::empty_lexicon()
                .terms(input)
                .expect("valid text analysis"),
            Vec::<String>::new(),
            "{input:?} must produce no tokens"
        );
    }
}

/// The tripwire.
///
/// Tokenization is a function of the Unicode tables it is generated from —
/// case folding, normalization, word-break properties and the alphanumeric
/// predicate — all generated in this crate from the vendored Unicode Character
/// Database in `crates/iri/unicode/`. A regenerated table can therefore change what a literal
/// tokenizes to, which changes the term dictionary, which changes which
/// documents a query retrieves. Nothing about that failure announces itself:
/// the engine still returns rows, just not the same rows, and a ranking that
/// was reproducible stops being so.
///
/// These vectors turn that into a loud failure at the exact place the change
/// enters. They deliberately span scripts that exercise different parts of the
/// tables — Latin case folding, Greek sigma, Cyrillic, right-to-left Arabic and
/// pointed Hebrew, Devanagari with dependent vowel signs, Han graphemes, mixed
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
        ("Straße Größe", &["strasse", "grosse"]),
        ("Ångström", &["angstrom"]),
        // Latin with a canonically decomposed input.
        ("cafe\u{0301} café", &["cafe", "cafe"]),
        // Titlecase digraph and Turkish dotted capital I.
        ("ǅungla", &["dzungla"]),
        ("İstanbul", &["istanbul"]),
        // Greek.
        ("Ελληνικά κείμενο", &["ελληνικα", "κειμενο"]),
        ("ΣΟΦΟΣ", &["σοφοσ"]),
        // Cyrillic.
        ("Привет, мир!", &["привет", "мир"]),
        // Arabic (right-to-left).
        ("مرحبا بالعالم", &["مرحبا", "بالعالم"]),
        // Hebrew with points.
        ("שָׁלוֹם עוֹלָם", &["שָׁלוֹם", "עוֹלָם"]),
        // Devanagari, whose dependent vowel signs must stay with their base.
        ("नमस्ते दुनिया", &["नमस्ते", "दुनिया"]),
        // Han, explicit grapheme fallback.
        ("中文全文検索", &["中", "文", "全", "文", "検", "索"]),
        // Hiragana and Katakana in one run.
        (
            "私はサンドイッチを食べます",
            &[
                "私", "は", "サ", "ン", "ド", "イ", "ッ", "チ", "を", "食", "べ", "ま", "す",
            ],
        ),
        // Hangul, space-delimited.
        ("한국어 전문 검색", &["한국어", "전문", "검색"]),
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
        assert_eq!(
            Analyzer::empty_lexicon()
                .terms(input)
                .expect("valid text analysis"),
            expected,
            "golden vector moved for {input:?}"
        );
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
    let analyzer = Analyzer::empty_lexicon();
    let mut out: Vec<Token<'_>> = Vec::new();

    analyzer
        .analyze("alpha beta gamma delta", &mut out)
        .expect("valid text analysis");
    assert_eq!(
        out.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
        vec!["alpha", "beta", "gamma", "delta"]
    );

    analyzer
        .analyze("epsilon", &mut out)
        .expect("valid text analysis");
    assert_eq!(
        out.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
        vec!["epsilon"],
        "the second call must leave only the second input's tokens"
    );
    assert_eq!(out[0].position, 0, "positions restart with the input");

    analyzer.analyze("", &mut out).expect("valid text analysis");
    assert!(out.is_empty(), "an empty input must empty the buffer");
}

/// Text that survives analysis unchanged is handed back borrowed, and text that
/// does not is not — which is the whole reason the token text is a [`Cow`].
#[test]
fn unchanged_text_is_borrowed_rather_than_copied() {
    let analyzer = Analyzer::empty_lexicon();
    let mut out: Vec<Token<'_>> = Vec::new();

    analyzer
        .analyze("already folded text", &mut out)
        .expect("valid text analysis");
    assert!(
        out.iter().all(|t| matches!(t.text, Cow::Borrowed(_))),
        "text needing no change must not be copied"
    );

    analyzer
        .analyze("Needs Folding", &mut out)
        .expect("valid text analysis");
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
    let analyzer = Analyzer::empty_lexicon();
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
        analyzer
            .analyze_each(input, &mut scratch, |token| {
                assert!(
                    matches!(token.text, Cow::Borrowed(_)),
                    "every token must borrow the scratch buffer, for {input:?}"
                );
                actual.push((token.text.into_owned(), token.position));
            })
            .expect("valid text analysis");
        assert_eq!(actual, expected, "the two forms disagreed on {input:?}");
    }
}

/// The reported versions are read from the tables actually linked in, and all
/// four are reported because they are not obliged to agree.
///
/// A single summary number would have to pick one of them, and picking would
/// hide exactly the case this crate has to survive: a fold table on one Unicode
/// release while the normalization and segmentation tables are on the next.
///
/// The versions are pinned **exactly** rather than merely sanity-checked. An
/// assertion that a major version is non-zero is satisfied by every possible
/// table and therefore says nothing; these numbers decide which literals
/// produce which terms, so the whole term dictionary and both index
/// fingerprints are functions of them, and a change to any of them has to be
/// seen rather than absorbed. Every table is generated from one Unicode
/// version, so all four report it: no table trails another, and none follows
/// the toolchain.
#[test]
fn the_reported_unicode_versions_are_the_pinned_tables() {
    let versions = unicode_versions();
    assert_eq!(
        versions,
        unicode_versions(),
        "the answer must be a constant"
    );

    for (name, version, expected) in [
        ("core", versions.core, "17.0.0"),
        ("normalization", versions.normalization, "17.0.0"),
        ("case folding", versions.case_folding, "17.0.0"),
        ("segmentation", versions.segmentation, "17.0.0"),
    ] {
        assert_eq!(
            version.to_string(),
            expected,
            "the {name} table moved to {version}. That is not a lint failure to silence: raising a \
             table can change which literals fold, decompose or segment into which terms, so the \
             term dictionary, every posting and BOTH index fingerprints change with it. Re-derive \
             this suite's golden token vectors and the fingerprint goldens, confirm the diff is \
             the one the new tables intend, and then move this pin."
        );
    }
}
