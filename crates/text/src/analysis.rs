// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The analysis pipeline: the one tokenizer both sides of a search agree on.
//!
//! Every literal that enters the index and every needle that enters a query is
//! put through [`Analyzer`], and through nothing else. That is not a tidiness
//! preference. A retrieval engine matches a query's terms against an index's
//! terms by equality, so if the two sides tokenize differently — a different
//! normal form, a different case rule, a different word boundary — the
//! comparison is between two vocabularies that merely resemble each other, and
//! the failure mode is silence: correct-looking queries that return nothing,
//! with nothing anywhere reporting an error. One pipeline, used identically at
//! both ends, is what makes a match mean what it says.
//!
//! # Compatibility case folding, not lowercasing
//!
//! The pipeline folds and normalizes before it segments. The choice of *which*
//! fold and *which* normal form is the difference between an index that
//! retrieves and one that quietly does not:
//!
//! * `str::to_lowercase` is **lowercasing**, which is not case folding. It
//!   leaves `ß` as `ß`, so `STRASSE` lowercases to `strasse` while `Straße`
//!   lowercases to `straße`: two terms, no match, no diagnostic. Full case
//!   folding (Unicode `CaseFolding.txt`, the `C` and `F` mappings) maps both to
//!   `strasse`.
//! * NFC alone is **canonical** normalization, which by construction preserves
//!   compatibility distinctions. Fullwidth `ｒｕｓｔ` stays distinct from
//!   `rust`, and the ligature `ﬁ` stays distinct from `fi`. Compatibility
//!   normalization is what collapses them.
//!
//! Compatibility normalization plus full case folding is the Unicode standard's
//! own answer for search and identifier matching (`UAX #31`, `UTS #18`), and it
//! is what this module implements.
//!
//! # The exact fold, spelled out
//!
//! The `caseless` crate exposes **no function that produces** the compatibility
//! caseless form: its `compatibility_caseless_match_str` is a *predicate* over
//! two strings, and its `default_case_fold_str` performs only the fold, with no
//! normalization at all. A tokenizer needs the form, not the verdict, so this
//! module composes the form from the same two pieces the predicate is built
//! from:
//!
//! 1. [`caseless::Caseless::default_case_fold`] — the full case fold, the
//!    identical iterator adaptor that backs `default_case_fold_str` and
//!    `compatibility_caseless_match_str`; and
//! 2. `unicode_normalization`'s `nfd` / `nfkd` / `nfc`.
//!
//! composed in the order the Unicode Standard defines compatibility caseless
//! matching in (`UAX #21`, "Default Case Algorithms"), which is also the exact
//! order `compatibility_caseless_match` compares under:
//!
//! ```text
//! NFKD( fold( NFKD( fold( NFD( x ) ) ) ) )
//! ```
//!
//! and then one final **NFC** recomposition, so a token is a composed string
//! rather than a base character trailed by loose combining marks.
//!
//! That last step cannot merge two terms the standard keeps apart. NFKD output
//! is already in canonical decomposed form, and NFC restricted to canonically
//! decomposed input is injective — decomposing an NFC result returns the input
//! it was composed from — so appending NFC preserves the equivalence class
//! exactly: two strings analyze to the same token text if and only if they are
//! compatibility caseless matches of one another. [`Analyzer`] is therefore
//! interchangeable with `caseless`'s predicate, and a test asserts that.
//!
//! # Fold first, then segment
//!
//! Normalization runs over the whole input **before** segmentation, never after
//! and never per token. A canonically decomposed `é` is `e` followed by
//! `U+0301 COMBINING ACUTE ACCENT`, and a lone combining mark is not
//! `Alphabetic`; the word-boundary rules of `UAX #29` would split it off the
//! base character it modifies, so the decomposed spelling of a word would
//! segment into different tokens than the precomposed spelling of the same
//! word. Normalizing first removes the question.
//!
//! # How this pipeline segments CJK, as measured
//!
//! `UAX #29` assigns Han ideographs and Hiragana the `Word_Break` property
//! value `Other`, and rule WB999 breaks between any pair of characters not
//! joined by an earlier rule. The observable consequence, confirmed against
//! `unicode-segmentation` and asserted by this crate's tests rather than
//! assumed:
//!
//! * **Han** segments to one token per ideograph — `中文全文検索` yields
//!   `中`, `文`, `全`, `文`, `検`, `索`, not one token for the phrase.
//! * **Hiragana** likewise segments one token per character.
//! * **Katakana** does not: it carries `Word_Break = Katakana` and rule WB13
//!   keeps a katakana run together, so `サンドイッチ` is a single token.
//! * **Hangul syllables** are `ALetter`, so Korean — which is written with
//!   spaces — segments into whole words.
//! * None of this is discarded by `unicode_words`. Its filter keeps any segment
//!   containing an alphanumeric character, and Han, Kana and Hangul are all
//!   `char::is_alphanumeric`.
//!
//! Because unspaced CJK arrives as a stream of one-character tokens, expanding
//! *each token* into bigrams would do nothing at all: every token is already a
//! single character. Bigrams have to be formed **across adjacent tokens**, so
//! this module segments with `unicode_word_indices` rather than `unicode_words`
//! — the same segmentation, but carrying the byte offsets that say whether two
//! tokens touched in the source. A maximal run of adjacent all-CJK tokens is
//! rejoined and expanded into overlapping character bigrams; `中文` and `中 文`
//! therefore analyze differently, which is the point.
//!
//! Bigrams are the standard answer to retrieval over a script with no spaces.
//! Indexing unigrams would reduce a phrase query to a bag of characters —
//! `全文` would match any document containing `全` and `文` anywhere — while a
//! dictionary segmenter would need a dictionary, which is per-language data
//! this crate does not have and would make results depend on its vintage. A
//! run of exactly one character has no bigram to form and is emitted whole, so
//! a single ideograph is still retrievable.
//!
//! # Positions
//!
//! Every emitted token carries its zero-based ordinal in the stream, bigrams
//! included and consecutively numbered. Positions are what make phrase and
//! proximity matching expressible downstream: two term occurrences bound to
//! `?p1` and `?p2` are adjacent exactly when `FILTER(?p2 = ?p1 + 1)` holds.

