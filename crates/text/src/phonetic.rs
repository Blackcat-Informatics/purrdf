// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Latin phonetic candidates and exact bounded scalar Levenshtein distance.
//!
//! Pronunciation rules follow Lawrence Philips' published Double Metaphone
//! description and its declarative rule table (US20090043584A1, table 4).
//! This implementation uses an independently designed contextual classifier;
//! frozen black-box answers validate the result. No upstream implementation
//! supplies code. Canonical spelling is separate from pronunciation codes and
//! is the spelling on which candidate refinement must operate.

use crate::{TextError, unicode};

/// The pronunciation, canonicalization and resource law.
pub const PROFILE_ID: &str = "purrdf-double-metaphone-latin-canonical-widening-v2";
/// Maximum expanded canonical spelling or distance input, in Unicode scalars.
pub const MAX_INPUT_SCALARS: usize = 2048;
/// Largest permitted phonetic output length.
pub const MAX_CODE_LENGTH: usize = 64;
/// Largest permitted exact distance threshold.
pub const MAX_EDIT_DISTANCE: usize = 64;

/// A complete reason a word cannot participate in phonetic retrieval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PhoneticRefusal {
    /// A retrieval probe must resolve to exactly one surface word.
    SurfaceWordCount {
        /// Number of surface words produced by the selected analyzer.
        observed: usize,
    },
    /// A scalar has no meaning under the declared Latin widening law.
    UnsupportedScalar {
        /// The unsupported scalar after canonical decomposition and folding.
        scalar: char,
    },
    /// An apostrophe is leading, trailing or adjacent to another apostrophe.
    InvalidApostrophe,
    /// No spelling remains after the declared removals.
    EmptySpelling,
    /// Expanded spelling exceeds its independent resource bound.
    InputTooLong {
        /// Maximum admitted expanded scalar count.
        limit: usize,
        /// Scalar count observed when the limit was exceeded.
        observed: usize,
    },
    /// The already-canonical entry point received a non-uppercase-ASCII scalar.
    NonCanonicalSpelling {
        /// The first scalar violating the canonical spelling contract.
        scalar: char,
    },
    /// Both pronunciation keys are empty; no universal bucket is formed.
    EmptyCode,
}

impl core::fmt::Display for PhoneticRefusal {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SurfaceWordCount { observed } => write!(
                formatter,
                "phonetic probe analyzes to {observed} surface words; exactly one is required"
            ),
            Self::UnsupportedScalar { scalar } => {
                write!(formatter, "unsupported phonetic scalar {scalar:?}")
            }
            Self::InvalidApostrophe => {
                formatter.write_str("phonetic apostrophe must be internal between Latin letters")
            }
            Self::EmptySpelling => formatter.write_str("phonetic spelling is empty"),
            Self::InputTooLong { limit, observed } => write!(
                formatter,
                "expanded phonetic spelling has {observed} scalars, exceeding {limit}"
            ),
            Self::NonCanonicalSpelling { scalar } => {
                write!(formatter, "noncanonical phonetic scalar {scalar:?}")
            }
            Self::EmptyCode => {
                formatter.write_str("surface has no Double Metaphone pronunciation code")
            }
        }
    }
}

impl std::error::Error for PhoneticRefusal {}

/// Primary and alternate pronunciation keys, kept even when equal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhoneticCodes {
    /// Primary pronunciation; may be empty when only the alternate exists.
    pub primary: String,
    /// Alternate pronunciation; a terminal-J space is a significant code byte.
    pub alternate: String,
}

/// A validated immutable phonetic code bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DoubleMetaphone {
    max_code_len: usize,
}

impl DoubleMetaphone {
    /// Select an output bound in `1..=64`.
    ///
    /// # Errors
    /// Returns a configuration error for a bound outside that interval.
    pub fn new(max_code_len: usize) -> Result<Self, TextError> {
        if !(1..=MAX_CODE_LENGTH).contains(&max_code_len) {
            return Err(TextError::config("phonetic code length must be in 1..=64"));
        }
        Ok(Self { max_code_len })
    }

    /// Maximum length of either pronunciation code.
    #[must_use]
    pub const fn max_code_len(&self) -> usize {
        self.max_code_len
    }