use std::borrow::Cow;
use std::fmt;

use caseless::Caseless as _;
use unicode_normalization::UnicodeNormalization as _;
use unicode_segmentation::UnicodeSegmentation as _;

/// One analyzed token: its text and its position in the token stream.
///
/// The text is a [`Cow`] because the analysis form of an input is usually, but
/// not always, byte-identical to the input. When it is, tokens borrow the
/// caller's string and nothing is allocated; when it is not, the normalized
/// text has to live somewhere, and through [`Analyzer::analyze`] the only owner
/// available is the token. [`Analyzer::analyze_each`] is the form that supplies
/// a buffer instead and keeps every token borrowed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    /// The token's text, in the analyzer's compatibility caseless form.
    pub text: Cow<'a, str>,
    /// The token's zero-based ordinal in the stream it was produced from.
    ///
    /// Consecutive for every emitted token, CJK bigrams included, so adjacency
    /// in the source is adjacency in this number.
    pub position: u32,
}

/// A Unicode table version, as `major.minor.patch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnicodeVersion {
    /// The major version — `17` in `17.0.0`.
    pub major: u64,
    /// The minor version — the first `0` in `17.0.0`.
    pub minor: u64,
    /// The patch version — the second `0` in `17.0.0`.
    pub patch: u64,
}

impl From<(u8, u8, u8)> for UnicodeVersion {
    /// Widen the `(u8, u8, u8)` shape `unicode_normalization` and the standard
    /// library publish their table versions in.
    fn from((major, minor, patch): (u8, u8, u8)) -> Self {
        Self {
            major: u64::from(major),
            minor: u64::from(minor),
            patch: u64::from(patch),
        }
    }
}

impl From<(u64, u64, u64)> for UnicodeVersion {
    /// Adopt the `(u64, u64, u64)` shape `unicode_segmentation` and `caseless`
    /// publish their table versions in.
    fn from((major, minor, patch): (u64, u64, u64)) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for UnicodeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The Unicode table versions this analyzer's output depends on.
///
/// Tokenization is not one table but four, each versioned independently by
/// whoever ships it, and this crate's headline promise is that the same corpus
/// and the same query produce the same ranking. A term dictionary is a function
/// of these tables: raise any of them and a literal may fold, decompose or
/// segment differently, the index's vocabulary changes, and queries that used
/// to match stop matching — silently, because nothing about a retrieval that
/// returns fewer rows announces itself as wrong.
///
/// Recording these alongside an index turns that into something detectable. A
/// later stage folds them into the index fingerprint, so an index built under
/// one set of tables is distinguishable from one built under another without
/// comparing a single term.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnicodeVersions {
    /// The standard library's own tables — `char::UNICODE_VERSION`.
    ///
    /// These back `char::is_alphanumeric`, which `unicode_words` filters
    /// segments with, so they are genuinely part of the pipeline's output and
    /// not merely ambient.
    pub core: UnicodeVersion,
    /// `unicode_normalization`'s tables: the NFD, NFKD and NFC steps.
    pub normalization: UnicodeVersion,
    /// `caseless`'s `CaseFolding.txt` tables: the full case fold.
    pub case_folding: UnicodeVersion,
    /// `unicode_segmentation`'s tables: the `UAX #29` word boundaries.
    pub segmentation: UnicodeVersion,
}

/// The Unicode table versions [`Analyzer`] currently resolves against.
///
/// Read from the dependencies themselves rather than restated here, so the
/// answer cannot drift away from the tables actually linked in.
///
/// These four are not obliged to agree, and in practice they do not: a case
/// folding table can trail the normalization and segmentation tables by a whole
/// Unicode release, because they are separate crates on separate schedules.
/// That is precisely why all four are reported rather than one summary number.
///
/// # The skew that is actually present, and its measured extent
///
/// As linked today the tables are **not** level:
///
/// | table | crate | version |
/// |---|---|---|
/// | `core` | `std` (`char::UNICODE_VERSION`) | 17.0.0 |
/// | `normalization` | `unicode-normalization` | 17.0.0 |
/// | `segmentation` | `unicode-segmentation` | 17.0.0 |
/// | **`case_folding`** | **`caseless`** | **16.0.0** |
///
/// The fold table trails the other three by one Unicode release, and it cannot
/// be levelled by upgrading: `caseless` 0.2.2 is the newest version published,
/// its `CaseFolding.txt` tables are at 16.0.0, and it is the only crate in this
/// workspace that implements the full (`C` + `F`) case fold the compatibility
/// caseless form is defined in terms of. Substituting `str::to_lowercase` is
/// not an option for the reason this module opens with — lowercasing is not
/// folding, and `STRASSE`/`Straße` would stop matching. So the skew is
/// **carried deliberately**, and the job here is to state exactly what it costs
/// rather than to leave it as an unquantified caveat.
///
/// The cost is measurable and is measured, by
/// `the_case_folding_skew_is_confined_to_where_it_is_measured` in this crate's
/// test suite. The characters on which the 16.0.0 fold table disagrees with the
/// 17.0.0 case-mapping tables — that is, every `c` for which `fold(c)`,
/// `fold(lowercase(c))` and `fold(uppercase(c))` are not all the same string —
/// are exactly these 57 code points and no others:
///
/// * `U+0131` LATIN SMALL LETTER DOTLESS I. **Not** a skew: Unicode excludes it
///   from the default fold on purpose (its case mappings are the Turkic `T`
///   status, which `C` + `F` folding does not apply), so every version of every
///   conforming fold table behaves this way.
/// * `U+A7CE..=U+A7CF` and `U+A7D2..=U+A7D5` — six Latin Extended-D letters.
/// * `U+16EA0..=U+16EB8` and `U+16EBB..=U+16ED3` — fifty Beria Erfe letters.
///
/// The 56 real ones are all characters whose *cased partner* the 17.0.0 tables
/// know about and the 16.0.0 fold table does not. An uppercase one of them
/// therefore indexes and queries as itself rather than folding, so it matches
/// its own spelling and not its lowercase partner. Nothing else is affected:
/// the test asserts that every ASCII, Latin-1, Latin Extended-A, Greek,
/// Cyrillic, Hebrew, Arabic, Hiragana, Katakana, Han and Hangul code point
/// folds consistently, which is to say that no corpus written before Unicode
/// 17.0 existed can contain a character this skew touches.
///
/// When `caseless` does ship 17.0.0 tables, that test fails — deliberately.
/// Raising the fold table changes which literals produce which terms, which
/// changes the term dictionary and both fingerprints, so it is a change that
/// has to be seen and its goldens re-derived rather than absorbed silently.
///
/// # Versions pin vintage, not contents
///
/// A dependency could ship a corrected mapping without moving its version. The
/// golden token-vector test in this crate's test suite is what catches that —
/// it asserts exact token vectors across a spread of scripts, so a changed
/// mapping fails a test rather than rewriting the term dictionary in silence.
pub fn unicode_versions() -> UnicodeVersions {
    UnicodeVersions {
        core: char::UNICODE_VERSION.into(),
        normalization: unicode_normalization::UNICODE_VERSION.into(),
        case_folding: caseless::UNICODE_VERSION.into(),
        segmentation: unicode_segmentation::UNICODE_VERSION.into(),
    }
}