    /// Canonicalize a surface word and derive its two pronunciation keys.
    ///
    /// # Errors
    /// Refuses unsupported spelling, expansion beyond 2,048 scalars, and words
    /// for which neither pronunciation contains a code.
    pub fn encode(&self, surface: &str) -> Result<PhoneticCodes, PhoneticRefusal> {
        self.encode_canonical(&canonicalize(surface)?)
    }

    /// Encode an already canonical uppercase ASCII spelling without refolding.
    ///
    /// # Errors
    /// Refuses empty, overlong or noncanonical input and empty pronunciation.
    pub fn encode_canonical(&self, spelling: &str) -> Result<PhoneticCodes, PhoneticRefusal> {
        if spelling.is_empty() {
            return Err(PhoneticRefusal::EmptySpelling);
        }
        if let Some(scalar) = spelling.chars().find(|letter| !letter.is_ascii_uppercase()) {
            return Err(PhoneticRefusal::NonCanonicalSpelling { scalar });
        }
        if spelling.len() > MAX_INPUT_SCALARS {
            return Err(PhoneticRefusal::InputTooLong {
                limit: MAX_INPUT_SCALARS,
                observed: spelling.len(),
            });
        }
        let codes = pronounce(spelling.as_bytes(), self.max_code_len);
        if codes.primary.is_empty() && codes.alternate.is_empty() {
            return Err(PhoneticRefusal::EmptyCode);
        }
        Ok(codes)
    }
}

/// Canonical widened Latin spelling used by both coding and refinement.
///
/// Script-compatible NFD nonspacing marks are stripped only after Latin bases. The fixed widening table is
/// applied after pinned case folding. Declared controls are removed; only
/// apostrophes between Latin letters are dropped. No whitespace is trimmed.
///
/// # Errors
/// Returns a typed refusal for unsupported scalars, leading/trailing apostrophes,
/// an empty spelling, or more than 2,048 expanded ASCII letters.
pub fn canonicalize(surface: &str) -> Result<String, PhoneticRefusal> {
    let folded = unicode::case_fold(surface);
    let decomposed = unicode::nfd(&folded);
    let mut result = String::with_capacity(decomposed.len().min(MAX_INPUT_SCALARS));
    let mut latin_base = false;
    let mut pending_apostrophe = false;
    for letter in decomposed.chars() {
        if unicode::is_phonetic_control(letter) {
            continue;
        }
        if unicode::AccentScript::Latin.admits_mark(letter) && latin_base {
            continue;
        }
        if matches!(letter, '\'' | '\u{2019}') {
            if !latin_base || pending_apostrophe {
                return Err(PhoneticRefusal::InvalidApostrophe);
            }
            pending_apostrophe = true;
            latin_base = false;
            continue;
        }
        if !unicode::is_latin_letter(letter) {
            return Err(PhoneticRefusal::UnsupportedScalar { scalar: letter });
        }
        let widening = match letter {
            'ß' => "SS",
            'æ' => "AE",
            'œ' => "OE",
            'þ' => "TH",
            'ø' => "O",
            'ł' => "L",
            'đ' | 'ð' => "D",
            'ħ' => "H",
            'ı' => "I",
            'ŋ' => "N",
            value if value.is_ascii_alphabetic() => {
                result.push(value.to_ascii_uppercase());
                ""
            }
            _ => return Err(PhoneticRefusal::UnsupportedScalar { scalar: letter }),
        };
        result.push_str(widening);
        if result.len() > MAX_INPUT_SCALARS {
            return Err(PhoneticRefusal::InputTooLong {
                limit: MAX_INPUT_SCALARS,
                observed: result.len(),
            });
        }
        latin_base = true;
        pending_apostrophe = false;
    }
    if pending_apostrophe {
        return Err(PhoneticRefusal::InvalidApostrophe);
    }
    if result.is_empty() {
        return Err(PhoneticRefusal::EmptySpelling);
    }
    Ok(result)
}

/// Whether a surface has a supported canonical spelling and a nonempty code.
#[must_use]
pub fn accepts(surface: &str) -> bool {
    DoubleMetaphone { max_code_len: 1 }.encode(surface).is_ok()
}

#[derive(Clone, Copy)]
struct Sound {
    width: usize,
    primary: &'static str,
    alternate: &'static str,
}