/// The analysis pipeline described in this module's documentation.
///
/// Config-free and zero-sized: there are no options, because an option here is
/// a way for the index side and the query side to disagree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Analyzer;

impl Analyzer {
    /// The analyzer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Analyze `input`, replacing the contents of `out`.
    ///
    /// `out` is cleared first and is the caller's to reuse across every literal
    /// in a corpus, so the token vector itself is allocated once rather than
    /// once per document.
    ///
    /// Tokens borrow `input` whenever the analysis form leaves it unchanged,
    /// which covers the common case of text that is already lowercase and
    /// already composed. When the form differs — any uppercase letter is enough
    /// — the normalized text has to live somewhere, and the only owner
    /// available through this signature is the token itself, so tokens are
    /// owned in that case. [`Analyzer::analyze_each`] is the form that allocates
    /// nothing at all; prefer it when indexing a corpus.
    pub fn analyze<'a>(&self, input: &'a str, out: &mut Vec<Token<'a>>) {
        out.clear();
        if is_in_analysis_form(input) {
            segment_each(input, |text, position| {
                out.push(Token {
                    text: Cow::Borrowed(text),
                    position,
                });
            });
        } else {
            let normalized: String = analysis_form_chars(input).collect();
            segment_each(&normalized, |text, position| {
                out.push(Token {
                    text: Cow::Owned(text.to_owned()),
                    position,
                });
            });
        }
    }

    /// Analyze `input` through `scratch`, handing each token to `sink`.
    ///
    /// The allocation-free form, and the one to drive a corpus with. `scratch`
    /// is cleared, grows once to the length of the longest analysis form seen
    /// and is then reused for every literal after it; each token is a borrowed
    /// slice of it. Indexing a million literals therefore allocates neither a
    /// `String` per token nor a `String` per literal.
    ///
    /// The tokens are delivered one at a time rather than collected into a
    /// vector because they borrow `scratch`, and a vector of them would pin that
    /// borrow for as long as the vector's own type exists — which is what stops
    /// one vector from being reused across a loop that re-fills the same
    /// buffer. An indexer consumes a token the moment it has it (intern the
    /// term, append a posting) and needs no such vector, so handing tokens over
    /// as they are produced costs it nothing and keeps the buffer reusable.
    pub fn analyze_each<F>(&self, input: &str, scratch: &mut String, mut sink: F)
    where
        F: FnMut(Token<'_>),
    {
        scratch.clear();
        scratch.extend(analysis_form_chars(input));
        segment_each(scratch, |text, position| {
            sink(Token {
                text: Cow::Borrowed(text),
                position,
            });
        });
    }

    /// The analysis form of `input` — folded and normalized, but not segmented.
    ///
    /// Exposed because the fold is half of what makes a match a match: a caller
    /// comparing a stored term against a needle, or explaining to a user why
    /// two strings did or did not match, needs the same form the tokenizer saw.
    #[must_use]
    pub fn analysis_form(&self, input: &str) -> String {
        analysis_form_chars(input).collect()
    }
}