const fn sound(width: usize, both: &'static str) -> Sound {
    Sound {
        width,
        primary: both,
        alternate: both,
    }
}
const fn choice(width: usize, primary: &'static str, alternate: &'static str) -> Sound {
    Sound {
        width,
        primary,
        alternate,
    }
}

struct Context<'a> {
    word: &'a [u8],
    at: usize,
    slavic: bool,
}

impl Context<'_> {
    fn next(&self, offset: usize) -> u8 {
        self.word.get(self.at + offset).copied().unwrap_or(0)
    }
    fn previous(&self, offset: usize) -> u8 {
        self.at
            .checked_sub(offset)
            .and_then(|at| self.word.get(at))
            .copied()
            .unwrap_or(0)
    }
    fn begins(&self, text: &[u8]) -> bool {
        self.word.starts_with(text)
    }
    fn tail(&self, text: &[u8]) -> bool {
        self.word[self.at..].starts_with(text)
    }
    fn left(&self, text: &[u8]) -> bool {
        self.word[..self.at].ends_with(text)
    }
    fn around(&self, before: usize, text: &[u8]) -> bool {
        self.at
            .checked_sub(before)
            .is_some_and(|at| self.word[at..].starts_with(text))
    }
    fn final_at(&self, offset: usize) -> bool {
        self.at + offset + 1 == self.word.len()
    }
    fn doubled(&self, letter: u8) -> usize {
        1 + usize::from(self.next(1) == letter)
    }
    fn germanic(&self) -> bool {
        self.begins(b"SCH")
    }
}

const fn vowel(letter: u8) -> bool {
    matches!(letter, b'A' | b'E' | b'I' | b'O' | b'U' | b'Y')
}

fn pronounce(word: &[u8], bound: usize) -> PhoneticCodes {
    let slavic = word.iter().any(|letter| matches!(letter, b'W' | b'K'))
        || word.windows(2).any(|pair| pair == b"CZ");
    let mut context = Context {
        word,
        at: usize::from(
            [b"GN", b"KN", b"PN", b"WR", b"PS"]
                .iter()
                .any(|prefix| word.starts_with(*prefix)),
        ),
        slavic,
    };
    let mut codes = PhoneticCodes {
        primary: String::with_capacity(bound),
        alternate: String::with_capacity(bound),
    };
    while context.at < word.len() && (codes.primary.len() < bound || codes.alternate.len() < bound)
    {
        let emission = classify(&context);
        let main = emission.primary.len().min(bound - codes.primary.len());
        let other = emission.alternate.len().min(bound - codes.alternate.len());
        codes.primary.push_str(&emission.primary[..main]);
        codes.alternate.push_str(&emission.alternate[..other]);
        context.at += emission.width;
    }
    codes
}

fn classify(c: &Context<'_>) -> Sound {
    match c.next(0) {
        b'A' | b'E' | b'I' | b'O' | b'U' | b'Y' => sound(1, if c.at == 0 { "A" } else { "" }),
        b'B' => sound(c.doubled(b'B'), "P"),
        b'F' => sound(c.doubled(b'F'), "F"),
        b'K' => sound(c.doubled(b'K'), "K"),
        b'N' => sound(c.doubled(b'N'), "N"),
        b'Q' => sound(c.doubled(b'Q'), "K"),
        b'V' => sound(c.doubled(b'V'), "F"),
        b'H' => sound(
            if (c.at == 0 || vowel(c.previous(1))) && vowel(c.next(1)) {
                2
            } else {
                1
            },
            if (c.at == 0 || vowel(c.previous(1))) && vowel(c.next(1)) {
                "H"
            } else {
                ""
            },
        ),
        b'C' => velar_c(c),
        b'G' => velar_g(c),
        b'D' => {
            if c.tail(b"DG") {
                if matches!(c.next(2), b'I' | b'E' | b'Y') {
                    sound(3, "J")
                } else {
                    sound(2, "TK")
                }
            } else {
                sound(1 + usize::from(matches!(c.next(1), b'D' | b'T')), "T")
            }
        }
        b'J' => palatal_j(c),
        b'L' => liquid_l(c),
        b'M' => sound(
            if c.next(1) == b'M'
                || (c.around(1, b"UMB")
                    && (c.final_at(1)
                        || c.word
                            .get(c.at + 2..)
                            .is_some_and(|tail| tail.starts_with(b"ER"))))
            {
                2
            } else {
                1
            },
            "M",
        ),
        b'P' => {
            if c.tail(b"PH") {
                sound(2, "F")
            } else {
                sound(1 + usize::from(matches!(c.next(1), b'P' | b'B')), "P")
            }
        }
        b'R' => {
            if c.final_at(0)
                && !c.slavic
                && c.left(b"IE")
                && !c.around(4, b"MEIER")
                && !c.around(4, b"MAIER")
            {
                choice(c.doubled(b'R'), "", "R")
            } else {
                sound(c.doubled(b'R'), "R")
            }
        }
        b'S' => sibilant_s(c),
        b'T' => dental_t(c),
        b'W' => glide_w(c),
        b'X' => {
            let width = 1 + usize::from(matches!(c.next(1), b'C' | b'X'));
            if c.at == 0 {
                sound(1, "S")
            } else if c.final_at(0)
                && [b"IAU".as_slice(), b"EAU", b"AU", b"OU"]
                    .iter()
                    .any(|ending| c.left(ending))
            {
                sound(width, "")
            } else {
                sound(width, "KS")
            }
        }
        b'Z' => {
            if c.tail(b"ZH") {
                sound(2, "J")
            } else if (c.next(1) == b'Z' && matches!(c.next(2), b'O' | b'I' | b'A'))
                || (c.slavic && c.at > 0 && c.previous(1) != b'T')
            {
                choice(c.doubled(b'Z'), "S", "TS")
            } else {
                sound(c.doubled(b'Z'), "S")
            }
        }
        _ => unreachable!("canonical input contains only uppercase ASCII letters"),
    }
}

fn velar_c(c: &Context<'_>) -> Sound {
    let ach = c.at > 1
        && !vowel(c.previous(2))
        && c.around(1, b"ACH")
        && (c.next(2) != b'I' && c.next(2) != b'E'
            || c.around(2, b"BACHER")
            || c.around(2, b"MACHER"));
    if ach || c.tail(b"CHIA") {
        return sound(2, "K");
    }
    if c.at == 0 && c.tail(b"CAESAR") {
        return sound(2, "S");
    }
    if c.tail(b"CH") {
        return aspirate_c(c);
    }
    if c.tail(b"CZ") && !c.around(2, b"WICZ") {
        return choice(2, "S", "X");
    }
    if c.word
        .get(c.at + 1..)
        .is_some_and(|tail| tail.starts_with(b"CIA"))
    {
        return sound(3, "X");
    }
    if c.tail(b"CC") && !(c.at == 1 && c.previous(1) == b'M') {
        if matches!(c.next(2), b'I' | b'E' | b'H') && !(c.next(2) == b'H' && c.next(3) == b'U') {
            return sound(
                3,
                if (c.at == 1 && c.previous(1) == b'A')
                    || c.around(1, b"UCCEE")
                    || c.around(1, b"UCCES")
                {
                    "KS"
                } else {
                    "X"
                },
            );
        }
        return sound(2, "K");
    }
    if matches!(c.next(1), b'K' | b'G' | b'Q') {
        return sound(2, "K");
    }
    if matches!(c.next(1), b'I' | b'E' | b'Y') {
        return if c.tail(b"CIO") || c.tail(b"CIE") || c.tail(b"CIA") {
            choice(2, "S", "X")
        } else {
            sound(2, "S")
        };
    }
    sound(
        1 + usize::from(
            matches!(c.next(1), b'C' | b'K' | b'Q') && !matches!(c.next(2), b'E' | b'I'),
        ),
        "K",
    )
}