/// Hangul Jamo — conjoining jamo (`U+1100..=U+11FF`).
///
/// Hangul Compatibility Jamo (`U+3130..=U+318F`) is absent deliberately: it has
/// compatibility decompositions into this block, so normalization has already
/// rewritten it by the time segmentation runs.
const HANGUL_JAMO: (char, char) = ('\u{1100}', '\u{11FF}');
/// Hiragana (`U+3040..=U+309F`).
const HIRAGANA: (char, char) = ('\u{3040}', '\u{309F}');
/// Katakana (`U+30A0..=U+30FF`).
///
/// Halfwidth katakana (`U+FF66..=U+FF9F`) is absent deliberately: NFKD maps it
/// into this block before segmentation sees it.
const KATAKANA: (char, char) = ('\u{30A0}', '\u{30FF}');
/// Katakana Phonetic Extensions (`U+31F0..=U+31FF`).
const KATAKANA_PHONETIC_EXTENSIONS: (char, char) = ('\u{31F0}', '\u{31FF}');
/// CJK Unified Ideographs Extension A (`U+3400..=U+4DBF`).
const CJK_UNIFIED_IDEOGRAPHS_EXTENSION_A: (char, char) = ('\u{3400}', '\u{4DBF}');
/// CJK Unified Ideographs (`U+4E00..=U+9FFF`) — the main Han block.
const CJK_UNIFIED_IDEOGRAPHS: (char, char) = ('\u{4E00}', '\u{9FFF}');
/// Hangul Syllables (`U+AC00..=U+D7A3`).
const HANGUL_SYLLABLES: (char, char) = ('\u{AC00}', '\u{D7A3}');
/// CJK Compatibility Ideographs (`U+F900..=U+FAFF`).
///
/// Most of this block has canonical singleton decompositions and is gone before
/// segmentation runs, but a dozen code points in it have none and survive
/// normalization, so the block stays in the set.
const CJK_COMPATIBILITY_IDEOGRAPHS: (char, char) = ('\u{F900}', '\u{FAFF}');
/// CJK Unified Ideographs Extension B (`U+20000..=U+2A6DF`).
const CJK_UNIFIED_IDEOGRAPHS_EXTENSION_B: (char, char) = ('\u{20000}', '\u{2A6DF}');
/// CJK Unified Ideographs Extensions C onward (`U+2A700..=U+2EBEF`).
const CJK_UNIFIED_IDEOGRAPHS_EXTENSIONS_BEYOND_B: (char, char) = ('\u{2A700}', '\u{2EBEF}');

/// The blocks whose tokens are rejoined and expanded into bigrams.
///
/// Ascending by start code point, and non-overlapping.
const CJK_BLOCKS: [(char, char); 10] = [
    HANGUL_JAMO,
    HIRAGANA,
    KATAKANA,
    KATAKANA_PHONETIC_EXTENSIONS,
    CJK_UNIFIED_IDEOGRAPHS_EXTENSION_A,
    CJK_UNIFIED_IDEOGRAPHS,
    HANGUL_SYLLABLES,
    CJK_COMPATIBILITY_IDEOGRAPHS,
    CJK_UNIFIED_IDEOGRAPHS_EXTENSION_B,
    CJK_UNIFIED_IDEOGRAPHS_EXTENSIONS_BEYOND_B,
];

/// The compatibility caseless form of `input`, one `char` at a time.
///
/// The composition is the module documentation's, and the reason it is an
/// iterator rather than a `String` is that its two callers want different
/// things from it: one collects it, the other compares it against the input
/// without allocating at all.
fn analysis_form_chars(input: &str) -> impl Iterator<Item = char> + '_ {
    input
        .chars()
        .nfd()
        .default_case_fold()
        .nfkd()
        .default_case_fold()
        .nfkd()
        .nfc()
}

/// Whether `input` is already its own analysis form, decided without
/// allocating.
///
/// This is what lets [`Analyzer::analyze`] hand back borrowed tokens. It costs
/// one pass of the fold pipeline, which is why it compares lazily and stops at
/// the first character that differs rather than building the form and testing
/// equality.
fn is_in_analysis_form(input: &str) -> bool {
    let mut original = input.chars();
    for folded in analysis_form_chars(input) {
        if original.next() != Some(folded) {
            return false;
        }
    }
    original.next().is_none()
}

/// Whether `word` is entirely CJK, and so joins a bigram run.
fn is_cjk_word(word: &str) -> bool {
    word.chars().all(is_cjk_char)
}

/// Whether `c` lies in one of [`CJK_BLOCKS`].
fn is_cjk_char(c: char) -> bool {
    CJK_BLOCKS.iter().any(|&(low, high)| c >= low && c <= high)
}