fn aspirate_c(c: &Context<'_>) -> Sound {
    if c.at > 0 && c.tail(b"CHAE") {
        return choice(2, "K", "X");
    }
    let greek_start = c.at == 0
        && [
            b"CHARAC".as_slice(),
            b"CHARIS",
            b"CHOR",
            b"CHYM",
            b"CHIA",
            b"CHEM",
        ]
        .iter()
        .any(|prefix| c.tail(prefix))
        && !c.begins(b"CHORE");
    let hard = greek_start
        || c.germanic()
        || [b"ORCHES".as_slice(), b"ARCHIT", b"ORCHID"]
            .iter()
            .any(|pattern| c.around(2, pattern))
        || matches!(c.next(2), b'T' | b'S')
        || ((c.at == 0 || matches!(c.previous(1), b'A' | b'O' | b'U' | b'E'))
            && matches!(
                c.next(2),
                0 | b'L' | b'R' | b'N' | b'M' | b'B' | b'H' | b'F' | b'V' | b'W'
            ));
    if hard || (c.at > 0 && c.begins(b"MC")) {
        sound(2, "K")
    } else if c.at > 0 {
        choice(2, "X", "K")
    } else {
        sound(2, "X")
    }
}

fn velar_g(c: &Context<'_>) -> Sound {
    if c.tail(b"GH") {
        return aspirate_g(c);
    }
    if c.tail(b"GN") {
        return if c.at == 1 && vowel(c.previous(1)) && !c.slavic {
            choice(2, "KN", "N")
        } else if !c
            .word
            .get(c.at + 2..)
            .is_some_and(|tail| tail.starts_with(b"EY"))
            && !c.slavic
        {
            choice(2, "N", "KN")
        } else {
            sound(2, "KN")
        };
    }
    if c.tail(b"GLI") && !c.slavic {
        return choice(2, "KL", "L");
    }
    let initial_ambiguous = c.at == 0
        && (c.next(1) == b'Y'
            || [
                b"ES", b"EP", b"EB", b"EL", b"EY", b"IB", b"IL", b"IN", b"IE", b"EI", b"ER",
            ]
            .iter()
            .any(|pattern| c.word[1..].starts_with(*pattern)));
    let medial_ambiguous = (c.tail(b"GER") || c.tail(b"GY"))
        && ![b"DANGER", b"RANGER", b"MANGER"]
            .iter()
            .any(|prefix| c.begins(*prefix))
        && !matches!(c.previous(1), b'E' | b'I')
        && !c.around(1, b"RGY")
        && !c.around(1, b"OGY");
    if initial_ambiguous || medial_ambiguous {
        return choice(2, "K", "J");
    }
    if matches!(c.next(1), b'E' | b'I' | b'Y') || c.around(1, b"AGGI") || c.around(1, b"OGGI") {
        return if c.germanic() || c.tail(b"GET") {
            sound(2, "K")
        } else if c.tail(b"GIER") {
            sound(2, "J")
        } else {
            choice(2, "J", "K")
        };
    }
    sound(c.doubled(b'G'), "K")
}

fn aspirate_g(c: &Context<'_>) -> Sound {
    if c.at > 0 && !vowel(c.previous(1)) {
        return sound(2, "K");
    }
    if c.at == 0 {
        return sound(2, if c.next(2) == b'I' { "J" } else { "K" });
    }
    let silent = (c.at > 1 && matches!(c.previous(2), b'B' | b'H' | b'D'))
        || (c.at > 2 && matches!(c.previous(3), b'B' | b'H' | b'D'))
        || (c.at > 3 && matches!(c.previous(4), b'B' | b'H'));
    if silent {
        sound(2, "")
    } else if c.at > 2
        && c.previous(1) == b'U'
        && matches!(c.previous(3), b'C' | b'G' | b'L' | b'R' | b'T')
    {
        sound(2, "F")
    } else {
        sound(2, if c.previous(1) == b'I' { "" } else { "K" })
    }
}

fn palatal_j(c: &Context<'_>) -> Sound {
    let width = c.doubled(b'J');
    if c.tail(b"JOSE") {
        return if c.at == 0 && c.word.len() == 4 {
            sound(width, "H")
        } else {
            choice(width, "J", "H")
        };
    }
    if c.at == 0 {
        return choice(width, "J", "A");
    }
    if vowel(c.previous(1)) && !c.slavic && matches!(c.next(1), b'A' | b'O') {
        return choice(width, "J", "H");
    }
    if c.final_at(0) {
        return choice(width, "J", " ");
    }
    if matches!(
        c.next(1),
        b'L' | b'T' | b'K' | b'S' | b'N' | b'M' | b'B' | b'Z'
    ) || matches!(c.previous(1), b'S' | b'K' | b'L')
    {
        sound(width, "")
    } else {
        sound(width, "J")
    }
}

fn liquid_l(c: &Context<'_>) -> Sound {
    let width = c.doubled(b'L');
    let spanish = width == 2
        && ((c.at + 3 == c.word.len()
            && [b"ILLO", b"ILLA", b"ALLE"]
                .iter()
                .any(|pattern| c.around(1, *pattern)))
            || ((c.word.ends_with(b"AS")
                || c.word.ends_with(b"OS")
                || matches!(c.word.last(), Some(b'A' | b'O')))
                && c.around(1, b"ALLE")));
    if spanish {
        choice(width, "L", "")
    } else {
        sound(width, "L")
    }
}

fn sibilant_s(c: &Context<'_>) -> Sound {
    if c.around(1, b"ISL") || c.around(1, b"YSL") {
        return sound(1, "");
    }
    if c.at == 0 && c.tail(b"SUGAR") {
        return choice(1, "X", "S");
    }
    if c.tail(b"SH") {
        let german = [b"SHEIM".as_slice(), b"SHOEK", b"SHOLM", b"SHOLZ"]
            .iter()
            .any(|pattern| c.tail(pattern));
        return sound(2, if german { "S" } else { "X" });
    }
    if c.tail(b"SIO") || c.tail(b"SIA") || c.tail(b"SIAN") {
        return if c.slavic {
            sound(3, "S")
        } else {
            choice(3, "S", "X")
        };
    }
    if (c.at == 0 && matches!(c.next(1), b'M' | b'N' | b'L' | b'W')) || c.next(1) == b'Z' {
        return choice(1 + usize::from(c.next(1) == b'Z'), "S", "X");
    }
    if c.tail(b"SC") {
        if c.next(2) == b'H' {
            if [b"OO", b"ER", b"EN", b"UY", b"ED", b"EM"]
                .iter()
                .any(|pattern| c.word[c.at + 3..].starts_with(*pattern))
            {
                return if matches!(c.next(3), b'E') && matches!(c.next(4), b'R' | b'N') {
                    choice(3, "X", "SK")
                } else {
                    sound(3, "SK")
                };
            }
            return if c.at == 0 && !vowel(c.next(3)) && c.next(3) != b'W' {
                choice(3, "X", "S")
            } else {
                sound(3, "X")
            };
        }
        return sound(
            3,
            if matches!(c.next(2), b'I' | b'E' | b'Y') {
                "S"
            } else {
                "SK"
            },
        );
    }
    let width = 1 + usize::from(matches!(c.next(1), b'S' | b'Z'));
    if c.final_at(0) && (c.left(b"AI") || c.left(b"OI")) {
        choice(width, "", "S")
    } else {
        sound(width, "S")
    }
}

fn dental_t(c: &Context<'_>) -> Sound {
    if c.tail(b"TION") || c.tail(b"TIA") || c.tail(b"TCH") {
        return sound(3, "X");
    }
    if c.tail(b"TH") || c.tail(b"TTH") {
        return if c
            .word
            .get(c.at + 2..)
            .is_some_and(|tail| tail.starts_with(b"OM") || tail.starts_with(b"AM"))
            || c.germanic()
        {
            sound(2, "T")
        } else {
            choice(2, "0", "T")
        };
    }
    sound(1 + usize::from(matches!(c.next(1), b'T' | b'D')), "T")
}

fn glide_w(c: &Context<'_>) -> Sound {
    if c.tail(b"WR") {
        return sound(2, "R");
    }
    if c.at == 0 && (vowel(c.next(1)) || c.tail(b"WH")) {
        return if vowel(c.next(1)) {
            choice(1, "A", "F")
        } else {
            sound(1, "A")
        };
    }
    if (c.final_at(0) && vowel(c.previous(1)))
        || [b"EWSKI", b"EWSKY", b"OWSKI", b"OWSKY"]
            .iter()
            .any(|pattern| c.around(1, *pattern))
        || c.germanic()
    {
        return choice(1, "", "F");
    }
    if c.tail(b"WICZ") || c.tail(b"WITZ") {
        return choice(4, "TS", "FX");
    }
    sound(1, "")
}