/// Hand one token to `sink`, reporting whether the position counter can carry
/// another.
///
/// Positions are a `u32`, and refusing to emit past `u32::MAX` is the honest
/// alternative to wrapping (which would give two tokens the same position and
/// silently corrupt phrase matching). The bound is unreachable in practice:
/// every token spans at least one byte of its input, so exceeding it needs a
/// single literal larger than four gigabytes — which cannot be addressed at all
/// on `wasm32`, where `usize` is 32 bits.
fn emit<'t, F>(sink: &mut F, text: &'t str, position: &mut u32) -> bool
where
    F: FnMut(&'t str, u32),
{
    sink(text, *position);
    match position.checked_add(1) {
        Some(next) => {
            *position = next;
            true
        }
        None => false,
    }
}

/// Segment already-normalized `text`, handing each token's slice and position
/// to `sink` in order.
///
/// Every caller wants the same segmentation but a different token type — a
/// borrowed [`Cow`], an owned one, or no [`Token`] at all — so this yields the
/// slices themselves and lets each caller decide. Nothing here allocates: a
/// token is always a subslice of `text`, bigrams included, because a bigram
/// spans two adjacent characters of one contiguous run.
fn segment_each<'t, F>(text: &'t str, mut sink: F)
where
    F: FnMut(&'t str, u32),
{
    let mut position: u32 = 0;
    let mut words = text.unicode_word_indices().peekable();
    while let Some((start, word)) = words.next() {
        if !is_cjk_word(word) {
            if !emit(&mut sink, word, &mut position) {
                return;
            }
            continue;
        }

        // Extend the run over every following word that is both CJK and
        // physically adjacent. Adjacency is what a space would break, and it is
        // the only reason this segments with offsets instead of bare words.
        let mut end = start + word.len();
        while let Some(&(next_start, next_word)) = words.peek() {
            if next_start != end || !is_cjk_word(next_word) {
                break;
            }
            end = next_start + next_word.len();
            words.next();
        }
        let run = &text[start..end];

        // Overlapping character bigrams. Walking the run from its second
        // character gives each bigram's closing character; the previous
        // character's offset is its opening one.
        let mut closers = run.char_indices();
        closers.next();
        let mut opener = 0_usize;
        let mut any_bigram = false;
        for (offset, closer) in closers {
            let bigram = &run[opener..offset + closer.len_utf8()];
            if !emit(&mut sink, bigram, &mut position) {
                return;
            }
            opener = offset;
            any_bigram = true;
        }
        // A one-character run has no bigram, and dropping it would make a
        // single ideograph unretrievable, so it is emitted whole.
        if !any_bigram && !emit(&mut sink, run, &mut position) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{Analyzer, CJK_BLOCKS, analysis_form_chars, is_cjk_char, is_in_analysis_form};

    /// The no-allocation predicate must agree with the form it is a shortcut
    /// for, or [`Analyzer::analyze`] would borrow text it had no right to.
    #[test]
    fn the_borrow_check_agrees_with_the_form() {
        for input in [
            "",
            "rust",
            "Rust",
            "STRASSE",
            "straße",
            "strasse",
            "café",
            "cafe\u{0301}",
            "ｒｕｓｔ",
            "ﬁ",
            "中文",
            "σοφός",
            "ΣΟΦΟΣ",
            "don't",
            "3.14",
        ] {
            let form: String = analysis_form_chars(input).collect();
            assert_eq!(
                is_in_analysis_form(input),
                form == input,
                "the predicate disagreed with the form for {input:?}"
            );
        }
    }

    /// The block table is ordered and disjoint, as its documentation claims.
    #[test]
    fn the_cjk_blocks_are_ordered_and_disjoint() {
        for window in CJK_BLOCKS.windows(2) {
            let [(low, high), (next_low, _)] = window else {
                unreachable!("windows(2) yields pairs")
            };
            assert!(low <= high, "a block ends before it starts");
            assert!(high < next_low, "two blocks overlap or are out of order");
        }
    }

    /// Latin text is never treated as CJK, which is what keeps bigram expansion
    /// away from scripts that segment into whole words.
    #[test]
    fn latin_is_not_cjk() {
        for c in ['a', 'Z', '0', ' ', '-', 'é', 'Я', 'א', 'ا'] {
            assert!(!is_cjk_char(c), "{c:?} must not be classified as CJK");
        }
    }

    /// The analysis form is what the tokens are built from, so the two must not
    /// be able to drift apart.
    #[test]
    fn the_analysis_form_matches_the_tokens() {
        let analyzer = Analyzer::new();
        assert_eq!(analyzer.analysis_form("Straße"), "strasse");
        assert_eq!(analyzer.analysis_form("ｒｕｓｔ"), "rust");
    }

    /// The form this module composes is the same relation `caseless`'s own
    /// predicate decides, in both directions.
    ///
    /// This is the claim the module documentation makes and the one the final
    /// NFC step could have broken: two strings must analyze to the same text
    /// exactly when they are compatibility caseless matches of one another. A
    /// normalization that merged two inequivalent strings would merge two terms
    /// the standard keeps apart, and one that split an equivalent pair would
    /// lose a match — neither is visible from the outside, so it is checked
    /// against an independent implementation of the same relation rather than
    /// against this module's own idea of it.
    #[test]
    fn the_form_decides_what_the_caseless_predicate_decides() {
        let analyzer = Analyzer::new();
        let samples = [
            "STRASSE",
            "Straße",
            "strasse",
            "straße",
            "rust",
            "ｒｕｓｔ",
            "RUST",
            "fi",
            "ﬁ",
            "café",
            "cafe\u{0301}",
            "CAFÉ",
            "σοφος",
            "ΣΟΦΟΣ",
            "σοφός",
            "中文",
            "",
        ];
        for left in samples {
            for right in samples {
                assert_eq!(
                    analyzer.analysis_form(left) == analyzer.analysis_form(right),
                    caseless::compatibility_caseless_match_str(left, right),
                    "the form and the predicate disagreed on {left:?} vs {right:?}"
                );
            }
        }
    }

    /// The extent of the case-folding table's one-release lag, measured rather than
    /// asserted — and confined to characters no pre-Unicode-17.0 corpus can hold.
    ///
    /// `caseless` ships `CaseFolding.txt` at 16.0.0 while the other three tables are
    /// at 17.0.0, and it cannot be levelled: 0.2.2 is the newest version published
    /// and it is the only full (`C` + `F`) fold in this workspace. So the skew is
    /// carried, and this is what carrying it costs.
    ///
    /// A character is affected exactly when the fold stops being a case invariant on
    /// it — when `fold(c)`, `fold(lowercase(c))` and `fold(uppercase(c))` are not all
    /// the same string. Scanning the whole code space finds 57 such characters and no
    /// others. One of them, `U+0131`, is not a skew at all: Unicode excludes the
    /// dotless i from the default fold on purpose. The remaining 56 are six Latin
    /// Extended-D letters and fifty Beria Erfe letters, all of them characters whose
    /// cased partner the 17.0.0 tables know and the 16.0.0 fold table does not.
    ///
    /// The scan is exhaustive and the answer is pinned as a set, so this fails the
    /// day `caseless` ships 17.0.0 tables — which is the point. Raising the fold
    /// table rewrites the term dictionary, and that must be a visible change with
    /// re-derived goldens rather than a silent one.
    #[test]
    fn the_case_folding_skew_is_confined_to_where_it_is_measured() {
        /// The contiguous runs of affected code points, as measured.
        const AFFECTED: [(u32, u32); 5] = [
            // Unicode's own deliberate exclusion: the Turkic dotless i has `T`
            // status case mappings, which `C` + `F` folding does not apply. Every
            // conforming fold table of every version behaves this way.
            (0x0131, 0x0131),
            // Latin Extended-D letters whose cased partner postdates the fold table.
            (0xA7CE, 0xA7CF),
            (0xA7D2, 0xA7D5),
            // Beria Erfe — a bicameral script the fold table does not yet carry.
            (0x16EA0, 0x16EB8),
            (0x16EBB, 0x16ED3),
        ];

        let fold = |text: &str| -> String {
            use caseless::Caseless as _;
            text.chars().default_case_fold().collect()
        };

        let mut measured: Vec<u32> = Vec::new();
        for code_point in 0..=0x0010_FFFF_u32 {
            let Some(c) = char::from_u32(code_point) else {
                continue;
            };
            let itself = c.to_string();
            let lowered: String = c.to_lowercase().collect();
            let raised: String = c.to_uppercase().collect();
            let folded = fold(&itself);
            if folded != fold(&lowered) || folded != fold(&raised) {
                measured.push(code_point);
            }
        }

        let expected: Vec<u32> = AFFECTED
            .iter()
            .flat_map(|&(low, high)| low..=high)
            .collect();
        assert_eq!(
            measured.len(),
            57,
            "the measured skew is 57 code points; got {}",
            measured.len()
        );
        assert_eq!(
            measured, expected,
            "the case-folding table's disagreement with the case-mapping tables moved. If `caseless` \
             has shipped Unicode 17.0.0 tables this set should shrink to just U+0131 — re-derive this \
             crate's golden token vectors and fingerprints, then narrow this pin. If it grew instead, \
             a table moved underneath the crate and the term dictionary moved with it."
        );

        // And the guarantee that makes the skew survivable: every script a corpus
        // written before Unicode 17.0 could possibly hold folds consistently.
        for (script, low, high) in [
            ("ASCII", 0x0000_u32, 0x007F_u32),
            ("Latin-1 Supplement", 0x0080, 0x00FF),
            ("Latin Extended-A", 0x0100, 0x017F),
            ("Greek and Coptic", 0x0370, 0x03FF),
            ("Cyrillic", 0x0400, 0x04FF),
            ("Hebrew", 0x0590, 0x05FF),
            ("Arabic", 0x0600, 0x06FF),
            ("Hiragana", 0x3040, 0x309F),
            ("Katakana", 0x30A0, 0x30FF),
            ("CJK Unified Ideographs", 0x4E00, 0x9FFF),
            ("Hangul Syllables", 0xAC00, 0xD7A3),
        ] {
            for code_point in low..=high {
                let Some(c) = char::from_u32(code_point) else {
                    continue;
                };
                // U+0131 sits inside Latin Extended-A and is Unicode's own
                // exclusion rather than a table lag, so it is named rather than
                // quietly skipped.
                if code_point == 0x0131 {
                    continue;
                }
                let itself = c.to_string();
                let lowered: String = c.to_lowercase().collect();
                let raised: String = c.to_uppercase().collect();
                let folded = fold(&itself);
                assert_eq!(
                    (folded.clone(), folded),
                    (fold(&lowered), fold(&raised)),
                    "the fold is not a case invariant on U+{code_point:04X} in {script}, so the table \
                     skew has reached a script real corpora already contain"
                );
            }
        }
    }

    /// The measured `UAX #29` behaviour the CJK model is built on.
    ///
    /// The bigram path exists because of what this asserts, and the assertion
    /// is here rather than in prose because the premise is the sort that is
    /// easy to state confidently and get backwards. Han and Hiragana segment to
    /// one token per character, so expanding each token into bigrams would
    /// expand nothing; Katakana does not, because rule WB13 keeps a katakana
    /// run together; Hangul syllables are `ALetter` and segment into whole
    /// space-delimited words; and none of it is dropped by the alphanumeric
    /// filter `unicode_words` applies.
    #[test]
    fn the_measured_cjk_segmentation_matches_the_documented_model() {
        use unicode_segmentation::UnicodeSegmentation as _;

        assert_eq!(
            "中文全文検索".unicode_words().collect::<Vec<_>>(),
            vec!["中", "文", "全", "文", "検", "索"],
            "Han must segment to one token per ideograph"
        );
        assert_eq!(
            "私はサンドイッチを食べます"
                .unicode_words()
                .collect::<Vec<_>>(),
            vec!["私", "は", "サンドイッチ", "を", "食", "べ", "ま", "す"],
            "Hiragana must segment per character while a Katakana run stays whole"
        );
        assert_eq!(
            "한국어 전문 검색".unicode_words().collect::<Vec<_>>(),
            vec!["한국어", "전문", "검색"],
            "Hangul must segment into whole space-delimited words"
        );

        for c in ['中', 'は', 'サ', '한'] {
            assert!(
                c.is_alphanumeric(),
                "{c:?} must pass the alphanumeric filter `unicode_words` applies"
            );
        }
    }
}