/// Prepared query scalars and reusable endpoint-aware band working storage.
///
/// Cells are admitted only when the minimum indel cost through that cell can
/// reach both global endpoints within the threshold. A second lower bound adds
/// the remaining length imbalance to the exact prefix score. Neither bound
/// changes the global, unit-cost scalar Levenshtein metric.
#[derive(Clone, Debug)]
pub struct PreparedDistance {
    query: Vec<char>,
    candidate: Vec<char>,
    previous: Vec<usize>,
    current: Vec<usize>,
    threshold: usize,
}

impl PreparedDistance {
    /// Decode the query once and reserve two rows of at most `threshold + 1` cells.
    ///
    /// # Errors
    /// Refuses thresholds above 64 and inputs longer than 2,048 scalars.
    pub fn new(query: &str, threshold: usize) -> Result<Self, TextError> {
        validate_threshold(threshold)?;
        let mut scalars = Vec::new();
        read_scalars(query, &mut scalars)?;
        Ok(Self {
            query: scalars,
            candidate: Vec::new(),
            previous: Vec::with_capacity(threshold + 1),
            current: Vec::with_capacity(threshold + 1),
            threshold,
        })
    }

    /// Return the exact distance if within the configured threshold.
    ///
    /// # Errors
    /// Refuses candidates longer than 2,048 scalars, even when a length or
    /// equality shortcut could otherwise decide the result.
    pub fn distance(&mut self, candidate: &str) -> Result<Option<usize>, TextError> {
        read_scalars(candidate, &mut self.candidate)?;
        let (a, b) = trim_equal_ends(&self.query, &self.candidate);
        Ok(banded(
            a,
            b,
            self.threshold,
            &mut self.previous,
            &mut self.current,
        ))
    }
}

fn validate_threshold(threshold: usize) -> Result<(), TextError> {
    if threshold > MAX_EDIT_DISTANCE {
        Err(TextError::config(
            "edit-distance threshold must be in 0..=64",
        ))
    } else {
        Ok(())
    }
}

fn read_scalars(input: &str, target: &mut Vec<char>) -> Result<(), TextError> {
    target.clear();
    target.extend(input.chars().take(MAX_INPUT_SCALARS + 1));
    if target.len() > MAX_INPUT_SCALARS {
        Err(TextError::data("edit-distance input exceeds 2048 scalars"))
    } else {
        Ok(())
    }
}

fn trim_equal_ends<'a>(a: &'a [char], b: &'a [char]) -> (&'a [char], &'a [char]) {
    let prefix = a.iter().zip(b).take_while(|(a, b)| a == b).count();
    let a = &a[prefix..];
    let b = &b[prefix..];
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    (&a[..a.len() - suffix], &b[..b.len() - suffix])
}

fn banded(
    a: &[char],
    b: &[char],
    k: usize,
    previous: &mut Vec<usize>,
    current: &mut Vec<usize>,
) -> Option<usize> {
    let m = a.len();
    let n = b.len();
    let gap = m.abs_diff(n);
    if gap > k {
        return None;
    }
    if m == 0 || n == 0 {
        return Some(m.max(n));
    }
    let slack = (k - gap) / 2;
    let left = slack + n.saturating_sub(m);
    let right = slack + m.saturating_sub(n);
    let infinity = k + 1;
    let mut previous_start = 0;
    previous.clear();
    previous.extend(0..=right.min(m));
    for row in 1..=n {
        let start = row.saturating_sub(left);
        let end = m.min(row + right);
        current.clear();
        let mut viable = false;
        for column in start..=end {
            let from_previous = |column: usize| {
                column
                    .checked_sub(previous_start)
                    .and_then(|offset| previous.get(offset))
                    .copied()
                    .unwrap_or(infinity)
            };
            let distance = if column == 0 {
                row
            } else {
                let deletion = from_previous(column) + 1;
                let insertion = current.last().copied().unwrap_or(infinity) + 1;
                let substitution =
                    from_previous(column - 1) + usize::from(a[column - 1] != b[row - 1]);
                deletion.min(insertion).min(substitution)
            };
            let value = if distance + (m - column).abs_diff(n - row) <= k {
                distance
            } else {
                infinity
            };
            viable |= value <= k;
            current.push(value);
        }
        if !viable {
            return None;
        }
        std::mem::swap(previous, current);
        previous_start = start;
    }
    previous
        .get(m - previous_start)
        .copied()
        .filter(|distance| *distance <= k)
}

/// Exact bounded scalar distance, with a fresh prepared query.
///
/// Use [`PreparedDistance`] when comparing one query to a bucket of candidates.
///
/// # Errors
/// Refuses thresholds above 64 and either input above 2,048 scalars.
pub fn bounded_edit_distance(
    a: &str,
    b: &str,
    threshold: usize,
) -> Result<Option<usize>, TextError> {
    PreparedDistance::new(a, threshold)?.distance(b)
}

/// Prepared Myers bit-vector kernel for measuring an alternative exact engine.
///
/// Query equality masks are reused across candidates. Unsigned additions and
/// shifts carry across 64-bit blocks, including the 64/128 boundaries. This
/// kernel computes the same global scalar metric as [`PreparedDistance`].
#[derive(Clone, Debug)]
pub struct PreparedMyers {
    length: usize,
    masks: Vec<(char, Vec<u64>)>,
    positive: Vec<u64>,
    negative: Vec<u64>,
    candidate: Vec<char>,
    threshold: usize,
}

impl PreparedMyers {
    /// Precompute scalar equality masks for the complete query.
    ///
    /// # Errors
    /// Refuses thresholds above 64 and queries above 2,048 scalars.
    pub fn new(query: &str, threshold: usize) -> Result<Self, TextError> {
        validate_threshold(threshold)?;
        let mut scalars = Vec::new();
        read_scalars(query, &mut scalars)?;
        let length = scalars.len();
        let blocks = length.div_ceil(64);
        let mut alphabet = scalars.clone();
        alphabet.sort_unstable();
        alphabet.dedup();
        let mut masks: Vec<_> = alphabet
            .into_iter()
            .map(|letter| (letter, vec![0_u64; blocks]))
            .collect();
        for (index, letter) in scalars.into_iter().enumerate() {
            let at = masks
                .binary_search_by_key(&letter, |(value, _)| *value)
                .expect("query alphabet contains its scalar");
            masks[at].1[index / 64] |= 1 << (index % 64);
        }
        Ok(Self {
            length,
            masks,
            positive: vec![u64::MAX; blocks],
            negative: vec![0; blocks],
            candidate: Vec::new(),
            threshold,
        })
    }

    /// Compute the exact global distance and retain only results within the bound.
    ///
    /// # Errors
    /// Refuses a candidate longer than 2,048 scalars.
    pub fn distance(&mut self, candidate: &str) -> Result<Option<usize>, TextError> {
        read_scalars(candidate, &mut self.candidate)?;
        if self.length.abs_diff(self.candidate.len()) > self.threshold {
            return Ok(None);
        }
        if self.length == 0 {
            return Ok(Some(self.candidate.len()));
        }
        self.positive.fill(u64::MAX);
        self.negative.fill(0);
        let mut score = self.length;
        let last_bit = 1_u64 << ((self.length - 1) % 64);
        for letter in &self.candidate {
            let mask = self
                .masks
                .binary_search_by_key(letter, |(value, _)| *value)
                .ok()
                .map(|at| self.masks[at].1.as_slice());
            let mut add_carry = false;
            let mut positive_shift = 1;
            let mut negative_shift = 0;
            for block in 0..self.positive.len() {
                let equal = mask.map_or(0, |mask| mask[block]);
                let positive = self.positive[block];
                let negative = self.negative[block];
                let vertical = equal | negative;
                let (sum, carry1) = (equal & positive).overflowing_add(positive);
                let (sum, carry2) = sum.overflowing_add(u64::from(add_carry));
                add_carry = carry1 || carry2;
                let horizontal = (sum ^ positive) | equal;
                let plus = negative | !(horizontal | positive);
                let minus = positive & horizontal;
                if block + 1 == self.positive.len() {
                    score += usize::from(plus & last_bit != 0);
                    score -= usize::from(minus & last_bit != 0);
                }
                let shifted_plus = (plus << 1) | positive_shift;
                let shifted_minus = (minus << 1) | negative_shift;
                positive_shift = plus >> 63;
                negative_shift = minus >> 63;
                self.positive[block] = shifted_minus | !(vertical | shifted_plus);
                self.negative[block] = shifted_plus & vertical;
            }
        }
        Ok((score <= self.threshold).then_some(score))
    }
}
